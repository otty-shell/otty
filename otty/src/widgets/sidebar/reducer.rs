use iced::Task;

use super::event::{SidebarEffect, SidebarEvent, SidebarIntent};
use super::model::SidebarItem;
use super::state::SidebarState;

const SIDEBAR_MIN_TAB_CONTENT_RATIO: f32 = 0.2;

/// Read-only context for sidebar reduction.
pub(crate) struct SidebarCtx;

/// Reduce a sidebar intent event into state updates and effect events.
pub(crate) fn reduce(
    state: &mut SidebarState,
    event: SidebarIntent,
    _ctx: &SidebarCtx,
) -> Task<SidebarEvent> {
    match event {
        SidebarIntent::SelectTerminal => {
            state.set_active_item(SidebarItem::Terminal);
            if state.ensure_workspace_open(max_sidebar_workspace_ratio()) {
                Task::done(SidebarEvent::Effect(
                    SidebarEffect::SyncTerminalGridSizes,
                ))
            } else {
                Task::none()
            }
        },
        SidebarIntent::SelectExplorer => {
            state.set_active_item(SidebarItem::Explorer);
            if state.ensure_workspace_open(max_sidebar_workspace_ratio()) {
                Task::done(SidebarEvent::Effect(
                    SidebarEffect::SyncTerminalGridSizes,
                ))
            } else {
                Task::none()
            }
        },
        SidebarIntent::ToggleWorkspace => {
            let _ = state.toggle_workspace(max_sidebar_workspace_ratio());
            Task::done(SidebarEvent::Effect(
                SidebarEffect::SyncTerminalGridSizes,
            ))
        },
        SidebarIntent::OpenSettings => {
            Task::done(SidebarEvent::Effect(SidebarEffect::OpenSettingsTab))
        },
        SidebarIntent::AddMenuOpen => {
            state.open_add_menu();
            Task::none()
        },
        SidebarIntent::AddMenuDismiss => {
            state.dismiss_add_menu();
            Task::none()
        },
        SidebarIntent::AddMenuCreateTab => {
            state.dismiss_add_menu();
            Task::done(SidebarEvent::Effect(SidebarEffect::OpenTerminalTab))
        },
        SidebarIntent::AddMenuCreateCommand => {
            state.dismiss_add_menu();
            Task::done(SidebarEvent::Effect(
                SidebarEffect::QuickLaunchHeaderCreateCommand,
            ))
        },
        SidebarIntent::AddMenuCreateFolder => {
            state.dismiss_add_menu();
            Task::done(SidebarEvent::Effect(
                SidebarEffect::QuickLaunchHeaderCreateFolder,
            ))
        },
        SidebarIntent::WorkspaceCursorMoved { position } => {
            state.update_cursor(position);
            Task::none()
        },
        SidebarIntent::ToggleVisibility => {
            state.toggle_visibility();
            Task::done(SidebarEvent::Effect(
                SidebarEffect::SyncTerminalGridSizes,
            ))
        },
        SidebarIntent::PaneGridCursorMoved { position } => {
            state.update_cursor(position);
            Task::none()
        },
        SidebarIntent::Resized(event) => {
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
        SidebarIntent::DismissAddMenu => {
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
    use crate::widgets::sidebar::{SidebarIntent, SidebarWidget};

    #[test]
    fn given_toggle_visibility_command_when_reduced_then_sidebar_hidden_state_toggles()
     {
        let mut widget = SidebarWidget::new();
        let _task = widget.reduce(SidebarIntent::ToggleVisibility, &SidebarCtx);
        assert!(widget.is_hidden());
    }

    #[test]
    fn given_select_explorer_command_when_reduced_then_active_item_changes() {
        let mut widget = SidebarWidget::new();
        let _task = widget.reduce(SidebarIntent::SelectExplorer, &SidebarCtx);
        let vm = widget.vm();
        assert_eq!(vm.active_item, SidebarItem::Explorer);
        assert!(vm.is_workspace_open);
    }

    #[test]
    fn given_add_menu_open_and_dismiss_commands_when_reduced_then_overlay_state_changes()
     {
        let mut widget = SidebarWidget::new();
        let _open_task = widget.reduce(SidebarIntent::AddMenuOpen, &SidebarCtx);
        assert!(widget.has_add_menu_open());

        let _dismiss_task =
            widget.reduce(SidebarIntent::AddMenuDismiss, &SidebarCtx);
        assert!(!widget.has_add_menu_open());
    }

    #[test]
    fn given_cursor_move_command_when_reduced_then_cursor_snapshot_is_updated()
    {
        let mut widget = SidebarWidget::new();
        let expected = Point::new(42.0, 24.0);
        let _task = widget.reduce(
            SidebarIntent::PaneGridCursorMoved { position: expected },
            &SidebarCtx,
        );
        assert_eq!(widget.cursor(), expected);
    }
}
