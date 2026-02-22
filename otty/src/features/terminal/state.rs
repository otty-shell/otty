use std::collections::HashMap;

use iced::{Point, Size, Task, widget::Id, widget::pane_grid};
use otty_ui_term::{
    BlockCommand, SurfaceMode, TerminalView,
    settings::{Settings, ThemeSettings},
};

use crate::{app::Event as AppEvent, features::tab::TabEvent};

use super::errors::TerminalError;
use super::model::{BlockSelection, TerminalEntry, TerminalKind};

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

    pub(crate) fn focus_task<Message: 'static>(&self) -> Task<Message> {
        iced::widget::operation::focus(self.focus_target.clone())
    }

    pub(crate) fn pane(&self) -> pane_grid::Pane {
        self.pane
    }

    pub(crate) fn cursor(&self) -> Point {
        self.cursor
    }

    pub(crate) fn grid_size(&self) -> Size {
        self.grid_size
    }

    pub(crate) fn terminal_id(&self) -> u64 {
        self.terminal_id
    }

    pub(crate) fn focus_target(&self) -> &Id {
        &self.focus_target
    }
}

/// Runtime state for a terminal tab with pane management and selection.
pub(crate) struct TerminalState {
    tab_id: u64,
    title: String,
    kind: TerminalKind,
    terminal_settings: Settings,
    panes: pane_grid::State<u64>,
    terminals: HashMap<u64, TerminalEntry>,
    focus: Option<pane_grid::Pane>,
    context_menu: Option<PaneContextMenuState>,
    selected_block: Option<BlockSelection>,
    default_title: String,
    grid_cursor: Option<Point>,
    grid_size: Size,
}

impl TerminalState {
    pub(crate) fn new(
        tab_id: u64,
        default_title: String,
        terminal_id: u64,
        settings: Settings,
        kind: TerminalKind,
    ) -> Result<(Self, Task<AppEvent>), TerminalError> {
        let terminal =
            otty_ui_term::Terminal::new(terminal_id, settings.clone())
                .map_err(|err| TerminalError::Init {
                    message: format!("{err}"),
                })?;
        let widget_id = terminal.widget_id().clone();

        let (panes, initial_pane) = pane_grid::State::new(terminal_id);

        let mut terminals = HashMap::new();
        terminals.insert(
            terminal_id,
            TerminalEntry {
                pane: initial_pane,
                terminal,
                title: default_title.clone(),
            },
        );

        let tab = TerminalState {
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

        Ok((tab, TerminalView::focus(widget_id)))
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Report whether this terminal tab represents a shell session.
    pub(crate) fn is_shell(&self) -> bool {
        matches!(self.kind, TerminalKind::Shell)
    }

    pub(crate) fn panes(&self) -> &pane_grid::State<u64> {
        &self.panes
    }

    pub(crate) fn terminals(&self) -> &HashMap<u64, TerminalEntry> {
        &self.terminals
    }

    pub(crate) fn focus(&self) -> Option<pane_grid::Pane> {
        self.focus
    }

    pub(crate) fn context_menu(&self) -> Option<&PaneContextMenuState> {
        self.context_menu.as_ref()
    }

    pub(crate) fn selected_block(&self) -> Option<&BlockSelection> {
        self.selected_block.as_ref()
    }

    pub(crate) fn contains_terminal(&self, id: u64) -> bool {
        self.terminals.contains_key(&id)
    }

    pub(crate) fn pane_terminal_id(
        &self,
        pane: pane_grid::Pane,
    ) -> Option<u64> {
        self.panes.get(pane).copied()
    }

    pub(crate) fn focused_terminal_id(&self) -> Option<u64> {
        let pane = self.focus?;
        self.pane_terminal_id(pane)
    }

    pub(crate) fn focused_terminal_entry(&self) -> Option<&TerminalEntry> {
        let terminal_id = self.focused_terminal_id()?;
        self.terminals.get(&terminal_id)
    }

    pub(crate) fn terminal_entry_mut(
        &mut self,
        terminal_id: u64,
    ) -> Option<&mut TerminalEntry> {
        self.terminals.get_mut(&terminal_id)
    }

    pub(crate) fn selected_block_terminal(&self) -> Option<u64> {
        self.selected_block
            .as_ref()
            .map(BlockSelection::terminal_id)
    }

    pub(crate) fn set_selected_block(
        &mut self,
        terminal_id: u64,
        block_id: String,
    ) {
        self.selected_block = Some(BlockSelection::new(terminal_id, block_id));
    }

    pub(crate) fn clear_selected_block(&mut self) {
        self.selected_block = None;
    }

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

    pub(crate) fn split_pane(
        &mut self,
        pane: pane_grid::Pane,
        axis: pane_grid::Axis,
        terminal_id: u64,
    ) -> Task<AppEvent> {
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
                    return Task::none();
                },
            };
            let widget_id = terminal.widget_id().clone();

            self.terminals.insert(
                terminal_id,
                TerminalEntry {
                    pane: new_pane,
                    terminal,
                    title: self.default_title.clone(),
                },
            );
            self.focus = Some(new_pane);
            self.context_menu = None;
            self.update_title_from_terminal(Some(terminal_id));

            return TerminalView::focus(widget_id);
        }

