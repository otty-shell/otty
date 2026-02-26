use std::collections::{BTreeMap, HashMap};

use iced::widget::{Id, pane_grid};
use iced::{Point, Size};
use otty_ui_term::SurfaceMode;
use otty_ui_term::settings::{Settings, ThemeSettings};

use super::errors::TerminalWorkspaceError;
use super::model::{BlockSelection, TerminalEntry, TerminalKind};

// ---------------------------------------------------------------------------
// Internal commands returned by state mutation helpers
// ---------------------------------------------------------------------------

/// Commands returned by state mutation helpers to be executed by the reducer.
pub(super) enum StateCommand {
    /// Nothing to do.
    None,
    /// Focus a terminal widget by its iced widget id.
    FocusTerminal(Id),
    /// Select the hovered block in a terminal widget.
    SelectHovered(Id),
    /// Focus a generic iced element by id.
    FocusElement(Id),
    /// Close the tab (all panes were removed).
    CloseTab { tab_id: u64 },
    /// Execute multiple commands.
    Batch(Vec<StateCommand>),
}

// ---------------------------------------------------------------------------
// Top-level workspace state
// ---------------------------------------------------------------------------

/// Per-widget terminal workspace state keyed by tab id.
#[derive(Default)]
pub(crate) struct TerminalWorkspaceState {
    tabs: BTreeMap<u64, TerminalTabState>,
}

impl TerminalWorkspaceState {
    /// Return terminal tab state by tab id.
    pub(crate) fn tab(&self, tab_id: u64) -> Option<&TerminalTabState> {
        self.tabs.get(&tab_id)
    }

    /// Return mutable terminal tab state by tab id.
    pub(crate) fn tab_mut(
        &mut self,
        tab_id: u64,
    ) -> Option<&mut TerminalTabState> {
        self.tabs.get_mut(&tab_id)
    }

    /// Insert terminal tab state.
    pub(crate) fn insert_tab(&mut self, tab_id: u64, tab: TerminalTabState) {
        self.tabs.insert(tab_id, tab);
    }

    /// Remove terminal tab state when tab is closed.
    pub(crate) fn remove_tab(
        &mut self,
        tab_id: u64,
    ) -> Option<TerminalTabState> {
        self.tabs.remove(&tab_id)
    }

    /// Iterate terminal tabs.
    pub(crate) fn tabs(
        &self,
    ) -> impl Iterator<Item = (&u64, &TerminalTabState)> {
        self.tabs.iter()
    }

    /// Iterate terminal tabs mutably.
    pub(crate) fn tabs_mut(
        &mut self,
    ) -> impl Iterator<Item = (&u64, &mut TerminalTabState)> {
        self.tabs.iter_mut()
    }
}

// ---------------------------------------------------------------------------
// Context menu state
// ---------------------------------------------------------------------------

/// State for a pane context menu.
#[derive(Debug, Clone)]
pub(crate) struct PaneContextMenuState {
    pane: pane_grid::Pane,
    cursor: Point,
    grid_size: Size,
    terminal_id: u64,
    focus_target: Id,
}

impl PaneContextMenuState {
    /// Create a new context menu state.
    pub(crate) fn new(
        pane: pane_grid::Pane,
        cursor: Point,
        grid_size: Size,
        terminal_id: u64,
    ) -> Self {
        Self {
            pane,
            cursor,
            grid_size,
            terminal_id,
            focus_target: Id::unique(),
        }
    }

    /// Return the pane this menu was opened on.
    pub(crate) fn pane(&self) -> pane_grid::Pane {
        self.pane
    }

    /// Return the cursor position at the time the menu was opened.
    pub(crate) fn cursor(&self) -> Point {
        self.cursor
    }

    /// Return the grid bounds used for menu placement.
    pub(crate) fn grid_size(&self) -> Size {
        self.grid_size
    }

    /// Return the terminal id for the pane.
    pub(crate) fn terminal_id(&self) -> u64 {
        self.terminal_id
    }

