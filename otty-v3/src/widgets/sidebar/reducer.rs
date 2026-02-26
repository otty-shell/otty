use iced::Task;

use super::command::SidebarCommand;
use super::event::SidebarEffect;
use super::model::SidebarItem;
use super::state::SidebarState;

const SIDEBAR_MIN_TAB_CONTENT_RATIO: f32 = 0.2;

/// Read-only context for sidebar reduction.
pub(crate) struct SidebarCtx;

/// Reduce a sidebar command into state updates and effect events.
pub(crate) fn reduce(
    state: &mut SidebarState,
    command: SidebarCommand,
    _ctx: &SidebarCtx,
) -> Task<SidebarEffect> {
    match command {
        SidebarCommand::SelectTerminal => {
            state.set_active_item(SidebarItem::Terminal);
            if state.ensure_workspace_open(max_sidebar_workspace_ratio()) {
                Task::done(SidebarEffect::SyncTerminalGridSizes)
            } else {
                Task::none()
            }
        },
        SidebarCommand::SelectExplorer => {
            state.set_active_item(SidebarItem::Explorer);
            if state.ensure_workspace_open(max_sidebar_workspace_ratio()) {
                Task::done(SidebarEffect::SyncTerminalGridSizes)
            } else {
                Task::none()
            }
        },
        SidebarCommand::ToggleWorkspace => {
            let _ = state.toggle_workspace(max_sidebar_workspace_ratio());
            Task::done(SidebarEffect::SyncTerminalGridSizes)
        },
        SidebarCommand::OpenSettings => {
            Task::done(SidebarEffect::OpenSettingsTab)
        },
        SidebarCommand::AddMenuOpen => {
            state.open_add_menu();
            Task::none()
        },
        SidebarCommand::AddMenuDismiss => {
            state.dismiss_add_menu();
            Task::none()
        },
        SidebarCommand::AddMenuCreateTab => {
            state.dismiss_add_menu();
            Task::done(SidebarEffect::OpenTerminalTab)
        },
        SidebarCommand::AddMenuCreateCommand => {
            state.dismiss_add_menu();
            Task::done(SidebarEffect::QuickLaunchHeaderCreateCommand)
        },
        SidebarCommand::AddMenuCreateFolder => {
            state.dismiss_add_menu();
            Task::done(SidebarEffect::QuickLaunchHeaderCreateFolder)
        },
        SidebarCommand::WorkspaceCursorMoved { position } => {
            state.update_cursor(position);
            Task::none()
        },
        SidebarCommand::ToggleVisibility => {
            state.toggle_visibility();
            Task::done(SidebarEffect::SyncTerminalGridSizes)
        },
        SidebarCommand::PaneGridCursorMoved { position } => {
            state.update_cursor(position);
            Task::none()
        },
        SidebarCommand::Resized(event) => {
            state.mark_resizing();
            let mut tasks = vec![Task::done(
                SidebarEffect::QuickLaunchResetInteractionState,
            )];

            if state.apply_resize(event, max_sidebar_workspace_ratio()) {
                tasks.push(Task::done(SidebarEffect::SyncTerminalGridSizes));
            }

            Task::batch(tasks)
        },
        SidebarCommand::DismissAddMenu => {
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
    use crate::widgets::sidebar::SidebarWidget;
    use crate::widgets::sidebar::command::SidebarCommand;
    use crate::widgets::sidebar::model::SidebarItem;

    #[test]
    fn given_toggle_visibility_command_when_reduced_then_sidebar_hidden_state_toggles()
     {
        let mut widget = SidebarWidget::new();
        let _task =
            widget.reduce(SidebarCommand::ToggleVisibility, &SidebarCtx);
        assert!(widget.is_hidden());
    }

    #[test]
    fn given_select_explorer_command_when_reduced_then_active_item_changes() {
        let mut widget = SidebarWidget::new();
        let _task = widget.reduce(SidebarCommand::SelectExplorer, &SidebarCtx);
        let vm = widget.vm();
        assert_eq!(vm.active_item, SidebarItem::Explorer);
        assert!(vm.is_workspace_open);
    }

    #[test]
    fn given_add_menu_open_and_dismiss_commands_when_reduced_then_overlay_state_changes()
     {
        let mut widget = SidebarWidget::new();
        let _open_task =
            widget.reduce(SidebarCommand::AddMenuOpen, &SidebarCtx);
        assert!(widget.has_add_menu_open());

        let _dismiss_task =
            widget.reduce(SidebarCommand::AddMenuDismiss, &SidebarCtx);
        assert!(!widget.has_add_menu_open());
    }

    #[test]
    fn given_cursor_move_command_when_reduced_then_cursor_snapshot_is_updated()
    {
        let mut widget = SidebarWidget::new();
        let expected = Point::new(42.0, 24.0);
        let _task = widget.reduce(
            SidebarCommand::PaneGridCursorMoved { position: expected },
            &SidebarCtx,
        );
        assert_eq!(widget.cursor(), expected);
    }
}
