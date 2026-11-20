fn main() -> std::process::ExitCode {
    ai_profiles_core::logging::runtime_entry(
        "restore-updraft",
        ai_profiles_core::wordpress::restore::cli_main,
    )
}
