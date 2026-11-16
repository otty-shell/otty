//! What the command blocks of a terminal say about its shell: the folder it
//! is in, the program it runs and whether a command runs; and which command
//! block the block keys move the selection to, and its place among them.
//!
//! A command block is a block a command started
//! ([`BlockSnapshot::is_command`]). The prompt is none, and neither is the
//! first block of a terminal, which holds what the shell printed before its
//! first prompt, nor the one block of a shell without the integration. A
//! command that still runs is a command block too.

use std::path::{Path, PathBuf};

use otty_ui_term::{BlockKey, BlockSnapshot};

/// Which way the block keys move the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BlockStep {
    /// To the command block before, an older one.
    Previous,
    /// To the command block after, a newer one.
    Next,
}

/// The place of the selected block among the command blocks of its pane,
/// as the status bar counts it: `Block 5 of 6`.
///
/// # Example
///
/// ```
/// use otty_app_terminal::TerminalWorkspace;
///
/// // No terminal tab, so no selected block to count.
/// let terminal = TerminalWorkspace::new();
///
/// assert_eq!(terminal.selected_block_place(), None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockPlace {
    number: usize,
    count: usize,
}

impl BlockPlace {
    /// Return the number of the block, counted from 1 at the oldest command
    /// block.
    pub fn number(&self) -> usize {
        self.number
    }

    /// Return how many command blocks the pane has.
    pub fn count(&self) -> usize {
        self.count
    }

    /// The block `number` of `count` command blocks.
    pub(super) fn new(number: usize, count: usize) -> Self {
        Self { number, count }
    }
}

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

/// Whether a command that has a status line still runs, so its status
/// counts the seconds. A full-screen program shows no status line, so
/// while one is on nothing counts.
pub(super) fn has_running_command(blocks: &[BlockSnapshot]) -> bool {
    blocks
        .iter()
        .any(|block| block.has_status_line && !block.meta.is_finished)
}

/// Return the command block the selection moves to from `selected` by
/// `step`: the one before or after it. With no block selected, or one that
/// is gone from the history, `Previous` takes the latest command block and
/// `Next` none. `None` also when the selection would stay: at the first
/// block going back, at the last one going forward.
pub(super) fn block_after_step(
    blocks: &[BlockSnapshot],
    selected: Option<BlockKey>,
    step: BlockStep,
) -> Option<BlockKey> {
    let commands: Vec<BlockKey> = command_block_keys(blocks).collect();
    let position = selected.and_then(|key| position_of(&commands, key));

    let target = match (position, step) {
        (None, BlockStep::Previous) => commands.len().checked_sub(1)?,
        (None, BlockStep::Next) => return None,
        (Some(position), BlockStep::Previous) => position.checked_sub(1)?,
        (Some(position), BlockStep::Next) => position + 1,
    };

    commands.get(target).copied()
}

/// Return the place of the block `block_key` among the command blocks of
/// `blocks`; `None` when it is not one of them.
pub(super) fn block_place(
    blocks: &[BlockSnapshot],
    block_key: BlockKey,
) -> Option<BlockPlace> {
    let commands: Vec<BlockKey> = command_block_keys(blocks).collect();
    let position = position_of(&commands, block_key)?;

    Some(BlockPlace::new(position + 1, commands.len()))
}

/// The keys of the command blocks, oldest first.
fn command_block_keys(
    blocks: &[BlockSnapshot],
) -> impl Iterator<Item = BlockKey> + '_ {
    blocks
        .iter()
        .filter(|block| block.is_command())
        .map(|block| block.key)
}

/// The position of `key` among `keys`. A key never repeats in a terminal,
/// even where the ids of the shell do.
fn position_of(keys: &[BlockKey], key: BlockKey) -> Option<usize> {
    keys.iter().position(|candidate| *candidate == key)
}

#[cfg(test)]
mod tests {
    use otty_app_testing::blocks::{command_block, command_history, key_of};
    use otty_ui_term::{BlockKey, BlockSnapshot};

    use super::{
        BlockPlace, BlockStep, block_after_step, block_place,
        running_program_from_blocks,
    };

    fn block(cmd: Option<&str>, is_finished: bool) -> BlockSnapshot {
        let mut block = BlockSnapshot::default();
        block.meta.cmd = cmd.map(String::from);
        block.meta.is_finished = is_finished;
        block
    }

    /// The name of the command block number `number`.
    fn name(number: u64) -> String {
        format!("cmd-{number}")
    }

    /// The key of the command block number `number`.
    fn key(number: u64) -> BlockKey {
        key_of(&name(number))
    }

