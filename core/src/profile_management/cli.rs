use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::{
    env,
    ffi::{OsStr, OsString},
    path::PathBuf,
};

use crate::{
    model::{ClientSelection, ProfileAction, RepositoryContext, VersionBump},
    profile_management::{
        migration::migrate_legacy,
        profiles::{bump_profile, create_profile, rename_profile},
        validation::validate_sources,
    },
    project::selected_project,
    repository::profile_names,
};

#[cfg(test)]
#[path = "../../tests/cli.rs"]
mod cli_tests;

const COMMAND_HELP: &str = include_str!("../../help.txt");

struct Options {
    client: Option<ClientSelection>,
    action: Option<ProfileAction>,
    profile: Option<String>,
    project: Option<PathBuf>,
    accept_existing: bool,
    part: Option<VersionBump>,
    description: Option<String>,
    new_name: Option<String>,
}

enum CliFailure {
    Parse(String),
    Operation(String),
}

pub fn main() -> i32 {
    let mut args = env::args_os();
    let _program = args.next();
    let parsed = match parse_args(args.collect()) {
        Ok(ParseResult::Help) => {
            print!("{COMMAND_HELP}");
            return 0;
        }
        Ok(ParseResult::Options(options)) => options,
        Err(message) => return parser_error(&message),
    };

    if let Err(message) = validate_option_combinations(&parsed) {
        return parser_error(&message);
    }
    if matches!(parsed.action, None | Some(ProfileAction::Help)) {
        print!("{COMMAND_HELP}");
        return 0;
    }
    match run(parsed) {
        Ok(()) => 0,
        Err(CliFailure::Parse(message)) => parser_error(&message),
        Err(CliFailure::Operation(message)) => {
            tracing::error!(event = "cli.failed", error_kind = "operation", message = %message);
            eprintln!("ERROR: {message}");
            1
        }
    }
}

enum ParseResult {
    Help,
    Options(Options),
}

fn parse_args(args: Vec<OsString>) -> Result<ParseResult, String> {
    let mut action = None;
    let mut client = None;
    let mut profile = None;
    let mut positional_count = 0;
    let mut project = None;
    let mut accept_existing = false;
    let mut part = None;
    let mut description = None;
    let mut new_name = None;
    let mut positional_only = false;
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if !positional_only && argument == OsStr::new("--project") {
            let value = option_os_value(&args, &mut index, "--project")?;
            project = Some(if value.is_empty() {
                PathBuf::from(".")
            } else {
                PathBuf::from(value)
            });
            continue;
        }
        if !positional_only {
            if let Some(value) = attached_os_value(argument, b"--project=") {
                project = Some(if value.is_empty() {
                    PathBuf::from(".")
                } else {
                    PathBuf::from(value)
                });
                index += 1;
                continue;
            }
        }
        let Some(text) = argument.to_str() else {
            return Err("argument must be valid UTF-8".to_owned());
        };
        if !positional_only && matches!(text, "-h" | "--help") {
            return Ok(ParseResult::Help);
        }
        if !positional_only && text == "--" {
            positional_only = true;
            index += 1;
            continue;
        }
        if !positional_only && text == "--accept-existing" {
            accept_existing = true;
            index += 1;
            continue;
        }
        if !positional_only && (text == "--part" || text.starts_with("--part=")) {
            let value = option_value(&args, &mut index, "--part")?;
            part = Some(match value.as_str() {
                "major" => VersionBump::Major,
                "minor" => VersionBump::Minor,
                "patch" => VersionBump::Patch,
                _ => return Err(format!("argument --part: invalid choice: '{value}' (choose from 'major', 'minor', 'patch')")),
            });
            continue;
        }
        if !positional_only && (text == "--description" || text.starts_with("--description=")) {
            description = Some(option_value(&args, &mut index, "--description")?);
            continue;
        }
        if !positional_only && (text == "--client" || text.starts_with("--client=")) {
            if client.is_some() {
                return Err("--client podano więcej niż raz".into());
            }
            client = Some(ClientSelection::parse(&option_value(
                &args, &mut index, "--client",
            )?)?);
            continue;
        }
        if !positional_only && (text == "--new-name" || text.starts_with("--new-name=")) {
            new_name = Some(option_value(&args, &mut index, "--new-name")?);
            continue;
        }
        if !positional_only && text.starts_with('-') {
            return Err(format!("unrecognized arguments: {text}"));
        }
        match positional_count {
            0 => action = Some(parse_action(text)?),
            1 => profile = Some(text.to_owned()),
            _ => return Err(format!("unrecognized arguments: {text}")),
        }
        positional_count += 1;
        index += 1;
    }

    Ok(ParseResult::Options(Options {
        client,
        action,
        profile,
        project,
        accept_existing,
        part,
        description,
        new_name,
    }))
}

