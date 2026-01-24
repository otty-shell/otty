use std::collections::HashMap;

use iced::widget::pane_grid;
use iced::{Size, Task};
use otty_ui_term::TerminalView;

use crate::app::config::AppConfig;
use crate::app::state::AppEvent;
use crate::app::terminal_state::{TabAction, TerminalTabState};
use crate::components::tab_button::TabButtonEvent;
use crate::widgets::tab::TabEvent;
use crate::widgets::tab_bar::{TabBarEvent, TabBarMetrics, TabSummary};

pub(crate) type TabId = u64;
pub(crate) type TerminalId = u64;

#[derive(Debug, Clone)]
pub(crate) enum WorkspaceEvent {
    NewTab { kind: TabKind },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum TabKind {
    Terminal,
}

pub(crate) struct TabItem {
    pub(crate) id: TabId,
    pub(crate) title: String,
    pub(crate) content: TabContent,
}

pub(crate) enum TabContent {
    Terminal(TerminalTabState),
}

pub(crate) struct WorkspaceState {
    pub(crate) tabs: Vec<TabId>,
    pub(crate) active_tab_id: Option<TabId>,
    pub(crate) tab_items: HashMap<TabId, TabItem>,
    pub(crate) next_tab_id: TabId,
    pub(crate) next_terminal_id: TerminalId,
    pub(crate) window_size: Size,
    pub(crate) screen_size: Size,
}

impl WorkspaceState {
    pub(crate) fn new(window_size: Size, screen_size: Size) -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_id: None,
            tab_items: HashMap::new(),
            next_tab_id: 0,
            next_terminal_id: 0,
            window_size,
            screen_size,
        }
    }

    pub(crate) fn active_tab_title(&self) -> Option<&str> {
        self.active_tab_id
            .and_then(|id| self.tab_items.get(&id))
            .map(|tab| tab.title.as_str())
    }

    pub(crate) fn tab_summaries(&self) -> Vec<TabSummary> {
        self.tabs
            .iter()
            .filter_map(|id| self.tab_items.get(id))
            .map(|tab| TabSummary {
                id: tab.id,
                title: tab.title.clone(),
            })
            .collect()
    }

    pub(crate) fn active_tab(&self) -> Option<&TabItem> {
        let id = self.active_tab_id?;
        self.tab_items.get(&id)
    }

    pub(crate) fn set_screen_size(&mut self, size: Size) {
        self.screen_size = size;
        self.sync_tab_grid_sizes();
    }

    pub(crate) fn sync_tab_grid_sizes(&mut self) {
        let size = self.pane_grid_size();
        for tab in self.tab_items.values_mut() {
            let TabContent::Terminal(terminal) = &mut tab.content;
            terminal.set_grid_size(size);
        }
    }

    pub(crate) fn pane_grid_size(&self) -> Size {
        let tab_bar_height = TabBarMetrics::default().height;
        let height = (self.screen_size.height - tab_bar_height).max(0.0);
        Size::new(self.screen_size.width, height)
    }
}

