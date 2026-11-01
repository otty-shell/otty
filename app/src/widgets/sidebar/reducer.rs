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
        SidebarIntent::ToggleSessions => {
            state.toggle_sessions();
            Task::none()
        },
        SidebarIntent::ToggleLaunches => {
            state.toggle_launches();
            Task::none()
        },
        SidebarIntent::OpenSettings => {
            Task::done(SidebarEvent::Effect(SidebarEffect::OpenSettingsTab))
        },
        SidebarIntent::HeaderMenuOpen(menu) => {
            state.open_header_menu(menu);
            Task::none()
        },
        SidebarIntent::HeaderMenuDismiss => {
            state.dismiss_header_menu();
            Task::none()
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
        SidebarIntent::FocusNoteFrame(now) => {
            state.advance_focus_note(now);
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
    use iced::time::{Duration, Instant};
    use iced::widget::pane_grid;

    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::sidebar::side_panel::SIDE_PANEL_MIN_RATIO;
    use crate::widgets::sidebar::types::{HeaderMenu, OpenHeaderMenu};
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
    fn new_sidebar_shows_both_panels_at_the_minimum_width() {
        let widget = SidebarWidget::new();
        let vm = widget.vm();

        assert!(vm.is_left_open);
        assert!(vm.is_right_open);
        assert!(!vm.is_focus_mode);
        assert_eq!(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert_near(widget.side_panels_ratio(), 2.0 * SIDE_PANEL_MIN_RATIO);
        assert!(!widget.is_resizing());
    }

    #[test]
    fn toggle_left_hides_the_panel_and_shows_it_again_at_the_minimum_width() {
        let mut widget = SidebarWidget::new();

        let hide = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert!(!widget.vm().is_left_open);
        assert_eq!(widget.left_ratio(), 0.0);
        assert!(has_effect(&hide, is_sync_grid));

        let show = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert!(widget.vm().is_left_open);
        assert_eq!(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(has_effect(&show, is_sync_grid));
    }

    #[test]
    fn toggle_right_hides_the_panel_and_shows_it_again_at_the_minimum_width() {
        let mut widget = SidebarWidget::new();

        let hide = reduce(&mut widget, SidebarIntent::ToggleRight);

        assert!(!widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), 0.0);
        assert_eq!(widget.side_panels_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(has_effect(&hide, is_sync_grid));

        let show = reduce(&mut widget, SidebarIntent::ToggleRight);

        assert!(widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(has_effect(&show, is_sync_grid));
    }

    #[test]
    fn focus_mode_hides_both_panels_and_leaving_it_restores_their_widths() {
        let mut widget = SidebarWidget::new();
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
        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(has_effect(&leave, is_sync_grid));
    }

    #[test]
    fn leaving_focus_mode_keeps_a_panel_closed_that_was_closed_before() {
        let mut widget = SidebarWidget::new();
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);
        let _leave = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let vm = widget.vm();
        assert!(vm.is_left_open);
        assert!(!vm.is_right_open);
    }

    #[test]
    fn toggle_left_in_focus_mode_leaves_focus_mode_and_shows_both_panels() {
        let mut widget = SidebarWidget::new();
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
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);
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
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        let _events = drag_left_to(&mut widget, 0.95);

        assert_eq!(widget.left_ratio(), 0.8);
    }

    #[test]
    fn dragging_the_left_edge_too_far_leaves_room_for_the_open_right_panel() {
        let mut widget = SidebarWidget::new();

        let _events = drag_left_to(&mut widget, 0.95);

        // The main area keeps a fifth of the window.
        assert_near(widget.left_ratio(), 0.8 - SIDE_PANEL_MIN_RATIO);
        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn dragging_the_left_edge_below_the_collapse_width_closes_the_panel() {
        // The collapse width is half of the minimum width.
        let mut widget = SidebarWidget::new();

        let events = drag_left_to(&mut widget, 0.03);

        assert!(!widget.vm().is_left_open);
        assert_eq!(widget.left_ratio(), 0.0);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_the_left_edge_just_above_the_collapse_width_keeps_the_minimum_width()
     {
        let mut widget = SidebarWidget::new();

        let _events = drag_left_to(&mut widget, 0.1);

        assert!(widget.vm().is_left_open);
        assert_eq!(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn dragging_a_closed_left_panel_open_opens_it_at_the_minimum_width() {
        let mut widget = SidebarWidget::new();
        let _close = reduce(&mut widget, SidebarIntent::ToggleLeft);

        let events = drag_left_to(&mut widget, 0.5);

        assert!(widget.vm().is_left_open);
        assert_near(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
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

        let _events = drag_left_to(&mut widget, 0.3);

        // Main takes what the panels leave, as a share of the 0.7 wide area
        // right of the left panel.
        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert_near(
            right_split_ratio(&widget),
            (0.7 - SIDE_PANEL_MIN_RATIO) / 0.7,
        );
    }

    #[test]
    fn dragging_the_right_edge_sets_its_width_and_resyncs_the_grid() {
        let mut widget = SidebarWidget::new();

        // The middle of the area right of the left panel.
        let events = drag_right_split_to(&mut widget, 0.5);

        assert_near(widget.right_ratio(), (1.0 - SIDE_PANEL_MIN_RATIO) / 2.0);
        assert_eq!(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(widget.is_resizing());
        assert!(has_effect(&events, is_reset_quick_launch));
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_the_right_edge_too_far_keeps_a_fifth_for_the_main_area() {
        let mut widget = SidebarWidget::new();

        let _events = drag_right_split_to(&mut widget, 0.0);

        assert_near(widget.right_ratio(), 0.8 - SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn dragging_the_right_edge_below_the_collapse_width_closes_the_panel() {
        let mut widget = SidebarWidget::new();

        // 0.01 of the area right of the left panel is under 0.01 of the
        // window.
        let events = drag_right_split_to(&mut widget, 0.99);

        assert!(!widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), 0.0);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_a_closed_right_panel_open_opens_it_at_the_minimum_width() {
        let mut widget = SidebarWidget::new();
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        let events = drag_right_split_to(&mut widget, 0.5);

        assert!(widget.vm().is_right_open);
        assert_near(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(has_effect(&events, is_sync_grid));
    }

    #[test]
    fn dragging_a_closed_right_panel_a_little_does_not_resync_the_grid() {
        let mut widget = SidebarWidget::new();
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);

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
    fn entering_focus_mode_shows_the_note_until_a_frame_after_it_fades() {
        let mut widget = SidebarWidget::new();
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        assert_eq!(widget.vm().focus_note_opacity, Some(1.0));
        assert!(widget.is_focus_note_shown());

        let events = reduce(
            &mut widget,
            SidebarIntent::FocusNoteFrame(
                Instant::now() + Duration::from_secs(10),
            ),
        );

        assert!(events.is_empty());
        assert!(widget.vm().is_focus_mode);
        assert_eq!(widget.vm().focus_note_opacity, None);
        assert!(!widget.is_focus_note_shown());
    }

    #[test]
    fn open_settings_opens_the_settings_tab_without_resizing() {
        let mut widget = SidebarWidget::new();

        let events = reduce(&mut widget, SidebarIntent::OpenSettings);

        assert!(matches!(
            events.as_slice(),
            [SidebarEvent::Effect(SidebarEffect::OpenSettingsTab)]
        ));
        assert_near(widget.side_panels_ratio(), 2.0 * SIDE_PANEL_MIN_RATIO);
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
    fn sessions_section_starts_open_and_toggles_without_effects() {
        let mut widget = SidebarWidget::new();
        assert!(widget.vm().is_sessions_open);

        let close = reduce(&mut widget, SidebarIntent::ToggleSessions);

        assert!(!widget.vm().is_sessions_open);
        assert!(widget.vm().is_launches_open);
        assert!(close.is_empty());

        let open = reduce(&mut widget, SidebarIntent::ToggleSessions);

        assert!(widget.vm().is_sessions_open);
        assert!(open.is_empty());
    }

    #[test]
    fn header_menu_opens_at_the_cursor_and_dismisses() {
        let mut widget = SidebarWidget::new();
        let cursor = Point::new(30.0, 40.0);
        let _moved = reduce(
            &mut widget,
            SidebarIntent::WorkspaceCursorMoved { position: cursor },
        );

        let open = reduce(
            &mut widget,
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
        );
        assert_eq!(
            widget.open_header_menu(),
            Some(OpenHeaderMenu {
                menu: HeaderMenu::Sessions,
                cursor,
            })
        );
        assert!(open.is_empty());

        let dismiss = reduce(&mut widget, SidebarIntent::HeaderMenuDismiss);
        assert_eq!(widget.open_header_menu(), None);
        assert!(dismiss.is_empty());
    }

    #[test]
    fn launches_header_menu_opens_at_the_cursor() {
        let mut widget = SidebarWidget::new();
        let cursor = Point::new(12.0, 18.0);
        let _moved = reduce(
            &mut widget,
            SidebarIntent::WorkspaceCursorMoved { position: cursor },
        );

        let open = reduce(
            &mut widget,
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Launches),
        );

        assert_eq!(
            widget.vm().open_header_menu,
            Some(OpenHeaderMenu {
                menu: HeaderMenu::Launches,
                cursor,
            })
        );
        assert!(open.is_empty());
    }

    #[test]
    fn opening_one_header_menu_closes_the_other() {
        let mut widget = SidebarWidget::new();
        let _sessions = reduce(
            &mut widget,
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
        );

        let _launches = reduce(
            &mut widget,
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Launches),
        );

        let open = widget.open_header_menu().expect("a menu should be open");
        assert_eq!(open.menu, HeaderMenu::Launches);
    }

    #[test]
    fn dismissing_a_closed_header_menu_changes_nothing() {
        let mut widget = SidebarWidget::new();

        let dismiss = reduce(&mut widget, SidebarIntent::HeaderMenuDismiss);

        assert_eq!(widget.open_header_menu(), None);
        assert!(dismiss.is_empty());
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
