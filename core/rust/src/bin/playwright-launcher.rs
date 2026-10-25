fn main() -> std::process::ExitCode {
    ai_profiles_core::logging::runtime_entry(
        "playwright-launcher",
        ai_profiles_core::playwright::runtime::main,
    )
}