pub(crate) fn tab_reducer(
    workspace: &mut WorkspaceState,
    config: &AppConfig,
    event: TabEvent,
) -> Task<AppEvent> {
    match event {
        TabEvent::Terminal(inner) => terminal_reducer(workspace, config, inner),
        TabEvent::OpenContextMenu {
            tab_id,
            pane,
            terminal_id,
        } => with_terminal_tab(workspace, tab_id, |tab| {
            tab.open_context_menu(pane, terminal_id)
        }),
        TabEvent::CloseContextMenu { tab_id } => with_terminal_tab(
            workspace,
            tab_id,
            TerminalTabState::close_context_menu,
        ),
        TabEvent::ContextMenuInput => Task::none(),
        TabEvent::CopySelectedBlockContent {
            tab_id,
            terminal_id,
        } => copy_selected_block(
            workspace,
            tab_id,
            terminal_id,
            CopyKind::Content,
        ),
        TabEvent::CopySelectedBlockPrompt {
            tab_id,
            terminal_id,
        } => copy_selected_block(
            workspace,
            tab_id,
            terminal_id,
            CopyKind::Prompt,
        ),
        TabEvent::CopySelectedBlockCommand {
            tab_id,
            terminal_id,
        } => copy_selected_block(
            workspace,
            tab_id,
            terminal_id,
            CopyKind::Command,
        ),
        TabEvent::CopySelection {
            tab_id,
            terminal_id,
        } => copy_selection(workspace, tab_id, terminal_id),
        TabEvent::PasteIntoPrompt {
            tab_id,
            terminal_id,
        } => paste_into_prompt(workspace, tab_id, terminal_id),
        TabEvent::SplitPane { tab_id, pane, axis } => {
            split_pane(workspace, config, tab_id, pane, axis)
        },
        TabEvent::ClosePane { tab_id, pane } => {
            close_pane_by_id(workspace, tab_id, pane)
        },
        TabEvent::PaneClicked { tab_id, pane } => {
            with_terminal_tab(workspace, tab_id, |tab| tab.focus_pane(pane))
        },
        TabEvent::PaneResized { tab_id, event } => {
            if let Some(tab) = terminal_tab_mut(workspace, tab_id) {
                tab.resize(event);
                sync_tab_title(workspace, tab_id);
            }
            Task::none()
        },
        TabEvent::PaneGridCursorMoved { tab_id, position } => {
            with_terminal_tab(workspace, tab_id, move |tab| {
                tab.update_grid_cursor(position);
                TabAction::none()
            })
        },
        TabEvent::ActivateTab { tab_id } => activate_tab(workspace, tab_id),
        TabEvent::CloseTab { tab_id } => close_tab(workspace, tab_id),
    }
}

pub(crate) fn workspace_reducer(
    workspace: &mut WorkspaceState,
    config: &AppConfig,
    event: WorkspaceEvent,
) -> Task<AppEvent> {
    match event {
        WorkspaceEvent::NewTab { kind } => match kind {
            TabKind::Terminal => create_terminal_tab(workspace, config),
        },
    }
}

pub(crate) fn terminal_reducer(
    workspace: &mut WorkspaceState,
    config: &AppConfig,
    event: otty_ui_term::Event,
) -> Task<AppEvent> {
    let terminal_id = *event.terminal_id();
    let Some(tab_id) = tab_id_by_terminal(workspace, terminal_id) else {
        return Task::none();
    };

    let refresh_titles = matches!(
        event,
        otty_ui_term::Event::TitleChanged { .. }
            | otty_ui_term::Event::ResetTitle { .. }
    );

    let selection_task = update_block_selection(workspace, tab_id, &event);
    let action = with_terminal_tab_action(workspace, tab_id, |tab| {
        tab.handle_terminal_event(event, &config.shell_name)
    });
    let mut update = resolve_tab_action(workspace, tab_id, action);

    update = Task::batch(vec![selection_task, update]);

    if refresh_titles {
        sync_tab_title(workspace, tab_id);
    }

    update
}

fn create_terminal_tab(
    workspace: &mut WorkspaceState,
    config: &AppConfig,
) -> Task<AppEvent> {
    let tab_id = workspace.next_tab_id;
    workspace.next_tab_id += 1;

    let terminal_id = workspace.next_terminal_id;
    workspace.next_terminal_id += 1;

    let (mut tab, focus_task) = TerminalTabState::new(
        config.shell_name.clone(),
        terminal_id,
        &config.terminal_settings,
    );

    tab.set_grid_size(workspace.pane_grid_size());

    let title = tab.title().to_string();
    workspace.tabs.push(tab_id);
    workspace.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title,
            content: TabContent::Terminal(tab),
        },
    );
    workspace.active_tab_id = Some(tab_id);

    let sync_task = sync_tab_block_selection(workspace, tab_id);

    Task::batch(vec![focus_task, sync_task])
}

