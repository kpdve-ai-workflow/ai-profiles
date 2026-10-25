use std::process::ExitCode;

#[path = "ai-profile/invocation.rs"]
mod invocation;

fn main() -> ExitCode {
    // Scope the synchronous subscriber around the entire invocation, including
    // parsing/help, and drop it before returning the process exit status.
    let invoke = || {
        tracing::info!(event = "cli.started");
        invocation::record();
        let code = ai_profiles_core::profile_management::cli::main();
        tracing::info!(event = "cli.finished", exit_code = code);
        ExitCode::from(code as u8)
    };
    match ai_profiles_core::profile_management::cli::diagnostic_subscriber() {
        Ok(subscriber) => tracing::subscriber::with_default(subscriber, invoke),
        Err(error) => {
            eprintln!("WARNING: diagnostics unavailable: {error}");
            invoke()
        }
    }
}
