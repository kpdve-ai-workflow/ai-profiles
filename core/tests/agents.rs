//! Oracles: explicit opt-in, separate models, read-only worker, one in-flight job,
//! no duplicate delivery, private final reports, and owned-process cleanup.
//! Herdr is mocked only at its process boundary; the live integration is separate.
use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

use serde_json::{json, Value};

use super::{
    config::{codex_args, Config, Role},
    herdr::{Context, Control},
    mcp,
    session::{Data, Delivery, Handle, Session, Task},
    AgentError, Result,
};

const REPORT: &str = "SUMMARY\nThe public entry point is Makefile.\nRELEVANT FILES\nMakefile:1 — public commands.\nEXECUTION FLOW\nMake includes command categories.\nIMPORTANT FINDINGS\nAgent startup requires an explicit command.\nUNCERTAINTIES\nNo external services were inspected.";

#[derive(Clone, Copy)]
enum PromptOutcome {
    Report,
    Timeout,
    MissingReport,
}

struct FakeHerdr {
    directory: PathBuf,
    calls: RefCell<Vec<Vec<String>>>,
    handles: RefCell<HashMap<String, (String, String)>>,
    outcome: RefCell<PromptOutcome>,
    state: RefCell<String>,
    replaced_terminal: RefCell<bool>,
}

impl Control for FakeHerdr {
    fn mcp_names(&self) -> Result<Vec<String>> {
        Ok(vec!["production_database".into()])
    }

    fn call(&self, args: &[String], _timeout: Duration) -> Result<Value> {
        self.calls.borrow_mut().push(args.to_vec());
        match (args[0].as_str(), args[1].as_str()) {
            ("pane", "layout") => Ok(json!({"layout":{"area":{"width":120,"height":30}}})),
            ("pane", "split") => {
                let n = self
                    .calls
                    .borrow()
                    .iter()
                    .filter(|a| a[0] == "pane" && a[1] == "split")
                    .count();
                Ok(json!({"pane":{"pane_id":format!("w1:p{n}"),"terminal_id":format!("term{n}")}}))
            }
            ("agent", "start") => {
                // Herdr's documented alias grammar; tempfile suffixes can contain capitals.
                assert!(regex::Regex::new(r"^[a-z][a-z0-9_-]{0,31}$")
                    .unwrap()
                    .is_match(&args[2]));
                let pane_index = args.iter().position(|v| v == "--pane").unwrap() + 1;
                let pane = args[pane_index].clone();
                let terminal = format!(
                    "term{}",
                    pane.rsplit(':').next().unwrap().trim_start_matches('p')
                );
                self.handles
                    .borrow_mut()
                    .insert(args[2].clone(), (pane, terminal));
                Ok(json!({}))
            }
            ("agent", "get") => {
                let handles = self.handles.borrow();
                let (pane, terminal) = handles
                    .get(&args[2])
                    .ok_or_else(|| AgentError::Herdr("agent_not_running".into()))?;
                Ok(
                    json!({"agent":{"pane_id":pane,"terminal_id":terminal,"agent_status":*self.state.borrow()}}),
                )
            }
            ("agent", "prompt") => match *self.outcome.borrow() {
                PromptOutcome::Report => {
                    let task: Value =
                        serde_json::from_str(args[3].split_once('\n').unwrap().1).unwrap();
                    Session::load(self.directory.clone())?
                        .submit_report(task["task_id"].as_str().unwrap(), REPORT)?;
                    Ok(json!({"agent":{"agent_status":"done"}}))
                }
                PromptOutcome::Timeout => {
                    Err(AgentError::Herdr("timeout after accepted bytes".into()))
                }
                PromptOutcome::MissingReport => Ok(json!({"agent":{"agent_status":"blocked"}})),
            },
            ("pane", "get") => {
                let terminal = if *self.replaced_terminal.borrow() {
                    "someone_else".into()
                } else {
                    format!(
                        "term{}",
                        args[2].rsplit(':').next().unwrap().trim_start_matches('p')
                    )
                };
                Ok(json!({"pane":{"terminal_id":terminal}}))
            }
            ("pane", "close") => Ok(json!({})),
            _ => panic!("Unexpected Herdr operation: {args:?}"),
        }
    }
}