fn option_value(args: &[OsString], index: &mut usize, option: &str) -> Result<String, String> {
    let text = args[*index]
        .to_str()
        .ok_or_else(|| format!("argument {option}: expected a UTF-8 value"))?;
    if let Some((_, value)) = text.split_once('=') {
        *index += 1;
        return Ok(value.to_owned());
    }
    let next = args
        .get(*index + 1)
        .and_then(|value| value.to_str())
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| format!("argument {option}: expected one argument"))?;
    *index += 2;
    Ok(next.to_owned())
}

fn option_os_value(args: &[OsString], index: &mut usize, option: &str) -> Result<OsString, String> {
    let next = args
        .get(*index + 1)
        .filter(|value| !value.as_bytes().starts_with(b"-"))
        .ok_or_else(|| format!("argument {option}: expected one argument"))?;
    *index += 2;
    Ok(next.clone())
}

fn attached_os_value(argument: &OsStr, prefix: &[u8]) -> Option<OsString> {
    argument
        .as_bytes()
        .strip_prefix(prefix)
        .map(|value| OsString::from_vec(value.to_vec()))
}

fn parse_action(value: &str) -> Result<ProfileAction, String> {
    match value {
        "create" => Ok(ProfileAction::Create),
        "rename" => Ok(ProfileAction::Rename),
        "help" => Ok(ProfileAction::Help),
        "install" => Ok(ProfileAction::Install),
        "update" => Ok(ProfileAction::Update),
        "uninstall" => Ok(ProfileAction::Uninstall),
        "switch" => Ok(ProfileAction::Switch),
        "status" => Ok(ProfileAction::Status),
        "validate" => Ok(ProfileAction::Validate),
        "migrate-legacy" => Ok(ProfileAction::MigrateLegacy),
        "bump" => Ok(ProfileAction::Bump),
        _ => Err(format!("argument action: invalid choice: '{value}'")),
    }
}

fn validate_option_combinations(options: &Options) -> Result<(), String> {
    let action = options.action;
    let lifecycle = matches!(
        action,
        Some(
            ProfileAction::Install
                | ProfileAction::Update
                | ProfileAction::Switch
                | ProfileAction::Uninstall
        )
    );
    if lifecycle && options.client.is_none() {
        return Err("Wymagane --client opencode|claude|codex|all".into());
    }
    if options.client.is_some() && !lifecycle && action != Some(ProfileAction::Status) {
        return Err(
            "--client jest dostępne tylko dla install, update, switch, uninstall i status".into(),
        );
    }
    if options.new_name.is_some() && action != Some(ProfileAction::Rename) {
        return Err("--new-name jest dostępne tylko dla rename".to_owned());
    }
    if action == Some(ProfileAction::Rename)
        && options.new_name.as_deref().is_none_or(str::is_empty)
    {
        return Err("rename wymaga --new-name NOWA_NAZWA".to_owned());
    }
    if options.part.is_some() && action != Some(ProfileAction::Bump) {
        return Err("--part jest dostępne tylko dla bump".to_owned());
    }
    if options.description.is_some() && action != Some(ProfileAction::Create) {
        return Err("--description jest dostępne tylko dla create".to_owned());
    }
    if options.profile.is_some()
        && !matches!(
            action,
            Some(
                ProfileAction::Create
                    | ProfileAction::Rename
                    | ProfileAction::Install
                    | ProfileAction::Switch
                    | ProfileAction::Bump
                    | ProfileAction::Validate
            )
        )
    {
        return Err("Ta komenda nie przyjmuje nazwy profilu".to_owned());
    }
    if options.project.is_some()
        && matches!(
            action,
            None | Some(
                ProfileAction::Help
                    | ProfileAction::Create
                    | ProfileAction::Rename
                    | ProfileAction::Bump
                    | ProfileAction::Validate
            )
        )
    {
        return Err("Ta komenda nie używa --project".to_owned());
    }
    if options.accept_existing && action != Some(ProfileAction::MigrateLegacy) {
        return Err("--accept-existing jest dostępne tylko dla migrate-legacy".to_owned());
    }
    Ok(())
}

