use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::layout::pane_grid_size;
use crate::widgets::explorer::{ExplorerEvent, ExplorerIntent};
use crate::widgets::sidebar::constants::SIDEBAR_MENU_WIDTH;
use crate::widgets::tabs::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceCtx, TerminalWorkspaceEffect, TerminalWorkspaceEvent,
    TerminalWorkspaceIntent,
};

pub(crate) fn handle(
    app: &mut App,
    event: TerminalWorkspaceEvent,
) -> Task<AppEvent> {
    match event {
        TerminalWorkspaceEvent::Intent(event) => {
            handle_intent_event(app, event)
        },
        TerminalWorkspaceEvent::Effect(effect) => {
            handle_effect_event(app, effect)
        },
    }
}

fn handle_intent_event(
    app: &mut App,
    event: TerminalWorkspaceIntent,
) -> Task<AppEvent> {
    let sidebar = &app.widgets.sidebar;

    let pane_grid_size = pane_grid_size(
        app.state.screen_size,
        sidebar.is_hidden(),
        SIDEBAR_MENU_WIDTH,
        sidebar.effective_workspace_ratio(),
    );

    let ctx = build_ctx_from_parts(
        app.widgets.tabs.active_tab_id(),
        pane_grid_size,
        app.state.screen_size,
        app.widgets.sidebar.cursor(),
    );

    let should_sync = should_sync_explorer(&event);
    let workspace_task = app
        .widgets
        .terminal_workspace
        .reduce(event, &ctx)
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
        TerminalWorkspaceEffect::CommandTabOpenFailed {
            tab_id,
            title,
            message,
        } => Task::batch(vec![
            Task::done(AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::CloseTab { tab_id },
            ))),
            Task::done(AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenErrorTab { title, message },
            ))),
        ]),
        TerminalWorkspaceEffect::TitleChanged { tab_id, title } => {
            Task::done(AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::SetTitle { tab_id, title },
            )))
        },
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
        .shell_cwd_for_active_tab(active_tab_id)
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
) -> TerminalWorkspaceCtx {
    TerminalWorkspaceCtx {
        active_tab_id,
        pane_grid_size,
        screen_size,
        sidebar_cursor,
    }
}

fn should_sync_explorer(event: &TerminalWorkspaceIntent) -> bool {
    matches!(
        event,
        TerminalWorkspaceIntent::PaneClicked { .. }
            | TerminalWorkspaceIntent::SplitPane { .. }
            | TerminalWorkspaceIntent::ClosePane { .. }
            | TerminalWorkspaceIntent::Widget(
                otty_ui_term::Event::ContentSync { .. }
            )
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{handle_effect_event, should_sync_explorer};
    use crate::app::App;
    use crate::widgets::terminal_workspace::{
        TerminalWorkspaceEffect, TerminalWorkspaceIntent,
    };

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
    fn given_non_sync_terminal_intent_when_checked_then_returns_false() {
        assert!(!should_sync_explorer(
            &TerminalWorkspaceIntent::SyncPaneGridSize
        ));
    }

    #[test]
    fn given_command_tab_failed_effect_when_handled_then_close_and_error_tasks_emitted()
     {
        let (mut app, _) = App::new();
        let task = handle_effect_event(
            &mut app,
            TerminalWorkspaceEffect::CommandTabOpenFailed {
                tab_id: 5,
                title: String::from("Failed"),
                message: String::from("boom"),
            },
        );

        assert_eq!(task.units(), 2);
    }
}
