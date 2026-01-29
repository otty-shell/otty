use iced::Task;
use otty_ui_term::settings::Settings;

use crate::app::Event as AppEvent;
use crate::effects::close_window;
use crate::features::quick_commands::editor::QuickCommandEditorState;
use crate::features::terminal::event as terminal;
use crate::features::terminal::shell::ShellSession;
use crate::features::terminal::term::TerminalState;
use crate::state::State;

/// Supported kinds of tabs in the workspace.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TabKind {
    Terminal,
    /// Settings screen tab.
    Settings,
}

/// High-level events routed to the tab reducer.
#[derive(Debug, Clone)]
pub(crate) enum TabEvent {
    NewTab { kind: TabKind },
    ActivateTab { tab_id: u64 },
    CloseTab { tab_id: u64 },
}

/// Tab payloads stored in app state.
pub(crate) enum TabContent {
    Terminal(Box<TerminalState>),
    /// Placeholder settings screen.
    Settings,
    /// Editor for quick command definitions.
    QuickCommandEditor(Box<QuickCommandEditorState>),
    /// Error tab shown when quick command launch fails.
    QuickCommandError(QuickCommandErrorState),
}

/// Snapshot of a failed quick command launch.
#[derive(Debug, Clone)]
pub(crate) struct QuickCommandErrorState {
    pub(crate) title: String,
    pub(crate) message: String,
}

/// Metadata for a single tab entry.
pub(crate) struct TabItem {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) content: TabContent,
}

pub(crate) fn tab_reducer(
    state: &mut State,
    terminal_settings: &Settings,
    shell_session: &ShellSession,
    event: TabEvent,
) -> Task<AppEvent> {
    match event {
        TabEvent::NewTab { kind } => match kind {
            TabKind::Terminal => terminal::create_terminal_tab(
                state,
                terminal_settings,
                shell_session,
            ),
            TabKind::Settings => create_settings_tab(state),
        },
        TabEvent::ActivateTab { tab_id } => activate_tab(state, tab_id),
        TabEvent::CloseTab { tab_id } => close_tab(state, tab_id),
    }
}

fn create_settings_tab(state: &mut State) -> Task<AppEvent> {
    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;

    state.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title: String::from("Settings"),
            content: TabContent::Settings,
        },
    );
    state.active_tab_id = Some(tab_id);

    Task::none()
}

fn close_tab(state: &mut State, tab_id: u64) -> Task<AppEvent> {
    if !state.tab_items.contains_key(&tab_id) {
        return Task::none();
    }

    if state.tab_items.len() == 1 {
        return close_window();
    }

    let next_active = if state.active_tab_id == Some(tab_id) {
        let prev = state
            .tab_items
            .range(..tab_id)
            .next_back()
            .map(|(&id, _)| id);

        let last = state.tab_items.keys().next_back().copied();

        prev.or(last)
    } else {
        state.active_tab_id
    };

    state.tab_items.remove(&tab_id);

    if state.tab_items.is_empty() {
        state.active_tab_id = None;
        return Task::none();
    }

    if state.active_tab_id == Some(tab_id) {
        state.active_tab_id = next_active;
    }

    let focus_task = terminal::focus_active_terminal(state);
    let sync_task = if let Some(active_id) = state.active_tab_id {
        terminal::sync_tab_block_selection(state, active_id)
    } else {
        Task::none()
    };

    Task::batch(vec![focus_task, sync_task])
}

fn activate_tab(state: &mut State, tab_id: u64) -> Task<AppEvent> {
    if !state.tab_items.contains_key(&tab_id) {
        return Task::none();
    }

    state.active_tab_id = Some(tab_id);
    let focus_task = terminal::focus_active_terminal(state);
    let sync_task = terminal::sync_tab_block_selection(state, tab_id);

    Task::batch(vec![focus_task, sync_task])
}
