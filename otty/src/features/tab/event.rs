use iced::Task;
use otty_ui_term::settings::Settings;

use super::model::{TabContent, TabItem, TabOpenRequest};
use crate::app::Event as AppEvent;
use crate::features::explorer::ExplorerEvent;
use crate::features::quick_launch::{
    NodePath, QuickLaunch, QuickLaunchEvent, quick_launch_error_message,
};
use crate::features::quick_launch_wizard::QuickLaunchWizardEvent;
use crate::features::settings::SettingsEvent;
use crate::features::terminal::{
    ShellSession, TerminalEvent, TerminalKind, terminal_settings_for_session,
};
use crate::state::State;

/// High-level events routed to the tab reducer.
#[derive(Debug, Clone)]
pub(crate) enum TabEvent {
    NewTab { request: TabOpenRequest },
    ActivateTab { tab_id: u64 },
    CloseTab { tab_id: u64 },
    SetTitle { tab_id: u64, title: String },
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
            TabOpenRequest::QuickLaunchWizardCreate { parent_path } => {
                open_quick_launch_wizard_create_tab(state, parent_path)
            },
            TabOpenRequest::QuickLaunchWizardEdit { path, command } => {
                open_quick_launch_wizard_edit_tab(state, path, *command)
            },
            TabOpenRequest::QuickLaunchError { title, message } => {
                open_quick_launch_error_tab(state, title, message)
            },
            TabOpenRequest::CommandTerminal { title, settings } => {
                open_command_terminal_tab(state, title, *settings)
            },
            TabOpenRequest::QuickLaunchCommandTerminal {
                title,
                settings,
                command,
            } => open_quick_launch_command_terminal_tab(
                state, title, *settings, command,
            ),
        },
        TabEvent::ActivateTab { tab_id } => activate_tab(state, tab_id),
        TabEvent::CloseTab { tab_id } => close_tab(state, tab_id),
        TabEvent::SetTitle { tab_id, title } => {
            state.tab.set_title(tab_id, title);
            Task::none()
        },
    }
}

fn open_settings_tab(state: &mut State) -> Task<AppEvent> {
    let tab_id = state.allocate_tab_id();

    state.tab.insert(
        tab_id,
        TabItem::new(tab_id, String::from("Settings"), TabContent::Settings),
    );
    state.tab.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(AppEvent::Settings(SettingsEvent::Reload)),
        request_sync_explorer(),
    ])
}

fn open_quick_launch_wizard_create_tab(
    state: &mut State,
    parent_path: NodePath,
) -> Task<AppEvent> {
    let tab_id = state.allocate_tab_id();

    state.tab.insert(
        tab_id,
        TabItem::new(
            tab_id,
            String::from("Create launch"),
            TabContent::QuickLaunchWizard,
        ),
    );
    state.tab.activate(Some(tab_id));

    Task::done(AppEvent::QuickLaunchWizard {
        tab_id,
        event: QuickLaunchWizardEvent::InitializeCreate { parent_path },
    })
}

fn open_quick_launch_wizard_edit_tab(
    state: &mut State,
    path: NodePath,
    command: QuickLaunch,
) -> Task<AppEvent> {
    let tab_id = state.allocate_tab_id();

    let command_title = command.title.as_str();
    let title = format!("Edit {command_title}");
    state.tab.insert(
        tab_id,
        TabItem::new(tab_id, title, TabContent::QuickLaunchWizard),
    );
    state.tab.activate(Some(tab_id));

    Task::done(AppEvent::QuickLaunchWizard {
        tab_id,
        event: QuickLaunchWizardEvent::InitializeEdit {
            path,
            command: Box::new(command),
        },
    })
}