    /// Return the iced widget id used for focus trapping.
    pub(crate) fn focus_target(&self) -> &Id {
        &self.focus_target
    }
}

// ---------------------------------------------------------------------------
// Per-tab state
// ---------------------------------------------------------------------------

/// Runtime state for a single terminal tab with pane management and selection.
pub(crate) struct TerminalTabState {
    pub(super) tab_id: u64,
    pub(super) title: String,
    pub(super) kind: TerminalKind,
    pub(super) terminal_settings: Settings,
    pub(super) panes: pane_grid::State<u64>,
    pub(super) terminals: HashMap<u64, TerminalEntry>,
    pub(super) focus: Option<pane_grid::Pane>,
    pub(super) context_menu: Option<PaneContextMenuState>,
    pub(super) selected_block: Option<BlockSelection>,
    pub(super) default_title: String,
    pub(super) grid_cursor: Option<Point>,
    pub(super) grid_size: Size,
}

impl TerminalTabState {
    /// Create a new terminal tab with an initial pane.
    pub(crate) fn new(
        tab_id: u64,
        default_title: String,
        terminal_id: u64,
        settings: Settings,
        kind: TerminalKind,
    ) -> Result<(Self, Id), TerminalWorkspaceError> {
        let terminal =
            otty_ui_term::Terminal::new(terminal_id, settings.clone())
                .map_err(|err| TerminalWorkspaceError::Init {
                    message: format!("{err}"),
                })?;
        let widget_id = terminal.widget_id().clone();

        let (panes, initial_pane) = pane_grid::State::new(terminal_id);

        let mut terminals = HashMap::new();
        terminals.insert(
            terminal_id,
            TerminalEntry {
                terminal,
                title: default_title.clone(),
            },
        );

        let tab = Self {
            tab_id,
            title: default_title.clone(),
            kind,
            terminal_settings: settings,
            panes,
            terminals,
            focus: Some(initial_pane),
            context_menu: None,
            selected_block: None,
            default_title,
            grid_cursor: None,
            grid_size: Size::ZERO,
        };

        Ok((tab, widget_id))
    }

    /// Return the current tab title.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Report whether this terminal tab represents a shell session.
    pub(crate) fn is_shell(&self) -> bool {
        matches!(self.kind, TerminalKind::Shell)
    }

    /// Return the pane grid state.
    pub(crate) fn panes(&self) -> &pane_grid::State<u64> {
        &self.panes
    }

    /// Return the terminals map.
    pub(crate) fn terminals(&self) -> &HashMap<u64, TerminalEntry> {
        &self.terminals
    }

    /// Return the currently focused pane.
    pub(crate) fn focus(&self) -> Option<pane_grid::Pane> {
        self.focus
    }

    /// Return the context menu state, if active.
    pub(crate) fn context_menu(&self) -> Option<&PaneContextMenuState> {
        self.context_menu.as_ref()
    }

    /// Return the selected block, if any.
    pub(crate) fn selected_block(&self) -> Option<&BlockSelection> {
        self.selected_block.as_ref()
    }

    /// Check whether a terminal with the given id exists.
    pub(crate) fn contains_terminal(&self, id: u64) -> bool {
        self.terminals.contains_key(&id)
    }

    /// Resolve the terminal id for a pane.
    pub(crate) fn pane_terminal_id(
        &self,
        pane: pane_grid::Pane,
    ) -> Option<u64> {
        self.panes.get(pane).copied()
    }

    /// Return the terminal id of the focused pane.
    pub(crate) fn focused_terminal_id(&self) -> Option<u64> {
        let pane = self.focus?;
        self.pane_terminal_id(pane)
    }

    /// Return the focused terminal entry.
    pub(crate) fn focused_terminal_entry(&self) -> Option<&TerminalEntry> {
        let terminal_id = self.focused_terminal_id()?;
        self.terminals.get(&terminal_id)
    }

    /// Return a mutable terminal entry by id.
    pub(crate) fn terminal_entry_mut(
        &mut self,
        terminal_id: u64,
    ) -> Option<&mut TerminalEntry> {
        self.terminals.get_mut(&terminal_id)
    }

