use iced::Task;
use otty_app_containers::{ContainersIntent, ContainersOutcome};
use otty_app_containers_io as containers_io;
use otty_app_domain::exit_behavior::ExitBehavior;
use otty_app_domain::tab::TabMachine;
use otty_app_notifications::NotificationsIntent;
use otty_app_slots::Component;
use otty_app_tabs::TabsIntent;
use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

use super::{notifications, sidebar, tabs, terminal};
use crate::app::App;
use crate::frame::sidebar::SidebarIntent;
use crate::message::AppEvent;
use crate::terminal_settings;

/// Give a Containers mode intent to the containers feature and read what it
/// reports.
///
/// The active mode reaches the feature as
/// [`ContainersIntent::ModeChanged`], and whether the right panel is on
/// screen as [`ContainersIntent::RightPanelHidden`] after every update (see
/// `update.rs`).
pub(crate) fn deliver(
    app: &mut App,
    intent: ContainersIntent,
) -> Task<AppEvent> {
    let outcomes = app.containers.reduce(intent, ());

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

fn read(app: &mut App, outcome: ContainersOutcome) -> Task<AppEvent> {
    match outcome {
        ContainersOutcome::Discover => {
            containers_io::discover_engines().map(AppEvent::Containers)
        },
        ContainersOutcome::DiscoverInBackground => {
            containers_io::discover_in_background().map(AppEvent::Containers)
        },
        ContainersOutcome::LoadLists(info) => {
            containers_io::load_lists(info).map(AppEvent::Containers)
        },
        ContainersOutcome::LoadList {
            info,
            resource,
            containers,
        } => containers_io::load_list(info, resource, containers)
            .map(AppEvent::Containers),
        ContainersOutcome::CopyToClipboard(text) => {
            containers_io::copy_to_clipboard(text).map(AppEvent::Containers)
        },
        ContainersOutcome::OpenUrl(url) => {
            containers_io::open_url(url).map(AppEvent::Containers)
        },
        ContainersOutcome::RevealInspector => {
            sidebar::deliver(app, SidebarIntent::RevealRight)
        },
        ContainersOutcome::Failed(text) => {
            notifications::deliver(app, NotificationsIntent::Push(text))
        },
        ContainersOutcome::TakeKeyboard => super::take_keyboard(),
        ContainersOutcome::RunAction(request) => {
            containers_io::run_action(request).map(AppEvent::Containers)
        },
        ContainersOutcome::OpenShell {
            title,
            program,
            args,
        } => {
            let intent = shell_tab_intent(app, title, &program, args);
            tabs::deliver(app, intent)
        },
        ContainersOutcome::PullImage { engine, image, tag } => {
            containers_io::pull_image(engine, image, tag)
                .map(AppEvent::Containers)
        },
        ContainersOutcome::PullRunImage { engine, image, tag } => {
            containers_io::pull_run_image(engine, image, tag)
                .map(AppEvent::Containers)
        },
        ContainersOutcome::CancelPull(pull) => {
            containers_io::cancel_pull(&pull).map(AppEvent::Containers)
        },
        ContainersOutcome::LoadImageDefaults { engine, reference } => {
            containers_io::load_image_defaults(engine, reference)
                .map(AppEvent::Containers)
        },
        ContainersOutcome::RunContainer { engine, request } => {
            containers_io::run_container(engine, request)
                .map(AppEvent::Containers)
        },
        ContainersOutcome::CheckFolder(path) => {
            containers_io::check_folder(path).map(AppEvent::Containers)
        },
        ContainersOutcome::WaitForTypingPause(path) => {
            containers_io::wait_for_typing_pause(path).map(AppEvent::Containers)
        },
        ContainersOutcome::WaitForLateAnswer(path) => {
            containers_io::wait_for_late_answer(path).map(AppEvent::Containers)
        },
        ContainersOutcome::LoadInspect { engine, target } => {
            containers_io::load_inspect(engine, target)
                .map(AppEvent::Containers)
        },
        ContainersOutcome::CheckLogRun(target) => {
            containers_io::check_log_run(target).map(AppEvent::Containers)
        },
    }
}

/// The command tab on this computer that runs `program` with `args`, a shell
/// in a container on its own engine, the way a launch opens one. The tabs
/// show Terminal mode, as for every opened tab.
fn shell_tab_intent(
    app: &App,
    title: String,
    program: &str,
    args: Vec<String>,
) -> TabsIntent {
    let options = LocalSessionOptions::default()
        .with_program(program)
        .with_args(args);
    let settings = terminal_settings::for_session(
        &terminal::base_settings(app),
        SessionKind::from_local_options(options),
    );

    TabsIntent::OpenCommandTab {
        title,
        settings: Box::new(settings),
        exit_behavior: ExitBehavior::default(),
        machine: TabMachine::ThisComputer,
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_containers::LoadedLists;
    use otty_containers::{EngineKind, fixtures};

    use super::*;
    use crate::testing::window::app;

    /// An app with the right panel closed.
    fn app_with_closed_right_panel() -> App {
        let mut app = app();
        if app.sidebar.vm().is_right_open {
            let _ = sidebar::deliver(&mut app, SidebarIntent::ToggleRight);
        }
        assert!(!app.sidebar.vm().is_right_open);

        app
    }

    /// An app with the fixture lists of one engine loaded and the right panel
    /// closed.
    fn app_with_rows_and_closed_right_panel() -> App {
        let mut app = app_with_closed_right_panel();
        let resources = fixtures::resources(EngineKind::Docker);

        for intent in [
            ContainersIntent::Enter,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
            ContainersIntent::ListsLoaded {
                engine: fixtures::engine(EngineKind::Docker).address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        ] {
            let _ = app.containers.reduce(intent, ());
        }

        app
    }

    #[test]
    fn an_intent_is_reduced_by_the_feature_and_its_engine_work_starts() {
        let mut app = app();

        let task = deliver(&mut app, ContainersIntent::Enter);

        assert!(app.containers.is_active());
        assert_eq!(task.units(), 1);
    }

    #[test]
    fn pressing_a_row_opens_the_closed_right_panel() {
        let mut app = app_with_rows_and_closed_right_panel();

        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));

        assert!(app.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_a_row_with_the_right_panel_open_leaves_it_open() {
        let mut app = app_with_rows_and_closed_right_panel();
        let _ = sidebar::deliver(&mut app, SidebarIntent::ToggleRight);
        assert!(app.sidebar.vm().is_right_open);

        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));
        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));

        assert!(app.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_the_selected_row_again_opens_the_closed_right_panel() {
        let mut app = app_with_rows_and_closed_right_panel();
        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));
        let _ = sidebar::deliver(&mut app, SidebarIntent::ToggleRight);
        assert!(!app.sidebar.vm().is_right_open);

        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));

        assert!(app.sidebar.vm().is_right_open);
    }

    #[test]
    fn a_press_that_selects_no_row_leaves_the_right_panel_closed() {
        let mut no_rows = app_with_closed_right_panel();
        let mut past_the_last_row = app_with_rows_and_closed_right_panel();

        let _ = deliver(&mut no_rows, ContainersIntent::SelectRow(0));
        let _ =
            deliver(&mut past_the_last_row, ContainersIntent::SelectRow(9999));

        assert!(!no_rows.sidebar.vm().is_right_open);
        assert!(!past_the_last_row.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_a_row_leaves_focus_mode_with_the_right_panel_visible() {
        let mut app = app_with_rows_and_closed_right_panel();
        let _ = sidebar::deliver(&mut app, SidebarIntent::ToggleFocusMode);
        assert!(app.sidebar.vm().is_focus_mode);

        let _ = deliver(&mut app, ContainersIntent::SelectRow(0));

        let sidebar = app.sidebar.vm();
        assert!(!sidebar.is_focus_mode);
        assert!(sidebar.is_right_open);
        assert!(app.sidebar.right_ratio() > 0.0);
    }

    /// A port link opens a page and leaves the panel as it is. The browser
    /// is not started: only the reduce runs here.
    #[test]
    fn a_port_link_leaves_the_right_panel_closed() {
        let mut app = app_with_rows_and_closed_right_panel();
        let rows = fixtures::resources(EngineKind::Docker).containers().len();
        let mut opened = Vec::new();

        for row in 0..rows {
            opened.extend(
                app.containers.reduce(
                    ContainersIntent::OpenPortLink { row, link: 0 },
                    (),
                ),
            );
        }

        assert!(!app.sidebar.vm().is_right_open);
        assert!(
            opened.iter().any(|outcome| matches!(
                outcome,
                ContainersOutcome::OpenUrl(_)
            ))
        );
        assert!(!opened.contains(&ContainersOutcome::RevealInspector));
    }

    /// The row menu button and the right press open the row menu and leave
    /// the right panel as it is; the menu takes the keyboard.
    #[test]
    fn the_row_menu_leaves_the_right_panel_closed_and_takes_the_keyboard() {
        use crate::testing::task_outputs::focuses_nothing;

        let mut app = app_with_rows_and_closed_right_panel();

        for intent in [
            ContainersIntent::RowMenuPressed {
                index: 1,
                position: iced::Point::ORIGIN,
            },
            ContainersIntent::OpenRowMenu {
                index: 1,
                position: iced::Point::ORIGIN,
            },
        ] {
            let task = deliver(&mut app, intent);

            assert!(focuses_nothing(task));
            assert!(app.containers.has_row_menu());
            assert!(!app.sidebar.vm().is_right_open);
        }
    }

    /// Owner, Q1 of the old app: a failed action shows a toast over the
    /// table and goes to the list, in place of a notice above the table.
    #[test]
    fn a_failed_action_shows_a_toast_over_the_table() {
        use otty_app_containers::{
            ActionRequest, ActionsIntent, ContainersError, EngineAction,
        };

        let mut app = app_with_rows_and_closed_right_panel();
        let request = ActionRequest::new(
            fixtures::engine(EngineKind::Docker),
            EngineAction::StopContainer(String::from("4f1c2b9a8e7d")),
            String::from("web-1"),
        );

        let _ = deliver(
            &mut app,
            ContainersIntent::Actions(ActionsIntent::Finished {
                request,
                result: Err(ContainersError::new("busy")),
            }),
        );

        let text = otty_app_i18n::containers_action_failed(
            otty_app_i18n::Key::TplContainersStopFailed,
            "web-1",
            "busy",
        );
        assert_eq!(app.notifications.unseen_count(), 1);
        let mut ui = simulator(crate::view::view(&app));
        assert!(ui.find(text.as_str()).is_ok());
    }

    /// A shell in a container opens a command tab on this computer that
    /// runs the engine's tool, and Terminal mode shows it.
    #[test]
    fn the_shell_of_a_container_opens_one_command_tab() {
        use otty_app_domain::mode::Mode;
        use otty_ui_term::settings::SessionKind;

        const SHELL_ARGS: [&str; 7] = [
            "--host",
            "unix:///var/run/docker.sock",
            "exec",
            "-it",
            "--",
            "abc123",
            "sh",
        ];
        let mut app = app_with_rows_and_closed_right_panel();
        let args = || SHELL_ARGS.map(String::from).to_vec();

        let intent = shell_tab_intent(
            &app,
            String::from("shell web-1"),
            "/opt/homebrew/bin/docker",
            args(),
        );
        // The tab starts its program, so the one opened here is the test
        // shell, which needs no engine.
        let _ = read(
            &mut app,
            ContainersOutcome::OpenShell {
                title: String::from("shell web-1"),
                program: String::from(crate::testing::window::TEST_SHELL_PATH),
                args: Vec::new(),
            },
        );

        let TabsIntent::OpenCommandTab {
            title,
            settings,
            machine,
            ..
        } = intent
        else {
            panic!("expected one command tab");
        };
        assert_eq!(title, "shell web-1");
        assert_eq!(machine, TabMachine::ThisComputer);
        let SessionKind::Local(options) = &settings.backend.session else {
            panic!("expected a local session");
        };
        assert_eq!(options.program(), "/opt/homebrew/bin/docker");
        assert_eq!(options.args(), SHELL_ARGS);
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs.active_tab_title(), Some("shell web-1"));
        assert_eq!(app.modes.active(), Mode::Terminal);
    }

    /// A browser that can not be started is told in a toast, in the same
    /// update as its answer.
    #[test]
    fn a_link_that_can_not_be_opened_shows_a_toast() {
        let mut app = app();

        let _ = deliver(
            &mut app,
            ContainersIntent::LinkOpened(Err(String::from(
                "cannot run xdg-open: not found",
            ))),
        );

        let text = otty_app_i18n::with_error(
            otty_app_i18n::Key::TplLinkOpenFailed,
            "cannot run xdg-open: not found",
        );
        assert_eq!(app.notifications.unseen_count(), 1);
        let mut ui = simulator(crate::view::view(&app));
        assert!(ui.find(text.as_str()).is_ok());
    }
}
