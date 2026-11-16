use std::collections::HashMap;
use std::path::{Component, Path};

use iced::widget::pane_grid;
use otty_ui_kit::components::block_outcome::BlockOutcome;
use otty_ui_kit::components::target_dot::TargetKind;
use otty_ui_term::{BlockKey, BlockSnapshot, SnapshotView};

use super::block_duration::{finished_duration_text, running_duration_text};
use super::blocks::{running_program_from_blocks, terminal_cwd_from_blocks};
use super::pane_fit;
use super::state::TerminalWorkspaceState;
use super::types::TerminalEntry;

/// Longest folder path, counted after `~` or `/`, shown in full.
const SHORT_PATH_FOLDERS: usize = 3;
/// Folders kept at the end of a longer path.
const SHORT_PATH_TAIL: usize = 2;

/// View model for a single terminal tab, used by the pane grid view.
#[derive(Clone, Copy)]
pub(crate) struct PaneGridViewModel<'a> {
    pub(crate) tab_id: u64,
    pub(crate) panes: &'a pane_grid::State<u64>,
    pub(crate) terminals: &'a HashMap<u64, TerminalEntry>,
    pub(crate) focus: Option<pane_grid::Pane>,
    /// Name shown in the machine chip of every pane header: this
    /// computer's host name or the SSH host.
    pub(crate) machine: &'a str,
    /// Dot drawn in the machine chip.
    pub(crate) machine_target: TargetKind,
    /// Smallest pane width and height for dragging and layout: the full
    /// minimum, or less when the panes no longer fit the grid.
    pub(crate) min_pane_size: f32,
    /// The time now, in milliseconds since 1970: the clock the terminal
    /// stamps the blocks with. A running command counts its time up to it.
    pub(crate) now_ms: i64,
}

/// View model for the entire terminal workspace, keyed by active tab id.
#[derive(Clone, Copy)]
pub(crate) struct TerminalWorkspaceViewModel<'a> {
    pub(crate) tab: Option<PaneGridViewModel<'a>>,
}

impl<'a> TerminalWorkspaceViewModel<'a> {
    /// Build the view model of the tab in front from the workspace state at
    /// `now_ms`, milliseconds since 1970. `tab` is `None` when the tab in
    /// front is not a terminal tab.
    pub(super) fn from_state(
        state: &'a TerminalWorkspaceState,
        now_ms: i64,
    ) -> Self {
        let tab = state.active_tab_id().and_then(|tab_id| {
            let tab = state.tab(tab_id)?;
            let machine = tab.machine();

            Some(PaneGridViewModel {
                tab_id,
                panes: tab.panes(),
                terminals: tab.terminals(),
                focus: tab.focus(),
                machine: &machine.name,
                machine_target: machine.target,
                min_pane_size: pane_fit::fitting_min_size(
                    tab.panes().layout(),
                    tab.grid_size(),
                ),
                now_ms,
            })
        });

        Self { tab }
    }
}

/// What a pane runs and where. The pane header, the status bar and the
/// session row show parts of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PaneLabel {
    program: String,
    folder_name: Option<String>,
    folder_path: Option<String>,
}

impl PaneLabel {
    /// Return the program name.
    pub(crate) fn program(&self) -> &str {
        &self.program
    }

    /// Return the last folder of the working directory (`~` for home), if
    /// the shell has reported one.
    pub(crate) fn folder_name(&self) -> Option<&str> {
        self.folder_name.as_deref()
    }

    /// Return the working directory, shortened to its last folders when
    /// long, if the shell has reported one.
    pub(crate) fn folder_path(&self) -> Option<&str> {
        self.folder_path.as_deref()
    }

    /// Build the label of a pane: the running command, or the name the pane
    /// opened with while the shell waits at the prompt. A folder inside
    /// `home` is written with `~`.
    ///
    /// The blocks are read from the terminal's latest snapshot in place,
    /// without copying them.
    pub(crate) fn for_entry(
        entry: &TerminalEntry,
        home: Option<&Path>,
    ) -> Self {
        let snapshot = entry.terminal().snapshot();
        let view = snapshot.view();
        let blocks = view.blocks();
        let program =
            running_program_from_blocks(blocks).unwrap_or(entry.title());
        let cwd = terminal_cwd_from_blocks(blocks);

        Self::new(program, cwd.as_deref(), home)
    }

