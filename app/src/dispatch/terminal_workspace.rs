use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::guards::{self, Overlay};
use crate::layout::pane_grid_size;
use crate::widgets::terminal_workspace::TerminalWorkspaceCtx;
use crate::widgets::terminal_workspace::event::TerminalWorkspaceIntent;

/// Deliver a terminal workspace intent to the widget's reducer, with the
/// active tab, the room the panes have, the sidebar pointer, the saved
/// Equalize panes setting and the overlays that are open.
pub(crate) fn deliver(
    app: &mut App,
    intent: TerminalWorkspaceIntent,
) -> Task<AppEvent> {
    let top_overlay = guards::top_overlay(app);
    let ctx = TerminalWorkspaceCtx {
        active_tab_id: app.widgets.tabs.active_tab_id(),
        pane_grid_size: pane_grid_size(
            app.state.screen_size,
            app.widgets.sidebar.side_panels_ratio(),
        ),
        screen_size: app.state.screen_size,
        sidebar_cursor: app.widgets.sidebar.cursor(),
        equalize_panes: app.widgets.settings.saved().equalize_panes(),
        is_overlay_open: top_overlay.is_some(),
        is_other_overlay_open: top_overlay
            .is_some_and(|overlay| overlay != Overlay::TerminalContextMenu),
    };

    app.widgets
        .terminal_workspace
        .reduce(intent, &ctx)
        .map(AppEvent::TerminalWorkspace)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use iced::Size;
    use iced::widget::pane_grid;
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::*;
    use crate::dispatch::dispatch;
    use crate::domain::exit_behavior::ExitBehavior;
    use crate::domain::mode::Mode;
    use crate::i18n;
    use crate::testing::shell_tab::open_shell_tab;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::modes::event::ModesIntent;
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };
    use crate::widgets::settings::event::SettingsIntent;
    use crate::widgets::settings::types::{
        SettingsData, SettingsLoad, SettingsLoadStatus,
    };
    use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
    use crate::widgets::tabs::types::TabMachine;

    /// Owner, Q4: a split shares the room by the saved Equalize panes
    /// setting, not by a change in the open Settings popup that is not saved
    /// yet. Two splits with it on leave three panes of one width.
    #[test]
    fn a_split_follows_the_saved_equalize_setting_not_an_unsaved_one() {
        let (mut app, _) = App::new();
        let mut saved = SettingsData::default();
        saved.set_equalize_panes(true);
        let _ = app.widgets.settings.reduce(SettingsIntent::ReloadLoaded(
            SettingsLoad::new(saved, SettingsLoadStatus::Loaded),
        ));
        let tab_id = open_shell_tab(&mut app);
        let _ = app.widgets.settings.reduce(SettingsIntent::Open);
        let _ = app
            .widgets
            .settings
            .reduce(SettingsIntent::EqualizePanesToggled(false));

        for _ in 0..2 {
            let _ = deliver(
                &mut app,
                TerminalWorkspaceIntent::SplitFocusedPane {
                    axis: pane_grid::Axis::Vertical,
                },
            );
        }

        let regions = app
            .widgets
            .terminal_workspace
            .tab(tab_id)
            .expect("the shell tab should stay open")
            .panes()
            .layout()
            .pane_regions(0.0, 0.0, Size::new(900.0, 600.0));
        assert_eq!(regions.len(), 3);
        for region in regions.values() {
            assert!(
                (region.width - 300.0).abs() <= 0.5,
                "each pane should be near 300 px wide, got {}",
                region.width
            );
        }
    }

    /// Dispatch `event` and all it leads to through the tabs, the terminal
    /// workspace and the modes widget, and return every event on the way.
    /// Events of other widgets are kept but not dispatched, so they start no
    /// file or process work.
    fn follow(app: &mut App, event: AppEvent) -> Vec<AppEvent> {
        let mut seen = Vec::new();
        let mut queue = VecDeque::from([event]);

        while let Some(event) = queue.pop_front() {
            let is_followed = matches!(
                event,
                AppEvent::Tabs(_)
                    | AppEvent::TerminalWorkspace(_)
                    | AppEvent::Modes(_)
            );
            if is_followed {
                queue.extend(collect_outputs(dispatch(app, event.clone())));
            }
            seen.push(event);
        }

        seen
    }

    fn containers_app() -> App {
        let (mut app, _) = App::new();
        let _ = app
            .widgets
            .modes
            .reduce(ModesIntent::Select(Mode::Containers));

        app
    }

    fn run_launch(program: &str) -> AppEvent {
        let mut settings = Settings::default();
        settings.backend = settings.backend.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(program),
            ),
        );

        AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenCommandTab {
            title: String::from("deploy"),
            settings: Box::new(settings),
            exit_behavior: ExitBehavior::Close,
            machine: TabMachine::ThisComputer,
        }))
    }

    fn pushed_texts(events: &[AppEvent]) -> Vec<&str> {
        events
            .iter()
            .filter_map(|event| match event {
                AppEvent::Notifications(NotificationsEvent::Intent(
                    NotificationsIntent::Push(text),
                )) => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Owner: a split with no room gives a toast. The window here is
    /// narrower than two panes of the minimum width.
    #[test]
    fn a_split_with_no_room_tells_the_user() {
        let (mut app, _) = App::new();
        let tab_id = open_shell_tab(&mut app);
        app.state.set_screen_size(Size::new(200.0, 600.0));
        let _ = deliver(&mut app, TerminalWorkspaceIntent::SyncPaneGridSize);

        let task = deliver(
            &mut app,
            TerminalWorkspaceIntent::SplitFocusedPane {
                axis: pane_grid::Axis::Vertical,
            },
        );

        // Checked before the task runs: a split that happened would wait
        // for its terminals to answer.
        assert_eq!(
            app.widgets.terminal_workspace.pane_count(Some(tab_id)),
            Some(1)
        );
        let mut events = Vec::new();
        for event in collect_outputs(task) {
            events.extend(follow(&mut app, event));
        }
        assert_eq!(
            pushed_texts(&events),
            vec![i18n::t(i18n::Key::NoRoomForPane)]
        );
    }

    /// Owner: a launch whose terminal cannot start gives a toast and does not
    /// switch the mode. Its tab closes, so no tab is left.
    #[test]
    fn a_launch_whose_terminal_cannot_start_keeps_the_mode_and_tells_the_user()
    {
        let mut app = containers_app();

        let events = follow(
            &mut app,
            run_launch("/definitely/not/existing/otty-v3-missing-command"),
        );

        assert_eq!(app.widgets.modes.active(), Mode::Containers);
        assert_eq!(app.widgets.tabs.len(), 0);
        let texts = pushed_texts(&events);
        assert_eq!(texts.len(), 1, "{texts:?}");
        assert!(
            texts[0].starts_with(&i18n::launch_failed("deploy", "")),
            "{}",
            texts[0]
        );
    }

    /// A launch whose terminal starts shows its tab, so the app switches to
    /// Terminal mode, as before.
    #[cfg(unix)]
    #[test]
    fn a_launch_whose_terminal_starts_switches_to_terminal_mode() {
        let mut app = containers_app();

        let events = follow(&mut app, run_launch("/bin/sh"));

        assert_eq!(app.widgets.modes.active(), Mode::Terminal);
        assert_eq!(app.widgets.tabs.len(), 1);
        assert!(pushed_texts(&events).is_empty());
    }
}
