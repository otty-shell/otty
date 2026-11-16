//! What a snapshot says about a block: its kind, its metadata, its key and
//! where it is on screen. The terminal widget and the app read these types.

use std::sync::Arc;

use crate::escape::{
    BlockKind as EscapeBlockKind, BlockMeta as EscapeBlockMeta,
};

/// Kind of a terminal block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum BlockKind {
    #[default]
    Command,
    Prompt,
    FullScreen,
    /// The block every terminal starts with: what the shell prints before
    /// its first prompt, such as a message of the day. No command started
    /// it, so it is not a block for the user: its text is shown, but it is
    /// never hovered, selected or folded. Once the first prompt or command
    /// closes it, it takes no lines when it holds no text. A shell without
    /// the integration has only this block.
    Start,
}

impl From<EscapeBlockKind> for BlockKind {
    fn from(value: EscapeBlockKind) -> Self {
        match value {
            EscapeBlockKind::Command => Self::Command,
            EscapeBlockKind::FullScreen => Self::FullScreen,
            EscapeBlockKind::Prompt => Self::Prompt,
        }
    }
}

/// Minimal metadata associated with a terminal block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockMeta {
    /// The id the shell gave the block. It can repeat in a terminal (a
    /// shell started inside another one counts from 1 again), so it only
    /// matches the events of the shell to the block; everything after the
    /// block surface names a block by its [`BlockKey`].
    pub id: String,
    /// Semantic category (prompt, command, fullscreen) of the block.
    pub kind: BlockKind,
    /// Command line (if known) that generated the block.
    pub cmd: Option<String>,
    /// Working directory captured when the block started.
    pub cwd: Option<String>,
    /// Shell executable responsible for the block.
    pub shell: Option<String>,
    /// Exit status of the command, once the shell reported it. `None` while
    /// the command runs, and for a block the next prompt closed without an
    /// `exit` event.
    pub exit_code: Option<i32>,
    /// When the command started, in milliseconds measured by OTTY: the time
    /// passed to [`SurfaceActor::handle_block_event`] with the `preexec`
    /// event (the terminal passes milliseconds since the Unix epoch). The
    /// shell's own clock is not used.
    ///
    /// [`SurfaceActor::handle_block_event`]:
    /// crate::SurfaceActor::handle_block_event
    pub started_at: Option<i64>,
    /// When the block was closed, in the same milliseconds measured by OTTY:
    /// the time of the `exit` event, or of the next prompt when the shell
    /// sent no `exit`.
    pub finished_at: Option<i64>,
    /// Whether the block has finished producing output.
    pub is_finished: bool,
}

impl From<EscapeBlockMeta> for BlockMeta {
    /// Take what the shell reported about a block. The times of the shell
    /// are left out: the block surface stamps its own.
    fn from(value: EscapeBlockMeta) -> Self {
        BlockMeta {
            id: value.id,
            kind: BlockKind::from(value.kind),
            cmd: value.cmd,
            cwd: value.cwd,
            shell: value.shell,
            started_at: None,
            finished_at: None,
            exit_code: value.exit_code,
            is_finished: false,
        }
    }
}

/// The key OTTY gives a block when it makes it. No other block of the
/// terminal ever gets the same key, so the snapshot, the terminal widget and
/// the app name a block by it.
///
/// The id the shell sends ([`BlockMeta::id`]) is no key: a shell started
/// inside another one counts its blocks from 1 again, so two blocks of one
/// terminal can have the same id. It is only used to find the running block
/// an `exit` event of the shell belongs to.
///
/// # Examples
///
/// ```
/// use otty_surface::BlockKey;
///
/// assert_ne!(BlockKey::new(1), BlockKey::new(2));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BlockKey(u64);

impl BlockKey {
    /// The key numbered `value`. The block surface numbers the blocks it
    /// makes itself; a host or a test that builds a snapshot by hand picks
    /// the numbers.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Snapshot entry describing a block's extent within the viewport.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockSnapshot {
    /// The key of the block: it names the block in every place after the
    /// block surface. Unlike `meta.id`, it never repeats in a terminal.
    pub key: BlockKey,
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
    /// Whether the block has a status line: the empty line OTTY adds above
    /// the text of a block when its command starts (a `preexec` event). It
    /// is not terminal text, so the text of the block never contains it.
    pub has_status_line: bool,
    /// Whether the status line is on screen. It is the first line of the
    /// block, at `start_line`, and counts in `line_count`. It is hidden when
    /// the start of the block is above the viewport, and a full-screen
    /// program shows none.
    pub is_status_line_visible: bool,
    /// Whether the block can be folded: a finished command block with
    /// command lines and output under them. It stays `true` while the
    /// block is folded.
    pub is_foldable: bool,
    /// How many lines of output the fold hides, while the block is folded:
    /// the history shows only its status line and its command lines, and
    /// `line_count` counts only those. `None` while the block is not
    /// folded. The empty line the cursor waited on when the next prompt
    /// came is not output and is not counted; the history does not show it
    /// for an open block either.
    pub hidden_line_count: Option<usize>,
    /// How many lines at the start of the text of the block hold its
    /// command lines: the prompt and the typed command, a line the terminal
    /// wrapped counting once for each row. They are lines of `cached_text`
    /// when the block has it, counted when that text was made, since a
    /// resize wraps the grid again but not that text; else they are lines
    /// of the grid now. 0 for a block no `preexec` started.
    pub command_line_count: usize,
    /// How many lines of the text of the block are above the top edge of
    /// the pane: 0 while its first command line is on screen or below it,
    /// all of them for a block that is wholly above. The status line is
    /// not text and is not counted, and neither is the output a fold hides:
    /// only the lines the history shows. A full-screen program has none.
    pub text_lines_above: usize,
    /// How many lines the block has in the whole history, on screen or
    /// not: its status line and its text as the history shows it (only the
    /// command lines while it is folded; without the empty line the cursor
    /// waited on once the next prompt came). The blocks follow each other
    /// in the history, so the lines of the blocks before it tell where it
    /// starts. A full-screen program counts its screen.
    pub history_line_count: usize,
}

impl BlockSnapshot {
    /// Whether the block is folded: see [`Self::hidden_line_count`].
    pub fn is_folded(&self) -> bool {
        self.hidden_line_count.is_some()
    }

    /// Whether a command started the block. Only such a block is a block
    /// for the user: it can be hovered, selected and folded, and it has a
    /// status. The prompt and the block a terminal starts with are not.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_surface::{BlockKind, BlockSnapshot};
    ///
    /// let mut block = BlockSnapshot::default();
    /// block.meta.kind = BlockKind::Start;
    ///
    /// assert!(!block.is_command());
    /// ```
    pub fn is_command(&self) -> bool {
        self.meta.kind == BlockKind::Command
    }

    /// Whether the command of the block ended with an exit code other than
    /// 0. A command that still runs has not failed.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_surface::BlockSnapshot;
    ///
    /// let mut block = BlockSnapshot::default();
    /// block.meta.is_finished = true;
    /// block.meta.exit_code = Some(101);
    ///
    /// assert!(block.is_failed());
    /// ```
    pub fn is_failed(&self) -> bool {
        self.meta.is_finished
            && self.meta.exit_code.is_some_and(|code| code != 0)
    }

    /// The lines of the block on screen that hold its text: the first one
    /// and how many. The status line is left out.
    pub(crate) fn text_lines(&self) -> (i32, usize) {
        if self.is_status_line_visible && self.line_count > 0 {
            return (self.start_line + 1, self.line_count - 1);
        }

        (self.start_line, self.line_count)
    }
}
