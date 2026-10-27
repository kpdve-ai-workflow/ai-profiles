use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use super::{config::Role, herdr::Herdr, io_error, session::Session, AgentError, Result};

pub(super) fn serve(mut session: Session, role: Role) -> Result<()> {
    let control = Herdr::for_session(&session);
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    loop {
        let mut line = String::new();
        // Bound a single JSON-RPC request; avoid unbounded buffering from a malformed client.
        let read = std::io::Read::take(&mut input, 65537)
            .read_line(&mut line)
            .map_err(io_error("Odczyt MCP"))?;
        if read == 0 {
            return Ok(());
        }
        if line.len() > 65536 {
            return Err(AgentError::Input("Żądanie MCP przekracza 64 KiB"));
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => {
                write_response(
                    &mut output,
                    &json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}),
                )?;
                continue;
            }
        };
        let Some(id) = request.get("id") else {
            continue;
        };
        let result = match request["method"].as_str() {
            Some("initialize") => Ok(
                json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"ai-profiles-session","version":"0.1.0"}}),
            ),
            Some("ping") => Ok(json!({})),
            Some("tools/list") => Ok(json!({"tools": tools(role)})),
            Some("tools/call") => {
                let params = &request["params"];
                let args = &params["arguments"];
                let called = match (role, params["name"].as_str()) {
                    (Role::Brain, Some("research")) => text_arg(args, "question")
                        .and_then(|question| session.research(&control, question)),
                    (Role::Brain, Some("research_status")) => session.research_status(&control),
                    (Role::Brain, Some("reset_researcher")) if args["confirm_cancel"] == true => {
                        session.reset_worker(&control)
                    }
                    (Role::Researcher, Some("submit_report")) => text_arg(args, "task_id")
                        .and_then(|task| {
                            text_arg(args, "report")
                                .and_then(|report| session.submit_report(task, report))
                        }),
                    _ => Err(AgentError::Input(
                        "Niedozwolone narzędzie lub brak confirm_cancel=true",
                    )),
                };
                Ok(match called {
                    Ok(value) => {
                        json!({"content":[{"type":"text","text":serde_json::to_string(&value)?}],"isError":false})
                    }
                    Err(error) => {
                        json!({"content":[{"type":"text","text":error.to_string()}],"isError":true})
                    }
                })
            }
            _ => Err(AgentError::Input("Method not found")),
        };
        let response = match result {
            Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":value}),
            Err(error) => {
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":error.to_string()}})
            }
        };
        write_response(&mut output, &response)?;
    }
}

fn text_arg<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or(AgentError::Input(
            "Brak wymaganego argumentu tekstowego MCP",
        ))
}

fn write_response(output: &mut impl Write, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n").map_err(io_error("Zapis MCP"))?;
    output.flush().map_err(io_error("Flush MCP"))
}

pub(super) fn tools(role: Role) -> Vec<Value> {
    let tool = |name: &str, description: &str, properties: Value, required: Vec<&str>| {
        json!({"name":name,"description":description,
        "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
        "annotations":{"destructiveHint":false,"openWorldHint":false}})
    };
    match role {
        Role::Brain => vec![
            tool("research", "Delegate repository discovery to the separate read-only Codex Researcher in Herdr. Returns only its compact report. Never retry automatically after an error; call research_status.", json!({"question":{"type":"string","minLength":1,"maxLength":12000}}),vec!["question"]),
            tool("research_status", "Recover a submitted report or inspect the pending task without sending it again.",json!({}),vec![]),
            tool("reset_researcher", "Cancel and close this session's Researcher. Only after the user explicitly requests cancellation/reset. Retains earlier reports.",json!({"confirm_cancel":{"type":"boolean","const":true}}),vec!["confirm_cancel"]),
        ],
        Role::Researcher => vec![tool("submit_report", "Submit the completed evidence report for the current task. Writes only private session storage outside the repository; never overwrites a prior report.",
            json!({"task_id":{"type":"string"},"report":{"type":"string","maxLength":8000}}),vec!["task_id","report"])],
    }
}