    /// Build the label. A folder inside `home` is written with `~`.
    pub(crate) fn new(
        program: &str,
        cwd: Option<&Path>,
        home: Option<&Path>,
    ) -> Self {
        Self {
            program: program.to_string(),
            folder_name: cwd.map(|cwd| folder_name(cwd, home)),
            folder_path: cwd.map(|cwd| short_folder_path(cwd, home)),
        }
    }
}

/// The status of a command block: how its command ended and how long it
/// took, or how long it has run so far, and for a folded block how many
/// lines the fold hides. The status line of the block shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BlockStatus {
    block_key: BlockKey,
    outcome: BlockOutcome,
    exit_code: Option<i32>,
    duration: String,
    hidden_line_count: Option<usize>,
}

impl BlockStatus {
    /// Return the key of the block.
    pub(crate) fn block_key(&self) -> BlockKey {
        self.block_key
    }

    /// Return how the command ended.
    pub(crate) fn outcome(&self) -> BlockOutcome {
        self.outcome
    }

    /// Return the exit code of a command that failed, which the status
    /// shows as `exit 101`. `None` for any other outcome.
    pub(crate) fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    /// Return how long the command took, or has run so far: `0.2 s`,
    /// `12 s`, `1 m 12 s`.
    pub(crate) fn duration(&self) -> &str {
        &self.duration
    }

    /// Return how many lines of output the fold of a folded block hides;
    /// the status line shows it after the status. `None` for an open block.
    pub(crate) fn hidden_line_count(&self) -> Option<usize> {
        self.hidden_line_count
    }

    /// Build the status of `block` at `now_ms`, milliseconds since 1970.
    /// `None` for a block no command started: the prompt and the block a
    /// terminal starts with.
    pub(crate) fn of_block(block: &BlockSnapshot, now_ms: i64) -> Option<Self> {
        if !block.is_command() {
            return None;
        }

        let started_at = block.meta.started_at?;
        let hidden_line_count = block.hidden_line_count;

        if !block.meta.is_finished {
            return Some(Self {
                block_key: block.key,
                outcome: BlockOutcome::Running,
                exit_code: None,
                duration: running_duration_text(started_at, now_ms),
                hidden_line_count,
            });
        }

        let finished_at = block.meta.finished_at?;
        let (outcome, exit_code) = match block.meta.exit_code {
            Some(0) => (BlockOutcome::Ok, None),
            Some(code) => (BlockOutcome::Failed, Some(code)),
            None => (BlockOutcome::Unknown, None),
        };

        Some(Self {
            block_key: block.key,
            outcome,
            exit_code,
            duration: finished_duration_text(started_at, finished_at),
            hidden_line_count,
        })
    }
}

/// What folding does to a block: what the fold arrow and the fold item of
/// the right-click menu offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FoldAction {
    /// Fold the block, which shows its output. The arrow points down.
    Collapse,
    /// Unfold the folded block. The arrow points right.
    Expand,
}

impl FoldAction {
    /// Build what folding offers for `block`: `Expand` for a folded block,
    /// `Collapse` for one that can be folded, `None` for any other block
    /// (the prompt, a running command, a command with no output).
    fn of_block(block: &BlockSnapshot) -> Option<Self> {
        if block.is_folded() {
            return Some(Self::Expand);
        }

        block.is_foldable.then_some(Self::Collapse)
    }
}

/// The fold arrow of a command block, in the left inset of its status line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BlockFold {
    block_key: BlockKey,
    arrow: FoldAction,
}

impl BlockFold {
    /// Return the key of the block.
    pub(crate) fn block_key(&self) -> BlockKey {
        self.block_key
    }

    /// Return what pressing the arrow does, which is also the way it
    /// points.
    pub(crate) fn arrow(&self) -> FoldAction {
        self.arrow
    }

    /// Build the fold arrow of `block` when the pointer is over
    /// `hovered_block`. A folded block always shows its arrow; a block that
    /// can be folded shows it only under the pointer. `None` for any other
    /// block, and for a block whose status line is not on screen.
    pub(crate) fn of_block(
        block: &BlockSnapshot,
        hovered_block: Option<BlockKey>,
    ) -> Option<Self> {
        if !block.is_status_line_visible {
            return None;
        }

        let is_hovered = hovered_block == Some(block.key);
        let arrow = match FoldAction::of_block(block)? {
            FoldAction::Collapse if !is_hovered => return None,
            arrow => arrow,
        };

        Some(Self {
            block_key: block.key,
            arrow,
        })
    }
}

