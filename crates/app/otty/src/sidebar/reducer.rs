#![forbid(clippy::disallowed_methods)]

use super::event::{SidebarIntent, SidebarOutcome};
use super::state::SidebarState;

/// Reduce a sidebar intent into state changes and what the features around
/// the panels need to know.
///
/// Every change that resizes the main area reports
/// [`SidebarOutcome::MainAreaResized`], so the terminals redraw at their new
/// size.
pub(super) fn reduce(
    state: &mut SidebarState,
    intent: SidebarIntent,
) -> Vec<SidebarOutcome> {
    match intent {
        SidebarIntent::ToggleLeft => {
            state.toggle_left();
            vec![SidebarOutcome::MainAreaResized]
        },
        SidebarIntent::ToggleRight => {
            state.toggle_right();
            vec![SidebarOutcome::MainAreaResized]
        },
        SidebarIntent::ToggleFocusMode => {
            state.toggle_focus_mode();
            vec![SidebarOutcome::MainAreaResized]
        },
        SidebarIntent::ToggleSessions => {
            state.toggle_sessions();
            Vec::new()
        },
        SidebarIntent::ToggleLaunches => {
            state.toggle_launches();
            Vec::new()
        },
        SidebarIntent::RevealLaunches => {
            main_area_resized(state.reveal_launches())
        },
        SidebarIntent::RevealRight => main_area_resized(state.reveal_right()),
        // The menu takes the keyboard, so typed keys do not reach the shell
        // under it and Esc reaches the frame.
        SidebarIntent::HeaderMenuOpen(menu) => {
            state.open_header_menu(menu);
            vec![SidebarOutcome::HeaderMenuOpened]
        },
        SidebarIntent::HeaderMenuDismiss => {
            if !state.dismiss_header_menu() {
                return Vec::new();
            }
            vec![SidebarOutcome::HeaderMenuClosed]
        },
        SidebarIntent::WorkspaceCursorMoved { position } => {
            state.update_cursor(position);
            Vec::new()
        },
        SidebarIntent::Resized(event) => {
            state.mark_resizing();

            let mut outcomes = vec![SidebarOutcome::PanelEdgeDragged];
            if state.apply_resize(event) {
                outcomes.push(SidebarOutcome::MainAreaResized);
            }

            outcomes
        },
        SidebarIntent::FocusNoteFrame(now) => {
            state.advance_focus_note(now);
            Vec::new()
        },
    }
}