struct Fixture {
    _root: tempfile::TempDir,
    _private: tempfile::TempDir,
    session: Session,
    control: FakeHerdr,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    for folder in ["core/src", "profiles", "skills", "mcp/servers"] {
        fs::create_dir_all(root.path().join(folder)).unwrap();
    }
    fs::write(root.path().join("Makefile"), "# fixture entry point\n").unwrap();
    let private = tempfile::Builder::new()
        .prefix("ai-profiles-agents-")
        .tempdir()
        .unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let directory = private.path().to_owned();
    let session = Session {
        directory: directory.clone(),
        data: Data {
            format: 1,
            repository: root.path().to_owned(),
            executable: PathBuf::from("/fixture/ai-profile-agent"),
            config: Config::parse(|_, default| default.to_owned()).unwrap(),
            context: Context {
                environment: HashMap::new(),
                caller_pane: "w0:p0".into(),
            },
            disabled_mcp: vec![],
            brain: Some(Handle {
                name: "fixture-brain".into(),
                pane: "w0:p0".into(),
                terminal: "term0".into(),
            }),
            researcher: None,
            task: None,
            sequence: 0,
            worker_tasks: 0,
            stopped: false,
        },
    };
    session.save().unwrap();
    Fixture {
        _root: root,
        _private: private,
        session,
        control: FakeHerdr {
            directory,
            calls: RefCell::new(vec![]),
            handles: RefCell::new(HashMap::new()),
            outcome: RefCell::new(PromptOutcome::Report),
            state: RefCell::new("idle".into()),
            replaced_terminal: RefCell::new(false),
        },
    }
}

fn count(control: &FakeHerdr, group: &str, command: &str) -> usize {
    control
        .calls
        .borrow()
        .iter()
        .filter(|args| args[0] == group && args[1] == command)
        .count()
}

#[test]
fn lazy_research_reuses_one_worker_and_returns_only_report() {
    let mut f = fixture();
    assert!(f.session.data.researcher.is_none());
    let first = f
        .session
        .research(&f.control, "Find the entry point")
        .unwrap();
    let second = f
        .session
        .research(&f.control, "How are categories included?")
        .unwrap();
    assert_eq!(first["report"], REPORT);
    assert_eq!(second["report"], REPORT);
    assert_ne!(first["task_id"], second["task_id"]);
    assert_eq!(count(&f.control, "agent", "start"), 1);
    assert_eq!(count(&f.control, "agent", "prompt"), 2);
    assert_eq!(count(&f.control, "agent", "read"), 0);
    assert_eq!(
        fs::read_to_string(f._root.path().join("Makefile")).unwrap(),
        "# fixture entry point\n"
    );
    assert_eq!(f.session.data.disabled_mcp, ["production_database"]);
}

#[test]
fn timeout_is_not_retried_and_late_report_can_be_recovered() {
    let mut f = fixture();
    *f.control.outcome.borrow_mut() = PromptOutcome::Timeout;
    assert!(matches!(
        f.session.research(&f.control, "Find auth"),
        Err(AgentError::Delivery { .. })
    ));
    assert!(matches!(
        f.session.data.task.as_ref().unwrap().delivery,
        Delivery::Uncertain
    ));
    assert!(matches!(
        f.session.research(&f.control, "Find auth again"),
        Err(AgentError::Pending(_))
    ));
    assert_eq!(count(&f.control, "agent", "prompt"), 1);
    let id = f.session.data.task.as_ref().unwrap().id.clone();
    // Another MCP process submits the delayed report while the worker lock is held.
    let lock = f.session.lock().unwrap();
    Session::load(f.session.directory.clone())
        .unwrap()
        .submit_report(&id, REPORT)
        .unwrap();
    assert_eq!(
        f.session.research_status(&f.control).unwrap()["report"],
        REPORT
    );
    drop(lock);
    assert_eq!(count(&f.control, "agent", "prompt"), 1);
}

