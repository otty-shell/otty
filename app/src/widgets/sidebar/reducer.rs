use iced::Task;

use super::event::{SidebarEffect, SidebarEvent, SidebarIntent};
use super::state::SidebarState;

/// Reduce a sidebar intent event into state updates and effect events.
///
/// Every change that resizes the main area emits `SyncTerminalGridSizes`, so
/// the terminals redraw at their new size.
pub(crate) fn reduce(
    state: &mut SidebarState,
    event: SidebarIntent,
) -> Task<SidebarEvent> {
    match event {
        SidebarIntent::ToggleLeft => {
            state.toggle_left();
            sync_terminal_grid_sizes()
        },
        SidebarIntent::ToggleRight => {
            state.toggle_right();
            sync_terminal_grid_sizes()
        },
        SidebarIntent::ToggleFocusMode => {
            state.toggle_focus_mode();
            sync_terminal_grid_sizes()
        },
        SidebarIntent::ToggleLaunches => {
            state.toggle_launches();
            Task::none()
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
        SidebarIntent::PaneGridCursorMoved { position } => {
            state.update_cursor(position);
            Task::none()
        },
        SidebarIntent::Resized(event) => {
            state.mark_resizing();
            let mut tasks = vec![Task::done(SidebarEvent::Effect(
                SidebarEffect::QuickLaunchResetInteractionState,
            ))];

            if state.apply_resize(event) {
                tasks.push(sync_terminal_grid_sizes());
            }

            Task::batch(tasks)
        },
        SidebarIntent::DismissAddMenu => {
            state.dismiss_add_menu();
            Task::none()
        },
    }
}

fn sync_terminal_grid_sizes() -> Task<SidebarEvent> {
    Task::done(SidebarEvent::Effect(SidebarEffect::SyncTerminalGridSizes))
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced::widget::pane_grid;

    use crate::task_outputs::collect_outputs;
    use crate::widgets::sidebar::{
        SidebarEffect, SidebarEvent, SidebarIntent, SidebarWidget,
    };

    /// Ratios computed from other ratios are compared with this tolerance.
    const TOLERANCE: f32 = 1e-6;

    fn reduce(
        widget: &mut SidebarWidget,
        intent: SidebarIntent,
    ) -> Vec<SidebarEvent> {
        collect_outputs(widget.reduce(intent))
    }

    /// The split between the left panel and the rest, and the split between
    /// the main area and the right panel.
    fn splits(widget: &SidebarWidget) -> (pane_grid::Split, pane_grid::Split) {
        let pane_grid::Node::Split {
            id: left_split, b, ..
        } = widget.panes().layout()
        else {
            panic!("sidebar should split the left panel from the rest");
        };
        let pane_grid::Node::Split {
            id: right_split, ..
        } = b.as_ref()
        else {
            panic!("sidebar should split the main area from the right panel");
        };

        (*left_split, *right_split)
    }

    /// Ratio the pane grid holds for the split between main and right.
    fn right_split_ratio(widget: &SidebarWidget) -> f32 {
        let pane_grid::Node::Split { b, .. } = widget.panes().layout() else {
            panic!("sidebar should split the left panel from the rest");
        };
        let pane_grid::Node::Split { ratio, .. } = b.as_ref() else {
            panic!("sidebar should split the main area from the right panel");
        };

        *ratio
    }

    /// Drag the edge of the left panel to `ratio` of the window width.
    fn drag_left_to(
        widget: &mut SidebarWidget,
        ratio: f32,
    ) -> Vec<SidebarEvent> {
        let (split, _) = splits(widget);

        reduce(
            widget,
            SidebarIntent::Resized(pane_grid::ResizeEvent { split, ratio }),
        )
    }

    /// Drag the edge of the right panel. Like the pane grid, `ratio` is the
    /// position inside the area right of the left panel.
    fn drag_right_split_to(
        widget: &mut SidebarWidget,
        ratio: f32,
    ) -> Vec<SidebarEvent> {
        let (_, split) = splits(widget);

        reduce(
            widget,
            SidebarIntent::Resized(pane_grid::ResizeEvent { split, ratio }),
        )
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

    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < TOLERANCE,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn new_sidebar_shows_the_left_panel_at_a_fifth_and_hides_the_right() {
        let widget = SidebarWidget::new();
        let vm = widget.vm();

        assert!(vm.is_left_open);
        assert!(!vm.is_right_open);
        assert!(!vm.is_focus_mode);
        assert_eq!(widget.left_ratio(), 0.2);
        assert_eq!(widget.right_ratio(), 0.0);
        assert_eq!(widget.side_panels_ratio(), 0.2);
        assert!(!widget.is_resizing());
    }

    #[test]
    fn toggle_left_hides_the_panel_and_shows_it_again_at_the_same_width() {
        let mut widget = SidebarWidget::new();

        let hide = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert!(!widget.vm().is_left_open);
        assert_eq!(widget.left_ratio(), 0.0);
        assert!(has_effect(&hide, is_sync_grid));

        let show = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert!(widget.vm().is_left_open);
        assert_eq!(widget.left_ratio(), 0.2);
        assert!(has_effect(&show, is_sync_grid));
    }

    #[test]
    fn toggle_right_opens_the_closed_panel_at_a_fifth_and_resyncs_the_grid() {
        let mut widget = SidebarWidget::new();

        let events = reduce(&mut widget, SidebarIntent::ToggleRight);

        assert!(widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), 0.2);
        assert_eq!(widget.left_ratio(), 0.2);
        assert_near(widget.side_panels_ratio(), 0.4);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn toggle_right_twice_closes_the_panel_again() {
        let mut widget = SidebarWidget::new();
        let _open = reduce(&mut widget, SidebarIntent::ToggleRight);

        let events = reduce(&mut widget, SidebarIntent::ToggleRight);

        assert!(!widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), 0.0);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn focus_mode_hides_both_panels_and_leaving_it_restores_their_widths() {
        let mut widget = SidebarWidget::new();
        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _wider_left = drag_left_to(&mut widget, 0.3);

        let enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let vm = widget.vm();
        assert!(vm.is_focus_mode);
        assert!(!vm.is_left_open);
        assert!(!vm.is_right_open);
        assert_eq!(widget.side_panels_ratio(), 0.0);
        assert!(has_effect(&enter, is_sync_grid));

        let leave = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let vm = widget.vm();
        assert!(!vm.is_focus_mode);
        assert!(vm.is_left_open);
        assert!(vm.is_right_open);
        assert_eq!(widget.left_ratio(), 0.3);
        assert_eq!(widget.right_ratio(), 0.2);
        assert!(has_effect(&leave, is_sync_grid));
    }

    #[test]
    fn leaving_focus_mode_keeps_a_panel_closed_that_was_closed_before() {
        let mut widget = SidebarWidget::new();

        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);
        let _leave = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let vm = widget.vm();
        assert!(vm.is_left_open);
        assert!(!vm.is_right_open);
    }

    #[test]
    fn toggle_left_in_focus_mode_leaves_focus_mode_and_shows_both_panels() {
        let mut widget = SidebarWidget::new();
        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let events = reduce(&mut widget, SidebarIntent::ToggleLeft);

        let vm = widget.vm();
        assert!(!vm.is_focus_mode);
        assert!(vm.is_left_open);
        assert!(vm.is_right_open);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn toggle_right_in_focus_mode_opens_it_even_if_it_was_closed_before() {
        let mut widget = SidebarWidget::new();
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let events = reduce(&mut widget, SidebarIntent::ToggleRight);

        let vm = widget.vm();
        assert!(!vm.is_focus_mode);
        assert!(vm.is_left_open);
        assert!(vm.is_right_open);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_the_left_edge_sets_its_width_and_resyncs_the_grid() {
        let mut widget = SidebarWidget::new();

        let events = drag_left_to(&mut widget, 0.35);

        assert_eq!(widget.left_ratio(), 0.35);
        assert!(widget.is_resizing());
        assert!(has_effect(&events, is_reset_quick_launch));
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_the_left_edge_too_far_keeps_a_fifth_for_the_main_area() {
        let mut widget = SidebarWidget::new();

        let _events = drag_left_to(&mut widget, 0.95);

        assert_eq!(widget.left_ratio(), 0.8);
    }

    #[test]
    fn dragging_the_left_edge_too_far_leaves_room_for_the_open_right_panel() {
        let mut widget = SidebarWidget::new();
        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        let _events = drag_left_to(&mut widget, 0.95);

        assert_near(widget.left_ratio(), 0.6);
        assert_eq!(widget.right_ratio(), 0.2);
    }

    #[test]
    fn dragging_the_left_edge_below_the_collapse_width_closes_the_panel() {
        // The collapse width is 0.04 of the window width.
        let mut widget = SidebarWidget::new();

        let events = drag_left_to(&mut widget, 0.03);

        assert!(!widget.vm().is_left_open);
        assert_eq!(widget.left_ratio(), 0.0);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_the_left_edge_just_above_the_collapse_width_keeps_it_open() {
        let mut widget = SidebarWidget::new();

        let _events = drag_left_to(&mut widget, 0.05);

        assert!(widget.vm().is_left_open);
        assert_eq!(widget.left_ratio(), 0.05);
    }

    #[test]
    fn dragging_a_closed_left_panel_open_opens_it_at_the_collapse_width() {
        let mut widget = SidebarWidget::new();
        let _close = reduce(&mut widget, SidebarIntent::ToggleLeft);

        let events = drag_left_to(&mut widget, 0.5);

        // 0.2 * 0.2 in `f32` is not exactly 0.04.
        assert!(widget.vm().is_left_open);
        assert_near(widget.left_ratio(), 0.04);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_a_closed_left_panel_a_little_does_not_resync_the_grid() {
        let mut widget = SidebarWidget::new();
        let _close = reduce(&mut widget, SidebarIntent::ToggleLeft);

        let events = drag_left_to(&mut widget, 0.02);

        assert!(!widget.vm().is_left_open);
        assert!(has_effect(&events, is_reset_quick_launch));
        assert!(!has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_the_left_edge_keeps_the_right_panel_width() {
        let mut widget = SidebarWidget::new();
        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        let _events = drag_left_to(&mut widget, 0.3);

        // Main takes 0.5 of the window, that is 0.5 / 0.7 of the area right
        // of the left panel.
        assert_eq!(widget.right_ratio(), 0.2);
        assert_near(right_split_ratio(&widget), 0.5 / 0.7);
    }

    #[test]
    fn dragging_the_right_edge_sets_its_width_and_resyncs_the_grid() {
        let mut widget = SidebarWidget::new();
        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        // The middle of the area right of the left panel (0.8 wide).
        let events = drag_right_split_to(&mut widget, 0.5);

        assert_near(widget.right_ratio(), 0.4);
        assert_eq!(widget.left_ratio(), 0.2);
        assert!(widget.is_resizing());
        assert!(has_effect(&events, is_reset_quick_launch));
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_the_right_edge_too_far_keeps_a_fifth_for_the_main_area() {
        let mut widget = SidebarWidget::new();
        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        let _events = drag_right_split_to(&mut widget, 0.0);

        assert_near(widget.right_ratio(), 0.6);
    }

    #[test]
    fn dragging_the_right_edge_below_the_collapse_width_closes_the_panel() {
        let mut widget = SidebarWidget::new();
        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        // 0.01 of the 0.8 wide area is 0.008 of the window.
        let events = drag_right_split_to(&mut widget, 0.99);

        assert!(!widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), 0.0);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_a_closed_right_panel_open_opens_it_at_the_collapse_width() {
        let mut widget = SidebarWidget::new();

        let events = drag_right_split_to(&mut widget, 0.5);

        assert!(widget.vm().is_right_open);
        assert_near(widget.right_ratio(), 0.04);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_a_closed_right_panel_a_little_does_not_resync_the_grid() {
        let mut widget = SidebarWidget::new();

        let events = drag_right_split_to(&mut widget, 0.99);

        assert!(!widget.vm().is_right_open);
        assert!(has_effect(&events, is_reset_quick_launch));
        assert!(!has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_in_focus_mode_changes_nothing() {
        let mut widget = SidebarWidget::new();
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let events = drag_left_to(&mut widget, 0.5);

        assert!(widget.vm().is_focus_mode);
        assert_eq!(widget.side_panels_ratio(), 0.0);
        assert!(!has_effect(&events, is_sync_grid));
    }

    #[test]
    fn open_settings_opens_the_settings_tab_without_resizing() {
        let mut widget = SidebarWidget::new();

        let events = reduce(&mut widget, SidebarIntent::OpenSettings);

        assert!(matches!(
            events.as_slice(),
            [SidebarEvent::Effect(SidebarEffect::OpenSettingsTab)]
        ));
        assert_eq!(widget.side_panels_ratio(), 0.2);
    }

    #[test]
    fn launches_section_starts_open_and_toggles_without_effects() {
        let mut widget = SidebarWidget::new();
        assert!(widget.vm().is_launches_open);

        let close = reduce(&mut widget, SidebarIntent::ToggleLaunches);

        assert!(!widget.vm().is_launches_open);
        assert!(close.is_empty());

        let open = reduce(&mut widget, SidebarIntent::ToggleLaunches);

        assert!(widget.vm().is_launches_open);
        assert!(open.is_empty());
    }

    #[test]
    fn add_menu_opens_and_dismisses() {
        let mut widget = SidebarWidget::new();

        let _open = reduce(&mut widget, SidebarIntent::AddMenuOpen);
        assert!(widget.has_add_menu_open());

        let _dismiss = reduce(&mut widget, SidebarIntent::AddMenuDismiss);
        assert!(!widget.has_add_menu_open());
    }

    #[test]
    fn cursor_move_updates_the_cursor_snapshot() {
        let mut widget = SidebarWidget::new();
        let expected = Point::new(42.0, 24.0);

        let _events = reduce(
            &mut widget,
            SidebarIntent::PaneGridCursorMoved { position: expected },
        );

        assert_eq!(widget.cursor(), expected);
    }
}