    /// Return the terminal id of the currently selected block.
    pub(crate) fn selected_block_terminal(&self) -> Option<u64> {
        self.selected_block
            .as_ref()
            .map(BlockSelection::terminal_id)
    }

    /// Set the selected block for a terminal.
    pub(crate) fn set_selected_block(
        &mut self,
        terminal_id: u64,
        block_id: String,
    ) {
        self.selected_block = Some(BlockSelection::new(terminal_id, block_id));
    }

    /// Clear any block selection.
    pub(crate) fn clear_selected_block(&mut self) {
        self.selected_block = None;
    }

    /// Clear block selection if it belongs to the given terminal.
    pub(crate) fn clear_selected_block_for_terminal(
        &mut self,
        terminal_id: u64,
    ) {
        if self
            .selected_block
            .as_ref()
            .map(|sel| sel.terminal_id() == terminal_id)
            .unwrap_or(false)
        {
            self.selected_block = None;
        }
    }

    /// Split the given pane along an axis with a new terminal.
    pub(super) fn split_pane(
        &mut self,
        pane: pane_grid::Pane,
        axis: pane_grid::Axis,
        terminal_id: u64,
    ) -> StateCommand {
        let split = self.panes.split(axis, pane, terminal_id);

        if let Some((new_pane, _)) = split {
            let terminal = match otty_ui_term::Terminal::new(
                terminal_id,
                self.terminal_settings.clone(),
            ) {
                Ok(terminal) => terminal,
                Err(err) => {
                    log::warn!("split pane terminal init failed: {err}");
                    let _ = self.panes.close(new_pane);
                    return StateCommand::None;
                },
            };
            let widget_id = terminal.widget_id().clone();

            self.terminals.insert(
                terminal_id,
                TerminalEntry {
                    terminal,
                    title: self.default_title.clone(),
                },
            );
            self.focus = Some(new_pane);
            self.context_menu = None;
            self.update_title_from_terminal(Some(terminal_id));

            return StateCommand::FocusTerminal(widget_id);
        }

        StateCommand::None
    }

    /// Close the given pane, returning a close-tab command when the
    /// last pane is removed.
    pub(super) fn close_pane(&mut self, pane: pane_grid::Pane) -> StateCommand {
        if self.panes.len() == 1 {
            return StateCommand::CloseTab {
                tab_id: self.tab_id,
            };
        }

        let result = self.panes.close(pane);
        if let Some((terminal_id, sibling)) = result {
            self.clear_selected_block_for_terminal(terminal_id);
            self.context_menu = None;
            self.terminals.remove(&terminal_id);

            let needs_focus = self.focus == Some(pane) || self.focus.is_none();
            if needs_focus {
                self.focus = Some(sibling);
                if let Some(next_id) = self.pane_terminal_id(sibling) {
                    if let Some(entry) = self.terminals.get(&next_id) {
                        let widget_id = entry.terminal.widget_id().clone();
                        self.update_title_from_terminal(Some(next_id));
                        return StateCommand::FocusTerminal(widget_id);
                    }
                }
            }

            return StateCommand::None;
        }

        StateCommand::None
    }

    /// Apply a terminal color palette to all panes in this tab.
    pub(crate) fn apply_theme(&mut self, palette: otty_ui_term::ColorPalette) {
        self.terminal_settings.theme =
            ThemeSettings::new(Box::new(palette.clone()));
        for entry in self.terminals.values_mut() {
            entry.terminal.change_theme(palette.clone());
        }
    }

    /// Focus a specific pane.
    pub(super) fn focus_pane(&mut self, pane: pane_grid::Pane) -> StateCommand {
        self.set_focus_on_pane(pane, true, true)
    }

    /// Handle a pane resize event.
    pub(crate) fn resize(&mut self, event: pane_grid::ResizeEvent) {
        self.panes.resize(event.split, event.ratio);
    }