#[test]
fn settled_without_report_does_not_count_as_success() {
    let mut f = fixture();
    *f.control.outcome.borrow_mut() = PromptOutcome::MissingReport;
    assert!(matches!(
        f.session.research(&f.control, "Find auth"),
        Err(AgentError::Delivery { .. })
    ));
    let status = f.session.research_status(&f.control).unwrap();
    assert_eq!(status["task"]["report_ready"], false);
}

#[test]
fn concurrent_request_does_not_submit_or_spawn() {
    let mut f = fixture();
    let _lock = f.session.lock().unwrap();
    assert!(matches!(
        f.session.research(&f.control, "Find auth"),
        Err(AgentError::Busy)
    ));
    assert!(f.control.calls.borrow().is_empty());
}

#[test]
fn blocked_or_missing_worker_does_not_spawn_a_duplicate() {
    let mut f = fixture();
    f.session.research(&f.control, "Initial task").unwrap();
    *f.control.state.borrow_mut() = "blocked".into();
    assert!(f.session.research(&f.control, "Second task").is_err());
    f.control.handles.borrow_mut().clear();
    assert!(f.session.research(&f.control, "Third task").is_err());
    assert_eq!(count(&f.control, "agent", "start"), 1);
    assert_eq!(count(&f.control, "agent", "prompt"), 1);
}

#[test]
fn context_rotates_only_after_completed_task_limit() {
    let mut f = fixture();
    f.session.data.config.max_tasks = 1;
    f.session.save().unwrap();
    f.session.research(&f.control, "First").unwrap();
    f.session.research(&f.control, "Second").unwrap();
    assert_eq!(count(&f.control, "agent", "start"), 2);
    assert_eq!(count(&f.control, "pane", "close"), 1);
    assert_eq!(f.session.data.worker_tasks, 1);
}

#[test]
fn reports_reject_foreign_ids_incomplete_sections_and_utf8_overflow() {
    let mut f = fixture();
    f.session.data.task = Some(Task {
        id: "task-1".into(),
        question: "entry point".into(),
        delivery: Delivery::Submitting,
    });
    f.session.save().unwrap();
    assert!(f.session.submit_report("../outside", REPORT).is_err());
    assert!(f
        .session
        .submit_report("task-1", "SUMMARY\nUnknown")
        .is_err());
    assert!(f
        .session
        .submit_report(
            "task-1",
            "SUMMARY\nRELEVANT FILES\nEXECUTION FLOW\nIMPORTANT FINDINGS\nUNCERTAINTIES"
        )
        .is_err());
    assert!(f
        .session
        .submit_report("task-1", &format!("{REPORT}{}", "ż".repeat(4000)))
        .is_err());
    assert!(!f.session.directory.join("task-1.report.json").exists());
    f.session.submit_report("task-1", REPORT).unwrap();
    assert!(f
        .session
        .submit_report("task-1", &REPORT.replace("Makefile", "Otherfile"))
        .is_err());
    assert_eq!(
        f.session.research_status(&f.control).unwrap()["report"],
        REPORT
    );
    let metadata = fs::metadata(f.session.directory.join("task-1.report.json")).unwrap();
    assert_eq!(metadata.permissions().mode() & 0o077, 0);
}

#[test]
fn independent_sessions_cannot_receive_each_others_reports() {
    let mut a = fixture();
    let mut b = fixture();
    a.session.research(&a.control, "A").unwrap();
    assert!(b.session.submit_report("task-1", REPORT).is_err());
    assert!(b.session.data.task.is_none());
}