/// What the right-click menu offers for the block it acts on, besides
/// copying its output and its command, and the key of that block, which
/// every block item sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BlockMenu {
    block_key: BlockKey,
    fold: Option<FoldAction>,
    can_run_again: bool,
}

impl BlockMenu {
    /// Return the key of the block the menu acts on.
    pub(crate) fn block_key(&self) -> BlockKey {
        self.block_key
    }

    /// Return the fold item: `Collapse`, `Expand`, or none for a block
    /// that can not be folded.
    pub(crate) fn fold(&self) -> Option<FoldAction> {
        self.fold
    }

    /// Return whether the menu offers `Run again`.
    pub(crate) fn can_run_again(&self) -> bool {
        self.can_run_again
    }

    /// Build the menu items of the block `block_key` in the `view` of its
    /// terminal. `Run again` is offered when the terminal has a command to
    /// type for the block (see [`SnapshotView::run_again_command`]). A
    /// block that is gone offers only the copy items, which then copy
    /// nothing.
    pub(crate) fn of_block(
        view: &SnapshotView<'_>,
        block_key: BlockKey,
    ) -> Self {
        let Some(block) = view.block(block_key) else {
            return Self {
                block_key,
                fold: None,
                can_run_again: false,
            };
        };

        Self {
            block_key,
            fold: FoldAction::of_block(block),
            can_run_again: view.run_again_command(block_key).is_some(),
        }
    }
}

#[cfg(test)]
impl BlockMenu {
    /// Build the menu items of the block `block_key` that offers `fold`
    /// and, when `can_run_again` is set, `Run again`.
    pub(crate) fn new(
        block_key: BlockKey,
        fold: Option<FoldAction>,
        can_run_again: bool,
    ) -> Self {
        Self {
            block_key,
            fold,
            can_run_again,
        }
    }
}

/// The sticky header of the command block at the top of the pane whose
/// first line is above the top edge: the command, its status and how many
/// of its lines are above.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StickyHeader {
    status: BlockStatus,
    command: String,
    lines_above: usize,
}

impl StickyHeader {
    /// Return the key of the block.
    pub(crate) fn block_key(&self) -> BlockKey {
        self.status.block_key()
    }

    /// Return how the command ended.
    pub(crate) fn outcome(&self) -> BlockOutcome {
        self.status.outcome()
    }

    /// Return the exit code of a command that failed; `None` for any
    /// other outcome.
    pub(crate) fn exit_code(&self) -> Option<i32> {
        self.status.exit_code()
    }

    /// Return how long the command took, or has run so far.
    pub(crate) fn duration(&self) -> &str {
        self.status.duration()
    }

    /// Return the command on one line: a command typed over several lines
    /// has them joined with spaces.
    pub(crate) fn command(&self) -> &str {
        &self.command
    }

    /// Return how many lines of the text of the block are above the top
    /// edge of the pane. It is 0 while only the status line is above.
    pub(crate) fn lines_above(&self) -> usize {
        self.lines_above
    }
}

/// Build the statuses to draw in the pane at `now_ms`, milliseconds since
/// 1970: one for every block whose status line is on screen.
pub(crate) fn visible_block_statuses(
    blocks: &[BlockSnapshot],
    now_ms: i64,
) -> Vec<BlockStatus> {
    blocks
        .iter()
        .filter(|block| block.is_status_line_visible)
        .filter_map(|block| BlockStatus::of_block(block, now_ms))
        .collect()
}

/// Build the fold arrows to draw in the pane while the pointer is over
/// `hovered_block`: on every folded block and on the block under the
/// pointer when it can be folded.
pub(crate) fn visible_block_folds(
    blocks: &[BlockSnapshot],
    hovered_block: Option<BlockKey>,
) -> Vec<BlockFold> {
    blocks
        .iter()
        .filter_map(|block| BlockFold::of_block(block, hovered_block))
        .collect()
}