    /// Open a context menu for the given pane.
    pub(super) fn open_context_menu(
        &mut self,
        pane: pane_grid::Pane,
        terminal_id: u64,
        cursor: Point,
        grid_size: Size,
    ) -> StateCommand {
        let Some((widget_id, snapshot)) =
            self.terminals.get(&terminal_id).map(|entry| {
                (
                    entry.terminal.widget_id().clone(),
                    entry.terminal.snapshot_arc(),
                )
            })
        else {
            return StateCommand::None;
        };
        if snapshot.view().mode.contains(SurfaceMode::ALT_SCREEN) {
            return StateCommand::None;
        }

        let focus_cmd = self.set_focus_on_pane(pane, false, false);
        let select_cmd = StateCommand::SelectHovered(widget_id);
        let menu_state =
            PaneContextMenuState::new(pane, cursor, grid_size, terminal_id);
        let menu_focus_cmd =
            StateCommand::FocusElement(menu_state.focus_target().clone());
        self.context_menu = Some(menu_state);

        StateCommand::Batch(vec![focus_cmd, select_cmd, menu_focus_cmd])
    }

    /// Close an open context menu.
    pub(super) fn close_context_menu(&mut self) -> StateCommand {
        if self.context_menu.take().is_some() {
            if let Some(pane) = self.focus {
                return self.set_focus_on_pane(pane, false, true);
            }
        }

        StateCommand::None
    }

    /// Handle a terminal widget event (title changes, shutdown, etc.).
    pub(super) fn handle_terminal_event(
        &mut self,
        event: otty_ui_term::Event,
    ) -> StateCommand {
        use otty_ui_term::Event::*;

        let terminal_id = *event.terminal_id();

        match event {
            Shutdown { .. } => {
                if let Some(pane) = self.pane_for_terminal(terminal_id) {
                    return self.close_pane(pane);
                }
            },
            TitleChanged { title, .. } => {
                if let Some(entry) = self.terminal_entry_mut(terminal_id) {
                    entry.title = title.clone();
                }
                if self.focused_terminal_id() == Some(terminal_id) {
                    self.title = title;
                }
            },
            ResetTitle { .. } => {
                let reset_title = self.default_title.clone();
                if let Some(entry) = self.terminal_entry_mut(terminal_id) {
                    entry.title = reset_title.clone();
                }
                if self.focused_terminal_id() == Some(terminal_id) {
                    self.title = reset_title;
                }
            },
            other => {
                if let Some(entry) = self.terminal_entry_mut(terminal_id) {
                    entry.terminal.handle(other);
                }
            },
        }

        StateCommand::None
    }

    /// Update the cached cursor position within the pane grid.
    pub(super) fn update_grid_cursor(
        &mut self,
        position: Point,
    ) -> StateCommand {
        self.grid_cursor = Some(Self::clamp_point(position, self.grid_size));
        StateCommand::None
    }

