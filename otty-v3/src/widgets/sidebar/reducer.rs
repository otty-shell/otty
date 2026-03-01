use iced::Task;

use super::event::{SidebarEffect, SidebarEvent, SidebarUiEvent};
use super::model::SidebarItem;
use super::state::SidebarState;

const SIDEBAR_MIN_TAB_CONTENT_RATIO: f32 = 0.2;

/// Read-only context for sidebar reduction.
pub(crate) struct SidebarCtx;

/// Reduce a sidebar UI event into state updates and effect events.
pub(crate) fn reduce(
    state: &mut SidebarState,
    event: SidebarUiEvent,
    _ctx: &SidebarCtx,
) -> Task<SidebarEvent> {
    match event {
        SidebarUiEvent::SelectTerminal => {
            state.set_active_item(SidebarItem::Terminal);
            if state.ensure_workspace_open(max_sidebar_workspace_ratio()) {
                Task::done(SidebarEvent::Effect(
                    SidebarEffect::SyncTerminalGridSizes,
                ))
            } else {
                Task::none()
            }
        },
        SidebarUiEvent::SelectExplorer => {
            state.set_active_item(SidebarItem::Explorer);
            if state.ensure_workspace_open(max_sidebar_workspace_ratio()) {
                Task::done(SidebarEvent::Effect(
                    SidebarEffect::SyncTerminalGridSizes,
                ))
            } else {
                Task::none()
            }
        },
        SidebarUiEvent::ToggleWorkspace => {
            let _ = state.toggle_workspace(max_sidebar_workspace_ratio());
            Task::done(SidebarEvent::Effect(
                SidebarEffect::SyncTerminalGridSizes,
            ))
        },
        SidebarUiEvent::OpenSettings => {
            Task::done(SidebarEvent::Effect(SidebarEffect::OpenSettingsTab))
        },
        SidebarUiEvent::AddMenuOpen => {
            state.open_add_menu();
            Task::none()
        },
        SidebarUiEvent::AddMenuDismiss => {
            state.dismiss_add_menu();
            Task::none()
        },
        SidebarUiEvent::AddMenuCreateTab => {
            state.dismiss_add_menu();
            Task::done(SidebarEvent::Effect(SidebarEffect::OpenTerminalTab))
        },
        SidebarUiEvent::AddMenuCreateCommand => {
            state.dismiss_add_menu();
            Task::done(SidebarEvent::Effect(
                SidebarEffect::QuickLaunchHeaderCreateCommand,
            ))
        },
        SidebarUiEvent::AddMenuCreateFolder => {
            state.dismiss_add_menu();
            Task::done(SidebarEvent::Effect(
                SidebarEffect::QuickLaunchHeaderCreateFolder,
            ))
        },
        SidebarUiEvent::WorkspaceCursorMoved { position } => {
            state.update_cursor(position);
            Task::none()
        },
        SidebarUiEvent::ToggleVisibility => {
            state.toggle_visibility();
            Task::done(SidebarEvent::Effect(
                SidebarEffect::SyncTerminalGridSizes,
            ))
        },
        SidebarUiEvent::PaneGridCursorMoved { position } => {
            state.update_cursor(position);
            Task::none()
        },
        SidebarUiEvent::Resized(event) => {
            state.mark_resizing();
            let mut tasks = vec![Task::done(SidebarEvent::Effect(
                SidebarEffect::QuickLaunchResetInteractionState,
            ))];

            if state.apply_resize(event, max_sidebar_workspace_ratio()) {
                tasks.push(Task::done(SidebarEvent::Effect(
                    SidebarEffect::SyncTerminalGridSizes,
                )));
            }

            Task::batch(tasks)
        },
        SidebarUiEvent::DismissAddMenu => {
            state.dismiss_add_menu();
            Task::none()
        },
    }
}

/// Maximum workspace width ratio, leaving room for minimum tab content.
fn max_sidebar_workspace_ratio() -> f32 {
    (1.0 - SIDEBAR_MIN_TAB_CONTENT_RATIO).max(0.0)
}

#[cfg(test)]
mod tests {
    use iced::Point;

    use super::SidebarCtx;
    use crate::widgets::sidebar::model::SidebarItem;
    use crate::widgets::sidebar::{SidebarUiEvent, SidebarWidget};

    #[test]
    fn given_toggle_visibility_command_when_reduced_then_sidebar_hidden_state_toggles()
     {
        let mut widget = SidebarWidget::new();
        let _task =
            widget.reduce(SidebarUiEvent::ToggleVisibility, &SidebarCtx);
        assert!(widget.is_hidden());
    }

    #[test]
    fn given_select_explorer_command_when_reduced_then_active_item_changes() {
        let mut widget = SidebarWidget::new();
        let _task = widget.reduce(SidebarUiEvent::SelectExplorer, &SidebarCtx);
        let vm = widget.vm();
        assert_eq!(vm.active_item, SidebarItem::Explorer);
        assert!(vm.is_workspace_open);
    }

    #[test]
    fn given_add_menu_open_and_dismiss_commands_when_reduced_then_overlay_state_changes()
     {
        let mut widget = SidebarWidget::new();
        let _open_task =
            widget.reduce(SidebarUiEvent::AddMenuOpen, &SidebarCtx);
        assert!(widget.has_add_menu_open());

        let _dismiss_task =
            widget.reduce(SidebarUiEvent::AddMenuDismiss, &SidebarCtx);
        assert!(!widget.has_add_menu_open());
    }

    #[test]
    fn given_cursor_move_command_when_reduced_then_cursor_snapshot_is_updated()
    {
        let mut widget = SidebarWidget::new();
        let expected = Point::new(42.0, 24.0);
        let _task = widget.reduce(
            SidebarUiEvent::PaneGridCursorMoved { position: expected },
            &SidebarCtx,
        );
        assert_eq!(widget.cursor(), expected);
    }
}