fn close_tab(workspace: &mut WorkspaceState, tab_id: TabId) -> Task<AppEvent> {
    if workspace.tabs.len() == 1 {
        return close_window_task();
    }

    let Some(pos) = workspace.tabs.iter().position(|id| *id == tab_id) else {
        return Task::none();
    };

    workspace.tabs.remove(pos);
    workspace.tab_items.remove(&tab_id);

    if workspace.tabs.is_empty() {
        workspace.active_tab_id = None;
        return Task::none();
    }

    if workspace.active_tab_id == Some(tab_id) {
        let new_index = pos.min(workspace.tabs.len() - 1);
        workspace.active_tab_id = Some(workspace.tabs[new_index]);
    }

    let focus_task = focus_active_terminal(workspace);
    let sync_task = if let Some(active_id) = workspace.active_tab_id {
        sync_tab_block_selection(workspace, active_id)
    } else {
        Task::none()
    };

    Task::batch(vec![focus_task, sync_task])
}

fn activate_tab(
    workspace: &mut WorkspaceState,
    tab_id: TabId,
) -> Task<AppEvent> {
    if !workspace.tabs.contains(&tab_id) {
        return Task::none();
    }

    workspace.active_tab_id = Some(tab_id);
    let focus_task = focus_active_terminal(workspace);
    let sync_task = sync_tab_block_selection(workspace, tab_id);

    Task::batch(vec![focus_task, sync_task])
}

fn focus_active_terminal(workspace: &WorkspaceState) -> Task<AppEvent> {
    let Some(tab) = workspace.active_tab() else {
        return Task::none();
    };

    let TabContent::Terminal(terminal) = &tab.content;
    match terminal.focused_terminal_entry() {
        Some(entry) => TerminalView::focus(entry.terminal.widget_id().clone()),
        None => Task::none(),
    }
}

fn split_pane(
    workspace: &mut WorkspaceState,
    config: &AppConfig,
    tab_id: TabId,
    pane: pane_grid::Pane,
    axis: pane_grid::Axis,
) -> Task<AppEvent> {
    let terminal_id = workspace.next_terminal_id;
    workspace.next_terminal_id += 1;
    let settings = config.terminal_settings.clone();

    with_terminal_tab(workspace, tab_id, move |tab| {
        tab.split_pane(pane, axis, terminal_id, &settings)
    })
}

fn close_pane_by_id(
    workspace: &mut WorkspaceState,
    tab_id: TabId,
    pane: pane_grid::Pane,
) -> Task<AppEvent> {
    with_terminal_tab(workspace, tab_id, |tab| tab.close_pane(pane))
}