    /// Set the pane grid dimensions.
    pub(crate) fn set_grid_size(&mut self, size: Size) {
        self.grid_size = size;
        if let Some(cursor) = self.grid_cursor {
            self.grid_cursor = Some(Self::clamp_point(cursor, size));
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn grid_size(&self) -> Size {
        self.grid_size
    }

    // --- Private helpers ---

    fn pane_for_terminal(&self, terminal_id: u64) -> Option<pane_grid::Pane> {
        self.panes
            .iter()
            .find(|&(_, &id)| id == terminal_id)
            .map(|(pane, _)| pane)
            .copied()
    }

    fn set_focus_on_pane(
        &mut self,
        pane: pane_grid::Pane,
        close_menu: bool,
        focus_terminal_widget: bool,
    ) -> StateCommand {
        let Some(terminal_id) = self.pane_terminal_id(pane) else {
            return StateCommand::None;
        };

        self.focus = Some(pane);
        if close_menu {
            self.context_menu = None;
        }
        self.update_title_from_terminal(Some(terminal_id));

        if focus_terminal_widget {
            if let Some(entry) = self.terminals.get(&terminal_id) {
                return StateCommand::FocusTerminal(
                    entry.terminal.widget_id().clone(),
                );
            }
        }

        StateCommand::None
    }

    fn update_title_from_terminal(&mut self, terminal_id: Option<u64>) {
        if let Some(id) = terminal_id {
            if let Some(entry) = self.terminals.get(&id) {
                self.title = entry.title.clone();
                return;
            }
        }

        self.title = self.default_title.clone();
    }

    fn clamp_point(point: Point, bounds: Size) -> Point {
        Point::new(
            point.x.clamp(0.0, bounds.width.max(0.0)),
            point.y.clamp(0.0, bounds.height.max(0.0)),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::process::ExitStatus;

    use iced::widget::pane_grid;
    use iced::{Point, Size};
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::{PaneContextMenuState, TerminalTabState};
    use crate::widgets::terminal_workspace::model::TerminalKind;

    #[cfg(unix)]
    const TEST_SHELL_PATH: &str = "/bin/sh";
    #[cfg(target_os = "windows")]
    const TEST_SHELL_PATH: &str = "cmd.exe";

    fn test_settings() -> Settings {
        let mut settings = Settings::default();
        settings.backend = settings.backend.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(TEST_SHELL_PATH),
            ),
        );
        settings
    }

    fn build_terminal_state(default_title: &str) -> TerminalTabState {
        let (state, _task) = TerminalTabState::new(
            1,
            String::from(default_title),
            10,
            test_settings(),
            TerminalKind::Shell,
        )
        .expect("terminal state should initialize");
        state
    }

    #[allow(dead_code)]
    fn success_exit_status() -> ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            ExitStatus::from_raw(0)
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::ExitStatusExt;
            ExitStatus::from_raw(0)
        }
    }

    #[test]
    fn given_context_menu_state_when_accessors_called_then_values_match() {
        let (_grid, pane) = pane_grid::State::new(1_u64);
        let menu_state = PaneContextMenuState::new(
            pane,
            Point::new(20.0, 30.0),
            Size::new(300.0, 200.0),
            77,
        );

        assert_eq!(menu_state.pane(), pane);
        assert_eq!(menu_state.cursor(), Point::new(20.0, 30.0));
        assert_eq!(menu_state.grid_size(), Size::new(300.0, 200.0));
        assert_eq!(menu_state.terminal_id(), 77);
        let _focus_target = menu_state.focus_target();
    }

    #[test]
    fn given_new_state_when_constructed_then_initial_terminal_is_focused() {
        let state = build_terminal_state("Shell");
        let focused_pane = state.focus().expect("focused pane");
        let focused_terminal_id =
            state.pane_terminal_id(focused_pane).expect("terminal id");

        assert_eq!(state.title(), "Shell");
        assert!(state.is_shell());
        assert_eq!(state.panes().len(), 1);
        assert_eq!(state.terminals().len(), 1);
        assert_eq!(focused_terminal_id, 10);
        assert_eq!(state.focused_terminal_id(), Some(10));
        assert!(state.context_menu().is_none());
        assert!(state.selected_block().is_none());
    }

    #[test]
    fn given_selected_block_when_cleared_for_matching_terminal_then_selection_removed()
     {
        let mut state = build_terminal_state("Shell");
        state.set_selected_block(10, String::from("block-1"));

        state.clear_selected_block_for_terminal(11);
        assert_eq!(state.selected_block_terminal(), Some(10));

        state.clear_selected_block_for_terminal(10);
        assert!(state.selected_block().is_none());
    }

    #[test]
    fn given_existing_pane_when_split_then_new_terminal_is_added_and_focused() {
        let mut state = build_terminal_state("Shell");
        let pane = state.focus().expect("focused pane");

        let _task = state.split_pane(pane, pane_grid::Axis::Vertical, 11);

        assert_eq!(state.panes().len(), 2);
        assert_eq!(state.terminals().len(), 2);
        assert!(state.contains_terminal(11));
        assert_eq!(state.focused_terminal_id(), Some(11));
    }

