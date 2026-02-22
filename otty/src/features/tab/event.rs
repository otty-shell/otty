use iced::Task;
use otty_ui_term::settings::Settings;

use crate::app::Event as AppEvent;
use crate::features::explorer;
use crate::features::quick_launches::editor::{
    open_create_editor_tab, open_edit_editor_tab,
};
use crate::features::quick_launches::model::quick_launch_error_message;
use crate::features::settings;
use crate::features::terminal::event as terminal;
use crate::features::terminal::model::{ShellSession, TerminalKind};
use crate::features::terminal::state::TerminalState;
use crate::state::State;

use super::model::{TabContent, TabItem, TabOpenRequest};

/// High-level events routed to the tab reducer.
#[derive(Debug, Clone)]
pub(crate) enum TabEvent {
    NewTab { request: TabOpenRequest },
    ActivateTab { tab_id: u64 },
    CloseTab { tab_id: u64 },
}

pub(crate) fn tab_reducer(
    state: &mut State,
    terminal_settings: &Settings,
    shell_session: &ShellSession,
    event: TabEvent,
) -> Task<AppEvent> {
    match event {
        TabEvent::NewTab { request } => match request {
            TabOpenRequest::Terminal => {
                open_shell_terminal_tab(state, terminal_settings, shell_session)
            },
            TabOpenRequest::Settings => open_settings_tab(state),
            TabOpenRequest::QuickLaunchEditorCreate { parent_path } => {
                open_create_editor_tab(state, parent_path)
            },
            TabOpenRequest::QuickLaunchEditorEdit { path, command } => {
                open_edit_editor_tab(state, path, &command)
            },
            TabOpenRequest::QuickLaunchError { title, message } => {
                open_quick_launch_error_tab(state, title, message)
            },
            TabOpenRequest::CommandTerminal {
                tab_id,
                terminal_id,
                title,
                settings,
            } => open_command_terminal_tab(
                state,
                tab_id,
                terminal_id,
                title,
                *settings,
            ),
            TabOpenRequest::QuickLaunchCommandTerminal {
                tab_id,
                terminal_id,
                title,
                settings,
                command,
            } => open_quick_launch_command_terminal_tab(
                state,
                tab_id,
                terminal_id,
                title,
                *settings,
                command,
            ),
        },
        TabEvent::ActivateTab { tab_id } => activate_tab(state, tab_id),
        TabEvent::CloseTab { tab_id } => close_tab(state, tab_id),
    }
}

fn open_settings_tab(state: &mut State) -> Task<AppEvent> {
    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;

    let reload_task =
        settings::settings_reducer(state, settings::SettingsEvent::Reload);
    state.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title: String::from("Settings"),
            content: TabContent::Settings,
        },
    );
    state.active_tab_id = Some(tab_id);
    explorer::event::sync_explorer_from_active_terminal(state);

    reload_task
}

fn open_shell_terminal_tab(
    state: &mut State,
    terminal_settings: &Settings,
    shell_session: &ShellSession,
) -> Task<AppEvent> {
    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;

    let terminal_id = state.next_terminal_id;
    state.next_terminal_id += 1;

    let settings = terminal::settings_for_session(
        terminal_settings,
        shell_session.session.clone(),
    );
    let (terminal, focus_task) = match TerminalState::new(
        tab_id,
        shell_session.name.clone(),
        terminal_id,
        settings,
        TerminalKind::Shell,
    ) {
        Ok(result) => result,
        Err(err) => {
            log::warn!("failed to create terminal tab: {err}");
            return Task::none();
        },
    };

    terminal::insert_terminal_tab(state, tab_id, terminal, focus_task, true)
}

fn close_tab(state: &mut State, tab_id: u64) -> Task<AppEvent> {
    if !state.tab_items.contains_key(&tab_id) {
        return Task::none();
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
    state.remove_tab_terminals(tab_id);

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
    explorer::event::sync_explorer_from_active_terminal(state);

    Task::batch(vec![focus_task, sync_task])
}

fn activate_tab(state: &mut State, tab_id: u64) -> Task<AppEvent> {
    if !state.tab_items.contains_key(&tab_id) {
        return Task::none();
    }

    state.active_tab_id = Some(tab_id);
    let focus_task = terminal::focus_active_terminal(state);
    let sync_task = terminal::sync_tab_block_selection(state, tab_id);
    explorer::event::sync_explorer_from_active_terminal(state);

    Task::batch(vec![focus_task, sync_task])
}

fn open_quick_launch_error_tab(
    state: &mut State,
    title: String,
    message: String,
) -> Task<AppEvent> {
    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;

    state.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title: title.clone(),
            content: TabContent::QuickLaunchError(
                crate::features::quick_launches::QuickLaunchErrorState {
                    title,
                    message,
                },
            ),
        },
    );
    state.active_tab_id = Some(tab_id);

    Task::none()
}

fn open_quick_launch_command_terminal_tab(
    state: &mut State,
    tab_id: u64,
    terminal_id: u64,
    title: String,
    settings: Settings,
    command: Box<crate::features::quick_launches::model::QuickLaunch>,
) -> Task<AppEvent> {
    let (terminal, focus_task) = match TerminalState::new(
        tab_id,
        title,
        terminal_id,
        settings,
        TerminalKind::Command,
    ) {
        Ok(result) => result,
        Err(err) => {
            let command_title = &command.title;
            return open_quick_launch_error_tab(
                state,
                format!("Failed to launch \"{command_title}\""),
                quick_launch_error_message(&command, &err),
            );
        },
    };

    let insert_task = terminal::insert_terminal_tab(
        state, tab_id, terminal, focus_task, false,
    );
    let focus_active_task = terminal::focus_active_terminal(state);
    Task::batch(vec![insert_task, focus_active_task])
}

fn open_command_terminal_tab(
    state: &mut State,
    tab_id: u64,
    terminal_id: u64,
    title: String,
    settings: Settings,
) -> Task<AppEvent> {
    let (terminal, focus_task) = match TerminalState::new(
        tab_id,
        title,
        terminal_id,
        settings,
        TerminalKind::Command,
    ) {
        Ok(result) => result,
        Err(err) => {
            log::warn!("command terminal tab init failed: {err}");
            return Task::none();
        },
    };

    terminal::insert_terminal_tab(state, tab_id, terminal, focus_task, false)
}
