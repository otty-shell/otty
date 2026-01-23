use iced::widget::{column, container, pane_grid, text};
use iced::{Element, Length, Size, Subscription, Task, Theme, alignment};
use otty_ui_term::TerminalView;

use crate::app::config::AppConfig;
use crate::app::theme::ThemeProps;
use crate::widgets::tab::{TabEvent, TabProps, TabView};
use crate::widgets::tab_bar::{
    TabBar, TabBarEvent, TabBarMetrics, TabBarProps, TabSummary,
};

mod tab_state;

use tab_state::{TabAction, TabState};

/// UI events emitted by the terminal screen.
#[derive(Debug, Clone)]
pub(crate) enum TerminalScreenEvent {
    NewTab,
    TabBar(TabBarEvent),
    Tab(TabEvent),
}

/// Screen-level actions sent to the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerminalScreenAction {
    None,
    ActivateView,
    CloseWindow,
}

/// Update payload for screen event handling.
#[derive(Debug)]
pub(crate) struct TerminalScreenUpdate {
    pub action: TerminalScreenAction,
    pub task: Task<TerminalScreenEvent>,
}

impl TerminalScreenUpdate {
    fn none() -> Self {
        Self {
            action: TerminalScreenAction::None,
            task: Task::none(),
        }
    }

    fn with_task(task: Task<TerminalScreenEvent>) -> Self {
        Self {
            action: TerminalScreenAction::None,
            task,
        }
    }

    fn with_action(action: TerminalScreenAction) -> Self {
        Self {
            action,
            task: Task::none(),
        }
    }

    fn merge(self, other: Self) -> Self {
        let action = self.action.merge(other.action);
        let task = Task::batch(vec![self.task, other.task]);
        Self { action, task }
    }
}

impl TerminalScreenAction {
    fn merge(self, other: Self) -> Self {
        use TerminalScreenAction::*;

        match (self, other) {
            (CloseWindow, _) | (_, CloseWindow) => CloseWindow,
            (ActivateView, _) | (_, ActivateView) => ActivateView,
            _ => None,
        }
    }
}

/// Terminal screen holding tabs and pane state.
pub(crate) struct TerminalScreen {
    config: AppConfig,
    tabs: Vec<TabState>,
    tab_summaries: Vec<TabSummary>,
    active_tab_index: usize,
    next_tab_id: u64,
    next_terminal_id: u64,
    screen_size: Size,
}

impl TerminalScreen {
    pub(crate) fn new(config: AppConfig, screen_size: Size) -> Self {
        Self {
            config,
            tabs: Vec::new(),
            tab_summaries: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            next_terminal_id: 0,
            screen_size,
        }
    }

    pub(crate) fn update(
        &mut self,
        event: TerminalScreenEvent,
    ) -> TerminalScreenUpdate {
        match event {
            TerminalScreenEvent::NewTab => TerminalScreenUpdate {
                action: TerminalScreenAction::ActivateView,
                task: self.create_tab(),
            },
            TerminalScreenEvent::TabBar(event) => self.update_tab_bar(event),
            TerminalScreenEvent::Tab(event) => self.update_tab(event),
        }
    }

    pub(crate) fn view<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Element<'a, TerminalScreenEvent, Theme, iced::Renderer> {
        let active_tab_id = self
            .tabs
            .get(self.active_tab_index)
            .map(TabState::id)
            .unwrap_or(0);

        let tab_bar = TabBar::new(TabBarProps {
            tabs: &self.tab_summaries,
            active_tab_id,
            theme,
            metrics: TabBarMetrics::default(),
        })
        .view()
        .map(TerminalScreenEvent::TabBar);

        let main_content: Element<
            '_,
            TerminalScreenEvent,
            Theme,
            iced::Renderer,
        > = if let Some(tab) = self.tabs.get(self.active_tab_index) {
            let selected_block_terminal = tab.selected_block_terminal();
            TabView::new(TabProps {
                tab_id: tab.id(),
                panes: tab.panes(),
                terminals: tab.terminals(),
                focus: tab.focus(),
                context_menu: tab.context_menu(),
                selected_block_terminal,
                theme,
            })
            .view()
            .map(TerminalScreenEvent::Tab)
        } else {
            container(text("No tabs"))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .into()
        };