    #[test]
    fn given_single_pane_when_close_requested_then_state_keeps_terminal() {
        let mut state = build_terminal_state("Shell");
        let pane = state.focus().expect("focused pane");

        let _task = state.close_pane(pane);

        assert_eq!(state.panes().len(), 1);
        assert_eq!(state.terminals().len(), 1);
        assert_eq!(state.focus(), Some(pane));
    }

    #[test]
    fn given_two_panes_when_focused_pane_closed_then_focus_moves_to_sibling() {
        let mut state = build_terminal_state("Shell");
        let initial_pane = state.focus().expect("focused pane");
        let _task =
            state.split_pane(initial_pane, pane_grid::Axis::Horizontal, 11);
        let closing_pane = state.focus().expect("split pane should be focused");
        state.set_selected_block(11, String::from("block-2"));

        let _task = state.close_pane(closing_pane);

        assert_eq!(state.panes().len(), 1);
        assert_eq!(state.terminals().len(), 1);
        assert!(!state.contains_terminal(11));
        assert_eq!(state.focus(), Some(initial_pane));
        assert!(state.selected_block().is_none());
    }

    #[test]
    fn given_unknown_terminal_when_open_context_menu_then_state_is_unchanged() {
        let mut state = build_terminal_state("Shell");
        let pane = state.focus().expect("focused pane");

        let _task = state.open_context_menu(
            pane,
            999,
            Point::new(10.0, 10.0),
            Size::new(100.0, 100.0),
        );

        assert!(state.context_menu().is_none());
        assert_eq!(state.focus(), Some(pane));
    }

    #[test]
    fn given_valid_terminal_when_open_and_close_context_menu_then_menu_toggles()
    {
        let mut state = build_terminal_state("Shell");
        let pane = state.focus().expect("focused pane");

        let _task = state.open_context_menu(
            pane,
            10,
            Point::new(15.0, 25.0),
            Size::new(500.0, 300.0),
        );

        let menu_state = state.context_menu().expect("context menu state");
        assert_eq!(menu_state.pane(), pane);
        assert_eq!(menu_state.terminal_id(), 10);
        assert_eq!(menu_state.cursor(), Point::new(15.0, 25.0));
        assert_eq!(menu_state.grid_size(), Size::new(500.0, 300.0));

        let _task = state.close_context_menu();
        assert!(state.context_menu().is_none());
        assert_eq!(state.focus(), Some(pane));
    }

    #[test]
    fn given_title_events_when_handled_then_tab_title_updates_and_resets() {
        let mut state = build_terminal_state("Shell");

        let _task =
            state.handle_terminal_event(otty_ui_term::Event::TitleChanged {
                id: 10,
                title: String::from("Renamed"),
            });
        assert_eq!(state.title(), "Renamed");

        let _task = state
            .handle_terminal_event(otty_ui_term::Event::ResetTitle { id: 10 });
        assert_eq!(state.title(), "Shell");
    }

    #[test]
    fn given_secondary_terminal_shutdown_when_handled_then_terminal_is_closed()
    {
        let mut state = build_terminal_state("Shell");
        let pane = state.focus().expect("focused pane");
        let _task = state.split_pane(pane, pane_grid::Axis::Vertical, 11);
        assert!(state.contains_terminal(11));

        let _task =
            state.handle_terminal_event(otty_ui_term::Event::Shutdown {
                id: 11,
                exit_status: success_exit_status(),
            });

        assert!(!state.contains_terminal(11));
        assert_eq!(state.panes().len(), 1);
    }

    #[test]
    fn given_grid_cursor_when_updated_and_resized_then_position_is_clamped() {
        let mut state = build_terminal_state("Shell");
        state.set_grid_size(Size::new(100.0, 80.0));

        let _task = state.update_grid_cursor(Point::new(250.0, -5.0));
        assert_eq!(state.grid_cursor, Some(Point::new(100.0, 0.0)));

        state.set_grid_size(Size::new(10.0, 5.0));
        assert_eq!(state.grid_cursor, Some(Point::new(10.0, 0.0)));
    }
}