fn update_block_selection(
    workspace: &mut WorkspaceState,
    tab_id: TabId,
    event: &otty_ui_term::Event,
) -> Task<AppEvent> {
    use otty_ui_term::Event::*;

    match event {
        BlockSelected { block_id, .. } => {
            let terminal_id = *event.terminal_id();
            let Some(tab) = terminal_tab_mut(workspace, tab_id) else {
                return Task::none();
            };
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
            let Some(tab) = terminal_tab_mut(workspace, tab_id) else {
                return Task::none();
            };
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
    workspace: &mut WorkspaceState,
    tab_id: TabId,
    terminal_id: TerminalId,
    kind: CopyKind,
) -> Task<AppEvent> {
    let Some(tab) = terminal_tab_mut(workspace, tab_id) else {
        return Task::none();
    };

    let block_id = {
        let Some(selection) = tab.selected_block() else {
            return Task::none();
        };

        if selection.terminal_id != terminal_id {
            return Task::none();
        }

        selection.block_id.clone()
    };

    let widget_id = {
        let entry = match tab.terminals().get(&terminal_id) {
            Some(entry) => entry,
            None => return Task::none(),
        };

        entry.terminal.widget_id().clone()
    };
    let command = match kind {
        CopyKind::Content => otty_ui_term::BlockCommand::CopyContent(block_id),
        CopyKind::Prompt => otty_ui_term::BlockCommand::CopyPrompt(block_id),
        CopyKind::Command => otty_ui_term::BlockCommand::CopyCommand(block_id),
    };

    let close_menu_task = tab.close_context_menu().task;
    let copy_task = otty_ui_term::TerminalView::command(widget_id, command);

    Task::batch(vec![close_menu_task.unwrap_or_else(Task::none), copy_task])
}

fn copy_selection(
    workspace: &mut WorkspaceState,
    tab_id: TabId,
    terminal_id: TerminalId,
) -> Task<AppEvent> {
    let Some(tab) = terminal_tab_mut(workspace, tab_id) else {
        return Task::none();
    };

    let widget_id = {
        let entry = match tab.terminals().get(&terminal_id) {
            Some(entry) => entry,
            None => return Task::none(),
        };

        entry.terminal.widget_id().clone()
    };

    let close_menu_task = tab.close_context_menu().task;
    let copy_task = otty_ui_term::TerminalView::command(
        widget_id,
        otty_ui_term::BlockCommand::CopySelection,
    );

    Task::batch(vec![close_menu_task.unwrap_or_else(Task::none), copy_task])
}

fn paste_into_prompt(
    workspace: &mut WorkspaceState,
    tab_id: TabId,
    terminal_id: TerminalId,
) -> Task<AppEvent> {
    let Some(tab) = terminal_tab_mut(workspace, tab_id) else {
        return Task::none();
    };

    let widget_id = {
        let entry = match tab.terminals().get(&terminal_id) {
            Some(entry) => entry,
            None => return Task::none(),
        };

        entry.terminal.widget_id().clone()
    };

    let close_menu_task = tab.close_context_menu().task;
    let paste_task = otty_ui_term::TerminalView::command(
        widget_id,
        otty_ui_term::BlockCommand::PasteClipboard,
    );

    Task::batch(vec![close_menu_task.unwrap_or_else(Task::none), paste_task])
}

fn with_terminal_tab<F>(
    workspace: &mut WorkspaceState,
    tab_id: TabId,
    f: F,
) -> Task<AppEvent>
where
    F: FnOnce(&mut TerminalTabState) -> TabAction,
{
    let action = with_terminal_tab_action(workspace, tab_id, f);
    resolve_tab_action(workspace, tab_id, action)
}

fn with_terminal_tab_action<F>(
    workspace: &mut WorkspaceState,
    tab_id: TabId,
    f: F,
) -> TabAction
where
    F: FnOnce(&mut TerminalTabState) -> TabAction,
{
    let Some(tab) = terminal_tab_mut(workspace, tab_id) else {
        return TabAction::none();
    };

    let action = f(tab);
    sync_tab_title(workspace, tab_id);
    action
}

fn resolve_tab_action(
    workspace: &mut WorkspaceState,
    tab_id: TabId,
    action: TabAction,
) -> Task<AppEvent> {
    let mut update = if action.close_tab {
        close_tab(workspace, tab_id)
    } else {
        Task::none()
    };

    if let Some(task) = action.task {
        update = Task::batch(vec![update, task]);
    }

    update
}

fn terminal_tab_mut(
    workspace: &mut WorkspaceState,
    tab_id: TabId,
) -> Option<&mut TerminalTabState> {
    workspace.tab_items.get_mut(&tab_id).map(|tab| {
        let TabContent::Terminal(terminal) = &mut tab.content;
        terminal
    })
}

fn sync_tab_title(workspace: &mut WorkspaceState, tab_id: TabId) {
    let Some(tab) = workspace.tab_items.get_mut(&tab_id) else {
        return;
    };

    let TabContent::Terminal(terminal) = &tab.content;
    tab.title = terminal.title().to_string();
}

fn terminal_tab(
    workspace: &WorkspaceState,
    tab_id: TabId,
) -> Option<&TerminalTabState> {
    workspace.tab_items.get(&tab_id).map(|tab| {
        let TabContent::Terminal(terminal) = &tab.content;
        terminal
    })
}

fn tab_id_by_terminal(
    workspace: &WorkspaceState,
    terminal_id: TerminalId,
) -> Option<TabId> {
    workspace.tabs.iter().copied().find(|tab_id| {
        terminal_tab(workspace, *tab_id)
            .map(|terminal| terminal.contains_terminal(terminal_id))
            .unwrap_or(false)
    })
}

fn sync_tab_block_selection(
    workspace: &WorkspaceState,
    tab_id: TabId,
) -> Task<AppEvent> {
    let Some(tab) = workspace.tab_items.get(&tab_id) else {
        return Task::none();
    };

    let TabContent::Terminal(terminal) = &tab.content;
    let selection = terminal.selected_block().cloned();
    let mut tasks = Vec::new();
    for entry in terminal.terminals().values() {
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

fn close_window_task() -> Task<AppEvent> {
    iced::window::latest().and_then(iced::window::close)
}

fn map_tab_bar_event(event: TabBarEvent) -> TabEvent {
    match event {
        TabBarEvent::TabButton(button_event) => match button_event {
            TabButtonEvent::ActivateTab(id) => {
                TabEvent::ActivateTab { tab_id: id }
            },
            TabButtonEvent::CloseTab(id) => TabEvent::CloseTab { tab_id: id },
        },
    }
}

pub(crate) fn map_tab_bar_event_to_app(event: TabBarEvent) -> AppEvent {
    AppEvent::Tab(map_tab_bar_event(event))
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

    fn test_workspace(size: Size) -> (WorkspaceState, AppConfig) {
        let config = AppConfig {
            shell_name: String::from("zsh"),
            terminal_settings: Settings::default(),
        };
        let workspace = WorkspaceState::new(size, size);
        (workspace, config)
    }

    fn new_tab(workspace: &mut WorkspaceState, config: &AppConfig) {
        let _ = workspace_reducer(
            workspace,
            config,
            WorkspaceEvent::NewTab {
                kind: TabKind::Terminal,
            },
        );
    }

    fn tab_event(
        workspace: &mut WorkspaceState,
        config: &AppConfig,
        event: TabEvent,
    ) {
        let _ = tab_reducer(workspace, config, event);
    }

    fn term_event(
        workspace: &mut WorkspaceState,
        config: &AppConfig,
        event: otty_ui_term::Event,
    ) {
        let _ = terminal_reducer(workspace, config, event);
    }

    #[test]
    fn default_tab_titles_use_shell_name() {
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);
        new_tab(&mut workspace, &config);

        let first = workspace.tabs[0];
        let second = workspace.tabs[1];
        assert_eq!(workspace.tab_items[&first].title, "zsh");
        assert_eq!(workspace.tab_items[&second].title, "zsh");
    }

    #[test]
    fn closing_tab_updates_active_tab() {
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);
        new_tab(&mut workspace, &config);
        let first_id = workspace.tabs[0];

        tab_event(
            &mut workspace,
            &config,
            TabEvent::CloseTab { tab_id: first_id },
        );

        assert_eq!(workspace.tabs.len(), 1);
        assert_eq!(workspace.active_tab_id, Some(workspace.tabs[0]));
    }

    #[test]
    fn splitting_active_pane_creates_new_terminal() {
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);
        let tab_id = workspace.tabs[0];
        let pane = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.focus())
            .expect("expected focus");

        tab_event(
            &mut workspace,
            &config,
            TabEvent::SplitPane {
                tab_id,
                pane,
                axis: pane_grid::Axis::Horizontal,
            },
        );

        let tab = terminal_tab_mut(&mut workspace, tab_id).expect("tab");
        assert_eq!(tab.panes().len(), 2);
        assert_eq!(tab.terminals().len(), 2);
    }

    #[test]
    fn closing_pane_removes_terminal() {
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);
        let tab_id = workspace.tabs[0];
        let first_pane = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.focus())
            .expect("expected pane");

        tab_event(
            &mut workspace,
            &config,
            TabEvent::SplitPane {
                tab_id,
                pane: first_pane,
                axis: pane_grid::Axis::Vertical,
            },
        );

        let second_pane = terminal_tab_mut(&mut workspace, tab_id)
            .expect("tab")
            .panes()
            .iter()
            .map(|(pane, _)| pane)
            .copied()
            .find(|pane| *pane != first_pane)
            .expect("missing second pane");

        tab_event(
            &mut workspace,
            &config,
            TabEvent::ClosePane {
                tab_id,
                pane: second_pane,
            },
        );

        let tab = terminal_tab_mut(&mut workspace, tab_id).expect("tab");
        assert_eq!(tab.panes().len(), 1);
        assert_eq!(tab.terminals().len(), 1);
    }

    #[test]
    fn closing_last_pane_closes_the_tab() {
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);
        new_tab(&mut workspace, &config);
        let first_tab_id = workspace.tabs[0];
        let pane = terminal_tab_mut(&mut workspace, first_tab_id)
            .and_then(|tab| tab.focus())
            .expect("expected pane");

        tab_event(
            &mut workspace,
            &config,
            TabEvent::ClosePane {
                tab_id: first_tab_id,
                pane,
            },
        );

        assert_eq!(workspace.tabs.len(), 1);
        assert!(workspace.tabs.iter().all(|id| *id != first_tab_id));
    }

    #[test]
    fn activating_tab_updates_active_tab() {
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);
        new_tab(&mut workspace, &config);
        let first_id = workspace.tabs[0];

        tab_event(
            &mut workspace,
            &config,
            TabEvent::ActivateTab { tab_id: first_id },
        );

        assert_eq!(workspace.active_tab_id, Some(first_id));
    }

    #[test]
    fn context_menu_stays_visible_near_edges() {
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);
        workspace.sync_tab_grid_sizes();
        let tab_id = workspace.tabs[0];
        let pane = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.focus())
            .expect("focus");
        let terminal_id = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.pane_terminal_id(pane))
            .expect("terminal");

        let grid_size = terminal_tab_mut(&mut workspace, tab_id)
            .expect("tab")
            .grid_size();
        let cursor = Point::new(grid_size.width - 4.0, grid_size.height - 4.0);
        tab_event(
            &mut workspace,
            &config,
            TabEvent::PaneGridCursorMoved {
                tab_id,
                position: cursor,
            },
        );

        tab_event(
            &mut workspace,
            &config,
            TabEvent::OpenContextMenu {
                tab_id,
                pane,
                terminal_id,
            },
        );

        let tab = terminal_tab_mut(&mut workspace, tab_id).expect("tab");
        let menu = tab.context_menu().expect("menu");
        let has_block_selection = tab
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
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);

        let tab_id = workspace.tabs[0];
        let first_terminal_id = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.focused_terminal_id())
            .expect("first terminal");

        term_event(
            &mut workspace,
            &config,
            otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            },
        );

        let selection = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.selected_block())
            .expect("tab selection");
        assert_eq!(selection.terminal_id, first_terminal_id);
        assert_eq!(selection.block_id, "block-a");
    }

    #[test]
    fn tab_selection_persists_when_switching_tabs() {
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);
        new_tab(&mut workspace, &config);

        let first_tab_id = workspace.tabs[0];
        let second_tab_id = workspace.tabs[1];

        let first_terminal_id = terminal_tab_mut(&mut workspace, first_tab_id)
            .and_then(|tab| tab.focused_terminal_id())
            .expect("first terminal");
        let second_terminal_id =
            terminal_tab_mut(&mut workspace, second_tab_id)
                .and_then(|tab| tab.focused_terminal_id())
                .expect("second terminal");

        term_event(
            &mut workspace,
            &config,
            otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            },
        );
        term_event(
            &mut workspace,
            &config,
            otty_ui_term::Event::BlockSelected {
                id: second_terminal_id,
                block_id: String::from("block-b"),
            },
        );

        tab_event(
            &mut workspace,
            &config,
            TabEvent::ActivateTab {
                tab_id: first_tab_id,
            },
        );
        tab_event(
            &mut workspace,
            &config,
            TabEvent::ActivateTab {
                tab_id: second_tab_id,
            },
        );

        let selection_a = terminal_tab(&workspace, first_tab_id)
            .and_then(|tab| tab.selected_block())
            .map(|selection| selection.block_id.clone())
            .expect("tab0 selection");
        let selection_b = terminal_tab(&workspace, second_tab_id)
            .and_then(|tab| tab.selected_block())
            .map(|selection| selection.block_id.clone())
            .expect("tab1 selection");

        assert_eq!(selection_a, "block-a");
        assert_eq!(selection_b, "block-b");
    }

    #[test]
    fn creating_new_tab_preserves_existing_tab_selection() {
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);
        let first_tab_id = workspace.tabs[0];
        let first_terminal_id = terminal_tab_mut(&mut workspace, first_tab_id)
            .and_then(|tab| tab.focused_terminal_id())
            .expect("first terminal");

        term_event(
            &mut workspace,
            &config,
            otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            },
        );

        new_tab(&mut workspace, &config);

        let first_selection = terminal_tab(&workspace, first_tab_id)
            .and_then(|tab| tab.selected_block())
            .map(|selection| selection.block_id.clone());
        let second_tab_id = workspace.tabs[1];
        let second_selection = terminal_tab(&workspace, second_tab_id)
            .and_then(|tab| tab.selected_block())
            .map(|selection| selection.block_id.clone());

        assert!(first_selection.is_some());
        assert!(second_selection.is_none());
    }

    #[test]
    fn selecting_block_in_new_pane_replaces_previous_selection() {
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);

        let tab_id = workspace.tabs[0];
        let first_terminal_id = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.focused_terminal_id())
            .expect("first terminal");
        let first_pane = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.focus())
            .expect("first pane");

        term_event(
            &mut workspace,
            &config,
            otty_ui_term::Event::BlockSelected {
                id: first_terminal_id,
                block_id: String::from("block-a"),
            },
        );

        tab_event(
            &mut workspace,
            &config,
            TabEvent::SplitPane {
                tab_id,
                pane: first_pane,
                axis: pane_grid::Axis::Horizontal,
            },
        );

        let second_terminal_id = terminal_tab_mut(&mut workspace, tab_id)
            .expect("tab")
            .terminals()
            .keys()
            .copied()
            .find(|id| *id != first_terminal_id)
            .expect("second terminal id");

        term_event(
            &mut workspace,
            &config,
            otty_ui_term::Event::BlockSelected {
                id: second_terminal_id,
                block_id: String::from("block-b"),
            },
        );

        let selection = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.selected_block())
            .expect("tab selection");
        assert_eq!(selection.terminal_id, second_terminal_id);
        assert_eq!(selection.block_id, "block-b");
    }

    #[test]
    fn focusing_other_pane_does_not_clear_selection() {
        let (mut workspace, config) = test_workspace(Size::new(800.0, 600.0));

        new_tab(&mut workspace, &config);

        let tab_id = workspace.tabs[0];
        let first_pane = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.focus())
            .expect("initial pane");

        tab_event(
            &mut workspace,
            &config,
            TabEvent::SplitPane {
                tab_id,
                pane: first_pane,
                axis: pane_grid::Axis::Horizontal,
            },
        );

        let focused_terminal_id = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.focused_terminal_id())
            .expect("focused terminal after split");

        term_event(
            &mut workspace,
            &config,
            otty_ui_term::Event::BlockSelected {
                id: focused_terminal_id,
                block_id: String::from("block-a"),
            },
        );

        let other_pane = terminal_tab_mut(&mut workspace, tab_id)
            .expect("tab")
            .terminals()
            .iter()
            .find(|(terminal_id, _)| **terminal_id != focused_terminal_id)
            .map(|(_, entry)| entry.pane)
            .expect("other pane");

        tab_event(
            &mut workspace,
            &config,
            TabEvent::PaneClicked {
                tab_id,
                pane: other_pane,
            },
        );

        let selection = terminal_tab_mut(&mut workspace, tab_id)
            .and_then(|tab| tab.selected_block());
        assert!(selection.is_some());
    }
}