        Task::none()
    }

    pub(crate) fn close_pane(
        &mut self,
        pane: pane_grid::Pane,
    ) -> Task<AppEvent> {
        if self.panes.len() == 1 {
            return Task::done(AppEvent::Tab(TabEvent::CloseTab {
                tab_id: self.tab_id,
            }));
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
                        return TerminalView::focus(widget_id);
                    }
                }
            }

            return Task::none();
        }

        Task::none()
    }

    pub(crate) fn apply_theme(&mut self, palette: otty_ui_term::ColorPalette) {
        self.terminal_settings.theme =
            ThemeSettings::new(Box::new(palette.clone()));
        for entry in self.terminals.values_mut() {
            entry.terminal.change_theme(palette.clone());
        }
    }

    pub(crate) fn focus_pane(
        &mut self,
        pane: pane_grid::Pane,
    ) -> Task<AppEvent> {
        self.set_focus_on_pane(pane, true, true)
    }

    pub(crate) fn resize(&mut self, event: pane_grid::ResizeEvent) {
        self.panes.resize(event.split, event.ratio);
    }

    pub(crate) fn open_context_menu(
        &mut self,
        pane: pane_grid::Pane,
        terminal_id: u64,
        cursor: Point,
        grid_size: Size,
    ) -> Task<AppEvent> {
        let Some((widget_id, snapshot)) =
            self.terminals.get(&terminal_id).map(|entry| {
                (
                    entry.terminal.widget_id().clone(),
                    entry.terminal.snapshot_arc(),
                )
            })
        else {
            return Task::none();
        };
        if snapshot.view().mode.contains(SurfaceMode::ALT_SCREEN) {
            return Task::none();
        }

        let focus_task = self.set_focus_on_pane(pane, false, false);

        let select_task = TerminalView::command(
            widget_id.clone(),
            BlockCommand::SelectHovered,
        );

        let menu_state =
            PaneContextMenuState::new(pane, cursor, grid_size, terminal_id);
        let menu_focus_task = menu_state.focus_task();
        self.context_menu = Some(menu_state);

        Task::batch(vec![focus_task, select_task, menu_focus_task])
    }

    pub(crate) fn close_context_menu(&mut self) -> Task<AppEvent> {
        if self.context_menu.take().is_some() {
            if let Some(pane) = self.focus {
                return self.set_focus_on_pane(pane, false, true);
            }
        }

        Task::none()
    }

    pub(crate) fn handle_terminal_event(
        &mut self,
        event: otty_ui_term::Event,
    ) -> Task<AppEvent> {
        use otty_ui_term::Event::*;

        let terminal_id = *event.terminal_id();

        match event {
            Shutdown { .. } => {
                if let Some(entry) = self.terminals.get(&terminal_id) {
                    let pane = entry.pane;
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

        Task::none()
    }

    pub(crate) fn update_grid_cursor(
        &mut self,
        position: Point,
    ) -> Task<AppEvent> {
        self.grid_cursor = Some(Self::clamp_point(position, self.grid_size));
        Task::none()
    }

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

    fn set_focus_on_pane(
        &mut self,
        pane: pane_grid::Pane,
        close_menu: bool,
        focus_terminal_widget: bool,
    ) -> Task<AppEvent> {
        let Some(terminal_id) = self.pane_terminal_id(pane) else {
            return Task::none();
        };

        self.focus = Some(pane);
        if close_menu {
            self.context_menu = None;
        }
        self.update_title_from_terminal(Some(terminal_id));

        if focus_terminal_widget {
            if let Some(entry) = self.terminals.get(&terminal_id) {
                return TerminalView::focus(entry.terminal.widget_id().clone());
            }
        }

        Task::none()
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

    use iced::{Point, Size, widget::pane_grid};
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::{PaneContextMenuState, TerminalState};
    use crate::features::terminal::model::TerminalKind;

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

    fn build_terminal_state(default_title: &str) -> TerminalState {
        let (state, _task) = TerminalState::new(
            1,
            String::from(default_title),
            10,
            test_settings(),
            TerminalKind::Shell,
        )
        .expect("terminal state should initialize");
        state
    }

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
        let _task = menu_state.focus_task::<()>();
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
