use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::explorer::{ExplorerEvent, ExplorerUiEvent};
use crate::widgets::tabs::{TabsEvent, TabsUiEvent};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceCtx, TerminalWorkspaceEffect, TerminalWorkspaceEvent,
    TerminalWorkspaceUiEvent,
};

/// Route a terminal workspace event through widget reduction or app orchestration.
pub(crate) fn route(
    app: &mut App,
    event: TerminalWorkspaceEvent,
) -> Task<AppEvent> {
    match event {
        TerminalWorkspaceEvent::Ui(event) => route_ui_event(app, event),
        TerminalWorkspaceEvent::Effect(effect) => {
            route_effect_event(app, effect)
        },
    }
}

fn route_ui_event(
    app: &mut App,
    event: TerminalWorkspaceUiEvent,
) -> Task<AppEvent> {
    let ctx = build_ctx_from_parts(
        app.widgets.tabs.active_tab_id(),
        current_pane_grid_size_from_app(app),
        app.state.screen_size,
        app.widgets.sidebar.cursor(),
    );
    app.widgets
        .terminal_workspace
        .reduce(event, &ctx)
        .map(AppEvent::TerminalWorkspace)
}

/// Route a terminal workspace effect event to app-level tasks.
fn route_effect_event(
    app: &mut App,
    effect: TerminalWorkspaceEffect,
) -> Task<AppEvent> {
    match effect {
        TerminalWorkspaceEffect::TabClosed { tab_id } => {
            Task::done(AppEvent::Tabs(TabsEvent::Ui(TabsUiEvent::CloseTab {
                tab_id,
            })))
        },
        TerminalWorkspaceEffect::TitleChanged { tab_id, title } => {
            Task::done(AppEvent::Tabs(TabsEvent::Ui(TabsUiEvent::SetTitle {
                tab_id,
                title,
            })))
        },
        TerminalWorkspaceEffect::SyncExplorer => {
            sync_explorer_from_terminal(app)
        },
    }
}

/// Sync the explorer widget root from the active terminal CWD.
fn sync_explorer_from_terminal(app: &mut App) -> Task<AppEvent> {
    let active_tab_id = app.widgets.tabs.active_tab_id();
    let Some(cwd) = app
        .widgets
        .terminal_workspace
        .shell_cwd_for_active_tab(active_tab_id)
    else {
        return Task::none();
    };

    Task::done(AppEvent::Explorer(ExplorerEvent::Ui(
        ExplorerUiEvent::SyncRoot { cwd },
    )))
}

fn current_pane_grid_size_from_app(app: &App) -> iced::Size {
    let sidebar = &app.widgets.sidebar;
    crate::state::pane_grid_size(
        app.state.screen_size,
        sidebar.is_hidden(),
        crate::widgets::sidebar::SIDEBAR_MENU_WIDTH,
        sidebar.effective_workspace_ratio(),
    )
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