        column![tab_bar, main_content]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub(crate) fn subscription(&self) -> Subscription<TerminalScreenEvent> {
        let mut subscriptions = Vec::new();

        for tab in &self.tabs {
            for entry in tab.terminals().values() {
                subscriptions.push(entry.terminal.subscription());
            }
        }

        Subscription::batch(subscriptions)
            .map(|event| TerminalScreenEvent::Tab(TabEvent::Terminal(event)))
    }

    pub(crate) fn set_screen_size(&mut self, size: Size) {
        self.screen_size = size;
        self.sync_tab_grid_sizes();
    }

    pub(crate) fn active_tab_title(&self) -> Option<&str> {
        self.tabs.get(self.active_tab_index).map(TabState::title)
    }

    fn update_tab_bar(&mut self, event: TabBarEvent) -> TerminalScreenUpdate {
        match event {
            TabBarEvent::TabButton(button_event) => match button_event {
                crate::components::tab_button::TabButtonEvent::ActivateTab(
                    id,
                ) => TerminalScreenUpdate {
                    action: TerminalScreenAction::ActivateView,
                    task: self.activate_tab(id),
                },
                crate::components::tab_button::TabButtonEvent::CloseTab(id) => {
                    self.close_tab(id)
                },
            },
        }
    }

    fn update_tab(&mut self, event: TabEvent) -> TerminalScreenUpdate {
        match event {
            TabEvent::Terminal(inner) => self.update_terminal(inner),
            TabEvent::OpenContextMenu {
                tab_id,
                pane,
                terminal_id,
            } => self.with_tab_mut(tab_id, |tab| {
                tab.open_context_menu(pane, terminal_id)
            }),
            TabEvent::CloseContextMenu { tab_id } => {
                self.with_tab_mut(tab_id, TabState::close_context_menu)
            },
            TabEvent::ContextMenuInput => TerminalScreenUpdate::none(),
            TabEvent::CopySelectedBlockContent {
                tab_id,
                terminal_id,
            } => TerminalScreenUpdate::with_task(self.copy_selected_block(
                tab_id,
                terminal_id,
                CopyKind::Content,
            )),
            TabEvent::CopySelectedBlockPrompt {
                tab_id,
                terminal_id,
            } => TerminalScreenUpdate::with_task(self.copy_selected_block(
                tab_id,
                terminal_id,
                CopyKind::Prompt,
            )),
            TabEvent::CopySelectedBlockCommand {
                tab_id,
                terminal_id,
            } => TerminalScreenUpdate::with_task(self.copy_selected_block(
                tab_id,
                terminal_id,
                CopyKind::Command,
            )),
            TabEvent::CopySelection {
                tab_id,
                terminal_id,
            } => TerminalScreenUpdate::with_task(
                self.copy_selection(tab_id, terminal_id),
            ),
            TabEvent::PasteIntoPrompt {
                tab_id,
                terminal_id,
            } => TerminalScreenUpdate::with_task(
                self.paste_into_prompt(tab_id, terminal_id),
            ),
            TabEvent::SplitPane { tab_id, pane, axis } => {
                self.split_pane(tab_id, pane, axis)
            },
            TabEvent::ClosePane { tab_id, pane } => {
                self.close_pane_by_id(tab_id, pane)
            },
            TabEvent::PaneClicked { tab_id, pane } => {
                self.with_tab_mut(tab_id, |tab| tab.focus_pane(pane))
            },
            TabEvent::PaneResized { tab_id, event } => {
                if let Some(tab) = self.tab_mut(tab_id) {
                    tab.resize(event);
                }
                TerminalScreenUpdate::none()
            },
            TabEvent::PaneGridCursorMoved { tab_id, position } => self
                .with_tab_mut(tab_id, move |tab| {
                    tab.update_grid_cursor(position);
                    TabAction::none()
                }),
        }
    }

