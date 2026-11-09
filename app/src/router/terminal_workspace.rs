use iced::Task;

use super::{AppEvent, notifications};
use crate::app::App;
use crate::guards::{self, Overlay};
use crate::layout::pane_grid_size;
use crate::widgets::explorer::event::{ExplorerEvent, ExplorerIntent};
use crate::widgets::modes::event::{ModesEvent, ModesIntent};
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::TerminalWorkspaceCtx;
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEffect, TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Route a terminal workspace event: reduce its intents and turn its
/// effects into events of the other widgets.
pub(crate) fn route(
    app: &mut App,
    event: TerminalWorkspaceEvent,
) -> Task<AppEvent> {
    match event {
        TerminalWorkspaceEvent::Intent(intent) => {
            handle_intent_event(app, intent)
        },
        TerminalWorkspaceEvent::Effect(effect) => {
            handle_effect_event(app, effect)
        },
    }
}

fn handle_intent_event(
    app: &mut App,
    intent: TerminalWorkspaceIntent,
) -> Task<AppEvent> {
    let pane_grid_size = pane_grid_size(
        app.state.screen_size,
        app.widgets.sidebar.side_panels_ratio(),
    );

    let top_overlay = guards::top_overlay(app);
    let ctx = build_ctx_from_parts(
        app.widgets.tabs.active_tab_id(),
        pane_grid_size,
        app.state.screen_size,
        app.widgets.sidebar.cursor(),
        app.widgets.settings.saved().equalize_panes(),
        top_overlay.is_some(),
        top_overlay
            .is_some_and(|overlay| overlay != Overlay::TerminalContextMenu),
    );

    let should_sync = should_sync_explorer(&intent);
    let workspace_task = app
        .widgets
        .terminal_workspace
        .reduce(intent, &ctx)
        .map(AppEvent::TerminalWorkspace);

    if !should_sync {
        return workspace_task;
    }

    let sync_task = Task::done(AppEvent::TerminalWorkspace(
        TerminalWorkspaceEvent::Effect(TerminalWorkspaceEffect::SyncExplorer),
    ));
    Task::batch(vec![workspace_task, sync_task])
}

fn handle_effect_event(
    app: &mut App,
    effect: TerminalWorkspaceEffect,
) -> Task<AppEvent> {
    match effect {
        TerminalWorkspaceEffect::TabClosed { tab_id } => Task::done(
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseTab { tab_id })),
        ),
        TerminalWorkspaceEffect::Failed(text) => notifications::push(text),
        // Tabs only show in Terminal mode (spec Decision 5). A tab tells the
        // modes widget once its terminal runs, so a tab that cannot start
        // leaves the mode as it was.
        TerminalWorkspaceEffect::TabStarted => Task::done(AppEvent::Modes(
            ModesEvent::Intent(ModesIntent::TabShown),
        )),
        TerminalWorkspaceEffect::TitleChanged {
            tab_id,
            title,
            folder,
        } => Task::done(AppEvent::Tabs(TabsEvent::Intent(
            TabsIntent::SetTitle {
                tab_id,
                title,
                folder,
            },
        ))),
        TerminalWorkspaceEffect::SyncExplorer => {
            sync_explorer_from_terminal(app)
        },
    }
}

fn sync_explorer_from_terminal(app: &mut App) -> Task<AppEvent> {
    let active_tab_id = app.widgets.tabs.active_tab_id();
    let Some(cwd) = app
        .widgets
        .terminal_workspace
        .local_cwd_for_active_tab(active_tab_id)
    else {
        return Task::none();
    };

    Task::done(AppEvent::Explorer(ExplorerEvent::Intent(
        ExplorerIntent::SyncRoot { cwd },
    )))
}

fn build_ctx_from_parts(
    active_tab_id: Option<u64>,
    pane_grid_size: iced::Size,
    screen_size: iced::Size,
    sidebar_cursor: iced::Point,
    equalize_panes: bool,
    is_overlay_open: bool,
    is_other_overlay_open: bool,
) -> TerminalWorkspaceCtx {
    TerminalWorkspaceCtx {
        active_tab_id,
        pane_grid_size,
        screen_size,
        sidebar_cursor,
        equalize_panes,
        is_overlay_open,
        is_other_overlay_open,
    }
}

