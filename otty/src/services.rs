use std::path::Path;

use otty_shell_bootstrap::ShellLaunch;
use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

/// Clone terminal settings with a session descriptor.
pub(crate) fn terminal_settings_for_session(
    base_settings: &Settings,
    session: SessionKind,
) -> Settings {
    let mut settings = base_settings.clone();
    settings.backend = settings.backend.clone().with_session(session);
    settings
}

/// Clone terminal settings with a bootstrapped local shell launch.
pub(crate) fn terminal_settings_for_shell_launch(
    base_settings: &Settings,
    launch: &ShellLaunch,
) -> Settings {
    let options =
        local_session_options(launch.program(), launch.args(), launch.envs());
    let session = SessionKind::from_local_options(options);

    terminal_settings_for_session(base_settings, session)
}

pub(crate) fn editor_terminal_settings(
    editor: &str,
    base_terminal_settings: &Settings,
    file_path: &Path,
) -> Option<Settings> {
    let (program, mut args) = parse_command_line(editor)?;

    args.push(file_path.to_string_lossy().into_owned());

    let mut options = LocalSessionOptions::default()
        .with_program(&program)
        .with_args(args);

    if let Some(parent) = file_path.parent() {
        options = options.with_working_directory(parent.into());
    }

    let session = SessionKind::from_local_options(options);
    Some(terminal_settings_for_session(
        base_terminal_settings,
        session,
    ))
}

fn parse_command_line(input: &str) -> Option<(String, Vec<String>)> {
    let parts = match shell_words::split(input) {
        Ok(parts) => parts,
        Err(err) => {
            log::warn!("default editor parse failed: {err}");
            return None;
        },
    };
    let Some((program, args)) = parts.split_first() else {
        log::warn!("default editor command is empty");
        return None;
    };

    Some((program.clone(), args.to_vec()))
}

fn local_session_options(
    program: &str,
    args: &[String],
    envs: &[(String, String)],
) -> LocalSessionOptions {
    let mut options = LocalSessionOptions::default()
        .with_program(program)
        .with_args(args.to_vec());

    for (key, value) in envs {
        options = options.with_env(key, value);
    }

    options
}

#[cfg(test)]
mod tests {
    use super::{local_session_options, parse_command_line};

    #[test]
    fn given_shell_launch_parts_when_converted_then_terminal_defaults_remain() {
        let args = vec![String::from("--rcfile"), String::from("/tmp/ottyrc")];
        let envs =
            vec![(String::from("OTTY_INTEGRATION_VERSION"), String::from("2"))];

        let options = local_session_options("/bin/bash", &args, &envs);

        assert_eq!(options.program(), "/bin/bash");
        assert_eq!(options.args(), args);
        assert_eq!(
            options.envs().get("TERM").map(String::as_str),
            Some("xterm-256color"),
        );
        assert_eq!(
            options.envs().get("COLORTERM").map(String::as_str),
            Some("truecolor"),
        );
        assert_eq!(
            options
                .envs()
                .get("OTTY_INTEGRATION_VERSION")
                .map(String::as_str),
            Some("2"),
        );
    }

    #[test]
    fn given_valid_command_line_when_parsed_then_program_and_args_are_returned()
    {
        let parsed =
            parse_command_line("nvim -u NORC").expect("command should parse");
        assert_eq!(parsed.0, "nvim");
        assert_eq!(parsed.1, vec![String::from("-u"), String::from("NORC")]);
    }

    #[test]
    fn given_invalid_command_line_when_parsed_then_none_is_returned() {
        assert!(parse_command_line("nvim '").is_none());
    }
}
