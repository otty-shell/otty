use iced::Size;
use iced::widget::pane_grid::Node;
use otty_ui_kit::size;

/// Smallest width and height of a terminal pane, about 12 columns by 3
/// rows. A split that would leave a pane smaller is refused, and a drag
/// stops there.
pub(super) const MIN_PANE_SIZE: f32 = 100.0;

/// The grid size the tab knows leaves out the two 1 px lines beside the
/// side panels, so the real grid can be this much narrower.
const LINE_SLACK: f32 = 2.0;

/// Room for float error when a pane is compared with its minimum.
const TOLERANCE: f32 = 0.01;

/// Return whether every pane of `layout` lies inside `grid` and is at least
/// `min_size` wide and tall, as iced lays the panes out.
///
/// A grid with no area has not been measured yet, so nothing is refused.
pub(super) fn fits(layout: &Node, grid: Size, min_size: f32) -> bool {
    if grid.width <= 0.0 || grid.height <= 0.0 {
        return true;
    }

    let grid = Size::new(grid.width - LINE_SLACK, grid.height);
    let regions = layout.pane_regions(size::LINE, min_size, grid);

    regions.values().all(|region| {
        let is_inside = region.x >= 0.0
            && region.y >= 0.0
            && region.x + region.width <= grid.width + TOLERANCE
            && region.y + region.height <= grid.height + TOLERANCE;
        let is_big_enough = region.width >= min_size - TOLERANCE
            && region.height >= min_size - TOLERANCE;

        is_inside && is_big_enough
    })
}

/// Return the largest pane minimum, up to [`MIN_PANE_SIZE`], at which
/// `layout` still fits `grid`, in whole pixels; zero when none does.
///
/// Panes split while the window was larger keep it when the window
/// shrinks: they get smaller than [`MIN_PANE_SIZE`] but stay inside the
/// grid. Panes that fit get the full minimum at the first check.
pub(super) fn fitting_min_size(layout: &Node, grid: Size) -> f32 {
    let mut min_size = MIN_PANE_SIZE;
    while min_size > 0.0 && !fits(layout, grid, min_size) {
        min_size -= 1.0;
    }

    min_size
}

#[cfg(test)]
mod tests {
    use iced::widget::pane_grid::{self, Axis};
    use otty_ui_kit::size;

    use super::*;
    use crate::widgets::terminal_workspace::pane_balance;

    /// The pane grid at 1024×768 with both side panels open.
    const SMALL_GRID: Size = Size::new(696.0, 705.0);

    /// Build `count` panes in one row (`Axis::Vertical`) or one column
    /// (`Axis::Horizontal`), evened out, as repeated splits of the last
    /// pane leave them.
    fn panes_in_a_line(count: u64, axis: Axis) -> pane_grid::State<u64> {
        let (mut panes, mut last) = pane_grid::State::new(0_u64);

        for id in 1..count {
            let (new_pane, _) =
                panes.split(axis, last, id).expect("the last pane splits");
            for (split, ratio) in
                pane_balance::equalized_ratios(panes.layout(), new_pane)
            {
                panes.resize(split, ratio);
            }
            last = new_pane;
        }

        panes
    }

    /// Return whether every pane lies inside `grid`, left and top edges
    /// included.
    fn all_inside(
        panes: &pane_grid::State<u64>,
        min_size: f32,
        grid: Size,
    ) -> bool {
        panes
            .layout()
            .pane_regions(size::LINE, min_size, grid)
            .values()
            .all(|region| {
                region.x >= 0.0
                    && region.y >= 0.0
                    && region.x + region.width <= grid.width
                    && region.y + region.height <= grid.height
            })
    }

    #[test]
    fn one_pane_fits_a_grid_larger_than_the_minimum() {
        let (panes, _) = pane_grid::State::new(0_u64);

        assert!(fits(panes.layout(), SMALL_GRID, MIN_PANE_SIZE));
        assert!(!fits(panes.layout(), Size::new(90.0, 705.0), MIN_PANE_SIZE));
        assert!(!fits(panes.layout(), Size::new(696.0, 90.0), MIN_PANE_SIZE));
    }

