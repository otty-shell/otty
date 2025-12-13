use iced::widget::pane_grid;
use iced::widget::{column, container, mouse_area, row, stack, text};
use iced::window::Direction;
use iced::{Element, Length, Point, Size, Subscription, Task, Theme, window};
use iced::{alignment, mouse};
use otty_ui_term::TerminalView;
use otty_ui_term::settings::{
    BackendSettings, FontSettings, Settings, ThemeSettings,
};

use crate::action_bar::{self, ACTION_BAR_HEIGHT};
use crate::fonts::FontsConfig;
use crate::shell_integrations::setup_shell_session;
use crate::tab::{Tab, TabAction};
use crate::tab_bar::{self, TAB_BAR_HEIGHT};
use crate::theme::ThemeManager;

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;
const RESIZE_EDGE_MOUSE_AREA_THICKNESS: f32 = 6.0;
const RESIZE_CORNER_MOUSE_AREA_THICKNESS: f32 = 12.0;

/// Represents the currently active high-level view.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    #[default]
    Terminal,
}

#[derive(Debug, Clone)]
pub enum Event {
    IcedReady,
    Terminal(otty_ui_term::Event),
    NewTab,
    CloseTab(u64),
    ActivateTab(u64),
    FromWindow(window::Event),
    ToggleFullScreen,
    ToggleTray,
    CloseWindow,
    StartWindowDrag,
    ResizeWindow(Direction),
    SplitPane {
        tab_id: u64,
        pane: pane_grid::Pane,
        axis: pane_grid::Axis,
    },
    ClosePane {
        tab_id: u64,
        pane: pane_grid::Pane,
    },
    PaneClicked {
        tab_id: u64,
        pane: pane_grid::Pane,
    },
    PaneResized {
        tab_id: u64,
        event: pane_grid::ResizeEvent,
    },
    PaneGridCursorMoved {
        tab_id: u64,
        position: Point,
    },
    OpenPaneContextMenu {
        tab_id: u64,
        pane: pane_grid::Pane,
        terminal_id: u64,
    },
    ClosePaneContextMenu {
        tab_id: u64,
    },
    PaneContextMenuInput,
    CopySelectedBlockContent {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelectedBlockPrompt {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelectedBlockCommand {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelection {
        tab_id: u64,
        terminal_id: u64,
    },
    PasteIntoPrompt {
        tab_id: u64,
        terminal_id: u64,
    },
}

#[derive(Clone, Copy, Debug)]
enum CopyKind {
    Content,
    Prompt,
    Command,
}

pub struct App {
    shell_name: String,
    pub(crate) settings: Settings,
    pub(crate) tabs: Vec<Tab>,
    pub(crate) active_tab_index: usize,
    pub(crate) next_tab_id: u64,
    pub(crate) next_terminal_id: u64,
    pub(crate) window_size: Size,
    pub(crate) active_view: ActiveView,
    pub(crate) theme_manager: ThemeManager,
    pub(crate) fonts: FontsConfig,
    is_fullscreen: bool,
}

impl App {
    pub fn new() -> (Self, Task<Event>) {
        let (shell_name, session) =
            setup_shell_session().expect("failed to setup shell session");

        let theme_manager = ThemeManager::new();
        let current_theme = theme_manager.current();
        let fonts = FontsConfig::default();

        let font_settings = FontSettings {
            size: fonts.terminal.size,
            font_type: fonts.terminal.font_type,
            ..FontSettings::default()
        };
        let theme_settings = ThemeSettings::new(Box::new(
            current_theme.terminal_palette().clone(),
        ));

        let settings = Settings {
            font: font_settings,
            theme: theme_settings,
            backend: BackendSettings::default().with_session(session),
        };

        let app = App {
            shell_name,
            settings: settings.clone(),
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size {
                width: MIN_WINDOW_WIDTH,
                height: MIN_WINDOW_HEIGHT,
            },
            active_view: ActiveView::default(),
            theme_manager,
            fonts,
            is_fullscreen: false,
        };

        (app, Task::done(()).map(|_: ()| Event::IcedReady))
    }

    pub fn title(&self) -> String {
        String::from("OTTY")
    }

    pub fn theme(&self) -> Theme {
        self.theme_manager.iced_theme()
    }

    pub fn subscription(&self) -> Subscription<Event> {
        let mut subscriptions = Vec::new();

        for tab in &self.tabs {
            for entry in tab.terminals.values() {
                subscriptions.push(entry.terminal.subscription());
            }
        }

        let term_subs = Subscription::batch(subscriptions).map(Event::Terminal);
        let win_subs =
            window::events().map(|(_id, event)| Event::FromWindow(event));

        Subscription::batch(vec![term_subs, win_subs])
    }

    pub fn update(&mut self, event: Event) -> Task<Event> {
        match event {
            Event::IcedReady => self.create_tab(),
            Event::Terminal(inner) => self.update_terminal(inner),
            Event::NewTab => {
                self.active_view = ActiveView::Terminal;
                self.create_tab()
            },
            Event::CloseTab(id) => self.close_tab(id),
            Event::ActivateTab(id) => {
                self.active_view = ActiveView::Terminal;
                self.activate_tab(id)
            },
            Event::ToggleFullScreen => self.toggle_full_screen(),
            Event::ToggleTray => self.minimize_window(),
            Event::CloseWindow => window::latest().and_then(window::close),
            Event::StartWindowDrag => window::latest().and_then(window::drag),
            Event::FromWindow(window::Event::Resized(size)) => {
                self.window_size = size;
                self.sync_tab_grid_sizes();
                Task::none()
            },
            Event::FromWindow(_) => Task::none(),
            Event::ResizeWindow(dir) => iced::window::latest()
                .and_then(move |id| window::drag_resize(id, dir)),
            Event::SplitPane { tab_id, pane, axis } => {
                self.split_pane(tab_id, pane, axis)
            },
            Event::ClosePane { tab_id, pane } => {
                self.close_pane_by_id(tab_id, pane)
            },
            Event::PaneClicked { tab_id, pane } => {
                self.with_tab_mut(tab_id, |tab| tab.focus_pane(pane))
            },
            Event::PaneResized { tab_id, event } => {
                if let Some(tab) = self.tab_mut(tab_id) {
                    tab.resize(event);
                }
                Task::none()
            },
            Event::PaneGridCursorMoved { tab_id, position } => self
                .with_tab_mut(tab_id, move |tab| {
                    tab.update_grid_cursor(position);
                    TabAction::none()
                }),
            Event::OpenPaneContextMenu {
                tab_id,
                pane,
                terminal_id,
            } => self.with_tab_mut(tab_id, move |tab| {
                tab.open_context_menu(pane, terminal_id)
            }),
            Event::ClosePaneContextMenu { tab_id } => {
                self.with_tab_mut(tab_id, Tab::close_context_menu)
            },
            Event::PaneContextMenuInput => Task::none(),
            Event::CopySelectedBlockContent {
                tab_id,
                terminal_id,
            } => {
                self.copy_selected_block(tab_id, terminal_id, CopyKind::Content)
            },
            Event::CopySelectedBlockPrompt {
                tab_id,
                terminal_id,
            } => {
                self.copy_selected_block(tab_id, terminal_id, CopyKind::Prompt)
            },
            Event::CopySelectedBlockCommand {
                tab_id,
                terminal_id,
            } => {
                self.copy_selected_block(tab_id, terminal_id, CopyKind::Command)
            },
            Event::CopySelection {
                tab_id,
                terminal_id,
            } => self.copy_selection(tab_id, terminal_id),
            Event::PasteIntoPrompt {
                tab_id,
                terminal_id,
            } => self.paste_into_prompt(tab_id, terminal_id),
        }
    }

    pub fn view(&self) -> Element<'_, Event, Theme, iced::Renderer> {
        let header = action_bar::view_action_bar(self);
        let tabs_row = tab_bar::view_tab_bar(self);
        let main_content: Element<Event, Theme, iced::Renderer> =
            match self.active_view {
                ActiveView::Terminal => {
                    let content = self.view_active_terminal();
                    column![tabs_row, content]
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                },
            };

        let content_row =
            row![main_content].width(Length::Fill).height(Length::Fill);

        // Resizable grips
        let n_grip = mouse_area(
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS)),
        )
        .on_press(Event::ResizeWindow(Direction::North))
        .interaction(mouse::Interaction::ResizingVertically);

        let s_grip = mouse_area(
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS)),
        )
        .on_press(Event::ResizeWindow(Direction::South))
        .interaction(mouse::Interaction::ResizingVertically);

        let e_grip = mouse_area(
            container(text(""))
                .width(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS))
                .height(Length::Fill),
        )
        .on_press(Event::ResizeWindow(Direction::East))
        .interaction(mouse::Interaction::ResizingHorizontally);

        let w_grip = mouse_area(
            container(text(""))
                .width(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS))
                .height(Length::Fill),
        )
        .on_press(Event::ResizeWindow(Direction::West))
        .interaction(mouse::Interaction::ResizingHorizontally);

        // Corners
        let nw_grip = mouse_area(
            container(text(""))
                .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
                .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
        )
        .on_press(Event::ResizeWindow(Direction::NorthWest))
        .interaction(mouse::Interaction::ResizingDiagonallyDown);

        let ne_grip = mouse_area(
            container(text(""))
                .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
                .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
        )
        .on_press(Event::ResizeWindow(Direction::NorthEast))
        .interaction(mouse::Interaction::ResizingDiagonallyUp);

        let sw_grip = mouse_area(
            container(text(""))
                .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
                .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
        )
        .on_press(Event::ResizeWindow(Direction::SouthWest))
        .interaction(mouse::Interaction::ResizingDiagonallyUp);

        let se_grip = mouse_area(
            container(text(""))
                .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
                .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
        )
        .on_press(Event::ResizeWindow(Direction::SouthEast))
        .interaction(mouse::Interaction::ResizingDiagonallyDown);

        stack!(
            // Content
            column![header, content_row]
                .width(Length::Fill)
                .height(Length::Fill),
            // Edge grips
            container(n_grip)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Top),
            container(s_grip)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Bottom),
            container(e_grip)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
            container(w_grip)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Left),
            // Corners (aligned on top of edges)
            container(nw_grip)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Left)
                .align_y(iced::alignment::Vertical::Top),
            container(ne_grip)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Top),
            container(sw_grip)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Left)
                .align_y(iced::alignment::Vertical::Bottom),
            container(se_grip)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom),
        )
        .into()
    }

    fn view_active_terminal(
        &self,
    ) -> Element<'_, Event, Theme, iced::Renderer> {
        if let Some(tab) = self.tabs.get(self.active_tab_index) {
            let selected_terminal = tab.selected_block_terminal();
            tab.view_with_selection(selected_terminal)
        } else {
            container(text("No tabs"))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .into()
        }
    }

    fn update_terminal(&mut self, event: otty_ui_term::Event) -> Task<Event> {
        let id = *event.terminal_id();
        if let Some(index) = self.tab_index_by_terminal(id) {
            let selection_task = self.update_block_selection(index, &event);
            let action =
                self.tabs[index].handle_terminal_event(event, &self.shell_name);
            let tab_task = self.resolve_tab_action(index, action);

            return Task::batch(vec![selection_task, tab_task]);
        }

        Task::none()
    }

    fn update_block_selection(
        &mut self,
        tab_index: usize,
        event: &otty_ui_term::Event,
    ) -> Task<Event> {
        use otty_ui_term::Event::*;

        match event {
            BlockSelected { block_id, .. } => {
                let terminal_id = *event.terminal_id();
                let tab = &mut self.tabs[tab_index];
                tab.set_selected_block(terminal_id, block_id.clone());

                let mut tasks = Vec::new();
                for entry in tab.terminals.values() {
                    if entry.terminal.id != terminal_id {
                        tasks.push(otty_ui_term::TerminalView::command(
                            entry.terminal.widget_id().clone(),
                            otty_ui_term::BlockCommand::ClearSelection,
                        ));
                    }
                }

                if tasks.is_empty() {
                    return Task::none();
                }

                return Task::batch(tasks);
            },
            BlockSelectionCleared { .. } => {
                let terminal_id = *event.terminal_id();
                let tab = &mut self.tabs[tab_index];
                if tab
                    .selected_block()
                    .map(|sel| sel.terminal_id == terminal_id)
                    .unwrap_or(false)
                {
                    tab.clear_selected_block();
                }
            },
            BlockCopied { .. } => {},
            _ => {},
        }

        Task::none()
    }

    fn copy_selected_block(
        &mut self,
        tab_id: u64,
        terminal_id: u64,
        kind: CopyKind,
    ) -> Task<Event> {
        let Some(tab_index) = self.tab_index_by_id(tab_id) else {
            return Task::none();
        };

        let block_id = {
            let tab = &self.tabs[tab_index];
            let Some(selection) = tab.selected_block() else {
                return Task::none();
            };

            if selection.terminal_id != terminal_id {
                return Task::none();
            }

            selection.block_id.clone()
        };

        // We deliberately avoid borrowing `self.tabs` immutably and mutably
        // at the same time. First, locate the terminal widget id and clone
        // it, then close the context menu via `with_tab_mut`.
        let widget_id = {
            let tab = &self.tabs[tab_index];

            let entry = match tab.terminals.get(&terminal_id) {
                Some(entry) => entry,
                None => return Task::none(),
            };

            entry.terminal.widget_id().clone()
        };
        let command = match kind {
            CopyKind::Content => {
                otty_ui_term::BlockCommand::CopyContent(block_id)
            },
            CopyKind::Prompt => {
                otty_ui_term::BlockCommand::CopyPrompt(block_id)
            },
            CopyKind::Command => {
                otty_ui_term::BlockCommand::CopyCommand(block_id)
            },
        };

        let close_menu_task =
            self.with_tab_mut(tab_id, Tab::close_context_menu);
        let copy_task = otty_ui_term::TerminalView::command(widget_id, command);

        Task::batch(vec![close_menu_task, copy_task])
    }

    fn copy_selection(&mut self, tab_id: u64, terminal_id: u64) -> Task<Event> {
        let Some(tab_index) = self.tab_index_by_id(tab_id) else {
            return Task::none();
        };

        let widget_id = {
            let tab = &self.tabs[tab_index];
            let entry = match tab.terminals.get(&terminal_id) {
                Some(entry) => entry,
                None => return Task::none(),
            };

            entry.terminal.widget_id().clone()
        };

        let close_menu_task =
            self.with_tab_mut(tab_id, Tab::close_context_menu);
        let copy_task = otty_ui_term::TerminalView::command(
            widget_id,
            otty_ui_term::BlockCommand::CopySelection,
        );

        Task::batch(vec![close_menu_task, copy_task])
    }

    fn paste_into_prompt(
        &mut self,
        tab_id: u64,
        terminal_id: u64,
    ) -> Task<Event> {
        let Some(tab_index) = self.tab_index_by_id(tab_id) else {
            return Task::none();
        };

        let widget_id = {
            let tab = &self.tabs[tab_index];
            let entry = match tab.terminals.get(&terminal_id) {
                Some(entry) => entry,
                None => return Task::none(),
            };

            entry.terminal.widget_id().clone()
        };

        let close_menu_task =
            self.with_tab_mut(tab_id, Tab::close_context_menu);
        let paste_task = otty_ui_term::TerminalView::command(
            widget_id,
            otty_ui_term::BlockCommand::PasteClipboard,
        );

        Task::batch(vec![close_menu_task, paste_task])
    }
    fn create_tab(&mut self) -> Task<Event> {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let terminal_id = self.next_terminal_id;
        self.next_terminal_id += 1;

        let (mut tab, focus_task) = Tab::new(
            tab_id,
            self.shell_name.clone(),
            terminal_id,
            &self.settings,
            self.theme_manager.current().iced_palette(),
        );

        tab.set_grid_size(self.pane_grid_size());
        self.tabs.push(tab);

        self.active_tab_index = self.tabs.len() - 1;

        let sync_task = self.sync_tab_block_selection(self.active_tab_index);

        Task::batch(vec![focus_task, sync_task])
    }

    fn toggle_full_screen(&mut self) -> Task<Event> {
        self.is_fullscreen = !self.is_fullscreen;

        let mode = if self.is_fullscreen {
            window::Mode::Fullscreen
        } else {
            window::Mode::Windowed
        };

        window::latest().and_then(move |id| window::set_mode(id, mode))
    }

    fn minimize_window(&self) -> Task<Event> {
        window::latest().and_then(|id| window::minimize(id, true))
    }

    fn close_tab(&mut self, id: u64) -> Task<Event> {
        if self.tabs.len() == 1 {
            return window::latest().and_then(window::close);
        }

        if let Some(index) = self.tabs.iter().position(|tab| tab.id == id) {
            self.tabs.remove(index);

            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len().saturating_sub(1);
            }

            if self.tabs.is_empty() {
                return Task::none();
            }

            let focus_task = if let Some(active) =
                self.tabs.get(self.active_tab_index)
                && let Some(entry) = active.focused_terminal_entry()
            {
                TerminalView::focus(entry.terminal.widget_id().clone())
            } else {
                Task::none()
            };

            let sync_task =
                self.sync_tab_block_selection(self.active_tab_index);

            return Task::batch(vec![focus_task, sync_task]);
        }

        Task::none()
    }

    fn activate_tab(&mut self, id: u64) -> Task<Event> {
        if let Some(index) = self.tabs.iter().position(|tab| tab.id == id) {
            self.active_tab_index = index;
            let focus_task = if let Some(entry) =
                self.tabs[index].focused_terminal_entry()
            {
                TerminalView::focus(entry.terminal.widget_id().clone())
            } else {
                Task::none()
            };
            let sync_task = self.sync_tab_block_selection(index);
            return Task::batch(vec![focus_task, sync_task]);
        }

        Task::none()
    }

    fn split_pane(
        &mut self,
        tab_id: u64,
        pane: pane_grid::Pane,
        axis: pane_grid::Axis,
    ) -> Task<Event> {
        let terminal_id = self.next_terminal_id;
        self.next_terminal_id += 1;
        let settings = self.settings.clone();

        self.with_tab_mut(tab_id, move |tab| {
            tab.split_pane(pane, axis, terminal_id, &settings)
        })
    }

    fn close_pane_by_id(
        &mut self,
        tab_id: u64,
        pane: pane_grid::Pane,
    ) -> Task<Event> {
        self.with_tab_mut(tab_id, |tab| tab.close_pane(pane))
    }

    fn resolve_tab_action(
        &mut self,
        tab_index: usize,
        action: TabAction,
    ) -> Task<Event> {
        if action.close_tab {
            let tab_id = self.tabs[tab_index].id;
            return self.close_tab(tab_id);
        }

        action.task.unwrap_or_else(Task::none)
    }

    fn with_tab_mut<F>(&mut self, tab_id: u64, f: F) -> Task<Event>
    where
        F: FnOnce(&mut Tab) -> TabAction,
    {
        if let Some(index) = self.tab_index_by_id(tab_id) {
            let action = f(&mut self.tabs[index]);
            return self.resolve_tab_action(index, action);
        }

        Task::none()
    }

    fn tab_index_by_id(&self, id: u64) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }

    fn tab_index_by_terminal(&self, terminal_id: u64) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.contains_terminal(terminal_id))
    }

    fn tab_mut(&mut self, id: u64) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|tab| tab.id == id)
    }

    fn sync_tab_block_selection(&self, tab_index: usize) -> Task<Event> {
        let Some(tab) = self.tabs.get(tab_index) else {
            return Task::none();
        };

        let selection = tab.selected_block().cloned();
        let mut tasks = Vec::new();
        for entry in tab.terminals.values() {
            let cmd = if let Some(sel) = &selection
                && sel.terminal_id == entry.terminal.id
            {
                otty_ui_term::BlockCommand::Select(sel.block_id.clone())
            } else {
                otty_ui_term::BlockCommand::ClearSelection
            };

            tasks.push(otty_ui_term::TerminalView::command(
                entry.terminal.widget_id().clone(),
                cmd,
            ));
        }

        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    fn pane_grid_size(&self) -> Size {
        let height =
            (self.window_size.height - ACTION_BAR_HEIGHT - TAB_BAR_HEIGHT)
                .max(0.0);
        Size::new(self.window_size.width, height)
    }

    fn sync_tab_grid_sizes(&mut self) {
        let size = self.pane_grid_size();
        for tab in &mut self.tabs {
            tab.set_grid_size(size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_menu;
    use crate::theme::ThemeManager;

    #[test]
    fn default_tab_titles_are_indexed_from_one() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size::default(),
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
        };

        let _ = app.create_tab();
        let _ = app.create_tab();

        assert_eq!(app.tabs[0].title, "zsh");
        assert_eq!(app.tabs[1].title, "zsh");
    }

    #[test]
    fn closing_tab_updates_active_index() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size::default(),
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
        };

        let _ = app.create_tab();
        let _ = app.create_tab();
        let first_id = app.tabs[0].id;

        let _ = app.close_tab(first_id);

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab_index, 0);
    }

    #[test]
    fn splitting_active_pane_creates_new_terminal() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size::default(),
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
        };

        let _ = app.create_tab();
        let tab_id = app.tabs[0].id;
        let pane = app.tabs[0].focus.expect("expected focus");

        let _ = app.split_pane(tab_id, pane, pane_grid::Axis::Horizontal);

        assert_eq!(app.tabs[0].panes.len(), 2);
        assert_eq!(app.tabs[0].terminals.len(), 2);
    }

    #[test]
    fn closing_pane_removes_terminal() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size::default(),
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
        };

        let _ = app.create_tab();
        let tab_id = app.tabs[0].id;
        let first_pane = app.tabs[0].focus.expect("expected pane");
        let _ = app.split_pane(tab_id, first_pane, pane_grid::Axis::Vertical);

        let second_pane = app.tabs[0]
            .panes
            .iter()
            .map(|(pane, _)| pane)
            .copied()
            .find(|pane| *pane != first_pane)
            .expect("missing second pane");

        let _ = app.close_pane_by_id(tab_id, second_pane);

        assert_eq!(app.tabs[0].panes.len(), 1);
        assert_eq!(app.tabs[0].terminals.len(), 1);
    }

    #[test]
    fn closing_last_pane_closes_the_tab() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size::default(),
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
        };

        let _ = app.create_tab();
        let _ = app.create_tab();
        let first_tab_id = app.tabs[0].id;
        let pane = app.tabs[0].focus.expect("expected pane");

        let _ = app.close_pane_by_id(first_tab_id, pane);

        assert_eq!(app.tabs.len(), 1);
        assert!(app.tabs.iter().all(|tab| tab.id != first_tab_id));
    }

    #[test]
    fn activating_tab_from_settings_switches_to_main_layout() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            window_size: Size::default(),
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
            active_view: ActiveView::Terminal,
            next_terminal_id: 0,
        };

        let _ = app.create_tab();
        let first_id = app.tabs[0].id;

        let _ = app.update(Event::ActivateTab(first_id));

        assert!(matches!(app.active_view, ActiveView::Terminal));
    }

    #[test]
    fn new_tab_switches_to_main_layout() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            window_size: Size::default(),
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
            active_view: ActiveView::Terminal,
            next_terminal_id: 0,
        };

        let _ = app.update(Event::NewTab);

        assert!(matches!(app.active_view, ActiveView::Terminal));
    }

    #[test]
    fn default_active_view_is_main_layout() {
        let (_, task) = App::new();
        drop(task);

        let settings = Settings::default();
        let app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            window_size: Size::default(),
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
            next_terminal_id: 0,
        };

        assert!(matches!(app.active_view, ActiveView::Terminal));
    }

    #[test]
    fn context_menu_stays_visible_near_edges() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size {
                width: 800.0,
                height: 600.0,
            },
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
        };

        let _ = app.create_tab();
        app.sync_tab_grid_sizes();
        let tab_id = app.tabs[0].id;
        let pane = app.tabs[0].focus.expect("focus");
        let terminal_id = app.tabs[0].pane_terminal_id(pane).expect("terminal");

        let grid_size = app.tabs[0].grid_size();
        let cursor = Point::new(grid_size.width - 4.0, grid_size.height - 4.0);
        let _ = app.update(Event::PaneGridCursorMoved {
            tab_id,
            position: cursor,
        });

        let _ = app.update(Event::OpenPaneContextMenu {
            tab_id,
            pane,
            terminal_id,
        });

        let menu = app.tabs[0].context_menu.as_ref().expect("menu");
        let has_block_selection = app.tabs[0]
            .selected_block()
            .filter(|sel| sel.terminal_id == menu.terminal_id)
            .is_some();
        let mut item_count = 5;
        if has_block_selection {
            item_count += 3;
        }
        let menu_height = context_menu::menu_height_for_items(item_count);
        assert_eq!(
            menu.anchor_for_height(menu_height),
            context_menu::anchor_position(cursor, grid_size, menu_height)
        );
    }

    #[test]
    fn selecting_block_in_one_tab_updates_tab_selection() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size::default(),
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
        };

        let _ = app.create_tab();

        let first_terminal_id =
            app.tabs[0].focused_terminal_id().expect("first terminal");

        let _ =
            app.update(Event::Terminal(otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            }));

        let selection = app.tabs[0].selected_block().expect("tab selection");
        assert_eq!(selection.terminal_id, first_terminal_id);
        assert_eq!(selection.block_id, "block-a");
    }

    #[test]
    fn tab_selection_persists_when_switching_tabs() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size::default(),
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
        };

        let _ = app.create_tab();
        let _ = app.create_tab();

        let first_terminal_id =
            app.tabs[0].focused_terminal_id().expect("first terminal");
        let second_terminal_id =
            app.tabs[1].focused_terminal_id().expect("second terminal");

        let _ =
            app.update(Event::Terminal(otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            }));
        let _ =
            app.update(Event::Terminal(otty_ui_term::Event::BlockSelected {
                id: second_terminal_id,
                block_id: String::from("block-b"),
            }));

        let _ = app.update(Event::ActivateTab(app.tabs[0].id));
        let _ = app.update(Event::ActivateTab(app.tabs[1].id));

        assert_eq!(
            app.tabs[0]
                .selected_block()
                .expect("tab0 selection")
                .block_id,
            "block-a"
        );
        assert_eq!(
            app.tabs[1]
                .selected_block()
                .expect("tab1 selection")
                .block_id,
            "block-b"
        );
    }

    #[test]
    fn creating_new_tab_preserves_existing_tab_selection() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size::default(),
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
        };

        let _ = app.create_tab();
        let first_terminal_id =
            app.tabs[0].focused_terminal_id().expect("first terminal");

        let _ =
            app.update(Event::Terminal(otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            }));

        let _ = app.update(Event::NewTab);

        assert!(app.tabs[0].selected_block().is_some());
        assert!(app.tabs[1].selected_block().is_none());
    }

    #[test]
    fn selecting_block_in_new_pane_replaces_previous_selection() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size::default(),
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
        };

        let _ = app.create_tab();

        let tab_id = app.tabs[0].id;
        let first_terminal_id =
            app.tabs[0].focused_terminal_id().expect("first terminal");
        let first_pane = app.tabs[0].focus.expect("first pane");

        let _ =
            app.update(Event::Terminal(otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            }));

        let _ = app.update(Event::SplitPane {
            tab_id,
            pane: first_pane,
            axis: pane_grid::Axis::Horizontal,
        });

        let second_terminal_id = app.tabs[0]
            .terminals
            .keys()
            .copied()
            .find(|id| *id != first_terminal_id)
            .expect("second terminal id");

        let _ =
            app.update(Event::Terminal(otty_ui_term::Event::BlockSelected {
                id: second_terminal_id,
                block_id: String::from("block-b"),
            }));

        let selection = app.tabs[0].selected_block().expect("tab selection");
        assert_eq!(selection.terminal_id, second_terminal_id);
        assert_eq!(selection.block_id, "block-b");
    }

    #[test]
    fn focusing_other_pane_does_not_clear_selection() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size: Size::default(),
            active_view: ActiveView::Terminal,
            theme_manager: ThemeManager::new(),
            fonts: FontsConfig::default(),
            is_fullscreen: false,
        };

        let _ = app.create_tab();

        let tab_id = app.tabs[0].id;
        let first_pane = app.tabs[0].focus.expect("initial pane");

        let _ = app.update(Event::SplitPane {
            tab_id,
            pane: first_pane,
            axis: pane_grid::Axis::Horizontal,
        });

        let focused_terminal_id = app.tabs[0]
            .focused_terminal_id()
            .expect("focused terminal after split");

        let _ =
            app.update(Event::Terminal(otty_ui_term::Event::BlockSelected {
                id: focused_terminal_id,
                block_id: String::from("block-a"),
            }));

        let other_pane = app.tabs[0]
            .terminals
            .iter()
            .find(|(terminal_id, _)| **terminal_id != focused_terminal_id)
            .map(|(_, entry)| entry.pane)
            .expect("other pane");

        let _ = app.update(Event::PaneClicked {
            tab_id,
            pane: other_pane,
        });

        assert!(app.tabs[0].selected_block().is_some());
    }
}
