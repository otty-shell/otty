use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::explorer::{ExplorerEvent, ExplorerUiEvent};
use crate::widgets::tabs::TabsCommand;
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceCommand, TerminalWorkspaceCtx, TerminalWorkspaceEffect,
    TerminalWorkspaceEvent,
};

/// Route a terminal workspace UI event through the widget reducer.
pub(crate) fn route_event(
    app: &mut App,
    event: TerminalWorkspaceEvent,
) -> Task<AppEvent> {
    let command = map_event_to_command(event);
    route_command(app, command)
}

/// Route a terminal workspace command directly (used by flow routers).
pub(crate) fn route_command(
    app: &mut App,
    command: TerminalWorkspaceCommand,
) -> Task<AppEvent> {
    let ctx = build_ctx_from_parts(
        app.widgets.tabs.active_tab_id(),
        current_pane_grid_size_from_app(app),
        app.state.screen_size,
        app.widgets.sidebar.cursor(),
    );
    app.widgets
        .terminal_workspace
        .reduce(command, &ctx)
        .map(AppEvent::TerminalWorkspaceEffect)
}

/// Route a terminal workspace effect event to app-level tasks.
pub(crate) fn route_effect(
    app: &mut App,
    effect: TerminalWorkspaceEffect,
) -> Task<AppEvent> {
    match effect {
        TerminalWorkspaceEffect::TabClosed { tab_id } => {
            Task::done(AppEvent::TabsCommand(TabsCommand::Close { tab_id }))
        },
        TerminalWorkspaceEffect::TitleChanged { tab_id, title } => {
            Task::done(AppEvent::TabsCommand(TabsCommand::SetTitle {
                tab_id,
                title,
            }))
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

fn map_event_to_command(
    event: TerminalWorkspaceEvent,
) -> TerminalWorkspaceCommand {
    use {TerminalWorkspaceCommand as C, TerminalWorkspaceEvent as E};

    match event {
        E::OpenTab {
            tab_id,
            terminal_id,
            default_title,
            settings,
            kind,
            sync_explorer,
        } => C::OpenTab {
            tab_id,
            terminal_id,
            default_title,
            settings,
            kind,
            sync_explorer,
        },
        E::TabClosed { tab_id } => C::TabClosed { tab_id },
        E::Widget(event) => C::Widget(event),
        E::PaneClicked { tab_id, pane } => C::PaneClicked { tab_id, pane },
        E::PaneResized { tab_id, event } => C::PaneResized { tab_id, event },
        E::PaneGridCursorMoved { tab_id, position } => {
            C::PaneGridCursorMoved { tab_id, position }
        },
        E::OpenContextMenu {
            tab_id,
            pane,
            terminal_id,
        } => C::OpenContextMenu {
            tab_id,
            pane,
            terminal_id,
        },
        E::CloseContextMenu { tab_id } => C::CloseContextMenu { tab_id },
        E::ContextMenuInput { tab_id } => C::ContextMenuInput { tab_id },
        E::SplitPane { tab_id, pane, axis } => {
            C::SplitPane { tab_id, pane, axis }
        },
        E::ClosePane { tab_id, pane } => C::ClosePane { tab_id, pane },
        E::CopySelection {
            tab_id,
            terminal_id,
        } => C::CopySelection {
            tab_id,
            terminal_id,
        },
        E::PasteIntoPrompt {
            tab_id,
            terminal_id,
        } => C::PasteIntoPrompt {
            tab_id,
            terminal_id,
        },
        E::CopySelectedBlockContent {
            tab_id,
            terminal_id,
        } => C::CopySelectedBlockContent {
            tab_id,
            terminal_id,
        },
        E::CopySelectedBlockPrompt {
            tab_id,
            terminal_id,
        } => C::CopySelectedBlockPrompt {
            tab_id,
            terminal_id,
        },
        E::CopySelectedBlockCommand {
            tab_id,
            terminal_id,
        } => C::CopySelectedBlockCommand {
            tab_id,
            terminal_id,
        },
        E::ApplyTheme { palette } => C::ApplyTheme { palette },
        E::CloseAllContextMenus => C::CloseAllContextMenus,
        E::FocusActive => C::FocusActive,
        E::SyncSelection { tab_id } => C::SyncSelection { tab_id },
    }
}
