//! Helpers for the tests of the surface crate: block snapshots built by
//! hand, a grid size, the events a shell sends, and block surfaces that
//! ran commands.
//!
//! They live inside `block`, not at the crate root, because they read the
//! private parts of a block surface: its list of blocks, the key of each
//! block and the slices of the stitched history.

use crate::block::text::collect_block_text;
use crate::block::{BlockKey, BlockKind, BlockSnapshot, BlockSurface};
use crate::escape::{
    BlockEvent, BlockKind as EscapeBlockKind, BlockMeta as EscapeBlockMeta,
    BlockPhase,
};
use crate::index::{Column, Line};
use crate::snapshot::{SnapshotOwned, SurfaceModel};
use crate::{Dimensions, SurfaceActor, SurfaceConfig};

/// Time the shell puts into its events, in whole seconds of its own
/// clock. The model must not use it.
const SHELL_TIME: i64 = 1_700_000_000;

/// A size of the grid for the tests: `columns` wide, `lines` high.
pub(crate) struct TestDimensions {
    columns: usize,
    lines: usize,
}

impl TestDimensions {
    /// The size `columns` wide and `lines` high.
    pub(crate) fn new(columns: usize, lines: usize) -> Self {
        Self { columns, lines }
    }
}

impl Dimensions for TestDimensions {
    fn total_lines(&self) -> usize {
        self.lines
    }

    fn screen_lines(&self) -> usize {
        self.lines
    }

    fn columns(&self) -> usize {
        self.columns
    }

    fn last_column(&self) -> Column {
        Column(self.columns - 1)
    }

    fn bottommost_line(&self) -> Line {
        Line(self.lines as i32 - 1)
    }
}

/// A block snapshot of `kind` with the key `key`, as a test builds one
/// by hand; the other fields keep their defaults, for the test to set.
pub(crate) fn block_of_kind(key: u64, kind: BlockKind) -> BlockSnapshot {
    let mut block = BlockSnapshot {
        key: BlockKey::new(key),
        ..BlockSnapshot::default()
    };
    block.meta.kind = kind;

    block
}

/// The `preexec` event of the shell: the command `cmd` with the id `id`
/// starts in the folder `cwd`.
pub(crate) fn preexec_event(id: &str, cmd: &str, cwd: &str) -> BlockEvent {
    let mut event = shell_event(BlockPhase::Preexec, id);
    event.meta.cmd = Some(String::from(cmd));
    event.meta.cwd = Some(String::from(cwd));
    event.meta.started_at = Some(SHELL_TIME);
    event
}

/// The `exit` event of the shell: the command `id` ended with `exit_code`.
pub(crate) fn exit_event(id: &str, exit_code: i32) -> BlockEvent {
    let mut event = shell_event(BlockPhase::Exit, id);
    event.meta.exit_code = Some(exit_code);
    event
}

/// The `precmd` event of the shell: the prompt `id` is drawn.
pub(crate) fn precmd_event(id: &str) -> BlockEvent {
    let mut event = shell_event(BlockPhase::Precmd, id);
    event.meta.finished_at = Some(SHELL_TIME);
    event
}

/// The key of the latest block whose shell id is `id`.
pub(crate) fn key_of(surface: &BlockSurface, id: &str) -> BlockKey {
    surface
        .blocks
        .iter()
        .rev()
        .find(|block| block.meta.id == id)
        .map(|block| block.key)
        .expect("block with this id exists")
}

/// Print `text` at the cursor.
pub(crate) fn print_text(surface: &mut BlockSurface, text: &str) {
    text.chars().for_each(|c| surface.print(c));
}

/// Move the cursor to the start of the next line.
pub(crate) fn next_line(surface: &mut BlockSurface) {
    surface.carriage_return();
    surface.line_feed();
}

/// Print `lines` one under the other, each followed by a new line, as a
/// command prints its output.
pub(crate) fn print_lines(surface: &mut BlockSurface, lines: &[&str]) {
    for line in lines {
        print_text(surface, line);
        next_line(surface);
    }
}

/// Start the command `cmd` of the prompt the shell printed, the way the
/// shells do it: the cursor is already on the line under the command
/// when `preexec` arrives (checked with zsh 5.9 and bash 5.3).
pub(crate) fn start_command(surface: &mut BlockSurface, id: &str, cmd: &str) {
    next_line(surface);
    surface.handle_block_event(preexec_event(id, cmd, "/tmp"), 5_000);
}

/// End the running command `id` and show the next prompt `prompt_id`.
/// The cursor stays on the empty line under the output, as it does
/// with a real shell.
pub(crate) fn end_command(
    surface: &mut BlockSurface,
    id: &str,
    prompt_id: &str,
) {
    surface.handle_block_event(exit_event(id, 0), 6_000);
    surface.handle_block_event(precmd_event(prompt_id), 6_000);
}

/// The text of one line of the snapshot, without trailing spaces.
pub(crate) fn line_text(snapshot: &SnapshotOwned, line: i32) -> String {
    let text: String = snapshot
        .view()
        .cells
        .iter()
        .filter(|cell| cell.point.line.0 == line)
        .map(|cell| cell.cell.c)
        .collect();

    text.trim_end().to_string()
}