/// Build the sticky header of the pane at `now_ms`, milliseconds since
/// 1970, for the blocks of a snapshot. The block at the top of
/// the pane is the first one with a line on screen. It gets a header when
/// it is a command block with a status line and the pane does not show
/// that first line of the block: it is above the top edge, or more than
/// half of it is (see [`otty_ui_term::Terminal::blocks_in_pane`]); the
/// prompt and a block no command started get none. While a full-screen
/// program is on (`is_full_screen`: the snapshot says so now) there is no
/// header; a block whose full-screen program quit gets one as any other
/// block.
pub(crate) fn sticky_header(
    blocks: &[BlockSnapshot],
    is_full_screen: bool,
    now_ms: i64,
) -> Option<StickyHeader> {
    if is_full_screen {
        return None;
    }

    let top = blocks.iter().find(|block| block.line_count > 0)?;
    if !top.is_command() || top.is_status_line_visible {
        return None;
    }

    let status = BlockStatus::of_block(top, now_ms)?;
    let command = top
        .meta
        .cmd
        .as_deref()
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    Some(StickyHeader {
        status,
        command,
        lines_above: top.text_lines_above,
    })
}

fn folder_name(cwd: &Path, home: Option<&Path>) -> String {
    if home == Some(cwd) {
        return String::from("~");
    }

    match cwd.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => cwd.display().to_string(),
    }
}

