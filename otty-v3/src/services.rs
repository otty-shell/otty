use std::path::Path;

use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

use crate::widgets::terminal_workspace::services::terminal_settings_for_session;

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

#[cfg(test)]
mod tests {
    use super::parse_command_line;

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