fn open_shell_terminal_tab(
    state: &mut State,
    deps: TabDeps<'_>,
) -> Task<AppEvent> {
    let tab_id = state.allocate_tab_id();
    let terminal_id = state.allocate_terminal_id();

    state.tab.insert(
        tab_id,
        TabItem::new(
            tab_id,
            deps.shell_session.name().to_string(),
            TabContent::Terminal,
        ),
    );
    state.tab.activate(Some(tab_id));

    let settings = terminal_settings_for_session(
        deps.terminal_settings,
        deps.shell_session.session().clone(),
    );

    Task::done(AppEvent::Terminal(TerminalEvent::OpenTab {
        tab_id,
        terminal_id,
        default_title: deps.shell_session.name().to_string(),
        settings: Box::new(settings),
        kind: TerminalKind::Shell,
        sync_explorer: true,
        error_tab: None,
    }))
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

    if state.tab.is_empty() {
        state.tab.activate(None);
    } else if state.tab.active_tab_id() == Some(tab_id) {
        state.tab.activate(next_active);
    }

    let mut tasks = vec![
        request_terminal_event(TerminalEvent::TabClosed { tab_id }),
        Task::done(AppEvent::QuickLaunchWizard {
            tab_id,
            event: QuickLaunchWizardEvent::TabClosed,
        }),
        Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::TabClosed {
            tab_id,
        })),
    ];

    if !state.tab.is_empty() {
        tasks.push(request_terminal_event(TerminalEvent::FocusActive));
        if let Some(active_id) = state.tab.active_tab_id() {
            tasks.push(request_terminal_event(TerminalEvent::SyncSelection {
                tab_id: active_id,
            }));
        }
    }

    tasks.push(request_sync_explorer());

    Task::batch(tasks)
}

fn activate_tab(state: &mut State, tab_id: u64) -> Task<AppEvent> {
    if !state.tab.contains(tab_id) {
        return Task::none();
    }

    state.tab.activate(Some(tab_id));
    let focus_task = request_terminal_event(TerminalEvent::FocusActive);
    let sync_task =
        request_terminal_event(TerminalEvent::SyncSelection { tab_id });

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
        TabItem::new(tab_id, title.clone(), TabContent::QuickLaunchError),
    );
    state.tab.activate(Some(tab_id));

    Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::OpenErrorTab {
        tab_id,
        title,
        message,
    }))
}

fn open_quick_launch_command_terminal_tab(
    state: &mut State,
    title: String,
    settings: Settings,
    command: Box<QuickLaunch>,
) -> Task<AppEvent> {
    let tab_id = state.allocate_tab_id();
    let terminal_id = state.allocate_terminal_id();
    let command_title = &command.title;
    let init_error_message = quick_launch_error_message(
        &command,
        &"terminal tab initialization failed",
    );

    state.tab.insert(
        tab_id,
        TabItem::new(tab_id, title.clone(), TabContent::Terminal),
    );
    state.tab.activate(Some(tab_id));

    let open_task = Task::done(AppEvent::Terminal(TerminalEvent::OpenTab {
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
    }));
    let focus_active_task = request_terminal_event(TerminalEvent::FocusActive);
    Task::batch(vec![open_task, focus_active_task])
}

fn open_command_terminal_tab(
    state: &mut State,
    title: String,
    settings: Settings,
) -> Task<AppEvent> {
    let tab_id = state.allocate_tab_id();
    let terminal_id = state.allocate_terminal_id();

    state.tab.insert(
        tab_id,
        TabItem::new(tab_id, title.clone(), TabContent::Terminal),
    );
    state.tab.activate(Some(tab_id));

    Task::done(AppEvent::Terminal(TerminalEvent::OpenTab {
        tab_id,
        terminal_id,
        default_title: title,
        settings: Box::new(settings),
        kind: TerminalKind::Command,
        sync_explorer: false,
        error_tab: None,
    }))
}

fn request_sync_explorer() -> Task<AppEvent> {
    Task::done(AppEvent::Explorer(ExplorerEvent::SyncFromActiveTerminal))
}

fn request_terminal_event(event: TerminalEvent) -> Task<AppEvent> {
    Task::done(AppEvent::Terminal(event))
}

#[cfg(test)]
mod tests {
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::{TabDeps, TabEvent, tab_reducer};
    use crate::features::quick_launch::{
        QuickLaunchEvent, QuickLaunchesDeps, quick_launches_reducer,
    };
    use crate::features::quick_launch_wizard::{
        QuickLaunchWizardDeps, QuickLaunchWizardEvent,
        quick_launch_wizard_reducer,
    };
    use crate::features::tab::TabOpenRequest;
    use crate::features::terminal::{
        ShellSession, TerminalEvent, TerminalKind, terminal_reducer,
    };
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

    #[test]
    fn given_two_tabs_when_active_tab_closed_then_previous_tab_becomes_active()
    {
        let mut state = State::default();

        let _ = tab_reducer(
            &mut state,
            deps(),
            TabEvent::NewTab {
                request: TabOpenRequest::Settings,
            },
        );
        let _ = tab_reducer(
            &mut state,
            deps(),
            TabEvent::NewTab {
                request: TabOpenRequest::Settings,
            },
        );
        assert_eq!(state.tab.active_tab_id(), Some(1));

        let _ =
            tab_reducer(&mut state, deps(), TabEvent::CloseTab { tab_id: 1 });

        assert_eq!(state.tab.len(), 1);
        assert_eq!(state.tab.active_tab_id(), Some(0));
    }

