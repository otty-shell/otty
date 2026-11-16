//! Command blocks for tests, as a terminal reports them in its snapshot.
//!
//! A test that decides what a terminal shows builds the blocks of its
//! snapshot by hand. The helpers here build the usual ones: a command block,
//! the same block finished, the prompt the shell waits at, a history of
//! finished commands, and a snapshot that only reports a folder. A test sets
//! the fields it needs on top: the fields of a block snapshot are public.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use otty_ui_term::{BlockKey, BlockKind, BlockSnapshot, SnapshotOwned};

/// The key of the test block named `name`. Blocks with other names get
/// other keys, so a test can name a block by what it runs.
///
/// # Examples
///
/// ```
/// use otty_app_testing::blocks::key_of;
///
/// assert_eq!(key_of("ls"), key_of("ls"));
/// assert_ne!(key_of("ls"), key_of("pwd"));
/// ```
pub fn key_of(name: &str) -> BlockKey {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);

    BlockKey::new(hasher.finish())
}

/// A command block named `name` (its key [`key_of`] it and its shell id is
/// `name`) whose command started at `started_at` milliseconds and still
/// runs. Its status line and one command line are the first two lines of
/// the pane.
///
/// # Examples
///
/// ```
/// use otty_app_testing::blocks::{command_block, key_of};
///
/// let block = command_block("ls", 1_000);
///
/// assert_eq!(block.key, key_of("ls"));
/// assert!(block.is_command());
/// assert!(!block.meta.is_finished);
/// ```
pub fn command_block(name: &str, started_at: i64) -> BlockSnapshot {
    let mut block = BlockSnapshot {
        key: key_of(name),
        start_line: 0,
        line_count: 2,
        has_status_line: true,
        is_status_line_visible: true,
        ..BlockSnapshot::default()
    };
    block.meta.id = String::from(name);
    block.meta.kind = BlockKind::Command;
    block.meta.started_at = Some(started_at);

    block
}

/// `block` finished with `exit_code` `took_ms` milliseconds after it
/// started. A block whose start is unknown ends at `took_ms`.
///
/// # Examples
///
/// ```
/// use otty_app_testing::blocks::{command_block, finished};
///
/// let block = finished(command_block("false", 1_000), Some(1), 200);
///
/// assert!(block.is_failed());
/// assert_eq!(block.meta.finished_at, Some(1_200));
/// ```
pub fn finished(
    mut block: BlockSnapshot,
    exit_code: Option<i32>,
    took_ms: i64,
) -> BlockSnapshot {
    block.meta.is_finished = true;
    block.meta.exit_code = exit_code;
    block.meta.finished_at = Some(block.meta.started_at.unwrap_or(0) + took_ms);

    block
}

/// The prompt the shell waits at, named `prompt`: one line at the top of
/// the pane.
///
/// # Examples
///
/// ```
/// use otty_app_testing::blocks::{key_of, prompt_block};
///
/// let prompt = prompt_block();
///
/// assert_eq!(prompt.key, key_of("prompt"));
/// assert!(!prompt.is_command());
/// ```
pub fn prompt_block() -> BlockSnapshot {
    let mut block = BlockSnapshot {
        key: key_of("prompt"),
        line_count: 1,
        ..BlockSnapshot::default()
    };
    block.meta.id = String::from("prompt");
    block.meta.kind = BlockKind::Prompt;

    block
}

/// A screen whose blocks are the first block of the terminal, which no
/// command started, the finished command blocks `names` and the prompt.
/// Each command block has its status line and is named as
/// [`command_block`] names it.
///
/// # Examples
///
/// ```
/// use otty_app_testing::blocks::{command_history, key_of};
///
/// let snapshot = command_history(&["ls", "pwd"]);
///
/// assert_eq!(snapshot.blocks.len(), 4);
/// assert_eq!(snapshot.blocks[2].key, key_of("pwd"));
/// ```
pub fn command_history(names: &[&str]) -> SnapshotOwned {
    let mut snapshot = SnapshotOwned::default();
    let mut start = BlockSnapshot::default();
    start.meta.kind = BlockKind::Start;
    snapshot.blocks.push(start);

    for name in names {
        let mut block = BlockSnapshot {
            key: key_of(name),
            line_count: 2,
            has_status_line: true,
            ..BlockSnapshot::default()
        };
        block.meta.id = String::from(*name);
        block.meta.kind = BlockKind::Command;
        block.meta.is_finished = true;
        snapshot.blocks.push(block);
    }
    snapshot.blocks.push(prompt_block());

    snapshot
}

/// A snapshot whose one block reports the folder `cwd`, the way the shell
/// integration does after a prompt.
///
/// # Examples
///
/// ```
/// use otty_app_testing::blocks::snapshot_in;
///
/// let snapshot = snapshot_in("/tmp");
///
/// assert_eq!(snapshot.blocks[0].meta.cwd.as_deref(), Some("/tmp"));
/// ```
pub fn snapshot_in(cwd: &str) -> SnapshotOwned {
    let mut block = BlockSnapshot::default();
    block.meta.cwd = Some(String::from(cwd));

    let mut snapshot = SnapshotOwned::default();
    snapshot.blocks.push(block);

    snapshot
}