    fn update_terminal(
        &mut self,
        event: otty_ui_term::Event,
    ) -> TerminalScreenUpdate {
        let id = *event.terminal_id();
        if let Some(index) = self.tab_index_by_terminal(id) {
            let refresh_titles = matches!(
                event,
                otty_ui_term::Event::TitleChanged { .. }
                    | otty_ui_term::Event::ResetTitle { .. }
            );
            let selection_task = self.update_block_selection(index, &event);
            let action = self.tabs[index]
                .handle_terminal_event(event, &self.config.shell_name);
            let tab_update = self.resolve_tab_action(index, action);
            let selection_update =
                TerminalScreenUpdate::with_task(selection_task);

            if refresh_titles {
                self.refresh_tab_summaries();
            }

            return selection_update.merge(tab_update);
        }

        TerminalScreenUpdate::none()
    }

    fn update_block_selection(
        &mut self,
        tab_index: usize,
        event: &otty_ui_term::Event,
    ) -> Task<TerminalScreenEvent> {
        use otty_ui_term::Event::*;

        match event {
            BlockSelected { block_id, .. } => {
                let terminal_id = *event.terminal_id();
                let tab = &mut self.tabs[tab_index];
                tab.set_selected_block(terminal_id, block_id.clone());

                let mut tasks = Vec::new();
                for entry in tab.terminals().values() {
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
    ) -> Task<TerminalScreenEvent> {
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

        let widget_id = {
            let tab = &self.tabs[tab_index];

            let entry = match tab.terminals().get(&terminal_id) {
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
            self.with_tab_mut(tab_id, TabState::close_context_menu).task;
        let copy_task = otty_ui_term::TerminalView::command(widget_id, command);

        Task::batch(vec![close_menu_task, copy_task])
    }

    fn copy_selection(
        &mut self,
        tab_id: u64,
        terminal_id: u64,
    ) -> Task<TerminalScreenEvent> {
        let Some(tab_index) = self.tab_index_by_id(tab_id) else {
            return Task::none();
        };

        let widget_id = {
            let tab = &self.tabs[tab_index];
            let entry = match tab.terminals().get(&terminal_id) {
                Some(entry) => entry,
                None => return Task::none(),
            };

            entry.terminal.widget_id().clone()
        };

        let close_menu_task =
            self.with_tab_mut(tab_id, TabState::close_context_menu).task;
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
    ) -> Task<TerminalScreenEvent> {
        let Some(tab_index) = self.tab_index_by_id(tab_id) else {
            return Task::none();
        };

        let widget_id = {
            let tab = &self.tabs[tab_index];
            let entry = match tab.terminals().get(&terminal_id) {
                Some(entry) => entry,
                None => return Task::none(),
            };

            entry.terminal.widget_id().clone()
        };

        let close_menu_task =
            self.with_tab_mut(tab_id, TabState::close_context_menu).task;
        let paste_task = otty_ui_term::TerminalView::command(
            widget_id,
            otty_ui_term::BlockCommand::PasteClipboard,
        );

        Task::batch(vec![close_menu_task, paste_task])
    }

    fn create_tab(&mut self) -> Task<TerminalScreenEvent> {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let terminal_id = self.next_terminal_id;
        self.next_terminal_id += 1;

        let (mut tab, focus_task) = TabState::new(
            tab_id,
            self.config.shell_name.clone(),
            terminal_id,
            &self.config.terminal_settings,
        );

        tab.set_grid_size(self.pane_grid_size());
        self.tabs.push(tab);
        self.refresh_tab_summaries();

        self.active_tab_index = self.tabs.len() - 1;

        let sync_task = self.sync_tab_block_selection(self.active_tab_index);

        Task::batch(vec![focus_task, sync_task])
    }

    fn close_tab(&mut self, id: u64) -> TerminalScreenUpdate {
        if self.tabs.len() == 1 {
            return TerminalScreenUpdate::with_action(
                TerminalScreenAction::CloseWindow,
            );
        }

        if let Some(index) = self.tabs.iter().position(|tab| tab.id() == id) {
            self.tabs.remove(index);

            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len().saturating_sub(1);
            }

            if self.tabs.is_empty() {
                return TerminalScreenUpdate::none();
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

            self.refresh_tab_summaries();

            return TerminalScreenUpdate::with_task(Task::batch(vec![
                focus_task, sync_task,
            ]));
        }

        TerminalScreenUpdate::none()
    }

    fn activate_tab(&mut self, id: u64) -> Task<TerminalScreenEvent> {
        if let Some(index) = self.tabs.iter().position(|tab| tab.id() == id) {
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
    ) -> TerminalScreenUpdate {
        let terminal_id = self.next_terminal_id;
        self.next_terminal_id += 1;
        let settings = self.config.terminal_settings.clone();

        self.with_tab_mut(tab_id, move |tab| {
            tab.split_pane(pane, axis, terminal_id, &settings)
        })
    }

    fn close_pane_by_id(
        &mut self,
        tab_id: u64,
        pane: pane_grid::Pane,
    ) -> TerminalScreenUpdate {
        self.with_tab_mut(tab_id, |tab| tab.close_pane(pane))
    }

    fn resolve_tab_action(
        &mut self,
        tab_index: usize,
        action: TabAction,
    ) -> TerminalScreenUpdate {
        let mut update = if action.close_tab {
            let tab_id = self.tabs[tab_index].id();
            self.close_tab(tab_id)
        } else {
            TerminalScreenUpdate::none()
        };

        if let Some(task) = action.task {
            update = update.merge(TerminalScreenUpdate::with_task(task));
        }

        update
    }

    fn with_tab_mut<F>(&mut self, tab_id: u64, f: F) -> TerminalScreenUpdate
    where
        F: FnOnce(&mut TabState) -> TabAction,
    {
        if let Some(index) = self.tab_index_by_id(tab_id) {
            let action = f(&mut self.tabs[index]);
            return self.resolve_tab_action(index, action);
        }

        TerminalScreenUpdate::none()
    }

    fn tab_index_by_id(&self, id: u64) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id() == id)
    }

    fn tab_index_by_terminal(&self, terminal_id: u64) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.contains_terminal(terminal_id))
    }

    fn tab_mut(&mut self, id: u64) -> Option<&mut TabState> {
        self.tabs.iter_mut().find(|tab| tab.id() == id)
    }

    fn sync_tab_block_selection(
        &self,
        tab_index: usize,
    ) -> Task<TerminalScreenEvent> {
        let Some(tab) = self.tabs.get(tab_index) else {
            return Task::none();
        };

        let selection = tab.selected_block().cloned();
        let mut tasks = Vec::new();
        for entry in tab.terminals().values() {
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
        let tab_bar_height = TabBarMetrics::default().height;
        let height = (self.screen_size.height - tab_bar_height).max(0.0);
        Size::new(self.screen_size.width, height)
    }

    fn sync_tab_grid_sizes(&mut self) {
        let size = self.pane_grid_size();
        for tab in &mut self.tabs {
            tab.set_grid_size(size);
        }
    }

    fn refresh_tab_summaries(&mut self) {
        self.tab_summaries = self
            .tabs
            .iter()
            .map(|tab| TabSummary {
                id: tab.id(),
                title: tab.title().to_string(),
            })
            .collect();
    }
}

#[derive(Clone, Copy, Debug)]
enum CopyKind {
    Content,
    Prompt,
    Command,
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;
    use otty_ui_term::settings::Settings;

    fn test_screen(size: Size) -> TerminalScreen {
        let config = AppConfig {
            shell_name: String::from("zsh"),
            terminal_settings: Settings::default(),
        };
        TerminalScreen::new(config, size)
    }

    #[test]
    fn default_tab_titles_use_shell_name() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);
        let _ = screen.update(TerminalScreenEvent::NewTab);

        assert_eq!(screen.tabs[0].title(), "zsh");
        assert_eq!(screen.tabs[1].title(), "zsh");
    }

    #[test]
    fn closing_tab_updates_active_index() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);
        let _ = screen.update(TerminalScreenEvent::NewTab);
        let first_id = screen.tabs[0].id();

