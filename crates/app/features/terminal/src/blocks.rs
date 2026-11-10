//! What the command blocks of a terminal say about its shell: the folder it
//! is in and the program it runs.

use std::path::{Path, PathBuf};

use otty_ui_term::BlockSnapshot;

/// Resolve the active terminal working directory from block metadata.
pub(super) fn terminal_cwd_from_blocks(
    blocks: &[BlockSnapshot],
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
pub(super) fn running_program_from_blocks(
    blocks: &[BlockSnapshot],
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

#[cfg(test)]
mod tests {
    use otty_ui_term::BlockSnapshot;

    use super::running_program_from_blocks;

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
}