    /// The first block of a terminal, which no command started, then the
    /// command blocks numbered `numbers` and the prompt.
    fn history(numbers: &[u64]) -> Vec<BlockSnapshot> {
        let names: Vec<String> = numbers.iter().map(|&n| name(n)).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();

        command_history(&names).blocks
    }

    #[test]
    fn up_with_nothing_selected_selects_the_latest_command_block() {
        let blocks = history(&[1, 2, 3]);

        assert_eq!(
            block_after_step(&blocks, None, BlockStep::Previous),
            Some(key(3))
        );
    }

    #[test]
    fn down_with_nothing_selected_selects_nothing() {
        let blocks = history(&[1, 2, 3]);

        assert_eq!(block_after_step(&blocks, None, BlockStep::Next), None);
    }

    #[test]
    fn up_and_down_move_by_one_command_block() {
        let blocks = history(&[1, 2, 3]);

        assert_eq!(
            block_after_step(&blocks, Some(key(2)), BlockStep::Previous),
            Some(key(1))
        );
        assert_eq!(
            block_after_step(&blocks, Some(key(2)), BlockStep::Next),
            Some(key(3))
        );
    }

    /// The prompt and the first block of a terminal, which holds what the
    /// shell printed before its first prompt, are no command blocks.
    #[test]
    fn the_prompt_and_the_block_no_command_started_are_skipped() {
        let blocks = history(&[1, 2]);

        assert_eq!(
            block_after_step(&blocks, Some(key(1)), BlockStep::Previous),
            None
        );
        assert_eq!(
            block_after_step(&blocks, Some(key(2)), BlockStep::Next),
            None
        );
    }

    /// A command that still runs is the latest block and is a command
    /// block like the others.
    #[test]
    fn a_running_command_is_a_command_block() {
        let mut blocks = history(&[1]);
        blocks.pop();
        blocks.push(command_block(&name(2), 1_000));

        assert_eq!(
            block_after_step(&blocks, None, BlockStep::Previous),
            Some(key(2))
        );
        assert_eq!(block_place(&blocks, key(2)), Some(BlockPlace::new(2, 2)));
    }

    #[test]
    fn up_at_the_first_block_and_down_at_the_last_change_nothing() {
        let blocks = history(&[1, 2, 3]);

        assert_eq!(
            block_after_step(&blocks, Some(key(1)), BlockStep::Previous),
            None
        );
        assert_eq!(
            block_after_step(&blocks, Some(key(3)), BlockStep::Next),
            None
        );
    }

    /// A selected block that is gone from the history, for example pushed
    /// out by newer ones, counts as no selection.
    #[test]
    fn a_selected_block_that_is_gone_counts_as_no_selection() {
        let blocks = history(&[1, 2]);

        assert_eq!(
            block_after_step(&blocks, Some(key(9)), BlockStep::Previous),
            Some(key(2))
        );
        assert_eq!(
            block_after_step(&blocks, Some(key(9)), BlockStep::Next),
            None
        );
        assert_eq!(block_place(&blocks, key(9)), None);
    }

    #[test]
    fn the_fifth_of_six_command_blocks_is_5_of_6() {
        let blocks = history(&[1, 2, 3, 4, 5, 6]);

        let place = block_place(&blocks, key(5)).expect("the block is counted");

        assert_eq!(place.number(), 5);
        assert_eq!(place.count(), 6);
    }

    /// Owner, issue 13: a shell started inside another one counts its
    /// blocks from 1 again, so `ls`, `zsh`, `pwd` and `date` have the shell
    /// ids `cmd-1`, `cmd-2`, `cmd-1` and `cmd-2`. The keys still go
    /// through every block back to `ls`, and each block has its own place.
    #[test]
    fn blocks_with_repeated_shell_ids_are_each_reached_and_counted() {
        let mut blocks = history(&[1, 2, 3, 4]);
        let ids = ["cmd-1", "cmd-2", "cmd-1", "cmd-2"];
        for (block, id) in blocks[1..5].iter_mut().zip(ids) {
            block.meta.id = String::from(id);
        }

        let mut selected = None;
        let mut reached = Vec::new();
        while let Some(next) =
            block_after_step(&blocks, selected, BlockStep::Previous)
        {
            reached.push(next);
            selected = Some(next);
        }

        assert_eq!(reached, [key(4), key(3), key(2), key(1)]);
        assert_eq!(block_place(&blocks, key(1)), Some(BlockPlace::new(1, 4)));
        assert_eq!(block_place(&blocks, key(3)), Some(BlockPlace::new(3, 4)));
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
