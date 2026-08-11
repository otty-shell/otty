use std::sync::Arc;

use super::content::BlockContent;
use super::id::{BlockId, ShellInstanceId};
use crate::cell::Cell;
use crate::grid::Grid;
use crate::index::{Column, Line};
use crate::{Dimensions, Flags, Surface, SurfaceConfig};

/// Kind of a terminal block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum BlockKind {
    /// Command input and its output.
    #[default]
    Command,
    /// Prompt content without a started command.
    Prompt,
    /// Full-screen terminal application content.
    FullScreen,
}

/// Terminal execution state governed by the lifecycle reducer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockState {
    /// Prompt and command input exist, but execution has not started.
    BeforeExecution,
    /// The command currently owns ordinary PTY output.
    Executing,
    /// The command completed with a stable outcome.
    Finished(BlockOutcome),
    /// A background command is still producing output.
    BackgroundRunning,
    /// A background command has stopped producing output.
    BackgroundFinished,
    /// Content is not connected to a running shell command.
    Static,
}

/// Stable completion result for a terminal block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockOutcome {
    /// The process exited with the supplied status code.
    Exited(i32),
    /// The process was terminated by the supplied signal number.
    Signaled(i32),
    /// Execution was cancelled before a process outcome was available.
    Cancelled,
    /// The owning shell exited before sending command completion.
    ShellExited,
    /// Completion was recovered without an exact process outcome.
    Unknown,
}

/// Minimal metadata associated with a terminal block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockMeta {
    /// Unique identifier reported by the escape handler for this block.
    pub id: String,
    /// Semantic category (prompt, command, fullscreen) of the block.
    pub kind: BlockKind,
    /// Command line (if known) that generated the block.
    pub cmd: Option<String>,
    /// Working directory captured when the block started.
    pub cwd: Option<String>,
    /// Shell executable responsible for the block.
    pub shell: Option<String>,
    /// Exit status of the command, once finished.
    pub exit_code: Option<i32>,
    /// Timestamp marking when the block started executing.
    pub started_at: Option<i64>,
    /// Timestamp marking when the block finished executing.
    pub finished_at: Option<i64>,
    /// Whether the block ever entered alt-screen mode.
    pub is_alt_screen: bool,
    /// Whether the block has finished producing output.
    pub is_finished: bool,
}

/// Snapshot entry describing a block's extent within the viewport.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockSnapshot {
    /// Metadata describing the block captured in this snapshot entry.
    pub meta: BlockMeta,
    /// Viewport-relative line where this block begins.
    pub start_line: i32,
    /// Number of visible lines contributed by this block.
    pub line_count: usize,
    /// Cached full textual contents for finished blocks.
    ///
    /// This is intentionally detached from the viewport so UI actions like
    /// "copy content" can include scrollback lines that are currently off-screen.
    pub cached_text: Option<Arc<str>>,
    /// Prompt section captured from OSC 133 boundaries.
    pub prompt_text: Option<Arc<str>>,
    /// Output section captured independently from prompt and command text.
    pub output_text: Option<Arc<str>>,
    /// Whether this block snapshot corresponds to an alt screen.
    pub is_alt_screen: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct MetadataPatch {
    command: Option<String>,
    cwd_before: Option<String>,
    cwd_after: Option<String>,
    started_at: Option<i64>,
    finished_at: Option<i64>,
    exit_code: Option<i32>,
}

impl MetadataPatch {
    pub(super) fn prompt(cwd: Option<String>) -> Self {
        Self {
            cwd_before: cwd,
            ..Self::default()
        }
    }

    pub(super) fn command_start(
        command: Option<String>,
        cwd: Option<String>,
        started_at: Option<i64>,
    ) -> Self {
        Self {
            command,
            cwd_before: cwd,
            started_at,
            ..Self::default()
        }
    }

    pub(super) fn completion(
        outcome: &BlockOutcome,
        cwd: Option<String>,
        finished_at: Option<i64>,
    ) -> Self {
        let exit_code = match outcome {
            BlockOutcome::Exited(exit_code) => Some(*exit_code),
            _ => None,
        };

        Self {
            cwd_after: cwd,
            finished_at,
            exit_code,
            ..Self::default()
        }
    }
}

