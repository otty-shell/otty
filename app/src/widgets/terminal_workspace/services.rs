use std::env;
use std::path::{Path, PathBuf};

use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

use super::errors::TerminalWorkspaceError;
use super::types::ShellSession;
use crate::shell_integration::{
    shell_integration_dir, shell_name, with_shell_integration,
};

/// Build a shell session configured with OTTY integrations for a given shell.
pub(crate) fn setup_shell_session_with_shell(
    shell_path: &str,
) -> Result<ShellSession, TerminalWorkspaceError> {
    let shell_name = shell_name(shell_path);
    let options = with_shell_integration(
        shell_session_options(shell_path),
        &shell_integration_dir(),
    )?;

    Ok(ShellSession::new(
        shell_name,
        SessionKind::from_local_options(options),
    ))
}

/// Build a shell session without writing shell integration files.
pub(crate) fn fallback_shell_session_with_shell(
    shell_path: &str,
) -> ShellSession {
    let shell_name = shell_name(shell_path);
    let options = shell_session_options(shell_path);

    ShellSession::new(shell_name, SessionKind::from_local_options(options))
}

/// Resolve the active terminal working directory from block metadata.
pub(crate) fn terminal_cwd_from_blocks(
    blocks: &[otty_ui_term::BlockSnapshot],
) -> Option<PathBuf> {
    blocks
        .iter()
        .rev()
        .find_map(|block| block.meta.cwd.as_deref())
        .map(PathBuf::from)
}

/// Return the program of the command running in the latest block: its
/// first word, without leading `NAME=value` words and folders. `None` while
/// the shell waits at the prompt.
pub(crate) fn running_program_from_blocks(
    blocks: &[otty_ui_term::BlockSnapshot],
) -> Option<&str> {
    let meta = &blocks.last()?.meta;
    if meta.is_finished {
        return None;
    }

    let program = meta
        .cmd
        .as_deref()?
        .split_whitespace()
        .find(|word| !word.contains('='))?;

    Path::new(program).file_name()?.to_str()
}

fn shell_session_options(shell_path: &str) -> LocalSessionOptions {
    let options = LocalSessionOptions::default().with_program(shell_path);

    if let Some(home) = env::var_os("HOME") {
        return options.with_working_directory(PathBuf::from(home));
    }

    options
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_ui_term::BlockSnapshot;
    use otty_ui_term::settings::SessionKind;

    use super::{
        fallback_shell_session_with_shell, running_program_from_blocks,
    };

    fn block(cmd: Option<&str>, is_finished: bool) -> BlockSnapshot {
        let mut block = BlockSnapshot::default();
        block.meta.cmd = cmd.map(String::from);
        block.meta.is_finished = is_finished;
        block
    }

    #[test]
    fn given_running_command_when_read_then_its_first_word_is_the_program() {
        let blocks = [
            block(Some("ls"), true),
            block(Some("cargo build --release"), false),
        ];

        assert_eq!(running_program_from_blocks(&blocks), Some("cargo"));
    }

    #[test]
    fn given_finished_command_when_read_then_there_is_no_program() {
        let blocks = [block(Some("cargo build"), true)];

        assert_eq!(running_program_from_blocks(&blocks), None);
    }

    #[test]
    fn given_prompt_after_command_when_read_then_there_is_no_program() {
        let blocks = [block(Some("cargo build"), true), block(None, false)];

        assert_eq!(running_program_from_blocks(&blocks), None);
    }

    #[test]
    fn given_no_blocks_when_read_then_there_is_no_program() {
        assert_eq!(running_program_from_blocks(&[]), None);
    }

    #[test]
    fn given_env_words_and_a_path_when_read_then_only_the_program_name_stays() {
        let blocks = [block(
            Some("RUST_LOG=debug ./target/debug/otty --verbose"),
            false,
        )];

        assert_eq!(running_program_from_blocks(&blocks), Some("otty"));
    }

    #[test]
    fn given_blank_command_when_read_then_there_is_no_program() {
        let blocks = [block(Some("   "), false)];

        assert_eq!(running_program_from_blocks(&blocks), None);
    }

    #[test]
    fn given_shell_path_when_fallback_session_created_then_uses_local_program()
    {
        let session = fallback_shell_session_with_shell("/usr/bin/bash");

        assert_eq!(session.name(), "bash");
        match session.session() {
            SessionKind::Local(options) => {
                assert_eq!(options.program(), "/usr/bin/bash");
            },
            SessionKind::Ssh(_) => {
                panic!("expected local session kind");
            },
        }
    }

    #[test]
    fn given_home_when_fallback_session_created_then_starts_in_home() {
        let home = std::env::var_os("HOME").map(PathBuf::from);

        let session = fallback_shell_session_with_shell("/bin/zsh");

        match session.session() {
            SessionKind::Local(options) => {
                assert_eq!(options.working_directory(), &home);
            },
            SessionKind::Ssh(_) => {
                panic!("expected local session kind");
            },
        }
    }
}