fn short_folder_path(cwd: &Path, home: Option<&Path>) -> String {
    let (start, rest) = match home.and_then(|home| cwd.strip_prefix(home).ok())
    {
        Some(rest) => ("~", rest),
        None => ("", cwd),
    };
    let folders: Vec<_> = rest
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy()),
            _ => None,
        })
        .collect();

    if folders.is_empty() {
        return match start {
            "" => cwd.display().to_string(),
            _ => String::from(start),
        };
    }
    if folders.len() <= SHORT_PATH_FOLDERS {
        return format!("{start}/{}", folders.join("/"));
    }

    let tail = folders[folders.len() - SHORT_PATH_TAIL..].join("/");
    format!("{start}/…/{tail}")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use otty_app_testing::blocks::{
        command_block, finished, key_of, prompt_block,
    };
    use otty_ui_kit::components::block_outcome::BlockOutcome;
    use otty_ui_term::{BlockKey, BlockKind, BlockSnapshot, SnapshotOwned};

    use super::{
        BlockMenu, FoldAction, PaneLabel, sticky_header, visible_block_folds,
        visible_block_statuses,
    };

    const HOME: &str = "/home/ann";

    fn label(cwd: &str) -> PaneLabel {
        PaneLabel::new("zsh", Some(Path::new(cwd)), Some(Path::new(HOME)))
    }

    #[test]
    fn given_folder_inside_home_when_label_built_then_home_is_tilde() {
        let label = label("/home/ann/projects/otty");

        assert_eq!(label.program(), "zsh");
        assert_eq!(label.folder_name(), Some("otty"));
        assert_eq!(label.folder_path(), Some("~/projects/otty"));
    }

    #[test]
    fn given_folder_equal_to_home_when_label_built_then_folder_is_tilde() {
        let label = label(HOME);

        assert_eq!(label.folder_name(), Some("~"));
        assert_eq!(label.folder_path(), Some("~"));
    }

    #[test]
    fn given_folder_outside_home_when_label_built_then_path_is_unchanged() {
        let label = label("/var/log");

        assert_eq!(label.folder_name(), Some("log"));
        assert_eq!(label.folder_path(), Some("/var/log"));
    }

    #[test]
    fn given_root_folder_when_label_built_then_folder_is_slash() {
        let label = label("/");

        assert_eq!(label.folder_name(), Some("/"));
        assert_eq!(label.folder_path(), Some("/"));
    }

    #[test]
    fn given_sibling_with_same_prefix_as_home_when_label_built_then_not_shortened()
     {
        let label = label("/home/anna");

        assert_eq!(label.folder_path(), Some("/home/anna"));
    }

    #[test]
    fn given_three_folders_after_home_when_label_built_then_path_is_full() {
        let label = label("/home/ann/projects/main/ottyrun");

        assert_eq!(label.folder_path(), Some("~/projects/main/ottyrun"));
    }

    #[test]
    fn given_deep_folder_inside_home_when_label_built_then_last_two_stay() {
        let label = label("/home/ann/projects/main/ottyrun/otty");

        assert_eq!(label.folder_name(), Some("otty"));
        assert_eq!(label.folder_path(), Some("~/…/ottyrun/otty"));
    }

    #[test]
    fn given_deep_folder_outside_home_when_label_built_then_last_two_stay() {
        let label = label("/var/lib/containers/storage/overlay");

        assert_eq!(label.folder_path(), Some("/…/storage/overlay"));
    }

    #[test]
    fn given_unknown_folder_when_label_built_then_folder_is_missing() {
        let label = PaneLabel::new("zsh", None, None);

        assert_eq!(label.folder_name(), None);
        assert_eq!(label.folder_path(), None);
    }

    /// A start time like the real ones: milliseconds since 1970.
    const START: i64 = 1_800_000_000_000;

    #[test]
    fn given_block_with_exit_code_0_when_statuses_built_then_it_is_ok_with_its_duration()
     {
        let blocks = [finished(command_block("ls", START), Some(0), 200)];

        let statuses = visible_block_statuses(&blocks, START + 60_000);

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].block_key(), key_of("ls"));
        assert_eq!(statuses[0].outcome(), BlockOutcome::Ok);
        assert_eq!(statuses[0].exit_code(), None);
        assert_eq!(statuses[0].duration(), "0.2 s");
    }

    #[test]
    fn given_block_with_exit_code_101_when_statuses_built_then_it_failed_with_the_code()
     {
        let blocks = [finished(
            command_block("cargo test", START),
            Some(101),
            1_900,
        )];

        let statuses = visible_block_statuses(&blocks, START + 60_000);

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].outcome(), BlockOutcome::Failed);
        assert_eq!(statuses[0].exit_code(), Some(101));
        assert_eq!(statuses[0].duration(), "1.9 s");
    }

    #[test]
    fn given_unfinished_command_when_statuses_built_then_it_runs_for_whole_seconds_until_now()
     {
        let blocks = [command_block("sleep 30", START)];

        let statuses = visible_block_statuses(&blocks, START + 12_900);

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].outcome(), BlockOutcome::Running);
        assert_eq!(statuses[0].exit_code(), None);
        assert_eq!(statuses[0].duration(), "12 s");
    }

    #[test]
    fn given_block_finished_without_exit_code_when_statuses_built_then_only_its_duration_is_known()
     {
        let blocks = [finished(command_block("ls", START), None, 300)];

        let statuses = visible_block_statuses(&blocks, START + 60_000);

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].outcome(), BlockOutcome::Unknown);
        assert_eq!(statuses[0].exit_code(), None);
        assert_eq!(statuses[0].duration(), "0.3 s");
    }

    #[test]
    fn given_prompt_block_when_statuses_built_then_it_gives_nothing() {
        let blocks = [prompt_block()];

        let statuses = visible_block_statuses(&blocks, START);

        assert!(statuses.is_empty());
    }

    /// `ls`, finished, with one command line and 3 lines of output, all on
    /// screen.
    fn foldable(id: &str) -> BlockSnapshot {
        let mut block = finished(command_block(id, START), Some(0), 100);
        block.line_count = 1 + 1 + 3;
        block.is_foldable = true;

        block
    }

    /// The same block folded: its status line and its command line stay,
    /// and 3 lines are hidden.
    fn folded(id: &str) -> BlockSnapshot {
        let mut block = foldable(id);
        block.line_count = 2;
        block.hidden_line_count = Some(3);

        block
    }

    #[test]
    fn given_hovered_block_that_can_be_folded_when_folds_built_then_it_has_a_down_arrow()
     {
        let blocks = [foldable("ls"), foldable("pwd")];

        let folds = visible_block_folds(&blocks, Some(key_of("ls")));

        assert_eq!(folds.len(), 1);
        assert_eq!(folds[0].block_key(), key_of("ls"));
        assert_eq!(folds[0].arrow(), FoldAction::Collapse);
    }

    #[test]
    fn given_folded_block_away_from_the_pointer_when_folds_built_then_it_has_a_right_arrow()
     {
        let blocks = [folded("seq"), foldable("ls")];

        let folds = visible_block_folds(&blocks, None);

        assert_eq!(folds.len(), 1);
        assert_eq!(folds[0].block_key(), key_of("seq"));
        assert_eq!(folds[0].arrow(), FoldAction::Expand);
    }

    #[test]
    fn given_hovered_folded_block_when_folds_built_then_it_keeps_its_right_arrow()
     {
        let blocks = [folded("seq")];

        let folds = visible_block_folds(&blocks, Some(key_of("seq")));

        assert_eq!(folds.len(), 1);
        assert_eq!(folds[0].arrow(), FoldAction::Expand);
    }

    #[test]
    fn given_folded_block_when_statuses_built_then_its_status_says_how_many_lines_it_hides()
     {
        let blocks = [folded("seq"), foldable("ls")];

        let statuses = visible_block_statuses(&blocks, START + 60_000);

        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0].hidden_line_count(), Some(3));
        assert_eq!(statuses[1].hidden_line_count(), None);
    }

    #[test]
    fn given_hovered_running_block_or_prompt_when_folds_built_then_they_give_nothing()
     {
        let running = command_block("sleep 30", START);
        let blocks = [running, prompt_block()];

        let over_running =
            visible_block_folds(&blocks, Some(key_of("sleep 30")));
        let over_prompt = visible_block_folds(&blocks, Some(key_of("prompt")));

        assert!(over_running.is_empty());
        assert!(over_prompt.is_empty());
    }

    #[test]
    fn given_folded_block_whose_status_line_is_out_of_the_pane_when_folds_built_then_it_gives_nothing()
     {
        let mut scrolled = folded("seq");
        scrolled.is_status_line_visible = false;
        let blocks = [scrolled];

        let folds = visible_block_folds(&blocks, Some(key_of("seq")));

        assert!(folds.is_empty());
    }

    #[test]
    fn given_folded_block_with_only_its_status_line_on_screen_when_folds_built_then_it_has_its_arrow()
     {
        let mut cut = folded("make");
        cut.line_count = 1;
        let blocks = [cut];

        let folds = visible_block_folds(&blocks, None);

        assert_eq!(folds.len(), 1);
        assert_eq!(folds[0].arrow(), FoldAction::Expand);
    }

    /// The menu of the block `block_key` in a terminal that shows
    /// `blocks`.
    fn menu_of(blocks: &[BlockSnapshot], block_key: BlockKey) -> BlockMenu {
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks = blocks.to_vec();

        BlockMenu::of_block(&snapshot.view(), block_key)
    }

    /// A finished block that ran `cmd`.
    fn with_command(mut block: BlockSnapshot, cmd: &str) -> BlockSnapshot {
        block.meta.cmd = Some(String::from(cmd));

        block
    }

    #[test]
    fn given_open_block_that_can_be_folded_when_its_menu_built_then_it_offers_collapse()
     {
        let blocks = [foldable("ls"), prompt_block()];

        let menu = menu_of(&blocks, key_of("ls"));

        assert_eq!(menu.fold(), Some(FoldAction::Collapse));
    }

    #[test]
    fn given_folded_block_when_its_menu_built_then_it_offers_expand() {
        let blocks = [folded("seq"), prompt_block()];

        let menu = menu_of(&blocks, key_of("seq"));

        assert_eq!(menu.fold(), Some(FoldAction::Expand));
    }

    #[test]
    fn given_running_block_when_its_menu_built_then_it_offers_neither_collapse_nor_expand_nor_run_again()
     {
        let blocks =
            [with_command(command_block("sleep 30", START), "sleep 30")];

        let menu = menu_of(&blocks, key_of("sleep 30"));

        assert_eq!(menu.fold(), None);
        assert!(!menu.can_run_again());
    }

    #[test]
    fn given_command_with_no_output_when_its_menu_built_then_it_offers_no_fold()
    {
        let cd = with_command(
            finished(command_block("cd", START), Some(0), 100),
            "cd /tmp",
        );
        let blocks = [cd, prompt_block()];

        let menu = menu_of(&blocks, key_of("cd"));

        assert_eq!(menu.fold(), None);
        assert!(menu.can_run_again());
    }

    #[test]
    fn given_finished_block_while_the_prompt_waits_when_its_menu_built_then_it_offers_run_again()
     {
        let blocks = [with_command(foldable("ls"), "ls"), prompt_block()];

        let menu = menu_of(&blocks, key_of("ls"));

        assert!(menu.can_run_again());
    }

    #[test]
    fn given_finished_block_while_a_command_runs_when_its_menu_built_then_it_offers_no_run_again()
     {
        let blocks = [
            with_command(foldable("ls"), "ls"),
            with_command(command_block("sleep 30", START), "sleep 30"),
        ];

        let menu = menu_of(&blocks, key_of("ls"));

        assert!(!menu.can_run_again());
        assert_eq!(menu.fold(), Some(FoldAction::Collapse));
    }

    #[test]
    fn given_block_without_a_command_when_its_menu_built_then_it_offers_no_run_again()
     {
        let mut boot = finished(command_block("", START), None, 100);
        boot.meta.cmd = Some(String::from("  "));
        let blocks = [boot, prompt_block()];

        let menu = menu_of(&blocks, key_of(""));

        assert!(!menu.can_run_again());
    }

    #[test]
    fn given_block_that_is_gone_when_its_menu_built_then_it_offers_nothing() {
        let blocks = [prompt_block()];

        let menu = menu_of(&blocks, key_of("ls"));

        assert_eq!(menu, BlockMenu::new(key_of("ls"), None, false));
    }

    /// The menu items act on the block the menu was built for, so they
    /// carry its key.
    #[test]
    fn given_block_when_its_menu_built_then_the_menu_names_the_block() {
        let blocks = [foldable("ls"), prompt_block()];

        let menu = menu_of(&blocks, key_of("ls"));

        assert_eq!(menu.block_key(), key_of("ls"));
    }

    #[test]
    fn given_status_line_scrolled_out_of_the_pane_when_statuses_built_then_the_block_gives_nothing()
     {
        let mut scrolled = finished(command_block("seq", START), Some(0), 100);
        scrolled.is_status_line_visible = false;
        let mut out_of_view = command_block("sleep 30", START);
        out_of_view.line_count = 0;
        out_of_view.is_status_line_visible = false;

        let blocks = [scrolled, out_of_view];

        let statuses = visible_block_statuses(&blocks, START + 1_000);

        assert!(statuses.is_empty());
    }

    /// `cmd`, finished with `exit_code` after 48.2 s, whose first line is
    /// `lines_above` lines above the top edge of the pane: its status line
    /// is out of the pane, and it fills the pane of 20 lines.
    fn started_above(
        cmd: &str,
        exit_code: Option<i32>,
        lines_above: usize,
    ) -> BlockSnapshot {
        let mut block = finished(command_block(cmd, START), exit_code, 48_200);
        block.meta.cmd = Some(String::from(cmd));
        block.start_line = 0;
        block.line_count = 20;
        block.is_status_line_visible = false;
        block.text_lines_above = lines_above;

        block
    }

    /// The prompt at the bottom of the pane of 20 lines.
    fn prompt_at_the_bottom() -> BlockSnapshot {
        BlockSnapshot {
            start_line: 19,
            ..prompt_block()
        }
    }

    /// A finished block wholly above the pane: it is in the snapshot with
    /// no line on screen.
    fn wholly_above(cmd: &str) -> BlockSnapshot {
        let mut block = finished(command_block(cmd, START), Some(0), 100);
        block.line_count = 0;
        block.is_status_line_visible = false;
        block.text_lines_above = 3;

        block
    }

    #[test]
    fn given_block_of_500_lines_412_of_them_above_the_pane_when_header_built_then_it_shows_its_command_status_and_412()
     {
        let mut cargo = started_above("cargo build --release", Some(0), 412);
        cargo.line_count = 19;
        let blocks = [wholly_above("ls"), cargo, prompt_at_the_bottom()];

        let header = sticky_header(&blocks, false, START + 60_000)
            .expect("a header is shown");

        assert_eq!(header.block_key(), key_of("cargo build --release"));
        assert_eq!(header.command(), "cargo build --release");
        assert_eq!(header.lines_above(), 412);
        assert_eq!(header.outcome(), BlockOutcome::Ok);
        assert_eq!(header.exit_code(), None);
        assert_eq!(header.duration(), "48.2 s");
    }

    #[test]
    fn given_failed_block_above_the_pane_when_header_built_then_it_has_the_exit_code()
     {
        let blocks = [started_above("cargo test", Some(101), 40)];

        let header = sticky_header(&blocks, false, START + 60_000)
            .expect("a header is shown");

        assert_eq!(header.outcome(), BlockOutcome::Failed);
        assert_eq!(header.exit_code(), Some(101));
    }

    #[test]
    fn given_running_block_above_the_pane_when_header_built_then_it_runs_for_whole_seconds_until_now()
     {
        let mut running = started_above("cargo build", None, 30);
        running.meta.is_finished = false;
        running.meta.finished_at = None;
        let blocks = [running];

        let header = sticky_header(&blocks, false, START + 12_900)
            .expect("a header is shown");

        assert_eq!(header.outcome(), BlockOutcome::Running);
        assert_eq!(header.duration(), "12 s");
    }

    #[test]
    fn given_command_typed_over_two_lines_when_header_built_then_its_command_is_on_one_line()
     {
        let mut block = started_above("for", Some(0), 5);
        block.meta.cmd = Some(String::from("for i in 1 2\n  do echo $i; done"));
        let blocks = [block];

        let header = sticky_header(&blocks, false, START + 60_000)
            .expect("a header is shown");

        assert_eq!(header.command(), "for i in 1 2 do echo $i; done");
    }

    #[test]
    fn given_block_whose_first_line_is_on_screen_when_header_built_then_there_is_none()
     {
        let mut ls = finished(command_block("ls", START), Some(0), 100);
        ls.line_count = 5;
        let mut prompt = prompt_at_the_bottom();
        prompt.start_line = 5;
        let blocks = [wholly_above("seq 1 500"), ls, prompt];

        assert_eq!(sticky_header(&blocks, false, START + 60_000), None);
    }

    #[test]
    fn given_prompt_at_the_top_of_the_pane_when_header_built_then_there_is_none()
     {
        let mut prompt = prompt_at_the_bottom();
        prompt.start_line = 0;
        prompt.line_count = 20;
        prompt.text_lines_above = 6;
        let blocks = [wholly_above("ls"), prompt];

        assert_eq!(sticky_header(&blocks, false, START + 60_000), None);
    }

    #[test]
    fn given_block_no_command_started_above_the_pane_when_header_built_then_there_is_none()
     {
        let mut boot = started_above("", None, 10);
        boot.meta.kind = BlockKind::Start;
        boot.meta.cmd = None;
        boot.meta.started_at = None;
        boot.has_status_line = false;
        let blocks = [boot];

        assert_eq!(sticky_header(&blocks, false, START + 60_000), None);
    }

    #[test]
    fn given_full_screen_program_when_header_built_then_there_is_none() {
        let mut vim = started_above("vim", None, 0);
        vim.meta.is_finished = false;
        vim.has_status_line = false;
        let blocks = [vim];

        assert_eq!(sticky_header(&blocks, true, START + 60_000), None);
    }

    /// Issue 17: a script ran a full-screen program, which quit, and then
    /// printed 412 lines. The full-screen program is off now, so the block
    /// gets its header.
    #[test]
    fn given_block_that_ran_a_full_screen_program_that_quit_when_header_built_then_it_has_one()
     {
        let mut script = started_above("./deploy.sh", None, 412);
        script.meta.is_finished = false;
        script.meta.finished_at = None;
        let blocks = [script];

        let header = sticky_header(&blocks, false, START + 1_000);

        assert_eq!(header.map(|h| h.lines_above()), Some(412));
    }

    /// Two blocks that ran `ls`, the newer one in a shell started inside
    /// the other one, which counts from 1 again: both have the shell id
    /// `cmd-1`, each has a key of its own.
    fn two_blocks_with_one_shell_id() -> [BlockSnapshot; 3] {
        let mut older = with_command(foldable("ls"), "ls");
        let mut newer = with_command(folded("pwd"), "pwd");
        for block in [&mut older, &mut newer] {
            block.meta.id = String::from("cmd-1");
        }

        [older, newer, prompt_block()]
    }

    /// Owner, issue 13: of two blocks with one shell id, the arrow under
    /// the pointer and the menu are those of the block the user points at.
    #[test]
    fn given_two_blocks_with_one_shell_id_when_the_older_one_is_acted_on_then_its_arrow_and_menu_are_built()
     {
        let blocks = two_blocks_with_one_shell_id();

        let folds = visible_block_folds(&blocks, Some(key_of("ls")));
        let older = menu_of(&blocks, key_of("ls"));
        let newer = menu_of(&blocks, key_of("pwd"));

        let keys: Vec<BlockKey> = folds.iter().map(|f| f.block_key()).collect();
        assert_eq!(keys, [key_of("ls"), key_of("pwd")]);
        assert_eq!(folds[0].arrow(), FoldAction::Collapse);
        assert_eq!(folds[1].arrow(), FoldAction::Expand);
        assert_eq!(older.fold(), Some(FoldAction::Collapse));
        assert_eq!(newer.fold(), Some(FoldAction::Expand));
    }
}