    #[test]
    fn given_last_tab_when_closed_then_state_has_no_active_tab() {
        let mut state = State::default();

        let _ = tab_reducer(
            &mut state,
            deps(),
            TabEvent::NewTab {
                request: TabOpenRequest::Settings,
            },
        );
        assert_eq!(state.tab.active_tab_id(), Some(0));

        let _ =
            tab_reducer(&mut state, deps(), TabEvent::CloseTab { tab_id: 0 });

        assert!(state.tab.is_empty());
        assert_eq!(state.tab.active_tab_id(), None);
    }

    #[test]
    fn given_wizard_tab_lifecycle_when_close_then_wizard_owner_state_is_cleaned()
     {
        let mut state = State::default();
        let _ = tab_reducer(
            &mut state,
            deps(),
            TabEvent::NewTab {
                request: TabOpenRequest::QuickLaunchWizardCreate {
                    parent_path: vec![String::from("Root")],
                },
            },
        );
        assert!(state.tab.contains(0));

        let _ = quick_launch_wizard_reducer(
            &mut state,
            QuickLaunchWizardDeps { tab_id: 0 },
            QuickLaunchWizardEvent::InitializeCreate {
                parent_path: vec![String::from("Root")],
            },
        );
        assert!(state.quick_launch_wizard.editor(0).is_some());

        let _ =
            tab_reducer(&mut state, deps(), TabEvent::CloseTab { tab_id: 0 });
        let _ = quick_launch_wizard_reducer(
            &mut state,
            QuickLaunchWizardDeps { tab_id: 0 },
            QuickLaunchWizardEvent::TabClosed,
        );

        assert!(!state.tab.contains(0));
        assert!(state.quick_launch_wizard.editor(0).is_none());
    }

    #[test]
    fn given_quick_launch_error_tab_lifecycle_when_close_then_error_payload_is_cleaned()
     {
        let mut state = State::default();
        let _ = tab_reducer(
            &mut state,
            deps(),
            TabEvent::NewTab {
                request: TabOpenRequest::QuickLaunchError {
                    title: String::from("Failed"),
                    message: String::from("Boom"),
                },
            },
        );
        assert!(state.tab.contains(0));

        let _ = quick_launches_reducer(
            &mut state,
            QuickLaunchesDeps {
                terminal_settings: &Settings::default(),
            },
            QuickLaunchEvent::OpenErrorTab {
                tab_id: 0,
                title: String::from("Failed"),
                message: String::from("Boom"),
            },
        );
        assert!(state.quick_launches.error_tab(0).is_some());

        let _ =
            tab_reducer(&mut state, deps(), TabEvent::CloseTab { tab_id: 0 });
        let _ = quick_launches_reducer(
            &mut state,
            QuickLaunchesDeps {
                terminal_settings: &Settings::default(),
            },
            QuickLaunchEvent::TabClosed { tab_id: 0 },
        );

        assert!(!state.tab.contains(0));
        assert!(state.quick_launches.error_tab(0).is_none());
    }

    #[test]
    fn given_terminal_tab_lifecycle_when_close_then_terminal_owner_state_is_cleaned()
     {
        let mut state = State::default();
        let _ = tab_reducer(
            &mut state,
            deps(),
            TabEvent::NewTab {
                request: TabOpenRequest::Terminal,
            },
        );
        assert!(state.tab.contains(0));

        let _ = terminal_reducer(
            &mut state,
            TerminalEvent::OpenTab {
                tab_id: 0,
                terminal_id: 0,
                default_title: String::from("shell"),
                settings: Box::new(Settings::default()),
                kind: TerminalKind::Shell,
                sync_explorer: false,
                error_tab: None,
            },
        );
        assert!(state.terminal.tab(0).is_some());

        let _ =
            tab_reducer(&mut state, deps(), TabEvent::CloseTab { tab_id: 0 });
        let _ = terminal_reducer(
            &mut state,
            TerminalEvent::TabClosed { tab_id: 0 },
        );

        assert!(!state.tab.contains(0));
        assert!(state.terminal.tab(0).is_none());
        assert_eq!(state.terminal_tab_id(0), None);
    }
}