/// Report a new size of the main area when `is_resized`.
fn main_area_resized(is_resized: bool) -> Vec<SidebarOutcome> {
    if !is_resized {
        return Vec::new();
    }

    vec![SidebarOutcome::MainAreaResized]
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced::time::{Duration, Instant};
    use iced::widget::pane_grid;
    use otty_app_slots::Component;

    use crate::sidebar::SidebarWidget;
    use crate::sidebar::event::{SidebarIntent, SidebarOutcome};
    use crate::sidebar::side_panel::SIDE_PANEL_MIN_RATIO;
    use crate::sidebar::types::{HeaderMenu, OpenHeaderMenu};

    /// Ratios computed from other ratios are compared with this tolerance.
    const TOLERANCE: f32 = 1e-6;

    fn reduce(
        widget: &mut SidebarWidget,
        intent: SidebarIntent,
    ) -> Vec<SidebarOutcome> {
        widget.reduce(intent, ())
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
    ) -> Vec<SidebarOutcome> {
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
    ) -> Vec<SidebarOutcome> {
        let (_, split) = splits(widget);

        reduce(
            widget,
            SidebarIntent::Resized(pane_grid::ResizeEvent { split, ratio }),
        )
    }

    fn resizes_main(outcomes: &[SidebarOutcome]) -> bool {
        outcomes.contains(&SidebarOutcome::MainAreaResized)
    }

    fn drags_an_edge(outcomes: &[SidebarOutcome]) -> bool {
        outcomes.contains(&SidebarOutcome::PanelEdgeDragged)
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
    fn toggle_left_hides_the_panel_and_shows_it_again_at_the_width_it_had() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.3);

        let hide = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert!(!widget.vm().is_left_open);
        assert_eq!(widget.left_ratio(), 0.0);
        assert!(resizes_main(&hide));

        let show = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert!(widget.vm().is_left_open);
        assert_eq!(widget.left_ratio(), 0.3);
        assert!(resizes_main(&show));
    }

    #[test]
    fn toggle_right_hides_the_panel_and_shows_it_again_at_the_width_it_had() {
        let mut widget = SidebarWidget::new();
        let _wider_right = drag_right_split_to(&mut widget, 0.5);
        let wide = widget.right_ratio();

        let hide = reduce(&mut widget, SidebarIntent::ToggleRight);

        assert!(!widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), 0.0);
        assert_eq!(widget.side_panels_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(resizes_main(&hide));

        let show = reduce(&mut widget, SidebarIntent::ToggleRight);

        assert!(widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), wide);
        assert_near(right_split_ratio(&widget), 0.5);
        assert!(resizes_main(&show));
    }

    #[test]
    fn toggling_one_panel_leaves_the_width_of_the_other_alone() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.3);
        let _wider_right = drag_right_split_to(&mut widget, 0.5);
        let wide_right = widget.right_ratio();

        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        assert_eq!(widget.right_ratio(), wide_right);
        let _open_left = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert_eq!(widget.left_ratio(), 0.3);
        assert_eq!(widget.right_ratio(), wide_right);

        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        assert_eq!(widget.left_ratio(), 0.3);
        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        assert_eq!(widget.left_ratio(), 0.3);
        assert_eq!(widget.right_ratio(), wide_right);
    }

    #[test]
    fn toggle_shows_a_never_resized_panel_at_the_starting_width() {
        let mut widget = SidebarWidget::new();

        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _open_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        assert_eq!(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    /// Showing the other panel while this one is closed must not eat into the
    /// width this one comes back at.
    #[test]
    fn a_closed_panel_keeps_its_width_while_the_other_one_opens() {
        let mut widget = SidebarWidget::new();
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _wide_left = drag_left_to(&mut widget, 0.7);
        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);

        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _open_left = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert_eq!(widget.left_ratio(), 0.7);
    }

    /// The panel that opens keeps its width and the other one gives way, but
    /// neither goes below the minimum width.
    #[test]
    fn toggle_shows_a_wide_panel_beside_a_wide_panel_with_both_usable() {
        let mut widget = SidebarWidget::new();
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _wide_left = drag_left_to(&mut widget, 0.7);
        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _widest_right = drag_right_split_to(&mut widget, 0.0);
        assert_near(widget.right_ratio(), 0.8);

        let _open_left = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert_near(widget.left_ratio(), 0.8 - SIDE_PANEL_MIN_RATIO);
        assert_near(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn toggle_shows_a_panel_closed_by_a_quick_drag_at_its_last_width() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.5);

        let _shut = drag_left_to(&mut widget, 0.01);
        assert!(!widget.vm().is_left_open);
        let _open = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert_eq!(widget.left_ratio(), 0.5);
    }

    #[test]
    fn toggle_shows_a_panel_closed_by_a_slow_drag_at_the_minimum_width() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.5);
        let _narrow = drag_left_to(&mut widget, 0.1);

        let _shut = drag_left_to(&mut widget, 0.01);
        let _open = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert_eq!(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn toggle_shows_a_never_resized_panel_dragged_shut_at_the_minimum_width() {
        let mut widget = SidebarWidget::new();

        let _shut = drag_right_split_to(&mut widget, 0.99);
        assert!(!widget.vm().is_right_open);
        let _open = reduce(&mut widget, SidebarIntent::ToggleRight);

        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn dragging_a_closed_panel_a_little_keeps_the_width_it_comes_back_at() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.3);
        let _close = reduce(&mut widget, SidebarIntent::ToggleLeft);

        let _little = drag_left_to(&mut widget, 0.02);
        let _open = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert_eq!(widget.left_ratio(), 0.3);
    }

    /// A drag sets a new width, so the width from before is gone. The panel
    /// does not jump away from the pointer to the old width.
    #[test]
    fn dragging_a_closed_panel_open_replaces_the_width_it_comes_back_at() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.5);
        let _close = reduce(&mut widget, SidebarIntent::ToggleLeft);

        let _opened = drag_left_to(&mut widget, 0.3);
        assert_eq!(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
        let _followed = drag_left_to(&mut widget, 0.3);
        let _close = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _open = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert_eq!(widget.left_ratio(), 0.3);
    }

    #[test]
    fn dragging_a_closed_panel_open_keeps_the_width_of_the_other_closed_panel()
    {
        let mut widget = SidebarWidget::new();
        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _wide_right = drag_right_split_to(&mut widget, 0.3);
        assert_near(widget.right_ratio(), 0.7);
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        let _left_dragged_open = drag_left_to(&mut widget, 0.3);
        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _open_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        assert_near(widget.right_ratio(), 0.7);
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
        assert!(resizes_main(&enter));

        let leave = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let vm = widget.vm();
        assert!(!vm.is_focus_mode);
        assert!(vm.is_left_open);
        assert!(vm.is_right_open);
        assert_eq!(widget.left_ratio(), 0.3);
        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(resizes_main(&leave));
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
        assert!(resizes_main(&events));
    }

    #[test]
    fn toggle_in_focus_mode_brings_both_panels_back_at_their_widths() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.3);
        let _wider_right = drag_right_split_to(&mut widget, 0.5);
        let wide_right = widget.right_ratio();
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let _events = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert_eq!(widget.left_ratio(), 0.3);
        assert_eq!(widget.right_ratio(), wide_right);
    }

    #[test]
    fn toggle_in_focus_mode_brings_a_panel_closed_before_back_at_its_width() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.3);
        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let _events = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert!(!widget.vm().is_focus_mode);
        assert_eq!(widget.left_ratio(), 0.3);
    }

    #[test]
    fn panel_closed_before_focus_mode_keeps_its_width_after_it() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.3);
        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);

        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);
        let _leave = reduce(&mut widget, SidebarIntent::ToggleFocusMode);
        assert!(!widget.vm().is_left_open);
        let _open = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert_eq!(widget.left_ratio(), 0.3);
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
        assert!(resizes_main(&events));
    }

    #[test]
    fn dragging_the_left_edge_sets_its_width_and_resyncs_the_grid() {
        let mut widget = SidebarWidget::new();

        let events = drag_left_to(&mut widget, 0.35);

        assert_eq!(widget.left_ratio(), 0.35);
        assert!(widget.is_resizing());
        assert!(drags_an_edge(&events));
        assert!(resizes_main(&events));
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
        assert!(resizes_main(&events));
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
        assert!(resizes_main(&events));
    }

    #[test]
    fn dragging_a_closed_left_panel_a_little_does_not_resync_the_grid() {
        let mut widget = SidebarWidget::new();
        let _close = reduce(&mut widget, SidebarIntent::ToggleLeft);

        let events = drag_left_to(&mut widget, 0.02);

        assert!(!widget.vm().is_left_open);
        assert!(drags_an_edge(&events));
        assert!(!resizes_main(&events));
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
        assert!(drags_an_edge(&events));
        assert!(resizes_main(&events));
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
        assert!(resizes_main(&events));
    }

    #[test]
    fn dragging_a_closed_right_panel_open_opens_it_at_the_minimum_width() {
        let mut widget = SidebarWidget::new();
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        let events = drag_right_split_to(&mut widget, 0.5);

        assert!(widget.vm().is_right_open);
        assert_near(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(resizes_main(&events));
    }

    #[test]
    fn dragging_a_closed_right_panel_a_little_does_not_resync_the_grid() {
        let mut widget = SidebarWidget::new();
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);

        let events = drag_right_split_to(&mut widget, 0.99);

        assert!(!widget.vm().is_right_open);
        assert!(drags_an_edge(&events));
        assert!(!resizes_main(&events));
    }

    #[test]
    fn dragging_in_focus_mode_changes_nothing() {
        let mut widget = SidebarWidget::new();
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let events = drag_left_to(&mut widget, 0.5);

        assert!(widget.vm().is_focus_mode);
        assert_eq!(widget.side_panels_ratio(), 0.0);
        assert!(!resizes_main(&events));
    }

    #[test]
    fn entering_focus_mode_shows_the_note_until_a_frame_after_it_fades() {
        let first_frame = Instant::now();
        let mut widget = SidebarWidget::new();
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);
        let _first =
            reduce(&mut widget, SidebarIntent::FocusNoteFrame(first_frame));

        assert_eq!(widget.vm().focus_note_opacity, Some(1.0));
        assert!(widget.is_focus_note_shown());

        let events = reduce(
            &mut widget,
            SidebarIntent::FocusNoteFrame(
                first_frame + Duration::from_secs(10),
            ),
        );

        assert!(events.is_empty());
        assert!(widget.vm().is_focus_mode);
        assert_eq!(widget.vm().focus_note_opacity, None);
        assert!(!widget.is_focus_note_shown());
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
    fn reveal_launches_shows_a_closed_left_panel_at_the_minimum_width() {
        let mut widget = SidebarWidget::new();
        let _close = reduce(&mut widget, SidebarIntent::ToggleLeft);

        let events = reduce(&mut widget, SidebarIntent::RevealLaunches);

        let vm = widget.vm();
        assert!(vm.is_left_open);
        assert!(vm.is_right_open);
        assert!(vm.is_launches_open);
        assert_eq!(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(resizes_main(&events));
    }

    #[test]
    fn reveal_launches_brings_a_wider_closed_left_panel_back_at_its_width() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.3);
        let _close = reduce(&mut widget, SidebarIntent::ToggleLeft);

        let _events = reduce(&mut widget, SidebarIntent::RevealLaunches);

        assert!(widget.vm().is_left_open);
        assert_eq!(widget.left_ratio(), 0.3);
    }

    #[test]
    fn reveal_launches_in_focus_mode_brings_a_wider_closed_left_panel_back_at_its_width()
     {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.3);
        let _close = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let _events = reduce(&mut widget, SidebarIntent::RevealLaunches);

        assert!(!widget.vm().is_focus_mode);
        assert_eq!(widget.left_ratio(), 0.3);
    }

    #[test]
    fn reveal_launches_leaves_focus_mode_and_restores_the_panels() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.3);
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let events = reduce(&mut widget, SidebarIntent::RevealLaunches);

        let vm = widget.vm();
        assert!(!vm.is_focus_mode);
        assert_eq!(vm.focus_note_opacity, None);
        assert!(vm.is_left_open);
        assert!(!vm.is_right_open);
        assert!(vm.is_launches_open);
        assert_eq!(widget.left_ratio(), 0.3);
        assert!(resizes_main(&events));
    }

    #[test]
    fn reveal_launches_in_focus_mode_opens_a_left_panel_closed_before() {
        let mut widget = SidebarWidget::new();
        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let events = reduce(&mut widget, SidebarIntent::RevealLaunches);

        let vm = widget.vm();
        assert!(!vm.is_focus_mode);
        assert!(vm.is_left_open);
        assert_eq!(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(resizes_main(&events));
    }

    #[test]
    fn reveal_launches_opens_a_collapsed_section_without_resizing() {
        let mut widget = SidebarWidget::new();
        let _collapse = reduce(&mut widget, SidebarIntent::ToggleLaunches);

        let events = reduce(&mut widget, SidebarIntent::RevealLaunches);

        assert!(widget.vm().is_launches_open);
        assert!(events.is_empty());
        assert_near(widget.side_panels_ratio(), 2.0 * SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn reveal_launches_when_already_shown_changes_nothing() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.3);

        let events = reduce(&mut widget, SidebarIntent::RevealLaunches);

        let vm = widget.vm();
        assert!(vm.is_left_open);
        assert!(vm.is_right_open);
        assert!(vm.is_launches_open);
        assert!(vm.is_sessions_open);
        assert_eq!(widget.left_ratio(), 0.3);
        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(events.is_empty());
    }

    #[test]
    fn reveal_right_opens_a_closed_panel_at_the_minimum_width() {
        let mut widget = SidebarWidget::new();
        let _close = reduce(&mut widget, SidebarIntent::ToggleRight);

        let events = reduce(&mut widget, SidebarIntent::RevealRight);

        assert!(widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(resizes_main(&events));
    }

    #[test]
    fn reveal_right_keeps_an_open_panel_and_its_width() {
        let mut widget = SidebarWidget::new();
        let _wider_right = drag_right_split_to(&mut widget, 0.5);
        let wide = widget.right_ratio();

        let events = reduce(&mut widget, SidebarIntent::RevealRight);

        assert!(widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), wide);
        assert!(events.is_empty());
    }

    #[test]
    fn reveal_right_twice_leaves_the_panel_open() {
        let mut widget = SidebarWidget::new();
        let _close = reduce(&mut widget, SidebarIntent::ToggleRight);

        let _first = reduce(&mut widget, SidebarIntent::RevealRight);
        let second = reduce(&mut widget, SidebarIntent::RevealRight);

        assert!(widget.vm().is_right_open);
        assert!(second.is_empty());
    }

    #[test]
    fn reveal_right_brings_a_wider_closed_panel_back_at_its_width() {
        let mut widget = SidebarWidget::new();
        let _wider_right = drag_right_split_to(&mut widget, 0.5);
        let wide = widget.right_ratio();
        let _close = reduce(&mut widget, SidebarIntent::ToggleRight);

        let events = reduce(&mut widget, SidebarIntent::RevealRight);

        assert!(widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), wide);
        assert_near(right_split_ratio(&widget), 0.5);
        assert!(resizes_main(&events));
    }

    #[test]
    fn reveal_right_in_focus_mode_brings_a_wider_closed_panel_back_at_its_width()
     {
        let mut widget = SidebarWidget::new();
        let _wider_right = drag_right_split_to(&mut widget, 0.5);
        let wide = widget.right_ratio();
        let _close = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let _events = reduce(&mut widget, SidebarIntent::RevealRight);

        assert!(!widget.vm().is_focus_mode);
        assert!(widget.vm().is_right_open);
        assert_eq!(widget.right_ratio(), wide);
    }

    #[test]
    fn reveal_right_gives_a_never_widened_panel_the_minimum_width() {
        let mut through_focus = SidebarWidget::new();
        let _enter = reduce(&mut through_focus, SidebarIntent::ToggleFocusMode);
        let mut dragged_shut = SidebarWidget::new();
        let _shut = drag_right_split_to(&mut dragged_shut, 0.99);
        assert!(!dragged_shut.vm().is_right_open);

        let _events = reduce(&mut through_focus, SidebarIntent::RevealRight);
        let _events = reduce(&mut dragged_shut, SidebarIntent::RevealRight);

        assert_eq!(through_focus.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert_eq!(dragged_shut.right_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn reveal_right_at_its_width_beside_a_widest_left_panel_shrinks_the_left() {
        let mut widget = SidebarWidget::new();
        let _wider_right = drag_right_split_to(&mut widget, 0.5);
        let wide = widget.right_ratio();
        let _close = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _widest_left = drag_left_to(&mut widget, 0.95);

        let _events = reduce(&mut widget, SidebarIntent::RevealRight);

        assert_eq!(widget.right_ratio(), wide);
        assert_near(widget.left_ratio(), 0.8 - wide);
        assert!(widget.left_ratio() >= SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn left_panel_opened_after_revealing_a_widest_right_panel_takes_the_minimum_width()
     {
        let mut widget = SidebarWidget::new();
        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _widest_right = drag_right_split_to(&mut widget, 0.0);
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _reveal = reduce(&mut widget, SidebarIntent::RevealRight);
        assert_near(widget.right_ratio(), 0.8);

        let _open_left = reduce(&mut widget, SidebarIntent::ToggleLeft);

        assert_eq!(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
        assert_near(widget.right_ratio(), 0.8 - SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn toggle_right_shows_a_wider_panel_at_its_width() {
        let mut closed = SidebarWidget::new();
        let _wider_right = drag_right_split_to(&mut closed, 0.5);
        let wide = closed.right_ratio();
        let _close = reduce(&mut closed, SidebarIntent::ToggleRight);
        let mut in_focus = SidebarWidget::new();
        let _wider_right = drag_right_split_to(&mut in_focus, 0.5);
        let _enter = reduce(&mut in_focus, SidebarIntent::ToggleFocusMode);

        let _events = reduce(&mut closed, SidebarIntent::ToggleRight);
        let _events = reduce(&mut in_focus, SidebarIntent::ToggleRight);

        assert_eq!(closed.right_ratio(), wide);
        assert_eq!(in_focus.right_ratio(), wide);
    }

    /// Reveal and toggle share the rule: both panels stay usable.
    #[test]
    fn reveal_right_at_a_width_beside_an_open_left_panel_leaves_it_the_minimum()
    {
        let mut widget = SidebarWidget::new();
        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _widest_right = drag_right_split_to(&mut widget, 0.0);
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _open_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _wider_left = drag_left_to(&mut widget, 0.3);

        let _events = reduce(&mut widget, SidebarIntent::RevealRight);

        assert_near(widget.right_ratio(), 0.8 - SIDE_PANEL_MIN_RATIO);
        assert_near(widget.left_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    /// Both panels come back as they were before focus mode.
    #[test]
    fn reveal_right_leaves_focus_mode_with_the_panel_visible() {
        let mut widget = SidebarWidget::new();
        let _wider_left = drag_left_to(&mut widget, 0.3);
        let _wider_right = drag_right_split_to(&mut widget, 0.5);
        let wide = widget.right_ratio();
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let events = reduce(&mut widget, SidebarIntent::RevealRight);

        let vm = widget.vm();
        assert!(!vm.is_focus_mode);
        assert_eq!(vm.focus_note_opacity, None);
        assert!(vm.is_right_open);
        assert!(vm.is_left_open);
        assert_eq!(widget.right_ratio(), wide);
        assert_eq!(widget.left_ratio(), 0.3);
        assert!(resizes_main(&events));
    }

    #[test]
    fn reveal_right_in_focus_mode_opens_a_panel_closed_before_and_keeps_the_left_closed()
     {
        let mut widget = SidebarWidget::new();
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _close_left = reduce(&mut widget, SidebarIntent::ToggleLeft);
        let _enter = reduce(&mut widget, SidebarIntent::ToggleFocusMode);

        let events = reduce(&mut widget, SidebarIntent::RevealRight);

        let vm = widget.vm();
        assert!(!vm.is_focus_mode);
        assert!(vm.is_right_open);
        assert!(!vm.is_left_open);
        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!(resizes_main(&events));
    }

    #[test]
    fn reveal_right_beside_a_widest_left_panel_keeps_a_fifth_for_the_main_area()
    {
        let mut widget = SidebarWidget::new();
        let _close_right = reduce(&mut widget, SidebarIntent::ToggleRight);
        let _widest_left = drag_left_to(&mut widget, 0.95);

        let _events = reduce(&mut widget, SidebarIntent::RevealRight);

        assert_eq!(widget.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert_near(widget.left_ratio(), 0.8 - SIDE_PANEL_MIN_RATIO);
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
            widget.vm().open_header_menu,
            Some(OpenHeaderMenu::new(HeaderMenu::Sessions, cursor))
        );
        assert_eq!(open, vec![SidebarOutcome::HeaderMenuOpened]);

        let _dismiss = reduce(&mut widget, SidebarIntent::HeaderMenuDismiss);
        assert_eq!(widget.vm().open_header_menu, None);
    }

    /// The header menu takes the keyboard from the focused terminal, so
    /// typed keys do not reach its shell and Esc reaches the app.
    #[test]
    fn opening_a_header_menu_takes_the_keyboard_focus() {
        let mut widget = SidebarWidget::new();

        let open = reduce(
            &mut widget,
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
        );

        assert_eq!(open, vec![SidebarOutcome::HeaderMenuOpened]);
    }

    #[test]
    fn closing_an_open_header_menu_reports_it() {
        let mut widget = SidebarWidget::new();
        let _open = reduce(
            &mut widget,
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
        );

        let dismiss = reduce(&mut widget, SidebarIntent::HeaderMenuDismiss);

        assert!(!widget.has_header_menu_open());
        assert_eq!(dismiss, vec![SidebarOutcome::HeaderMenuClosed]);
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
            Some(OpenHeaderMenu::new(HeaderMenu::Launches, cursor))
        );
        assert_eq!(open, vec![SidebarOutcome::HeaderMenuOpened]);
    }

    #[test]
    fn opening_one_header_menu_closes_the_other() {
        let mut widget = SidebarWidget::new();
        let _sessions = reduce(
            &mut widget,
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
        );

        let launches = reduce(
            &mut widget,
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Launches),
        );

        let open = widget.vm().open_header_menu.expect("a menu should be open");
        assert_eq!(open.menu(), HeaderMenu::Launches);
        assert_eq!(launches, vec![SidebarOutcome::HeaderMenuOpened]);
    }

    #[test]
    fn closing_a_closed_header_menu_reports_nothing() {
        let mut widget = SidebarWidget::new();

        let dismiss = reduce(&mut widget, SidebarIntent::HeaderMenuDismiss);

        assert_eq!(widget.vm().open_header_menu, None);
        assert!(dismiss.is_empty());
    }

    #[test]
    fn cursor_move_updates_the_cursor_snapshot() {
        let mut widget = SidebarWidget::new();
        let expected = Point::new(42.0, 24.0);

        let _events = reduce(
            &mut widget,
            SidebarIntent::WorkspaceCursorMoved { position: expected },
        );

        assert_eq!(widget.cursor(), expected);
    }
}
