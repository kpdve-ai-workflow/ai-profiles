use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(ai_profiles_core::agents::main())
}
