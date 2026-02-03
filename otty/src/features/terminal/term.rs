use std::collections::HashMap;

use iced::{Point, Size, Task, widget::pane_grid};
use otty_ui_term::{
    BlockCommand, SurfaceMode, TerminalView,
    settings::{Settings, ThemeSettings},
};

use crate::{
    app::Event as AppEvent, features::tab::TabEvent,
    features::terminal::pane_context_menu::PaneContextMenuState,
};

/// Terminal entry used by the tab view.
pub(crate) struct TerminalEntry {
    pub(crate) pane: pane_grid::Pane,
    pub(crate) terminal: otty_ui_term::Terminal,
    pub(crate) title: String,
}

/// Terminal context determining whether shell metadata is available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalKind {
    Shell,
    Command,
}

#[derive(Clone, Debug)]
pub(crate) struct BlockSelection {
    pub terminal_id: u64,
    pub block_id: String,
}

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
    pub fn new(
        tab_id: u64,
        default_title: String,
        terminal_id: u64,
        settings: Settings,
        kind: TerminalKind,
    ) -> Result<(Self, Task<AppEvent>), String> {
        let terminal =
            otty_ui_term::Terminal::new(terminal_id, settings.clone())
                .map_err(|err| format!("{err}"))?;
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

    pub fn title(&self) -> &str {
        &self.title
    }

    /// Report whether this terminal tab represents a shell session.
    pub(crate) fn is_shell(&self) -> bool {
        matches!(self.kind, TerminalKind::Shell)
    }

    pub fn panes(&self) -> &pane_grid::State<u64> {
        &self.panes
    }

    pub fn terminals(&self) -> &HashMap<u64, TerminalEntry> {
        &self.terminals
    }

    pub fn focus(&self) -> Option<pane_grid::Pane> {
        self.focus
    }

    pub fn context_menu(&self) -> Option<&PaneContextMenuState> {
        self.context_menu.as_ref()
    }

    pub fn selected_block(&self) -> Option<&BlockSelection> {
        self.selected_block.as_ref()
    }

    pub fn contains_terminal(&self, id: u64) -> bool {
        self.terminals.contains_key(&id)
    }

    pub fn pane_terminal_id(&self, pane: pane_grid::Pane) -> Option<u64> {
        self.panes.get(pane).copied()
    }

    pub fn focused_terminal_id(&self) -> Option<u64> {
        let pane = self.focus?;
        self.pane_terminal_id(pane)
    }

    pub fn focused_terminal_entry(&self) -> Option<&TerminalEntry> {
        let terminal_id = self.focused_terminal_id()?;
        self.terminals.get(&terminal_id)
    }

    pub fn terminal_entry_mut(
        &mut self,
        terminal_id: u64,
    ) -> Option<&mut TerminalEntry> {
        self.terminals.get_mut(&terminal_id)
    }

    pub fn selected_block_terminal(&self) -> Option<u64> {
        self.selected_block.as_ref().map(|sel| sel.terminal_id)
    }

    pub fn set_selected_block(&mut self, terminal_id: u64, block_id: String) {
        self.selected_block = Some(BlockSelection {
            terminal_id,
            block_id,
        });
    }

    pub fn clear_selected_block(&mut self) {
        self.selected_block = None;
    }

    pub fn clear_selected_block_for_terminal(&mut self, terminal_id: u64) {
        if self
            .selected_block
            .as_ref()
            .map(|sel| sel.terminal_id == terminal_id)
            .unwrap_or(false)
        {
            self.selected_block = None;
        }
    }

    pub fn split_pane(
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

    pub fn close_pane(&mut self, pane: pane_grid::Pane) -> Task<AppEvent> {
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

    pub fn apply_theme(&mut self, palette: otty_ui_term::ColorPalette) {
        self.terminal_settings.theme =
            ThemeSettings::new(Box::new(palette.clone()));
        for entry in self.terminals.values_mut() {
            entry.terminal.change_theme(palette.clone());
        }
    }

    pub fn focus_pane(&mut self, pane: pane_grid::Pane) -> Task<AppEvent> {
        self.set_focus_on_pane(pane, true, true)
    }

    pub fn resize(&mut self, event: pane_grid::ResizeEvent) {
        self.panes.resize(event.split, event.ratio);
    }

    pub fn open_context_menu(
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

    pub fn close_context_menu(&mut self) -> Task<AppEvent> {
        if self.context_menu.take().is_some() {
            if let Some(pane) = self.focus {
                return self.set_focus_on_pane(pane, false, true);
            }
        }

        Task::none()
    }

    pub fn handle_terminal_event(
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

    pub fn update_grid_cursor(&mut self, position: Point) -> Task<AppEvent> {
        self.grid_cursor = Some(Self::clamp_point(position, self.grid_size));
        Task::none()
    }

    pub fn set_grid_size(&mut self, size: Size) {
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
