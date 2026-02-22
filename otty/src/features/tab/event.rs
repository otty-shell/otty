use iced::Task;
use otty_ui_term::settings::Settings;

use crate::app::Event as AppEvent;
use crate::features::explorer::ExplorerEvent;
use crate::features::quick_launches::{
    NodePath, QuickLaunch, QuickLaunchEditorState, quick_launch_error_message,
};
use crate::features::settings::SettingsEvent;
use crate::features::terminal::{
    self, ShellSession, TerminalEvent, TerminalKind,
    terminal_settings_for_session,
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

/// Runtime dependencies used by tab reducer operations.
pub(crate) struct TabDeps<'a> {
    pub(crate) terminal_settings: &'a Settings,
    pub(crate) shell_session: &'a ShellSession,
}

pub(crate) fn tab_reducer(
    state: &mut State,
    deps: TabDeps<'_>,
    event: TabEvent,
) -> Task<AppEvent> {
    match event {
        TabEvent::NewTab { request } => match request {
            TabOpenRequest::Terminal => open_shell_terminal_tab(state, deps),
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
    let tab_id = state.allocate_tab_id();

    state.tab.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title: String::from("Settings"),
            content: TabContent::Settings,
        },
    );
    state.tab.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(AppEvent::Settings(SettingsEvent::Reload)),
        request_sync_explorer(),
    ])
}

fn open_quick_launch_editor_create_tab(
    state: &mut State,
    parent_path: NodePath,
) -> Task<AppEvent> {
    let tab_id = state.allocate_tab_id();

    let editor = QuickLaunchEditorState::new_create(parent_path);
    state.tab.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title: String::from("Create launch"),
            content: TabContent::QuickLaunchEditor(Box::new(editor)),
        },
    );
    state.tab.activate(Some(tab_id));

    Task::none()
}

fn open_quick_launch_editor_edit_tab(
    state: &mut State,
    path: NodePath,
    command: QuickLaunch,
) -> Task<AppEvent> {
    let tab_id = state.allocate_tab_id();

    let command_title = command.title.as_str();
    let title = format!("Edit {command_title}");
    let editor = QuickLaunchEditorState::from_command(path, &command);
    state.tab.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title,
            content: TabContent::QuickLaunchEditor(Box::new(editor)),
        },
    );
    state.tab.activate(Some(tab_id));

    Task::none()
}

fn open_shell_terminal_tab(
    state: &mut State,
    deps: TabDeps<'_>,
) -> Task<AppEvent> {
    let tab_id = state.allocate_tab_id();
    let terminal_id = state.allocate_terminal_id();

    let settings = terminal_settings_for_session(
        deps.terminal_settings,
        deps.shell_session.session().clone(),
    );
    terminal::terminal_reducer(
        state,
        TerminalEvent::InsertTab {
            tab_id,
            terminal_id,
            default_title: deps.shell_session.name().to_string(),
            settings: Box::new(settings),
            kind: TerminalKind::Shell,
            sync_explorer: true,
            error_tab: None,
        },
    )
}

fn close_tab(state: &mut State, tab_id: u64) -> Task<AppEvent> {
    if !state.tab.contains(tab_id) {
        return Task::none();
    }

    let next_active = if state.tab.active_tab_id() == Some(tab_id) {
        state
            .tab
            .previous_tab_id(tab_id)
            .or_else(|| state.tab.last_tab_id())
    } else {
        state.tab.active_tab_id()
    };

    state.tab.remove(tab_id);
    state.remove_tab_terminals(tab_id);

    if state.tab.is_empty() {
        state.tab.activate(None);
        return request_sync_explorer();
    }

    if state.tab.active_tab_id() == Some(tab_id) {
        state.tab.activate(next_active);
    }

    let focus_task =
        terminal::terminal_reducer(state, TerminalEvent::FocusActive);
    let sync_task = if let Some(active_id) = state.tab.active_tab_id() {
        terminal::terminal_reducer(
            state,
            TerminalEvent::SyncSelection { tab_id: active_id },
        )
    } else {
        Task::none()
    };

    Task::batch(vec![focus_task, sync_task, request_sync_explorer()])
}

fn activate_tab(state: &mut State, tab_id: u64) -> Task<AppEvent> {
    if !state.tab.contains(tab_id) {
        return Task::none();
    }

    state.tab.activate(Some(tab_id));
    let focus_task =
        terminal::terminal_reducer(state, TerminalEvent::FocusActive);
    let sync_task = terminal::terminal_reducer(
        state,
        TerminalEvent::SyncSelection { tab_id },
    );

    Task::batch(vec![focus_task, sync_task, request_sync_explorer()])
}

fn open_quick_launch_error_tab(
    state: &mut State,
    title: String,
    message: String,
) -> Task<AppEvent> {
    let tab_id = state.allocate_tab_id();

    state.tab.insert(
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
    state.tab.activate(Some(tab_id));

    Task::none()
}

fn open_quick_launch_command_terminal_tab(
    state: &mut State,
    tab_id: u64,
    terminal_id: u64,
    title: String,
    settings: Settings,
    command: Box<QuickLaunch>,
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

fn request_sync_explorer() -> Task<AppEvent> {
    Task::done(AppEvent::Explorer(ExplorerEvent::SyncFromActiveTerminal))
}

#[cfg(test)]
mod tests {
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::{TabDeps, TabEvent, tab_reducer};
    use crate::features::tab::TabOpenRequest;
    use crate::features::terminal::ShellSession;
    use crate::state::State;

    #[cfg(unix)]
    const SHELL_PROGRAM: &str = "/bin/sh";
    #[cfg(target_os = "windows")]
    const SHELL_PROGRAM: &str = "cmd.exe";

    fn deps() -> TabDeps<'static> {
        let settings = Box::leak(Box::new(Settings::default()));
        let session = Box::leak(Box::new(ShellSession::new(
            String::from("shell"),
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(SHELL_PROGRAM),
            ),
        )));

        TabDeps {
            terminal_settings: settings,
            shell_session: session,
        }
    }

    #[test]
    fn given_settings_tab_request_when_reduced_then_tab_becomes_active() {
        let mut state = State::default();

        let _task = tab_reducer(
            &mut state,
            deps(),
            TabEvent::NewTab {
                request: TabOpenRequest::Settings,
            },
        );

        assert_eq!(state.tab.len(), 1);
        assert_eq!(state.tab.active_tab_id(), Some(0));
    }

    #[test]
    fn given_missing_tab_when_activate_then_reducer_ignores_event() {
        let mut state = State::default();

        let _task = tab_reducer(
            &mut state,
            deps(),
            TabEvent::ActivateTab { tab_id: 7 },
        );

        assert!(state.tab.is_empty());
        assert_eq!(state.tab.active_tab_id(), None);
    }
}