fn should_sync_explorer(intent: &TerminalWorkspaceIntent) -> bool {
    use TerminalWorkspaceIntent::*;

    matches!(
        intent,
        PaneClicked { .. }
            | SplitPane { .. }
            | SplitFocusedPane { .. }
            | ClosePane { .. }
            | Widget(otty_ui_term::Event::ContentSync { .. })
    )
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Arc;

    use iced::Size;
    use iced::widget::pane_grid;
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::{handle_effect_event, route, should_sync_explorer};
    use crate::app::App;
    use crate::domain::exit_behavior::ExitBehavior;
    use crate::domain::mode::Mode;
    use crate::i18n;
    use crate::router::AppEvent;
    use crate::testing::shell_tab::open_shell_tab;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::modes::event::{ModesEvent, ModesIntent};
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };
    use crate::widgets::settings::event::SettingsIntent;
    use crate::widgets::settings::types::{
        SettingsData, SettingsLoad, SettingsLoadStatus,
    };
    use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
    use crate::widgets::tabs::types::TabMachine;
    use crate::widgets::terminal_workspace::event::{
        TerminalWorkspaceEffect, TerminalWorkspaceEvent,
        TerminalWorkspaceIntent,
    };

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
            let _ = route(
                &mut app,
                TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::SplitFocusedPane {
                        axis: pane_grid::Axis::Vertical,
                    },
                ),
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

    #[test]
    fn given_sync_relevant_terminal_intent_when_checked_then_returns_true() {
        let (_grid, pane) = iced::widget::pane_grid::State::new(1_u64);
        assert!(should_sync_explorer(
            &TerminalWorkspaceIntent::PaneClicked { tab_id: 1, pane }
        ));
        assert!(should_sync_explorer(&TerminalWorkspaceIntent::SplitPane {
            tab_id: 1,
            pane,
            axis: iced::widget::pane_grid::Axis::Vertical,
        }));
        assert!(should_sync_explorer(&TerminalWorkspaceIntent::ClosePane {
            tab_id: 1,
            pane,
        }));
        assert!(should_sync_explorer(&TerminalWorkspaceIntent::Widget(
            otty_ui_term::Event::ContentSync {
                id: 42,
                frame: Arc::new(otty_libterm::surface::SnapshotOwned::default()),
            }
        )));
    }

    #[test]
    fn given_split_focused_pane_when_checked_then_returns_true() {
        // The split focuses the new pane, so the explorer follows its cwd.
        assert!(should_sync_explorer(
            &TerminalWorkspaceIntent::SplitFocusedPane {
                axis: iced::widget::pane_grid::Axis::Vertical,
            }
        ));
    }

    #[test]
    fn given_non_sync_terminal_intent_when_checked_then_returns_false() {
        assert!(!should_sync_explorer(
            &TerminalWorkspaceIntent::SyncPaneGridSize
        ));
    }

    #[test]
    fn a_failure_becomes_a_notification() {
        let (mut app, _) = App::new();

        let events = collect_outputs(handle_effect_event(
            &mut app,
            TerminalWorkspaceEffect::Failed(String::from("boom")),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::Push(text)
            ))] if text == "boom"
        ));
    }

    /// Tabs only show in Terminal mode, so a tab whose terminal started
    /// tells the modes widget, which switches from another mode.
    #[test]
    fn a_started_tab_tells_the_modes_widget_it_is_shown() {
        let (mut app, _) = App::new();

        let events = collect_outputs(handle_effect_event(
            &mut app,
            TerminalWorkspaceEffect::TabStarted,
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Intent(ModesIntent::TabShown))]
        ));
    }

    /// Route `event` and all it leads to through the tabs, the terminal
    /// workspace and the modes widget, and return every event on the way.
    /// Events of other widgets are kept but not routed, so they start no
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
                queue.extend(collect_outputs(crate::router::route(
                    app,
                    event.clone(),
                )));
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
