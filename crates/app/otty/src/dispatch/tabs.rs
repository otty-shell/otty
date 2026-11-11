//! Delivers the intents of the tabs, with the name of the user's shell for
//! new terminal tabs, and reads what they report.

use iced::Task;
use iced::widget::operation;
use otty_app_domain::quick_launch::WizardTabInit;
use otty_app_modes::ModesIntent;
use otty_app_quick_launch::QuickLaunchIntent;
use otty_app_tabs::{
    SESSION_LIST_SCROLL_ID, Tabs, TabsCtx, TabsIntent, TabsOutcome,
};
use otty_app_terminal::TerminalIntent;

use super::{Delivery, deliver, terminal};
use crate::app::App;
use crate::message::AppEvent;

/// The tabs get the name of the user's shell, and read what they report.
impl Delivery for TabsIntent {
    type Component = Tabs;

    fn parts(app: &mut App) -> (&mut Tabs, TabsCtx<'_>) {
        let ctx = TabsCtx {
            shell_name: app.shell_session.name(),
        };

        (&mut app.tabs, ctx)
    }

    fn read(app: &mut App, outcome: TabsOutcome) -> Task<AppEvent> {
        match outcome {
            // The modes feature decides whether the tab switches to Terminal
            // mode.
            TabsOutcome::Shown => deliver(app, ModesIntent::TabShown),
            // The terminal decides: the pane of the tab takes the keyboard, its
            // block selection is drawn again and the Files section follows its
            // folder. Quick launch shows the editor of the tab, if it has one.
            TabsOutcome::Activated { tab_id } => Task::batch([
                deliver(app, TerminalIntent::TabActivated { tab_id }),
                deliver(app, QuickLaunchIntent::TabActivated { tab_id }),
            ]),
            // The terminals of the tab end, and the terminal decides on the
            // focus and the folder of the tab in front now. Quick launch
            // forgets the launch editor of the tab.
            TabsOutcome::Closed {
                tab_id,
                new_active_id,
            } => Task::batch([
                deliver(
                    app,
                    TerminalIntent::TabClosed {
                        tab_id,
                        new_active_id,
                    },
                ),
                deliver(
                    app,
                    QuickLaunchIntent::TabClosed {
                        tab_id,
                        new_active_id,
                    },
                ),
            ]),
            // The shell starts; once it runs the terminal tells the modes
            // widget, which may switch to Terminal mode.
            TabsOutcome::TerminalTabOpened { tab_id, title } => {
                terminal::open_shell_tab(app, tab_id, title)
            },
            // The command starts with its own settings; once it runs the
            // terminal tells the modes feature.
            TabsOutcome::CommandTabOpened {
                tab_id,
                title,
                settings,
                exit_behavior,
            } => terminal::open_command_tab(
                app,
                tab_id,
                title,
                *settings,
                exit_behavior,
            ),
            // The launch editor is in front, so no terminal tab is: the
            // terminal knows that no pane of it takes the keys or is split.
            // Quick launch fills the editor of the tab: a new launch or the
            // saved one to edit.
            TabsOutcome::WizardTabOpened { tab_id, init } => Task::batch([
                deliver(app, TerminalIntent::TabActivated { tab_id }),
                deliver(app, editor_intent(tab_id, init)),
            ]),
            TabsOutcome::ScrollSessionsToEnd => {
                operation::snap_to_end(SESSION_LIST_SCROLL_ID)
            },
        }
    }
}

