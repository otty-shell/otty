//! Delivers the intents of quick launch, with the pointer, the panel drag
//! and the main area size, and reads what it reports: the tabs to open or
//! close, the I/O to start, the keyboard focus and a failure to tell.

use iced::Task;
use iced::widget::operation;
use otty_app_notifications::NotificationsIntent;
use otty_app_quick_launch::{
    QuickLaunchCtx, QuickLaunchIntent, QuickLaunchOutcome,
};
use otty_app_quick_launch_io as quick_launch_io;
use otty_app_slots::Feature;
use otty_app_tabs::TabsIntent;
use otty_app_terminal::TerminalIntent;

use super::{notifications, tabs, terminal};
use crate::app::App;
use crate::message::AppEvent;

/// Give a quick launch intent to the feature, with what it reads from the
/// frame, and read what it reports.
pub(crate) fn deliver(
    app: &mut App,
    intent: QuickLaunchIntent,
) -> Task<AppEvent> {
    let ctx = QuickLaunchCtx {
        cursor: app.sidebar.cursor(),
        is_panel_resizing: app.sidebar.is_resizing(),
        screen_size: app.screen_size,
    };

    let outcomes = app.quick_launch.reduce(intent, ctx);

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

fn read(app: &mut App, outcome: QuickLaunchOutcome) -> Task<AppEvent> {
    match outcome {
        // The tabs open the editor tab and hand `init` back once it is
        // open (`TabsOutcome::WizardTabOpened`), and the editor fills then.
        QuickLaunchOutcome::OpenEditorTab { title, init } => {
            tabs::deliver(app, TabsIntent::OpenWizardTab { title, init })
        },
        // The check runs in the background with the settings new terminals
        // start with: the theme colors and the terminal font.
        QuickLaunchOutcome::PrepareLaunch(request) => {
            quick_launch_io::prepare(request, terminal::base_settings(app))
                .map(AppEvent::QuickLaunch)
        },
        // The tab opens on the machine quick launch picked; its terminal
        // starts the launch.
        QuickLaunchOutcome::LaunchReady {
            title,
            settings,
            exit_behavior,
            machine,
        } => tabs::deliver(
            app,
            TabsIntent::OpenCommandTab {
                title,
                settings,
                exit_behavior,
                machine,
            },
        ),
        QuickLaunchOutcome::CloseTab { tab_id } => {
            tabs::deliver(app, TabsIntent::CloseTab { tab_id })
        },
        QuickLaunchOutcome::SaveLaunches(data) => {
            quick_launch_io::save(data).map(AppEvent::QuickLaunch)
        },
        QuickLaunchOutcome::FocusInput(id) => operation::focus(id),
        QuickLaunchOutcome::TakeKeyboard => super::take_keyboard(),
        // The terminal decides: in Terminal mode the active pane gets its
        // keys back; in Containers mode nothing gets them.
        QuickLaunchOutcome::OverlayClosed => {
            terminal::deliver(app, TerminalIntent::FocusActive)
        },
        QuickLaunchOutcome::Failed(text) => {
            notifications::deliver(app, NotificationsIntent::Push(text))
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_app_domain::commands::AppCommand;
    use otty_app_domain::mode::Mode;
    use otty_app_domain::quick_launch::QuickLaunch;
    use otty_app_i18n as i18n;
    use otty_app_palette::PaletteIntent;
    use otty_app_quick_launch::{
        ContextMenuAction, QuickLaunchFile, QuickLaunchScreen,
        QuickLaunchSetupOutcome, SshSecretKind, SshSecretRequest,
    };
    use otty_app_shell::shell_session_options;
    use otty_app_slots::{MainArea, MainContent};
    use otty_app_tabs::TabContent;
    use otty_app_theme::theme::ThemeProps;
    use otty_ui_kit::components::command_palette::PaletteAction;
    use otty_ui_term::settings::{SessionKind, Settings};

    use super::*;
    use crate::dispatch::{dispatch, terminal};
    use crate::guards::{Overlay, top_overlay};
    use crate::message::Message;
    use crate::modes::ModesIntent;
    use crate::terminal_settings;
    use crate::testing::task_outputs::{
        focuses_nothing, runs_widget_operation,
    };
    use crate::testing::window::{TEST_SHELL_PATH, app, app_with_launches};

    /// "laptop", an SSH launch at the top of the tree that logs in with a
    /// password.
    fn launches() -> QuickLaunchFile {
        serde_json::from_value(serde_json::json!({
            "version": 1,
            "root": {"title": "Root", "expanded": true, "children": [{
                "kind": "Command", "title": "laptop",
                "spec": {"type": "Ssh", "ssh": {"host": "192.168.1.20",
                    "port": 22, "user": "ubuntu", "identity_file": null,
                    "extra_args": []}}
            }]}
        }))
        .expect("the launches should parse")
    }

    fn laptop() -> Vec<String> {
        vec![String::from("laptop")]
    }

    fn laptop_launch() -> Box<QuickLaunch> {
        let saved: serde_json::Value = serde_json::json!({
            "title": "laptop",
            "spec": {"type": "Ssh", "ssh": {"host": "192.168.1.20",
                "port": 22, "user": "ubuntu", "identity_file": null,
                "extra_args": []}}
        });

        Box::new(serde_json::from_value(saved).expect("the launch"))
    }

    /// An app in Containers mode with the launch "laptop", so a tab that
    /// comes to the front switches the mode.
    fn app_in_containers_mode() -> App {
        let mut app = app_with_launches(launches());
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());

        app
    }

    fn input(app: &mut App, intent: QuickLaunchIntent) -> Task<Message> {
        app.update(Message::Input(AppEvent::QuickLaunch(intent)))
    }

    /// An answer of the SSH check in the background, which skips the input
    /// checks.
    fn answer(
        app: &mut App,
        outcome: QuickLaunchSetupOutcome,
    ) -> Task<Message> {
        app.update(Message::Internal(AppEvent::QuickLaunch(
            QuickLaunchIntent::SetupCompleted(outcome),
        )))
    }

    fn password_needed(
        launch_id: u64,
        is_retry: bool,
    ) -> QuickLaunchSetupOutcome {
        QuickLaunchSetupOutcome::SecretRequired {
            path: laptop(),
            launch_id,
            command: laptop_launch(),
            request: SshSecretRequest::new(
                SshSecretKind::Password,
                String::from("ubuntu@192.168.1.20:22"),
                is_retry,
            ),
        }
    }

    /// The check found that the launch can run. Its settings start a local
    /// shell here, so the test connects to no server.
    fn prepared(app: &App, launch_id: u64) -> QuickLaunchSetupOutcome {
        let session = SessionKind::from_local_options(shell_session_options(
            TEST_SHELL_PATH,
        ));
        let settings: Settings = terminal_settings::for_session(
            &terminal::base_settings(app),
            session,
        );

        QuickLaunchSetupOutcome::Prepared {
            path: laptop(),
            launch_id,
            command: laptop_launch(),
            settings: Box::new(settings),
        }
    }

    fn send_password(app: &mut App, password: &str) {
        let _none = input(
            app,
            QuickLaunchIntent::SecretPromptChanged(String::from(password)),
        );
        let _check = input(app, QuickLaunchIntent::SecretPromptSubmit);
    }

    /// The SSH example of the spec: a click on an SSH launch that asks for
    /// a password. Four messages go through iced: the click, two answers of
    /// the SSH check and the password. The ready launch opens its tab with
    /// a terminal on the SSH host and switches to Terminal mode in the same
    /// update.
    #[test]
    fn an_ssh_launch_that_asks_for_a_password_opens_its_tab_after_it() {
        let mut app = app_in_containers_mode();

        // 1, 2: the click asks for the SSH check, run in the background.
        let _check =
            input(&mut app, QuickLaunchIntent::LaunchNode { path: laptop() });
        let is_checking = app.quick_launch.subscription().units() == 1;

        // 3: the check answers that a password is needed: the prompt opens
        // on top and takes the keyboard.
        let focus = answer(&mut app, password_needed(1, false));
        let prompt_on_top = top_overlay(&app);
        let prompt_focused = runs_widget_operation(focus);

        // 4: the password runs the check again.
        send_password(&mut app, "hunter2");
        let prompt_after_password = app.quick_launch.has_secret_prompt();

        // 5 to 8: the launch is ready: its tab opens with a terminal, on
        // the SSH host, and the window switches to Terminal mode.
        let ready = prepared(&app, 2);
        let _tab = answer(&mut app, ready);

        assert!(is_checking);
        assert_eq!(prompt_on_top, Some(Overlay::SecretPrompt));
        assert!(prompt_focused);
        assert!(!prompt_after_password);
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs.active_tab_title(), Some("laptop"));
        assert_eq!(app.tabs.active_machine_name(), "192.168.1.20");
        assert_eq!(app.terminal.pane_count(), Some(1));
        assert_eq!(app.modes.active(), Mode::Terminal);
        assert_eq!(app.quick_launch.subscription().units(), 0);
    }

    /// A wrong password asks again, with the retry noted, and the right one
    /// opens the tab.
    #[test]
    fn a_wrong_password_asks_again_and_the_right_one_opens_the_tab() {
        let mut app = app_in_containers_mode();
        let _check =
            input(&mut app, QuickLaunchIntent::LaunchNode { path: laptop() });
        let _focus = answer(&mut app, password_needed(1, false));
        send_password(&mut app, "wrong");

        let _focus = answer(&mut app, password_needed(2, true));
        let asked_again = app.quick_launch.has_secret_prompt();
        send_password(&mut app, "right");
        let ready = prepared(&app, 3);
        let _tab = answer(&mut app, ready);

        assert!(asked_again);
        assert!(!app.quick_launch.has_secret_prompt());
        assert_eq!(app.tabs.active_tab_title(), Some("laptop"));
        assert_eq!(app.modes.active(), Mode::Terminal);
    }

    /// A launch that can not run opens no tab; the user is told why in a
    /// toast that names the launch.
    #[test]
    fn a_launch_that_can_not_run_becomes_a_toast() {
        let mut app = app_in_containers_mode();
        let _check =
            input(&mut app, QuickLaunchIntent::LaunchNode { path: laptop() });

        let _toast = answer(
            &mut app,
            QuickLaunchSetupOutcome::Failed {
                path: laptop(),
                launch_id: 1,
                command: laptop_launch(),
                error: String::from("no route to host"),
            },
        );

        assert_eq!(app.tabs.len(), 0);
        assert_eq!(app.notifications.unseen_count(), 1);
        assert_eq!(app.modes.active(), Mode::Containers);
    }

    /// New launch opens the editor in a tab, filled for a new launch; Save
    /// adds the launch and closes the tab, and the tree waits to be saved.
    #[test]
    fn a_saved_launch_editor_adds_the_launch_and_closes_its_tab() {
        let mut app = app_with_launches(QuickLaunchFile::default());
        let _tab = dispatch(&mut app, AppEvent::Command(AppCommand::NewLaunch));
        let tab_id = app.tabs.active_tab_id().expect("the editor tab");
        let is_editor = app.tabs.active_tab_content()
            == Some(TabContent::QuickLaunchWizard);

        for intent in [
            QuickLaunchIntent::WizardUpdateTitle {
                tab_id,
                value: String::from("build"),
            },
            QuickLaunchIntent::WizardUpdateProgram {
                tab_id,
                value: String::from("make"),
            },
            QuickLaunchIntent::WizardSave { tab_id },
        ] {
            let _ = input(&mut app, intent);
        }

        assert!(is_editor);
        assert_eq!(app.tabs.len(), 0);
        let titles: Vec<&str> = app
            .quick_launch
            .launches()
            .iter()
            .map(|launch| launch.title())
            .collect();
        assert_eq!(titles, vec!["build"]);
        assert!(app.quick_launch.subscription().units() == 1);
    }

    /// Edit in the context menu of a launch opens its editor, filled with
    /// the launch as it is saved, in a tab named after it.
    #[test]
    fn edit_in_the_menu_opens_the_editor_of_the_launch() {
        let mut app = app_with_launches(launches());
        let _keyboard = input(
            &mut app,
            QuickLaunchIntent::NodeRightClicked { path: laptop() },
        );

        let _tab = input(
            &mut app,
            QuickLaunchIntent::ContextMenuAction(ContextMenuAction::Edit),
        );

        let theme = app.theme_manager.current().clone();
        let MainContent::Screen { id, content } =
            app.quick_launch.main_area(ThemeProps::new(&theme))
        else {
            panic!("the editor is a screen");
        };
        let mut ui = iced_test::simulator(content);
        assert_eq!(
            app.tabs.active_tab_title(),
            Some(i18n::edit_tab_title("laptop").as_str())
        );
        assert!(matches!(id, QuickLaunchScreen::Editor(_)));
        assert!(ui.find("192.168.1.20").is_ok());
    }

    /// A closed editor tab takes its editor with it; the tab left in front
    /// shows again.
    #[test]
    fn a_closed_editor_tab_forgets_its_editor() {
        let mut app = app();
        let _first =
            dispatch(&mut app, AppEvent::Command(AppCommand::NewLaunch));
        let first = app.tabs.active_tab_id().expect("the first editor");
        let _second =
            dispatch(&mut app, AppEvent::Command(AppCommand::NewLaunch));
        let second = app.tabs.active_tab_id().expect("the second editor");

        let _closed = dispatch(
            &mut app,
            AppEvent::Tabs(TabsIntent::CloseTab { tab_id: second }),
        );

        let theme = app.theme_manager.current().clone();
        let shown = match app.quick_launch.main_area(ThemeProps::new(&theme)) {
            MainContent::Screen { id, .. } => Some(id),
            MainContent::ScrollsOnItsOwn(_) => None,
        };
        assert_eq!(shown, Some(QuickLaunchScreen::Editor(first)));
    }

    /// The palette lists every launch and runs the picked one, as the New
    /// session menu does.
    #[test]
    fn a_launch_picked_in_the_palette_runs() {
        let mut app = app_with_launches(launches());
        let _focus = dispatch(&mut app, AppEvent::Palette(PaletteIntent::Open));

        let _query = dispatch(
            &mut app,
            AppEvent::Palette(PaletteIntent::Action(
                PaletteAction::QueryChanged(String::from("laptop")),
            )),
        );
        let _run = dispatch(
            &mut app,
            AppEvent::Palette(PaletteIntent::Action(PaletteAction::RunActive)),
        );

        assert!(!app.palette.is_open());
        assert!(app.quick_launch.subscription().units() == 1);
    }

    /// New folder shows the Launches section and starts naming the folder.
    #[test]
    fn new_folder_starts_the_name_in_the_shown_launches_section() {
        let mut app = app();
        let _closed = dispatch(
            &mut app,
            AppEvent::Sidebar(crate::sidebar::SidebarIntent::ToggleLaunches),
        );

        let task = dispatch(&mut app, AppEvent::Command(AppCommand::NewFolder));

        assert!(app.sidebar.vm().is_launches_open);
        assert!(app.quick_launch.has_inline_edit());
        assert!(runs_widget_operation(task));
    }

    /// The context menu takes the keyboard when it opens; when it closes,
    /// the terminal decides: in Terminal mode the active pane gets its keys
    /// back.
    #[test]
    fn the_context_menu_takes_the_keyboard_and_gives_it_back() {
        let mut app = app_with_launches(launches());
        let _tab =
            dispatch(&mut app, AppEvent::Tabs(TabsIntent::OpenTerminalTab));

        let opened = dispatch(
            &mut app,
            AppEvent::QuickLaunch(QuickLaunchIntent::BackgroundRightClicked),
        );
        let closed = dispatch(
            &mut app,
            AppEvent::QuickLaunch(QuickLaunchIntent::ContextMenuDismiss),
        );

        assert!(focuses_nothing(opened));
        assert!(runs_widget_operation(closed));
    }

    /// The first failed save tells the user once; the next ones try again
    /// quietly.
    #[test]
    fn a_failed_save_is_told_once_per_streak() {
        let mut app = app();

        for _ in 0..2 {
            let _ = dispatch(
                &mut app,
                AppEvent::QuickLaunch(QuickLaunchIntent::PersistFailed(
                    String::from("disk full"),
                )),
            );
        }

        assert_eq!(app.notifications.unseen_count(), 1);
    }
}
