use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{
    config::{codex_args, Config, Role},
    herdr::{self, field, strings, Context, Control},
    io_error, AgentError, Result,
};

const REPORT_LIMIT: usize = 8000;
const HEADINGS: [&str; 5] = [
    "SUMMARY",
    "RELEVANT FILES",
    "EXECUTION FLOW",
    "IMPORTANT FINDINGS",
    "UNCERTAINTIES",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Handle {
    pub name: String,
    #[serde(default)]
    pub workspace: Option<String>,
    pub pane: String,
    pub terminal: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Delivery {
    Submitting,
    Accepted,
    Uncertain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Task {
    pub id: String,
    pub question: String,
    pub delivery: Delivery,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Data {
    pub format: u8,
    pub repository: PathBuf,
    pub executable: PathBuf,
    pub config: Config,
    pub context: Context,
    pub disabled_mcp: Vec<String>,
    pub brain: Option<Handle>,
    pub researcher: Option<Handle>,
    pub task: Option<Task>,
    pub sequence: u64,
    pub worker_tasks: u32,
    pub stopped: bool,
}

pub(super) struct Session {
    pub directory: PathBuf,
    pub data: Data,
}

impl Session {
    pub fn create(repository: PathBuf, config: Config, context: Context) -> Result<Self> {
        let disabled_mcp = herdr::configured_mcp(&repository)?;
        let directory = tempfile::Builder::new()
            .prefix("ai-profiles-agents-")
            .tempdir()
            .map_err(io_error("Katalog sesji"))?
            .keep();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .map_err(io_error("Uprawnienia sesji"))?;
        let session = Self {
            directory,
            data: Data {
                format: 1,
                repository,
                executable: herdr::executable()?,
                config,
                context,
                disabled_mcp,
                brain: None,
                researcher: None,
                task: None,
                sequence: 0,
                worker_tasks: 0,
                stopped: false,
            },
        };
        session.save()?;
        Ok(session)
    }

    pub fn load(directory: PathBuf) -> Result<Self> {
        let metadata = fs::symlink_metadata(&directory).map_err(io_error("Katalog sesji"))?;
        let owner = fs::metadata("/proc/self")
            .map_err(io_error("UID procesu"))?
            .uid();
        if !metadata.is_dir()
            || metadata.permissions().mode() & 0o077 != 0
            || metadata.uid() != owner
        {
            return Err(AgentError::Input(
                "SESSION musi być prywatnym katalogiem bieżącego użytkownika (0700), bez symlinku",
            ));
        }
        let directory = directory
            .canonicalize()
            .map_err(io_error("Ścieżka sesji"))?;
        let data: Data = read_json(&directory.join("state.json"))?;
        if data.format != 1 || !crate::repository::is_source_root(&data.repository) {
            return Err(AgentError::Input("Nieprawidłowa sesja ai-profiles"));
        }
        Ok(Self { directory, data })
    }

    pub fn save(&self) -> Result<()> {
        atomic_json(&self.directory, "state.json", &self.data)
    }

    fn reload(&mut self) -> Result<()> {
        self.data = read_json(&self.directory.join("state.json"))?;
        Ok(())
    }

    pub fn lock(&self) -> Result<File> {
        let file = open_private(&self.directory.join("worker.lock"), false)?;
        file.try_lock_exclusive().map_err(|error| {
            if error.kind() == std::io::ErrorKind::WouldBlock {
                AgentError::Busy
            } else {
                io_error("Blokada sesji")(error)
            }
        })?;
        Ok(file)
    }

    pub fn start(&mut self, control: &impl Control) -> Result<()> {
        let _lock = self.lock()?;
        if self.data.brain.is_some() {
            return Err(AgentError::Input("Brain już został utworzony"));
        }
        self.launch(control, Role::Brain)
    }

    fn launch(&mut self, control: &impl Control, role: Role) -> Result<()> {
        if role == Role::Researcher {
            // Re-read effective MCP inventory when creating/recreating the worker.
            self.data.disabled_mcp = control.mcp_names()?;
        }
        let suffix = self
            .directory
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("session")
            .trim_start_matches("ai-profiles-agents-");
        let name = format!("aip-{}-{}", role.as_str(), suffix.to_ascii_lowercase());
        let label = format!("ai-profiles {} {}", role.as_str(), suffix);
        let created = control.call(
            &strings(&[
                "workspace",
                "create",
                "--cwd",
                &self.data.repository.to_string_lossy(),
                "--label",
                &label,
                "--no-focus",
            ]),
            Duration::from_secs(10),
        )?;
        let workspace = field(&created, "/workspace/workspace_id")?;
        let pane = field(&created, "/root_pane/pane_id")?;
        let terminal = field(&created, "/root_pane/terminal_id")?;
        let handle = Handle {
            name: name.clone(),
            workspace: Some(workspace),
            pane: pane.clone(),
            terminal,
        };
        match role {
            Role::Brain => self.data.brain = Some(handle),
            Role::Researcher => self.data.researcher = Some(handle),
        }
        self.save()?; // Preserve ownership even if readiness or delivery fails.
        let mut args = strings(&[
            "agent",
            "start",
            &name,
            "--kind",
            "codex",
            "--pane",
            &pane,
            "--timeout",
            "60000",
            "--",
        ]);
        args.extend(codex_args(
            &self.data.config,
            role,
            &self.data.repository,
            &self.directory,
            &self.data.executable,
            &self.data.disabled_mcp,
        ));
        control.call(&args, Duration::from_secs(70))?;
        Ok(())
    }

    pub fn research(&mut self, control: &impl Control, question: &str) -> Result<Value> {
        if question.trim().is_empty() || question.len() > 12000 {
            return Err(AgentError::Input("Pytanie musi mieć 1..12000 bajtów"));
        }
        let _lock = self.lock()?;
        self.reload()?;
        self.ensure_running()?;
        if let Some(task) = &self.data.task {
            if !self.report_path(&task.id).exists() {
                return Err(AgentError::Pending(task.id.clone()));
            }
        }
        if self.data.worker_tasks >= self.data.config.max_tasks {
            self.close_worker(control)?;
        }
        if self.data.researcher.is_none() {
            if self.data.brain.is_none() {
                return Err(AgentError::Input("Brak panelu Brain"));
            }
            self.launch(control, Role::Researcher)?;
        }
        let worker = self
            .data
            .researcher
            .as_ref()
            .expect("launched worker")
            .clone();
        self.ready(control, &worker)?;
        self.data.sequence += 1;
        let id = format!("task-{}", self.data.sequence);
        self.data.task = Some(Task {
            id: id.clone(),
            question: question.to_owned(),
            delivery: Delivery::Submitting,
        });
        self.data.worker_tasks += 1;
        self.save()?;
        let prompt = format!("Research the following JSON task data. Submit your completed report using submit_report; do not modify files.\n{}", json!({"task_id":id,"question":question}));
        let milliseconds = (self.data.config.timeout_seconds * 1000).to_string();
        let result = control.call(
            &strings(&[
                "agent",
                "prompt",
                &worker.name,
                &prompt,
                "--wait",
                "--timeout",
                &milliseconds,
            ]),
            Duration::from_secs(self.data.config.timeout_seconds + 5),
        );
        self.data.task.as_mut().expect("saved task").delivery = if result.is_ok() {
            Delivery::Accepted
        } else {
            Delivery::Uncertain
        };
        self.save()?;
        // A successfully submitted report remains recoverable even when the terminal wait failed.
        if self.report_path(&id).exists() {
            return self.report(&id);
        }
        let reason = match result {
            Ok(value) => format!(
                "agent zakończył oczekiwanie bez raportu ({})",
                value
                    .pointer("/agent/agent_status")
                    .and_then(Value::as_str)
                    .unwrap_or("sprawdź agent-status/panel")
            ),
            Err(error) => error.to_string(),
        };
        Err(AgentError::Delivery {
            task_id: id,
            reason,
        })
    }

    fn ensure_running(&self) -> Result<()> {
        if self.data.stopped {
            Err(AgentError::Input("Sesja została zatrzymana"))
        } else {
            Ok(())
        }
    }

    fn ready(&self, control: &impl Control, handle: &Handle) -> Result<()> {
        let value = control.call(
            &strings(&["agent", "get", &handle.name]),
            Duration::from_secs(10),
        )?;
        if field(&value, "/agent/terminal_id")? != handle.terminal
            || field(&value, "/agent/pane_id")? != handle.pane
        {
            return Err(AgentError::Input(
                "Tożsamość procesu Herdr zmieniła się; nie wysłano zadania",
            ));
        }
        match field(&value, "/agent/agent_status")?.as_str() {
            "idle" | "done" => Ok(()),
            _ => Err(AgentError::Input("Researcher nie jest gotowy (working/blocked/unknown); sprawdź panel, nie ponawiaj zadania")),
        }
    }

    fn report_path(&self, id: &str) -> PathBuf {
        self.directory.join(format!("{id}.report.json"))
    }

    fn report(&self, id: &str) -> Result<Value> {
        let report: Value = read_json(&self.report_path(id))?;
        if report["task_id"] != id {
            return Err(AgentError::Input("Raport dotyczy innego zadania"));
        }
        validate_report(
            report["report"]
                .as_str()
                .ok_or(AgentError::Input("Brak treści raportu"))?,
        )?;
        Ok(report)
    }

    pub fn submit_report(&mut self, task_id: &str, report: &str) -> Result<Value> {
        self.reload()?;
        self.ensure_running()?;
        validate_report(report)?;
        let task = self
            .data
            .task
            .as_ref()
            .ok_or(AgentError::Input("Brak aktywnego zadania"))?;
        if task.id != task_id {
            return Err(AgentError::Input("Nieaktualny lub obcy task_id"));
        }
        let data =
            json!({"task_id":task_id,"report":report,"researcher": self.data.config.researcher});
        // Separate report storage avoids a lock cycle with research waiting for the worker.
        // persist_noclobber publishes a complete file once; duplicate submissions never overwrite.
        let mut file = tempfile::NamedTempFile::new_in(&self.directory)
            .map_err(io_error("Raport tymczasowy"))?;
        file.write_all(serde_json::to_string(&data)?.as_bytes())
            .map_err(io_error("Zapis raportu"))?;
        file.as_file()
            .sync_all()
            .map_err(io_error("Synchronizacja raportu"))?;
        file.persist_noclobber(self.report_path(task_id))
            .map_err(|error| {
                io_error("Raport już istnieje lub nie można go zapisać")(error.error)
            })?;
        Ok(json!({"submitted": task_id}))
    }

    pub fn research_status(&mut self, control: &impl Control) -> Result<Value> {
        self.reload()?;
        if let Some(task) = &self.data.task {
            if self.report_path(&task.id).exists() {
                return self.report(&task.id);
            }
        }
        self.status(control)
    }

    pub fn status(&self, control: &impl Control) -> Result<Value> {
        let inspect = |handle: &Option<Handle>| match handle {
            None => json!({"state":"not_started"}),
            Some(handle) => match control.call(
                &strings(&["agent", "get", &handle.name]),
                Duration::from_secs(10),
            ) {
                Ok(value) => {
                    json!({"name":handle.name,"workspace":handle.workspace,"pane":handle.pane,"agent":value.get("agent")})
                }
                Err(error) => {
                    json!({"name":handle.name,"workspace":handle.workspace,"pane":handle.pane,"error":error.to_string()})
                }
            },
        };
        Ok(
            json!({"session":self.directory,"repository":self.data.repository,"config":self.data.config,
            "stopped":self.data.stopped,"brain":inspect(&self.data.brain),"researcher":inspect(&self.data.researcher),
            "task":self.data.task.as_ref().map(|t| json!({"id":t.id,"delivery":t.delivery,"report_ready":self.report_path(&t.id).exists()})),
            "worker_tasks":self.data.worker_tasks}),
        )
    }

    pub fn reset_worker(&mut self, control: &impl Control) -> Result<Value> {
        let _lock = self.lock()?;
        self.reload()?;
        self.ensure_running()?;
        self.close_worker(control)?;
        self.data.task = None;
        self.save()?;
        Ok(json!({"reset":true}))
    }

    fn close_worker(&mut self, control: &impl Control) -> Result<()> {
        if let Some(worker) = &self.data.researcher {
            self.close_owned(control, worker)?;
        }
        self.data.researcher = None;
        self.data.worker_tasks = 0;
        self.save()
    }

    fn close_owned(&self, control: &impl Control, handle: &Handle) -> Result<()> {
        let pane = control.call(
            &strings(&["pane", "get", &handle.pane]),
            Duration::from_secs(10),
        )?;
        if field(&pane, "/pane/terminal_id")? != handle.terminal {
            return Err(AgentError::Input(
                "Panel nie należy już do tej sesji; nie został zamknięty",
            ));
        }
        control.call(
            &strings(&["pane", "close", &handle.pane]),
            Duration::from_secs(10),
        )?;
        Ok(())
    }

    pub fn stop(&mut self, control: &impl Control) -> Result<()> {
        let _lock = self.lock()?;
        self.reload()?;
        self.close_worker(control)?;
        if let Some(brain) = &self.data.brain {
            self.close_owned(control, brain)?;
        }
        self.data.brain = None;
        self.data.stopped = true;
        self.save()
    }
}

fn validate_report(report: &str) -> Result<()> {
    if report.len() > REPORT_LIMIT || report.trim().is_empty() {
        return Err(AgentError::Input("Raport musi mieć 1..8000 bajtów UTF-8"));
    }
    let lines: Vec<_> = report.lines().collect();
    let mut cursor = 0;
    let mut positions = Vec::new();
    for heading in HEADINGS {
        let Some(position) = lines[cursor..]
            .iter()
            .position(|line| line.trim().trim_matches('#').trim() == heading)
        else {
            return Err(AgentError::Input("Raport wymaga kolejno nagłówków SUMMARY, RELEVANT FILES, EXECUTION FLOW, IMPORTANT FINDINGS, UNCERTAINTIES"));
        };
        cursor += position;
        positions.push(cursor);
        cursor += 1;
    }
    positions.push(lines.len());
    if positions.windows(2).any(|pair| {
        lines[pair[0] + 1..pair[1]]
            .iter()
            .all(|line| line.trim().is_empty())
    }) {
        return Err(AgentError::Input(
            "Każda sekcja raportu wymaga treści; niepewność opisz jawnie",
        ));
    }
    Ok(())
}

fn open_private(path: &Path, exclusive: bool) -> Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
    if exclusive {
        options.create_new(true);
    } else {
        options.create(true).truncate(false);
    }
    options.open(path).map_err(io_error("Prywatny plik sesji"))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(io_error("Odczyt sesji"))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error("Odczyt sesji"))?;
    if bytes.len() > 1024 * 1024 {
        return Err(AgentError::Input("Zbyt duży plik sesji"));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn atomic_json(directory: &Path, name: &str, data: &impl Serialize) -> Result<()> {
    let mut file =
        tempfile::NamedTempFile::new_in(directory).map_err(io_error("Stan tymczasowy"))?;
    file.write_all(&serde_json::to_vec_pretty(data)?)
        .map_err(io_error("Zapis stanu"))?;
    file.as_file()
        .sync_all()
        .map_err(io_error("Synchronizacja stanu"))?;
    file.persist(directory.join(name))
        .map_err(|error| io_error("Publikacja stanu")(error.error))?;
    Ok(())
}
