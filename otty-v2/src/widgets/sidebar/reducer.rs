use iced::Task;

use super::command::SidebarCommand;
use super::event::SidebarEffectEvent;
use super::model::SidebarItem;
use super::state::SidebarState;
use crate::ui::widgets::{sidebar_menu, sidebar_workspace};
use crate::widgets::quick_launch::QuickLaunchEvent;

const SIDEBAR_MIN_TAB_CONTENT_RATIO: f32 = 0.2;

/// Runtime context dependencies for sidebar reducer.
pub(crate) struct SidebarCtx;

/// Reduce sidebar commands into state updates and side-effect events.
pub(crate) fn reduce(
    state: &mut SidebarState,
    command: SidebarCommand,
    _ctx: &SidebarCtx,
) -> Task<SidebarEffectEvent> {
    match command {
        SidebarCommand::Menu(event) => reduce_menu_event(state, event),
        SidebarCommand::Workspace(event) => {
            reduce_workspace_event(state, event)
        },
        SidebarCommand::ToggleVisibility => {
            state.toggle_visibility();
            Task::done(SidebarEffectEvent::SyncTerminalGridSizes)
        },
        SidebarCommand::PaneGridCursorMoved { position } => {
            state.update_cursor(position);
            Task::none()
        },
        SidebarCommand::DismissAddMenu => {
            state.dismiss_add_menu();
            Task::none()
        },
    }
}

fn reduce_menu_event(
    state: &mut SidebarState,
    event: sidebar_menu::SidebarMenuEvent,
) -> Task<SidebarEffectEvent> {
    match event {
        sidebar_menu::SidebarMenuEvent::SelectItem(item) => {
            let canonical = match item {
                sidebar_menu::SidebarMenuItem::Terminal => {
                    SidebarItem::Terminal
                },
                sidebar_menu::SidebarMenuItem::Explorer => {
                    SidebarItem::Explorer
                },
            };
            state.set_active_item(canonical);

            if state.ensure_workspace_open(max_sidebar_workspace_ratio()) {
                Task::done(SidebarEffectEvent::SyncTerminalGridSizes)
            } else {
                Task::none()
            }
        },
        sidebar_menu::SidebarMenuEvent::OpenSettings => {
            Task::done(SidebarEffectEvent::OpenSettingsTab)
        },
        sidebar_menu::SidebarMenuEvent::ToggleWorkspace => {
            let _ = state.toggle_workspace(max_sidebar_workspace_ratio());
            Task::done(SidebarEffectEvent::SyncTerminalGridSizes)
        },
        sidebar_menu::SidebarMenuEvent::Resized(event) => {
            state.mark_resizing();
            let mut tasks = vec![Task::done(SidebarEffectEvent::QuickLaunch(
                QuickLaunchEvent::ResetInteractionState,
            ))];

            if state.apply_resize(event, max_sidebar_workspace_ratio()) {
                tasks.push(Task::done(
                    SidebarEffectEvent::SyncTerminalGridSizes,
                ));
            }

            Task::batch(tasks)
        },
    }
}

fn reduce_workspace_event(
    state: &mut SidebarState,
    event: sidebar_workspace::SidebarWorkspaceEvent,
) -> Task<SidebarEffectEvent> {
    match event {
        sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuOpen => {
            state.open_add_menu();
            Task::none()
        },
        sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuDismiss => {
            state.dismiss_add_menu();
            Task::none()
        },
        sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuAction(
            action,
        ) => {
            state.dismiss_add_menu();
            match action {
                sidebar_workspace::SidebarWorkspaceAddMenuAction::CreateTab => {
                    Task::done(SidebarEffectEvent::OpenTerminalTab)
                },
                sidebar_workspace::SidebarWorkspaceAddMenuAction::CreateCommand => {
                    Task::done(SidebarEffectEvent::QuickLaunch(
                        QuickLaunchEvent::HeaderCreateCommand,
                    ))
                },
                sidebar_workspace::SidebarWorkspaceAddMenuAction::CreateFolder => {
                    Task::done(SidebarEffectEvent::QuickLaunch(
                        QuickLaunchEvent::HeaderCreateFolder,
                    ))
                },
            }
        },
        sidebar_workspace::SidebarWorkspaceEvent::WorkspaceCursorMoved {
            position,
        } => {
            state.update_cursor(position);
            Task::none()
        },
        sidebar_workspace::SidebarWorkspaceEvent::QuickLaunch(event) => {
            Task::done(SidebarEffectEvent::QuickLaunch(event))
        },
        sidebar_workspace::SidebarWorkspaceEvent::Explorer(event) => {
            Task::done(SidebarEffectEvent::Explorer(event))
        },
    }
}

/// Maximum workspace width ratio, leaving room for minimum tab content.
fn max_sidebar_workspace_ratio() -> f32 {
    (1.0 - SIDEBAR_MIN_TAB_CONTENT_RATIO).max(0.0)
}