#[test]
fn reset_retains_report_and_never_closes_replaced_terminal() {
    let mut f = fixture();
    f.session.research(&f.control, "First").unwrap();
    *f.control.replaced_terminal.borrow_mut() = true;
    assert!(f.session.reset_worker(&f.control).is_err());
    assert_eq!(count(&f.control, "pane", "close"), 0);
    *f.control.replaced_terminal.borrow_mut() = false;
    f.session.reset_worker(&f.control).unwrap();
    assert!(f.session.data.task.is_none());
    assert!(f.session.directory.join("task-1.report.json").exists());
    assert!(f.session.data.researcher.is_none());
}

#[test]
fn codex_instances_get_independent_models_and_worker_cannot_escalate() {
    let config = Config::parse(|_, default| default.to_owned()).unwrap();
    let brain = codex_args(
        &config,
        Role::Brain,
        Path::new("/repo with spaces"),
        Path::new("/private"),
        Path::new("/launcher"),
        &[],
    );
    let worker = codex_args(
        &config,
        Role::Researcher,
        Path::new("/repo with spaces"),
        Path::new("/private"),
        Path::new("/launcher"),
        &["write_db".into()],
    );
    let has_pair =
        |args: &[String], key: &str, value: &str| args.windows(2).any(|pair| pair == [key, value]);
    assert!(has_pair(&brain, "--model", "gpt-6.1-sol"));
    assert!(brain
        .iter()
        .any(|v| v == "model_reasoning_effort=\"xhigh\""));
    assert!(has_pair(&worker, "--model", "gpt-6-luna"));
    assert!(worker
        .iter()
        .any(|v| v == "model_reasoning_effort=\"medium\""));
    assert!(has_pair(&worker, "--sandbox", "read-only"));
    assert!(has_pair(&worker, "--ask-for-approval", "never"));
    assert!(worker
        .iter()
        .any(|v| v == "mcp_servers.\"write_db\".enabled=false"));
    for args in [&brain, &worker] {
        assert!(args.iter().any(|v| v == "features.multi_agent=false"));
        assert!(!args.iter().any(|v| v.contains("dangerously")));
        assert!(has_pair(args, "--cd", "/repo with spaces"));
        for pair in args.windows(2).filter(|p| p[0] == "--config") {
            pair[1]
                .parse::<toml_edit::DocumentMut>()
                .unwrap_or_else(|error| panic!("Invalid TOML {}: {error}", pair[1]));
        }
    }
}

#[test]
fn model_changes_are_explicit_and_invalid_limits_fail_before_launch() {
    let custom = Config::parse(|name, default| match name {
        "AI_AGENT_BRAIN_MODEL" => "custom-brain".into(),
        "AI_AGENT_RESEARCHER_THINKING" => "low".into(),
        _ => default.into(),
    })
    .unwrap();
    assert_eq!(
        serde_json::to_value(custom).unwrap()["brain"]["model"],
        "custom-brain"
    );
    for (name, value) in [
        ("AI_AGENT_RESEARCH_TIMEOUT", "0"),
        ("AI_AGENT_RESEARCHER_MAX_TASKS", "101"),
        ("AI_AGENT_BRAIN_THINKING", "ultra"),
        ("AI_AGENT_BRAIN_MODEL", "$(touch /tmp/unsafe)"),
    ] {
        assert!(Config::parse(|key, default| if key == name {
            value.into()
        } else {
            default.into()
        })
        .is_err());
    }
}

#[test]
fn mcp_roles_do_not_expose_each_others_capabilities() {
    let names = |role| {
        mcp::tools(role)
            .iter()
            .map(|v| v["name"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(Role::Brain),
        ["research", "research_status", "reset_researcher"]
    );
    assert_eq!(names(Role::Researcher), ["submit_report"]);
}

#[test]
fn public_or_symlinked_session_storage_is_rejected() {
    let f = fixture();
    fs::set_permissions(&f.session.directory, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Session::load(f.session.directory.clone()).is_err());
    fs::set_permissions(&f.session.directory, fs::Permissions::from_mode(0o700)).unwrap();
    let link = f._root.path().join("session-link");
    std::os::unix::fs::symlink(&f.session.directory, &link).unwrap();
    assert!(Session::load(link).is_err());
}
