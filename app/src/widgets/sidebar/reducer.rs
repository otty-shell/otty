use iced::Task;

use super::event::{SidebarEffect, SidebarEvent, SidebarIntent};
use super::state::SidebarState;
use super::types::SidebarItem;

/// Smallest share of the window width the main area keeps when the side
/// panel is dragged wider.
const SIDEBAR_MIN_TAB_CONTENT_RATIO: f32 = 0.2;

/// Reduce a sidebar intent event into state updates and effect events.
pub(crate) fn reduce(
    state: &mut SidebarState,
    event: SidebarIntent,
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
    use iced::widget::pane_grid;

    use crate::task_outputs::collect_outputs;
    use crate::widgets::sidebar::types::SidebarItem;
    use crate::widgets::sidebar::{
        SidebarEffect, SidebarEvent, SidebarIntent, SidebarWidget,
    };

    /// Drag the split between the side panel and the main area to `ratio`.
    fn resize_to(widget: &mut SidebarWidget, ratio: f32) -> Vec<SidebarEvent> {
        let pane_grid::Node::Split { id: split, .. } = widget.panes().layout()
        else {
            panic!("sidebar should be split into panel and main area");
        };
        let event = pane_grid::ResizeEvent {
            split: *split,
            ratio,
        };

        collect_outputs(widget.reduce(SidebarIntent::Resized(event)))
    }

    fn has_effect(
        events: &[SidebarEvent],
        expected: fn(&SidebarEffect) -> bool,
    ) -> bool {
        events.iter().any(|event| match event {
            SidebarEvent::Effect(effect) => expected(effect),
            _ => false,
        })
    }

    fn is_sync_grid(effect: &SidebarEffect) -> bool {
        matches!(effect, SidebarEffect::SyncTerminalGridSizes)
    }

    fn is_reset_quick_launch(effect: &SidebarEffect) -> bool {
        matches!(effect, SidebarEffect::QuickLaunchResetInteractionState)
    }

    #[test]
    fn given_toggle_visibility_command_when_reduced_then_sidebar_hidden_state_toggles()
     {
        let mut widget = SidebarWidget::new();
        let _task = widget.reduce(SidebarIntent::ToggleVisibility);
        assert!(widget.is_hidden());
    }

    #[test]
    fn given_select_explorer_command_when_reduced_then_active_item_changes() {
        let mut widget = SidebarWidget::new();
        let _task = widget.reduce(SidebarIntent::SelectExplorer);
        let vm = widget.vm();
        assert_eq!(vm.active_item, SidebarItem::Explorer);
        assert!(vm.is_workspace_open);
    }

    #[test]
    fn given_add_menu_open_and_dismiss_commands_when_reduced_then_overlay_state_changes()
     {
        let mut widget = SidebarWidget::new();
        let _open_task = widget.reduce(SidebarIntent::AddMenuOpen);
        assert!(widget.has_add_menu_open());

        let _dismiss_task = widget.reduce(SidebarIntent::AddMenuDismiss);
        assert!(!widget.has_add_menu_open());
    }

    #[test]
    fn given_cursor_move_command_when_reduced_then_cursor_snapshot_is_updated()
    {
        let mut widget = SidebarWidget::new();
        let expected = Point::new(42.0, 24.0);
        let _task = widget
            .reduce(SidebarIntent::PaneGridCursorMoved { position: expected });
        assert_eq!(widget.cursor(), expected);
    }

    #[test]
    fn given_hidden_sidebar_when_visibility_toggled_again_then_it_shows_at_same_width()
     {
        let mut widget = SidebarWidget::new();

        let hide =
            collect_outputs(widget.reduce(SidebarIntent::ToggleVisibility));

        assert!(widget.is_hidden());
        assert_eq!(widget.effective_workspace_ratio(), 0.0);
        assert!(has_effect(&hide, is_sync_grid));

        let show =
            collect_outputs(widget.reduce(SidebarIntent::ToggleVisibility));

        assert!(!widget.is_hidden());
        assert_eq!(widget.effective_workspace_ratio(), 0.2);
        assert!(has_effect(&show, is_sync_grid));
    }

    #[test]
    fn given_open_panel_when_workspace_toggled_then_it_closes_and_grid_resyncs()
    {
        let mut widget = SidebarWidget::new();

        let events =
            collect_outputs(widget.reduce(SidebarIntent::ToggleWorkspace));

        assert!(!widget.vm().is_workspace_open);
        assert_eq!(widget.effective_workspace_ratio(), 0.0);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn given_open_panel_when_dragged_then_width_follows_and_grid_resyncs() {
        let mut widget = SidebarWidget::new();

        let events = resize_to(&mut widget, 0.35);

        assert_eq!(widget.effective_workspace_ratio(), 0.35);
        assert!(widget.is_resizing());
        assert!(has_effect(&events, is_reset_quick_launch));
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn given_open_panel_when_dragged_too_wide_then_main_area_keeps_a_fifth() {
        let mut widget = SidebarWidget::new();

        let _events = resize_to(&mut widget, 0.95);

        assert_eq!(widget.effective_workspace_ratio(), 0.8);
    }

    #[test]
    fn given_open_panel_when_dragged_below_collapse_threshold_then_it_closes() {
        // The collapse threshold is 0.04 of the window width.
        let mut widget = SidebarWidget::new();

        let events = resize_to(&mut widget, 0.03);

        assert!(!widget.vm().is_workspace_open);
        assert_eq!(widget.effective_workspace_ratio(), 0.0);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn given_open_panel_when_dragged_just_above_collapse_threshold_then_it_stays_open()
     {
        let mut widget = SidebarWidget::new();

        let _events = resize_to(&mut widget, 0.05);

        assert!(widget.vm().is_workspace_open);
        assert_eq!(widget.effective_workspace_ratio(), 0.05);
    }

    #[test]
    fn given_closed_panel_when_dragged_open_then_it_opens_at_collapse_width() {
        let mut widget = SidebarWidget::new();
        let _close = widget.reduce(SidebarIntent::ToggleWorkspace);

        let events = resize_to(&mut widget, 0.5);

        // 0.2 * 0.2 in `f32` is not exactly 0.04.
        assert!(widget.vm().is_workspace_open);
        assert!((widget.effective_workspace_ratio() - 0.04).abs() < 1e-6);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn given_closed_panel_when_dragged_a_little_then_grid_is_not_resynced() {
        let mut widget = SidebarWidget::new();
        let _close = widget.reduce(SidebarIntent::ToggleWorkspace);

        let events = resize_to(&mut widget, 0.02);

        assert!(!widget.vm().is_workspace_open);
        assert!(has_effect(&events, is_reset_quick_launch));
        assert!(!has_effect(&events, is_sync_grid));
    }

    #[test]
    fn given_closed_panel_when_explorer_selected_then_it_reopens_at_default_width()
     {
        let mut widget = SidebarWidget::new();
        let _close = widget.reduce(SidebarIntent::ToggleWorkspace);

        let events =
            collect_outputs(widget.reduce(SidebarIntent::SelectExplorer));

        assert!(widget.vm().is_workspace_open);
        assert_eq!(widget.effective_workspace_ratio(), 0.2);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn given_open_panel_when_terminal_selected_then_grid_is_not_resynced() {
        let mut widget = SidebarWidget::new();

        let events =
            collect_outputs(widget.reduce(SidebarIntent::SelectTerminal));

        assert_eq!(widget.vm().active_item, SidebarItem::Terminal);
        assert!(events.is_empty());
    }
}