/// Metadata accumulated by sparse lifecycle patches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockMetadata {
    command: Option<String>,
    cwd_before: Option<String>,
    cwd_after: Option<String>,
    started_at: Option<i64>,
    finished_at: Option<i64>,
    exit_code: Option<i32>,
}

impl BlockMetadata {
    /// Return the canonical command text when known.
    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }

    /// Return the working directory captured before execution.
    pub fn cwd_before(&self) -> Option<&str> {
        self.cwd_before.as_deref()
    }

    /// Return the working directory captured after execution.
    pub fn cwd_after(&self) -> Option<&str> {
        self.cwd_after.as_deref()
    }

    /// Return the execution start timestamp.
    pub fn started_at(&self) -> Option<i64> {
        self.started_at
    }

    /// Return the execution finish timestamp.
    pub fn finished_at(&self) -> Option<i64> {
        self.finished_at
    }

    /// Return the process exit code when completion used an exit status.
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    fn apply_patch(&mut self, patch: MetadataPatch) {
        if let Some(command) = patch.command {
            self.command = Some(command);
        }
        if let Some(cwd) = patch.cwd_before {
            self.cwd_before = Some(cwd);
        }
        if let Some(cwd) = patch.cwd_after {
            self.cwd_after = Some(cwd);
        }
        if let Some(started_at) = patch.started_at {
            self.started_at = Some(started_at);
        }
        if let Some(finished_at) = patch.finished_at {
            self.finished_at = Some(finished_at);
        }
        if let Some(exit_code) = patch.exit_code {
            self.exit_code = Some(exit_code);
        }
    }
}

/// Canonical lifecycle record for one terminal block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRecord {
    id: BlockId,
    shell_instance_id: ShellInstanceId,
    state: BlockState,
    metadata: BlockMetadata,
}

impl BlockRecord {
    /// Return the stable block identity.
    pub fn id(&self) -> &BlockId {
        &self.id
    }

    /// Return the shell context that owns this block.
    pub fn shell_instance_id(&self) -> &ShellInstanceId {
        &self.shell_instance_id
    }

    /// Return the current lifecycle state.
    pub fn state(&self) -> &BlockState {
        &self.state
    }

    /// Return the merged block metadata.
    pub fn metadata(&self) -> &BlockMetadata {
        &self.metadata
    }

    pub(super) fn prepared(
        id: BlockId,
        shell_instance_id: ShellInstanceId,
        cwd: Option<String>,
    ) -> Self {
        let mut metadata = BlockMetadata::default();
        metadata.apply_patch(MetadataPatch::prompt(cwd));

        Self {
            id,
            shell_instance_id,
            state: BlockState::BeforeExecution,
            metadata,
        }
    }

    pub(super) fn executing(
        id: BlockId,
        shell_instance_id: ShellInstanceId,
        command: Option<String>,
        cwd: Option<String>,
        started_at: Option<i64>,
    ) -> Self {
        let mut record = Self::prepared(id, shell_instance_id, None);
        record.start(command, cwd, started_at);

        record
    }

    pub(super) fn patch_prompt(&mut self, cwd: Option<String>) {
        self.metadata.apply_patch(MetadataPatch::prompt(cwd));
    }

    pub(super) fn start(
        &mut self,
        command: Option<String>,
        cwd: Option<String>,
        started_at: Option<i64>,
    ) {
        self.metadata.apply_patch(MetadataPatch::command_start(
            command, cwd, started_at,
        ));
        self.state = BlockState::Executing;
    }

    pub(super) fn finish(
        &mut self,
        outcome: BlockOutcome,
        cwd: Option<String>,
        finished_at: Option<i64>,
    ) {
        self.metadata.apply_patch(MetadataPatch::completion(
            &outcome,
            cwd,
            finished_at,
        ));
        self.state = BlockState::Finished(outcome);
    }
}

pub(super) struct Block {
    pub(super) id: BlockId,
    pub(super) meta: BlockMeta,
    pub(super) state: BlockState,
    pub(super) surface: Surface,
    pub(super) cached_text: Option<Arc<str>>,
    pub(super) content: BlockContent,
}

impl Block {
    pub(super) fn visible_line_count(&self) -> usize {
        self.visible_extent().1
    }