/// The quick launch intent that fills the editor of tab `tab_id` with what
/// it starts with.
fn editor_intent(tab_id: u64, init: WizardTabInit) -> QuickLaunchIntent {
    match init {
        WizardTabInit::Create { parent_path } => {
            QuickLaunchIntent::WizardInitializeCreate {
                tab_id,
                parent_path,
            }
        },
        WizardTabInit::Edit { path, command } => {
            QuickLaunchIntent::WizardInitializeEdit {
                tab_id,
                path,
                command,
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use otty_app_domain::exit_behavior::ExitBehavior;
    use otty_app_domain::mode::Mode;
    use otty_app_domain::quick_launch::WizardTabInit;
    use otty_app_domain::tab::TabMachine;
    use otty_app_slots::Component;
    use otty_libterm::surface::{BlockMeta, BlockSnapshot, SnapshotOwned};
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::*;
    use crate::dispatch;
    use crate::testing::task_outputs::{runs_widget_operation, snaps_to_end};
    use crate::testing::window::{TEST_SHELL_PATH, app, open_shell_tab};

    fn containers_app() -> App {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());

        app
    }

    fn open_launch_editor() -> TabsIntent {
        TabsIntent::OpenWizardTab {
            title: String::from("New launch"),
            init: WizardTabInit::Create {
                parent_path: vec![String::from("Demo")],
            },
        }
    }

    #[test]
    fn open_terminal_tab_is_named_after_the_app_shell() {
        let mut app = app();
        let shell_name = app.shell_session.name().to_string();

        let _ = deliver(&mut app, TabsIntent::OpenTerminalTab);

        assert_eq!(app.tabs.active_tab_title(), Some(shell_name.as_str()));
    }

    /// A new tab is added at the end of the Sessions list, which scrolls to
    /// show it.
    #[test]
    fn a_new_tab_scrolls_the_sessions_list_to_its_end() {
        let mut app = app();

        let task = deliver(&mut app, TabsIntent::OpenTerminalTab);

        assert!(snaps_to_end(task, &SESSION_LIST_SCROLL_ID.into()));
    }

    /// A new terminal tab leaves the mode to its terminal: the terminal
    /// tells the modes feature once its shell runs, which switches to
    /// Terminal mode in the same update, and the new pane shows.
    #[test]
    fn a_terminal_tab_whose_shell_starts_shows_terminal_mode() {
        let mut app = containers_app();

        let _ = deliver(&mut app, TabsIntent::OpenTerminalTab);

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.terminal.pane_count(), Some(1));
        assert_eq!(app.modes.active(), Mode::Terminal);
    }

    /// Owner: a launch whose terminal cannot start gives a toast and does
    /// not switch the mode. Its tab closes, so no tab is left.
    #[test]
    fn a_launch_whose_terminal_cannot_start_keeps_the_mode_and_tells_the_user()
    {
        let mut app = containers_app();

        let _ = deliver(
            &mut app,
            command_tab("/definitely/not/existing/otty-v3-missing-command"),
        );

        assert_eq!(app.modes.active(), Mode::Containers);
        assert_eq!(app.tabs.len(), 0);
        assert_eq!(app.notifications.unseen_count(), 1);
    }

    /// A launch whose terminal starts shows its tab, so the app switches to
    /// Terminal mode, as before.
    #[test]
    fn a_launch_whose_terminal_starts_switches_to_terminal_mode() {
        let mut app = containers_app();

        let _ = deliver(&mut app, command_tab(TEST_SHELL_PATH));

        assert_eq!(app.modes.active(), Mode::Terminal);
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.notifications.unseen_count(), 0);
    }

    #[test]
    fn activating_a_session_from_containers_mode_switches_to_terminal_mode() {
        let mut app = containers_app();
        let _ = deliver(&mut app, TabsIntent::OpenTerminalTab);
        let first_id = app.tabs.active_tab_id().expect("first tab");
        let _ = deliver(&mut app, TabsIntent::OpenTerminalTab);

        let _ = deliver(&mut app, TabsIntent::ActivateTab { tab_id: first_id });

        assert_eq!(app.tabs.active_tab_id(), Some(first_id));
        assert_eq!(app.modes.active(), Mode::Terminal);
    }

    #[test]
    fn opening_the_launch_editor_from_containers_mode_switches_to_terminal_mode()
     {
        let mut app = containers_app();

        let _ = deliver(&mut app, open_launch_editor());

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.modes.active(), Mode::Terminal);
    }

    #[test]
    fn closing_a_tab_from_containers_mode_does_not_switch_modes() {
        let mut app = app();
        let tab_id = open_shell_tab(&mut app);
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());

        let _ = deliver(&mut app, TabsIntent::CloseTab { tab_id });

        assert_eq!(app.tabs.len(), 0);
        assert_eq!(app.terminal.pane_count(), None);
        assert_eq!(app.modes.active(), Mode::Containers);
    }

    /// A command tab that runs `program` on this computer.
    fn command_tab(program: &str) -> TabsIntent {
        let mut settings = Settings::default();
        settings.backend = settings.backend.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(program),
            ),
        );

        TabsIntent::OpenCommandTab {
            title: String::from("deploy"),
            settings: Box::new(settings),
            exit_behavior: ExitBehavior::Close,
            machine: TabMachine::ThisComputer,
        }
    }

    /// Open a shell tab and make its terminal report `cwd`, the way the
    /// shell integration does after a prompt.
    fn open_shell_tab_in(app: &mut App, cwd: &str) -> u64 {
        let tab_id = open_shell_tab(app);
        let terminal_id = app
            .terminal
            .focused_terminal_id()
            .expect("the shell tab has a terminal");
        let mut frame = SnapshotOwned::default();
        frame.blocks.push(BlockSnapshot {
            meta: BlockMeta {
                cwd: Some(String::from(cwd)),
                ..BlockMeta::default()
            },
            ..BlockSnapshot::default()
        });

        let _ = dispatch::dispatch(
            app,
            AppEvent::Terminal(TerminalIntent::Widget(
                otty_ui_term::Event::ContentSync {
                    id: terminal_id,
                    frame: Arc::new(frame),
                },
            )),
        );

        tab_id
    }

    /// The folder the Files section follows now.
    fn followed_folder(app: &App) -> Option<PathBuf> {
        app.explorer.watched_folders().first().cloned()
    }

    #[test]
    fn given_two_shell_tabs_when_background_tab_activated_then_explorer_follows_its_folder()
     {
        let mut app = app();
        let first_id = open_shell_tab_in(&mut app, "/tmp/otty-first");
        let _second_id = open_shell_tab_in(&mut app, "/tmp/otty-second");

        let task =
            deliver(&mut app, TabsIntent::ActivateTab { tab_id: first_id });

        assert_eq!(
            followed_folder(&app),
            Some(PathBuf::from("/tmp/otty-first"))
        );
        assert!(runs_widget_operation(task));
    }

    #[test]
    fn given_local_command_tab_reporting_a_folder_when_activated_then_explorer_follows_it()
     {
        let mut app = app();
        let _ = deliver(&mut app, command_tab(TEST_SHELL_PATH));
        let launch_id = app.tabs.active_tab_id().expect("the launch tab");
        let terminal_id =
            app.terminal.focused_terminal_id().expect("its terminal");
        let _shell_id = open_shell_tab_in(&mut app, "/tmp/otty-shell");
        report_folder(&mut app, terminal_id, "/tmp/otty-launch");

        let _ =
            deliver(&mut app, TabsIntent::ActivateTab { tab_id: launch_id });

        assert_eq!(
            followed_folder(&app),
            Some(PathBuf::from("/tmp/otty-launch"))
        );
    }

    #[test]
    fn given_active_shell_tab_when_closed_then_explorer_follows_the_new_active_tab()
     {
        let mut app = app();
        let first_id = open_shell_tab_in(&mut app, "/tmp/otty-first");
        let second_id = open_shell_tab_in(&mut app, "/tmp/otty-second");

        let task =
            deliver(&mut app, TabsIntent::CloseTab { tab_id: second_id });

        assert_eq!(app.tabs.active_tab_id(), Some(first_id));
        assert_eq!(
            followed_folder(&app),
            Some(PathBuf::from("/tmp/otty-first"))
        );
        assert!(runs_widget_operation(task));
    }

    #[test]
    fn given_launch_editor_tab_when_activated_then_explorer_keeps_its_folder() {
        let mut app = app();
        let _shell_id = open_shell_tab_in(&mut app, "/tmp/otty-first");
        let _ = deliver(&mut app, open_launch_editor());
        let editor_id = app.tabs.active_tab_id().expect("the editor tab");

        let _ =
            deliver(&mut app, TabsIntent::ActivateTab { tab_id: editor_id });

        assert_eq!(
            followed_folder(&app),
            Some(PathBuf::from("/tmp/otty-first"))
        );
        assert_eq!(app.terminal.focused_pane(), None);
    }

    #[test]
    fn given_last_tab_when_closed_then_focus_is_not_requested() {
        let mut app = app();
        let tab_id = open_shell_tab_in(&mut app, "/tmp/otty-first");

        let task = deliver(&mut app, TabsIntent::CloseTab { tab_id });

        assert!(!runs_widget_operation(task));
        assert_eq!(app.terminal.focused_pane(), None);
    }

    /// The pane title follows the program the focused pane runs; the tab
    /// is named after it.
    #[test]
    fn a_terminal_names_its_tab() {
        let mut app = app();
        let tab_id = open_shell_tab(&mut app);

        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Terminal(TerminalIntent::TabActivated { tab_id }),
        );

        assert_eq!(app.tabs.active_tab_title(), Some("sh"));
    }

    /// Make terminal `terminal_id` report `cwd`.
    fn report_folder(app: &mut App, terminal_id: u64, cwd: &str) {
        let mut frame = SnapshotOwned::default();
        frame.blocks.push(BlockSnapshot {
            meta: BlockMeta {
                cwd: Some(String::from(cwd)),
                ..BlockMeta::default()
            },
            ..BlockSnapshot::default()
        });

        let _ = dispatch::dispatch(
            app,
            AppEvent::Terminal(TerminalIntent::Widget(
                otty_ui_term::Event::ContentSync {
                    id: terminal_id,
                    frame: Arc::new(frame),
                },
            )),
        );
    }
}