/// The snapshot of the first block whose shell id is `id`.
pub(crate) fn block_snapshot<'a>(
    snapshot: &'a SnapshotOwned,
    id: &str,
) -> &'a BlockSnapshot {
    snapshot
        .blocks
        .iter()
        .find(|block| block.meta.id == id)
        .expect("block with this id is in the snapshot")
}

/// The text of a line of the snapshot, counted from the first line of
/// `block_id`.
pub(crate) fn block_line_text(
    snapshot: &SnapshotOwned,
    block_id: &str,
    line: i32,
) -> String {
    let block = block_snapshot(snapshot, block_id);

    line_text(snapshot, block.start_line + line)
}

/// Lines of the whole stitched history, on screen or not.
pub(crate) fn history_line_count(surface: &BlockSurface) -> usize {
    surface
        .block_slices()
        .last()
        .map(|slice| slice.end)
        .unwrap_or(0)
}

/// A prompt `$ ls` whose command started.
pub(crate) fn surface_running_ls() -> BlockSurface {
    let dims = TestDimensions::new(8, 6);
    let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
    surface.handle_block_event(precmd_event("prompt-1"), 1_000);
    print_text(&mut surface, "$ ls");
    surface.handle_block_event(preexec_event("cmd-1", "ls", "/tmp"), 5_000);

    surface
}

/// `seq 20` typed under a two-line prompt (`~/otty`, then `$ seq 20`):
/// a block with 2 command lines and 20 lines of output. The prompt
/// `$ ls` comes after it.
pub(crate) fn surface_with_seq_20(screen_lines: usize) -> BlockSurface {
    let dims = TestDimensions::new(12, screen_lines);
    let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
    surface.handle_block_event(precmd_event("prompt-1"), 1_000);
    print_text(&mut surface, "~/otty");
    next_line(&mut surface);
    print_text(&mut surface, "$ seq 20");
    start_command(&mut surface, "cmd-1", "seq 20");
    let output = twenty_lines();
    let output: Vec<&str> = output.iter().map(String::as_str).collect();
    print_lines(&mut surface, &output);
    end_command(&mut surface, "cmd-1", "prompt-2");
    print_text(&mut surface, "$ ls");

    surface
}

/// A pane `columns` wide and `screen_lines` high where the shell showed
/// the lines `typed` one under the other (the prompt and the command
/// `cmd`), then the command `cmd-1` started and printed `output`. It still
/// runs.
pub(crate) fn surface_running_typed_command(
    columns: usize,
    screen_lines: usize,
    typed: &[&str],
    cmd: &str,
    output: &[&str],
) -> BlockSurface {
    let dims = TestDimensions::new(columns, screen_lines);
    let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
    surface.handle_block_event(precmd_event("prompt-1"), 1_000);

    for (index, line) in typed.iter().enumerate() {
        if index > 0 {
            next_line(&mut surface);
        }
        print_text(&mut surface, line);
    }
    start_command(&mut surface, "cmd-1", cmd);
    print_lines(&mut surface, output);

    surface
}

/// `$ echo aaaa bbbb`, 16 characters, typed in a pane 10 columns wide and
/// 12 lines high, so the command `cmd-1` wraps over two lines; it printed
/// `aaaa bbbb` and still runs.
pub(crate) fn surface_with_wrapped_command() -> BlockSurface {
    surface_running_typed_command(
        10,
        12,
        &["$ echo aaaa bbbb"],
        "echo aaaa bbbb",
        &["aaaa bbbb"],
    )
}

/// What `Copy output` of the block menu copies from the block `id` of a
/// snapshot of `surface`.
pub(crate) fn copied_output(
    surface: &mut BlockSurface,
    id: &str,
) -> Option<String> {
    let key = key_of(surface, id);
    let snapshot = surface.snapshot_owned();

    snapshot.view().block_output_text(key)
}

/// The whole text of the block `key` of `snapshot`: its command lines and
/// its output, the text `Copy output` is cut from.
pub(crate) fn block_text(
    snapshot: &SnapshotOwned,
    key: BlockKey,
) -> Option<String> {
    let view = snapshot.view();
    let block = view.block(key)?;

    collect_block_text(block, view.cells)
}

/// The output `seq 20` prints: `1` to `20`.
fn twenty_lines() -> Vec<String> {
    (1..=20).map(|number| format!("{number}")).collect()
}

/// An event of the shell in `phase` for the block `id`, with nothing else
/// filled in.
fn shell_event(phase: BlockPhase, id: &str) -> BlockEvent {
    let kind = match phase {
        BlockPhase::Precmd => EscapeBlockKind::Prompt,
        _ => EscapeBlockKind::Command,
    };

    BlockEvent {
        phase,
        meta: EscapeBlockMeta {
            id: String::from(id),
            kind,
            cmd: None,
            cwd: None,
            started_at: None,
            finished_at: None,
            exit_code: None,
            shell: None,
        },
    }
}
