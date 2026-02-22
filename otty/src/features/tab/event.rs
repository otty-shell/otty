use iced::Task;
use otty_ui_term::settings::Settings;

use crate::app::Event as AppEvent;
use crate::features::explorer;
use crate::features::quick_launches::editor::QuickLaunchEditorState;
use crate::features::quick_launches::model::quick_launch_error_message;
use crate::features::settings;
use crate::features::terminal::{
    self, ShellSession, TerminalEvent, TerminalKind,
};
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
                open_quick_launch_editor_create_tab(state, parent_path)
            },
            TabOpenRequest::QuickLaunchEditorEdit { path, command } => {
                open_quick_launch_editor_edit_tab(state, path, *command)
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

fn open_quick_launch_editor_create_tab(
    state: &mut State,
    parent_path: crate::features::quick_launches::model::NodePath,
) -> Task<AppEvent> {
    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;

    let editor = QuickLaunchEditorState::new_create(parent_path);
    state.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title: String::from("Create launch"),
            content: TabContent::QuickLaunchEditor(Box::new(editor)),
        },
    );
    state.active_tab_id = Some(tab_id);

    Task::none()
}

fn open_quick_launch_editor_edit_tab(
    state: &mut State,
    path: crate::features::quick_launches::model::NodePath,
    command: crate::features::quick_launches::model::QuickLaunch,
) -> Task<AppEvent> {
    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;

    let command_title = command.title.as_str();
    let title = format!("Edit {command_title}");
    let editor = QuickLaunchEditorState::from_command(path, &command);
    state.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title,
            content: TabContent::QuickLaunchEditor(Box::new(editor)),
        },
    );
    state.active_tab_id = Some(tab_id);

    Task::none()
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

    let settings = settings_for_session(
        terminal_settings,
        shell_session.session().clone(),
    );
    terminal::terminal_reducer(
        state,
        TerminalEvent::InsertTab {
            tab_id,
            terminal_id,
            default_title: shell_session.name().to_string(),
            settings: Box::new(settings),
            kind: TerminalKind::Shell,
            sync_explorer: true,
            error_tab: None,
        },
    )
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

    let focus_task =
        terminal::terminal_reducer(state, TerminalEvent::FocusActive);
    let sync_task = if let Some(active_id) = state.active_tab_id {
        terminal::terminal_reducer(
            state,
            TerminalEvent::SyncSelection { tab_id: active_id },
        )
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
    let focus_task =
        terminal::terminal_reducer(state, TerminalEvent::FocusActive);
    let sync_task = terminal::terminal_reducer(
        state,
        TerminalEvent::SyncSelection { tab_id },
    );
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
    let command_title = &command.title;
    let init_error_message = quick_launch_error_message(
        &command,
        &"terminal tab initialization failed",
    );

    let insert_task = terminal::terminal_reducer(
        state,
        TerminalEvent::InsertTab {
            tab_id,
            terminal_id,
            default_title: title,
            settings: Box::new(settings),
            kind: TerminalKind::Command,
            sync_explorer: false,
            error_tab: Some((
                format!("Failed to launch \"{command_title}\""),
                init_error_message,
            )),
        },
    );
    let focus_active_task =
        terminal::terminal_reducer(state, TerminalEvent::FocusActive);
    Task::batch(vec![insert_task, focus_active_task])
}

fn open_command_terminal_tab(
    state: &mut State,
    tab_id: u64,
    terminal_id: u64,
    title: String,
    settings: Settings,
) -> Task<AppEvent> {
    terminal::terminal_reducer(
        state,
        TerminalEvent::InsertTab {
            tab_id,
            terminal_id,
            default_title: title,
            settings: Box::new(settings),
            kind: TerminalKind::Command,
            sync_explorer: false,
            error_tab: None,
        },
    )
}

fn settings_for_session(
    base_settings: &Settings,
    session: otty_ui_term::settings::SessionKind,
) -> Settings {
    let mut settings = base_settings.clone();
    settings.backend = settings.backend.clone().with_session(session);
    settings
}