fn run(options: Options) -> Result<(), CliFailure> {
    if matches!(
        options.action,
        Some(
            ProfileAction::Create
                | ProfileAction::Rename
                | ProfileAction::Install
                | ProfileAction::Switch
                | ProfileAction::Bump
        )
    ) && options.profile.as_deref().is_none_or(str::is_empty)
    {
        let Some(action) = options.action else {
            return Err(CliFailure::Parse("brak komendy".to_owned()));
        };
        let action = action_name(action);
        return Err(CliFailure::Parse(format!("{action} wymaga nazwy profilu")));
    }

    let context = RepositoryContext::discover().map_err(operation_error)?;
    if matches!(
        options.action,
        Some(
            ProfileAction::Install
                | ProfileAction::Switch
                | ProfileAction::Bump
                | ProfileAction::Rename
        )
    ) {
        let profile = options.profile.as_deref().ok_or_else(|| {
            CliFailure::Parse("install/switch/bump/rename wymagają nazwy profilu".to_owned())
        })?;
        if !profile_names(&context)
            .map_err(operation_error)?
            .iter()
            .any(|name| name == profile)
        {
            return Err(CliFailure::Parse(format!("Nieznany profil: {profile}")));
        }
    }

    match options.action {
        Some(ProfileAction::Rename) => rename_profile(
            &context,
            options
                .profile
                .as_deref()
                .ok_or_else(|| CliFailure::Parse("rename wymaga nazwy profilu".to_owned()))?,
            options
                .new_name
                .as_deref()
                .ok_or_else(|| CliFailure::Parse("rename wymaga --new-name".to_owned()))?,
        )
        .map_err(operation_error),
        Some(ProfileAction::Create) => create_profile(
            &context,
            options
                .profile
                .as_deref()
                .ok_or_else(|| CliFailure::Parse("create wymaga nazwy profilu".to_owned()))?,
            options.description.as_deref(),
        )
        .map_err(operation_error),
        Some(ProfileAction::Bump) => {
            let profile = options.profile.as_deref().ok_or_else(|| {
                CliFailure::Parse("bump wymaga nazwy profilu źródłowego".to_owned())
            })?;
            bump_profile(
                &context,
                profile,
                options.part.unwrap_or(VersionBump::Patch),
            )
            .map(|_| ())
            .map_err(operation_error)
        }
        Some(ProfileAction::Validate) => {
            validate_sources(&context, options.profile.as_deref()).map_err(operation_error)
        }
        Some(
            ProfileAction::Install
            | ProfileAction::Switch
            | ProfileAction::Update
            | ProfileAction::Uninstall
            | ProfileAction::Status
            | ProfileAction::MigrateLegacy,
        ) => {
            let project =
                selected_project(&context, options.project.as_deref()).map_err(operation_error)?;
            if let Some(
                action @ (ProfileAction::Install
                | ProfileAction::Update
                | ProfileAction::Switch
                | ProfileAction::Uninstall
                | ProfileAction::Status),
            ) = options.action
            {
                return crate::profile_management::scoped::run(
                    &context,
                    &project,
                    action,
                    options.client.unwrap_or(ClientSelection::All),
                    options.profile.as_deref(),
                )
                .map_err(operation_error);
            }
            match options.action {
                Some(ProfileAction::MigrateLegacy) => {
                    migrate_legacy(&context, &project, options.accept_existing)
                        .map_err(operation_error)
                }
                _ => Err(CliFailure::Operation("Nieobsługiwana komenda".to_owned())),
            }
        }
        None | Some(ProfileAction::Help) => Ok(()),
    }
}

fn action_name(action: ProfileAction) -> &'static str {
    match action {
        ProfileAction::Help => "help",
        ProfileAction::Create => "create",
        ProfileAction::Rename => "rename",
        ProfileAction::Install => "install",
        ProfileAction::Update => "update",
        ProfileAction::Uninstall => "uninstall",
        ProfileAction::Switch => "switch",
        ProfileAction::Status => "status",
        ProfileAction::Validate => "validate",
        ProfileAction::MigrateLegacy => "migrate-legacy",
        ProfileAction::Bump => "bump",
    }
}

fn operation_error(error: impl std::fmt::Display) -> CliFailure {
    CliFailure::Operation(error.to_string())
}

fn parser_error(message: &str) -> i32 {
    tracing::error!(event = "cli.failed", error_kind = "arguments", message = %message);
    eprintln!("usage: ai-profile KOMENDA [NAZWA] [opcje] (pomoc: make help)");
    eprintln!("ai-profile: error: {message}");
    2
}