    pub(super) fn visible_extent(&self) -> (Line, usize) {
        let grid = self.surface.grid();
        let history_lines = grid.history_size();
        let screen_lines = grid.screen_lines();
        let (viewport_head, viewport_tail) = viewport_content_bounds(grid);
        let trim_head = if history_lines == 0 { viewport_head } else { 0 };
        let trim_tail = screen_lines.saturating_sub(viewport_tail);
        let visible_viewport =
            screen_lines.saturating_sub(trim_head + trim_tail);
        let total_lines = history_lines + visible_viewport;
        let top_line = grid.topmost_line() + trim_head;

        (top_line, total_lines)
    }

    pub(super) fn new<D: Dimensions>(
        config: &SurfaceConfig,
        dimensions: &D,
        meta: BlockMeta,
    ) -> Self {
        let id = if meta.id.is_empty() {
            BlockId::new("otty:bootstrap:0")
        } else {
            BlockId::new(meta.id.clone())
        };
        let state = if meta.is_finished {
            BlockState::Finished(BlockOutcome::Unknown)
        } else if meta.kind == BlockKind::Prompt {
            BlockState::BeforeExecution
        } else if meta.id.is_empty() {
            BlockState::Static
        } else {
            BlockState::Executing
        };

        Self {
            id,
            meta,
            state,
            surface: Surface::new(config.clone(), dimensions),
            cached_text: None,
            content: BlockContent::default(),
        }
    }

    pub(super) fn update_cached_text(&mut self) {
        if self.meta.kind == BlockKind::Prompt || !self.meta.is_finished {
            return;
        }

        let grid = self.surface.grid();
        let (top_line, total_lines) = self.visible_extent();
        if total_lines == 0 || self.surface.columns() == 0 {
            self.cached_text = None;
            return;
        }

        let columns = self.surface.columns();
        let start = top_line.0;
        let end = start + total_lines as i32;

        let mut lines = Vec::with_capacity(total_lines);
        for line_value in start..end {
            let line = Line(line_value);
            let mut buffer = String::with_capacity(columns);
            for col in 0..columns {
                let column = Column(col);
                let cell = &grid[line][column];
                if !cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    buffer.push(cell.c);
                }
            }
            lines.push(buffer.trim_end_matches(' ').to_string());
        }

        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }

        let text = lines.join("\n");
        if text.is_empty() {
            self.cached_text = None;
        } else {
            self.cached_text = Some(Arc::<str>::from(text));
        }
        self.content.freeze(self.cached_text.as_ref());
    }
}

fn viewport_content_bounds(grid: &Grid<Cell>) -> (usize, usize) {
    let screen_lines = grid.screen_lines();
    let mut first_non_empty = None;
    let mut last_non_empty = None;
    let cursor_line = grid.cursor.point.line;

    for row_idx in 0..screen_lines {
        let line = Line(row_idx as i32);
        let row = &grid[line];
        let is_cursor_row = cursor_line == line;
        if is_cursor_row || !row.is_clear() {
            if first_non_empty.is_none() {
                first_non_empty = Some(row_idx);
            }
            last_non_empty = Some(row_idx + 1);
        }
    }

    match (first_non_empty, last_non_empty) {
        (Some(start), Some(end)) => (start, end),
        _ => (screen_lines, screen_lines),
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockMetadata, BlockOutcome, MetadataPatch};

    #[test]
    fn metadata_patch_keeps_fields_omitted_by_sparse_completion() {
        let mut metadata = BlockMetadata::default();
        metadata.apply_patch(MetadataPatch::command_start(
            Some(String::from("printf stable")),
            Some(String::from("/before")),
            Some(10),
        ));

        metadata.apply_patch(MetadataPatch::completion(
            &BlockOutcome::Exited(7),
            None,
            Some(20),
        ));

        assert_eq!(metadata.command(), Some("printf stable"));
        assert_eq!(metadata.cwd_before(), Some("/before"));
        assert_eq!(metadata.cwd_after(), None);
        assert_eq!(metadata.started_at(), Some(10));
        assert_eq!(metadata.finished_at(), Some(20));
        assert_eq!(metadata.exit_code(), Some(7));
    }
}