    /// Six panes of 100 px and five 1 px lines take 605 px of the 694 px
    /// left after the two lines beside the side panels; seven take 706.
    #[test]
    fn six_panes_fit_across_the_small_grid_and_seven_do_not() {
        let six = panes_in_a_line(6, Axis::Vertical);
        let seven = panes_in_a_line(7, Axis::Vertical);

        assert!(fits(six.layout(), SMALL_GRID, MIN_PANE_SIZE));
        assert!(!fits(seven.layout(), SMALL_GRID, MIN_PANE_SIZE));
    }

    #[test]
    fn six_panes_fit_stacked_in_the_small_grid_and_seven_do_not() {
        let six = panes_in_a_line(6, Axis::Horizontal);
        let seven = panes_in_a_line(7, Axis::Horizontal);

        assert!(fits(six.layout(), SMALL_GRID, MIN_PANE_SIZE));
        assert!(!fits(seven.layout(), SMALL_GRID, MIN_PANE_SIZE));
    }

    /// Two panes need 100 + 1 + 100 px, plus the two lines the grid size
    /// leaves out.
    #[test]
    fn two_panes_fit_exactly_at_their_minimum() {
        let two = panes_in_a_line(2, Axis::Vertical);

        assert!(fits(two.layout(), Size::new(203.0, 300.0), MIN_PANE_SIZE));
        assert!(!fits(two.layout(), Size::new(202.0, 300.0), MIN_PANE_SIZE));
    }

    /// A tab gets its grid size when it opens. Until then there is nothing
    /// to measure against, so nothing is refused.
    #[test]
    fn a_grid_not_measured_yet_fits_anything() {
        let many = panes_in_a_line(20, Axis::Vertical);

        assert!(fits(many.layout(), Size::ZERO, MIN_PANE_SIZE));
    }

    #[test]
    fn panes_that_fit_keep_the_full_minimum() {
        let six = panes_in_a_line(6, Axis::Vertical);

        assert_eq!(fitting_min_size(six.layout(), SMALL_GRID), MIN_PANE_SIZE);
    }

    /// Eight panes fit across a wide window. After the window shrinks they
    /// no longer do: at the full minimum iced pushes the left panes out of
    /// the grid, over the left panel. The smaller minimum keeps them in.
    #[test]
    fn squeezed_panes_get_a_smaller_minimum_that_keeps_them_inside() {
        let eight = panes_in_a_line(8, Axis::Vertical);
        let real_grid = Size::new(SMALL_GRID.width - 2.0, SMALL_GRID.height);

        let min_size = fitting_min_size(eight.layout(), SMALL_GRID);

        assert!(!all_inside(&eight, MIN_PANE_SIZE, real_grid));
        assert!(min_size < MIN_PANE_SIZE, "{min_size}");
        assert!(min_size >= 80.0, "{min_size}");
        assert!(all_inside(&eight, min_size, real_grid));
        assert!(fits(eight.layout(), SMALL_GRID, min_size));
    }

    #[test]
    fn squeezed_stacked_panes_stay_inside_too() {
        let eight = panes_in_a_line(8, Axis::Horizontal);

        let min_size = fitting_min_size(eight.layout(), SMALL_GRID);

        assert!(!all_inside(&eight, MIN_PANE_SIZE, SMALL_GRID));
        assert!(min_size < MIN_PANE_SIZE, "{min_size}");
        assert!(all_inside(&eight, min_size, SMALL_GRID));
    }

    /// So many panes that even no minimum lets them fit: the panes follow
    /// their ratios and still stay inside the grid.
    #[test]
    fn far_too_many_panes_get_no_minimum() {
        let many = panes_in_a_line(400, Axis::Vertical);

        let min_size = fitting_min_size(many.layout(), SMALL_GRID);

        assert_eq!(min_size, 0.0);
    }
}
