//! Delivers the intents of the tabs, with the name of the user's shell for
//! new terminal tabs, and reads what they report.

use iced::Task;
use iced::widget::operation;
use otty_app_slots::Feature;
use otty_app_tabs::{SESSION_LIST_SCROLL_ID, TabsCtx, TabsIntent, TabsOutcome};

use super::modes;
use crate::app::App;
use crate::message::AppEvent;
use crate::modes::ModesIntent;

/// Give a tabs intent to the feature, with the name of the user's shell,
/// and read what it reports.
pub(crate) fn deliver(app: &mut App, intent: TabsIntent) -> Task<AppEvent> {
    let ctx = TabsCtx {
        shell_name: app.shell_session.name(),
    };

    let outcomes = app.tabs.reduce(intent, ctx);

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

fn read(app: &mut App, outcome: TabsOutcome) -> Task<AppEvent> {
    match outcome {
        // The modes widget decides whether the tab switches to Terminal
        // mode.
        TabsOutcome::Shown => modes::deliver(app, ModesIntent::TabShown),
        // The terminal feature (issue 11) is told here: it focuses the
        // active pane, syncs the block selection of the tab and sends the
        // folder of its terminal to the explorer (`SyncRoot`).
        TabsOutcome::Activated { tab_id: _ } => Task::none(),
        // The terminal feature (issue 11) is told here: it ends the
        // terminals of the tab and, with `new_active_id`, focuses the pane
        // of the tab active now and sends its folder to the explorer. The
        // quick launch feature (issue 12) forgets the launch editor or the
        // launch of the tab.
        TabsOutcome::Closed {
            tab_id: _,
            new_active_id: _,
        } => Task::none(),
        // The terminal feature (issue 11) starts the shell of the tab with
        // `app.shell_session` and the terminal settings, and sends
        // `ModesIntent::TabShown` once it started.
        TabsOutcome::TerminalTabOpened {
            tab_id: _,
            title: _,
        } => Task::none(),
        // The terminal feature (issue 11) starts the command of the tab with
        // its settings and exit behavior, and sends `ModesIntent::TabShown`
        // once it started.
        TabsOutcome::CommandTabOpened {
            tab_id: _,
            title: _,
            settings: _,
            exit_behavior: _,
        } => Task::none(),
        // The quick launch feature (issue 12) fills the launch editor of the
        // tab: a new launch or the saved one to edit.
        TabsOutcome::WizardTabOpened { tab_id: _, init: _ } => Task::none(),
        TabsOutcome::ScrollSessionsToEnd => {
            operation::snap_to_end(SESSION_LIST_SCROLL_ID)
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_app_domain::mode::Mode;
    use otty_app_domain::quick_launch::WizardTabInit;

    use super::*;
    use crate::testing::task_outputs::snaps_to_end;
    use crate::testing::window::app;

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
    /// feature (issue 11) tells the modes widget once the terminal starts,
    /// so a tab whose terminal can not start does not switch the mode.
    #[test]
    fn a_terminal_tab_from_containers_mode_waits_for_its_terminal() {
        let mut app = containers_app();

        let _ = deliver(&mut app, TabsIntent::OpenTerminalTab);

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.modes.active(), Mode::Containers);
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
        let mut app = containers_app();
        let _ = deliver(&mut app, TabsIntent::OpenTerminalTab);
        let tab_id = app.tabs.active_tab_id().expect("tab");

        let _ = deliver(&mut app, TabsIntent::CloseTab { tab_id });

        assert_eq!(app.tabs.len(), 0);
        assert_eq!(app.modes.active(), Mode::Containers);
    }
}