        let _ =
            screen.update(TerminalScreenEvent::TabBar(TabBarEvent::TabButton(
                crate::components::tab_button::TabButtonEvent::CloseTab(
                    first_id,
                ),
            )));

        assert_eq!(screen.tabs.len(), 1);
        assert_eq!(screen.active_tab_index, 0);
    }

    #[test]
    fn splitting_active_pane_creates_new_terminal() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);
        let tab_id = screen.tabs[0].id();
        let pane = screen.tabs[0].focus().expect("expected focus");

        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::SplitPane {
            tab_id,
            pane,
            axis: pane_grid::Axis::Horizontal,
        }));

        assert_eq!(screen.tabs[0].panes().len(), 2);
        assert_eq!(screen.tabs[0].terminals().len(), 2);
    }

    #[test]
    fn closing_pane_removes_terminal() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);
        let tab_id = screen.tabs[0].id();
        let first_pane = screen.tabs[0].focus().expect("expected pane");
        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::SplitPane {
            tab_id,
            pane: first_pane,
            axis: pane_grid::Axis::Vertical,
        }));

        let second_pane = screen.tabs[0]
            .panes()
            .iter()
            .map(|(pane, _)| pane)
            .copied()
            .find(|pane| *pane != first_pane)
            .expect("missing second pane");

        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::ClosePane {
            tab_id,
            pane: second_pane,
        }));

        assert_eq!(screen.tabs[0].panes().len(), 1);
        assert_eq!(screen.tabs[0].terminals().len(), 1);
    }

    #[test]
    fn closing_last_pane_closes_the_tab() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);
        let _ = screen.update(TerminalScreenEvent::NewTab);
        let first_tab_id = screen.tabs[0].id();
        let pane = screen.tabs[0].focus().expect("expected pane");

        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::ClosePane {
            tab_id: first_tab_id,
            pane,
        }));

        assert_eq!(screen.tabs.len(), 1);
        assert!(screen.tabs.iter().all(|tab| tab.id() != first_tab_id));
    }

    #[test]
    fn activating_tab_emits_activate_view_action() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);
        let _ = screen.update(TerminalScreenEvent::NewTab);
        let first_id = screen.tabs[0].id();

        let update =
            screen.update(TerminalScreenEvent::TabBar(TabBarEvent::TabButton(
                crate::components::tab_button::TabButtonEvent::ActivateTab(
                    first_id,
                ),
            )));

        assert_eq!(update.action, TerminalScreenAction::ActivateView);
        assert_eq!(screen.active_tab_index, 0);
    }

    #[test]
    fn new_tab_emits_activate_view_action() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let update = screen.update(TerminalScreenEvent::NewTab);

        assert_eq!(update.action, TerminalScreenAction::ActivateView);
    }

    #[test]
    fn context_menu_stays_visible_near_edges() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);
        screen.sync_tab_grid_sizes();
        let tab_id = screen.tabs[0].id();
        let pane = screen.tabs[0].focus().expect("focus");
        let terminal_id =
            screen.tabs[0].pane_terminal_id(pane).expect("terminal");

        let grid_size = screen.tabs[0].grid_size();
        let cursor = Point::new(grid_size.width - 4.0, grid_size.height - 4.0);
        let _ = screen.update(TerminalScreenEvent::Tab(
            TabEvent::PaneGridCursorMoved {
                tab_id,
                position: cursor,
            },
        ));

        let _ = screen.update(TerminalScreenEvent::Tab(
            TabEvent::OpenContextMenu {
                tab_id,
                pane,
                terminal_id,
            },
        ));

        let menu = screen.tabs[0].context_menu().expect("menu");
        let has_block_selection = screen.tabs[0]
            .selected_block()
            .filter(|sel| sel.terminal_id == menu.terminal_id)
            .is_some();
        let mut item_count = 5;
        if has_block_selection {
            item_count += 3;
        }
        let menu_height =
            crate::widgets::pane_context_menu::menu_height_for_items(
                item_count,
            );
        assert_eq!(
            menu.anchor_for_height(menu_height),
            crate::widgets::pane_context_menu::anchor_position(
                cursor,
                grid_size,
                menu_height
            )
        );
    }

    #[test]
    fn selecting_block_in_one_tab_updates_tab_selection() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);

        let first_terminal_id = screen.tabs[0]
            .focused_terminal_id()
            .expect("first terminal");

        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::Terminal(
            otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            },
        )));

        let selection = screen.tabs[0].selected_block().expect("tab selection");
        assert_eq!(selection.terminal_id, first_terminal_id);
        assert_eq!(selection.block_id, "block-a");
    }

    #[test]
    fn tab_selection_persists_when_switching_tabs() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);
        let _ = screen.update(TerminalScreenEvent::NewTab);

        let first_terminal_id = screen.tabs[0]
            .focused_terminal_id()
            .expect("first terminal");
        let second_terminal_id = screen.tabs[1]
            .focused_terminal_id()
            .expect("second terminal");

        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::Terminal(
            otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            },
        )));
        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::Terminal(
            otty_ui_term::Event::BlockSelected {
                id: second_terminal_id,
                block_id: String::from("block-b"),
            },
        )));

        let _ =
            screen.update(TerminalScreenEvent::TabBar(TabBarEvent::TabButton(
                crate::components::tab_button::TabButtonEvent::ActivateTab(
                    screen.tabs[0].id(),
                ),
            )));
        let _ =
            screen.update(TerminalScreenEvent::TabBar(TabBarEvent::TabButton(
                crate::components::tab_button::TabButtonEvent::ActivateTab(
                    screen.tabs[1].id(),
                ),
            )));

        assert_eq!(
            screen.tabs[0]
                .selected_block()
                .expect("tab0 selection")
                .block_id,
            "block-a"
        );
        assert_eq!(
            screen.tabs[1]
                .selected_block()
                .expect("tab1 selection")
                .block_id,
            "block-b"
        );
    }

    #[test]
    fn creating_new_tab_preserves_existing_tab_selection() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);
        let first_terminal_id = screen.tabs[0]
            .focused_terminal_id()
            .expect("first terminal");

        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::Terminal(
            otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            },
        )));

        let _ = screen.update(TerminalScreenEvent::NewTab);

        assert!(screen.tabs[0].selected_block().is_some());
        assert!(screen.tabs[1].selected_block().is_none());
    }

    #[test]
    fn selecting_block_in_new_pane_replaces_previous_selection() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);

        let tab_id = screen.tabs[0].id();
        let first_terminal_id = screen.tabs[0]
            .focused_terminal_id()
            .expect("first terminal");
        let first_pane = screen.tabs[0].focus().expect("first pane");

        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::Terminal(
            otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            },
        )));

        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::SplitPane {
            tab_id,
            pane: first_pane,
            axis: pane_grid::Axis::Horizontal,
        }));

        let second_terminal_id = screen.tabs[0]
            .terminals()
            .keys()
            .copied()
            .find(|id| *id != first_terminal_id)
            .expect("second terminal id");

        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::Terminal(
            otty_ui_term::Event::BlockSelected {
                id: second_terminal_id,
                block_id: String::from("block-b"),
            },
        )));

        let selection = screen.tabs[0].selected_block().expect("tab selection");
        assert_eq!(selection.terminal_id, second_terminal_id);
        assert_eq!(selection.block_id, "block-b");
    }

    #[test]
    fn focusing_other_pane_does_not_clear_selection() {
        let mut screen = test_screen(Size::new(800.0, 600.0));

        let _ = screen.update(TerminalScreenEvent::NewTab);

        let tab_id = screen.tabs[0].id();
        let first_pane = screen.tabs[0].focus().expect("initial pane");

        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::SplitPane {
            tab_id,
            pane: first_pane,
            axis: pane_grid::Axis::Horizontal,
        }));

        let focused_terminal_id = screen.tabs[0]
            .focused_terminal_id()
            .expect("focused terminal after split");

        let _ = screen.update(TerminalScreenEvent::Tab(TabEvent::Terminal(
            otty_ui_term::Event::BlockSelected {
                id: focused_terminal_id,
                block_id: String::from("block-a"),
            },
        )));

        let other_pane = screen.tabs[0]
            .terminals()
            .iter()
            .find(|(terminal_id, _)| **terminal_id != focused_terminal_id)
            .map(|(_, entry)| entry.pane)
            .expect("other pane");

        let _ =
            screen.update(TerminalScreenEvent::Tab(TabEvent::PaneClicked {
                tab_id,
                pane: other_pane,
            }));

        assert!(screen.tabs[0].selected_block().is_some());
    }
}
