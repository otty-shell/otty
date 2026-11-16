use iced::event::Status;
use iced::{Rectangle, mouse};
use otty_libterm::surface::{SnapshotSize, SnapshotView};

use crate::view::TerminalViewState;

/// Width of the scroll mark, in pixels.
const SCROLL_MARK_WIDTH: f32 = 4.0;
/// Width of the scroll mark while the pointer is on its strip or drags it,
/// in pixels.
const ACTIVE_SCROLL_MARK_WIDTH: f32 = 8.0;
/// The shortest the scroll mark gets, in pixels, so that it stays easy to
/// see in a very long history.
const SCROLL_MARK_MIN_HEIGHT: f32 = 48.0;

/// The scroll mark of a pane for the history it shows now, and the strip
/// the mark runs in: the right inset of the pane, over its full height.
///
/// The mark is too narrow to hit, so the whole strip takes the pointer: a
/// press there takes the mark and a drag moves the history. The view draws
/// the mark and the pointer handling uses it, so both agree on whether
/// there is one and where it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollMark {
    pane: Rectangle,
    strip: Rectangle,
    mark: Rectangle,
    size: SnapshotSize,
}

impl ScrollMark {
    /// The strip the mark runs in: the right inset of the pane.
    pub(crate) fn strip(&self) -> Rectangle {
        self.strip
    }

    /// Where the mark lies: 4 px wide, or 8 px while `is_active` (the
    /// pointer is on the strip or drags the mark). It keeps to the right
    /// edge of the pane.
    pub(crate) fn rect(&self, is_active: bool) -> Rectangle {
        if !is_active {
            return self.mark;
        }

        let width = ACTIVE_SCROLL_MARK_WIDTH.min(self.pane.width);

        Rectangle {
            x: self.pane.x + self.pane.width - width,
            width,
            ..self.mark
        }
    }

    /// Build the scroll mark of `pane`, whose text fills `text_area`, for
    /// the history `view` shows. `None` when the history fits on screen or
    /// a full-screen program is on: then there is no mark and no strip.
    pub(crate) fn of_view(
        pane: Rectangle,
        text_area: Rectangle,
        view: &SnapshotView<'_>,
    ) -> Option<Self> {
        if view.is_full_screen() {
            return None;
        }

        let mark = scroll_mark_rect(pane, view.size, view.display_offset)?;

        let text_right = text_area.x + text_area.width;
        let strip = Rectangle {
            x: text_right,
            width: pane.x + pane.width - text_right,
            ..pane
        };

        Some(Self {
            pane,
            strip,
            mark,
            size: view.size,
        })
    }

    /// The scroll offset that puts the top edge of the mark nearest to
    /// `mark_y`, in whole lines; see [`scroll_offset_for_mark`].
    pub(crate) fn offset_for_mark_at(&self, mark_y: f32) -> Option<usize> {
        scroll_offset_for_mark(self.pane, self.size, mark_y)
    }
}

/// Go on with the drag of the scroll mark of the terminal `terminal_id`,
/// held `grab` px below its top edge: a move scrolls the history so that
/// the mark follows the pointer, wherever the pointer is, and the release
/// of the left button lets the mark go. `None` for any other event, which
/// the terminal handles as usual.
pub(crate) fn drag_scroll_mark<T>(
    terminal_id: u64,
    state: &mut TerminalViewState,
    scroll_mark: Option<&ScrollMark>,
    grab: f32,
    event: &mouse::Event,
    publisher: &mut impl FnMut(crate::Event<T>),
) -> Option<Status> {
    match event {
        mouse::Event::CursorMoved { position } => {
            state.set_pointer_position(Some(*position));

            if let Some(mark) = scroll_mark {
                scroll_to_mark_at(
                    terminal_id,
                    mark,
                    position.y - grab,
                    publisher,
                );
            }
            Some(Status::Captured)
        },
        mouse::Event::ButtonReleased(mouse::Button::Left) => {
            state.release_scroll_mark();
            publisher(crate::Event::Redraw { id: terminal_id });
            Some(Status::Captured)
        },
        _ => None,
    }
}

/// Take the scroll mark of the terminal `terminal_id` with a press `y` px
/// down the window. A press on the mark holds it where it was pressed. A
/// press on the strip above or below it moves the history at once, so
/// that the mark is centered under the pointer, and holds the mark at its
/// middle.
pub(crate) fn take_scroll_mark<T>(
    terminal_id: u64,
    state: &mut TerminalViewState,
    mark: &ScrollMark,
    y: f32,
    publisher: &mut impl FnMut(crate::Event<T>),
) {
    let rect = mark.rect(true);
    let is_on_mark = y >= rect.y && y < rect.y + rect.height;
    let grab = if is_on_mark {
        y - rect.y
    } else {
        rect.height / 2.0
    };

    if !is_on_mark {
        scroll_to_mark_at(terminal_id, mark, y - grab, publisher);
    }

    state.grab_scroll_mark(grab);
    publisher(crate::Event::Redraw { id: terminal_id });
}

/// Where the scroll mark lies in `pane`: a 4 px bar at its right edge that
/// shows which part of the history is on screen.
///
/// `size` tells how many lines the history has and how many rows are on
/// screen, and `display_offset` how many lines the view is scrolled up from
/// the bottom. The bar is as high as the share of the history on screen,
/// but never under 48 px, and moves evenly from the bottom edge of the pane
/// (at the bottom of the history) to the top edge (at its top). There is
/// none when the whole history fits on screen.
fn scroll_mark_rect(
    pane: Rectangle,
    size: SnapshotSize,
    display_offset: usize,
) -> Option<Rectangle> {
    let lines_above = size.max_display_offset();
    if lines_above == 0 || size.screen_lines == 0 || pane.height <= 0.0 {
        return None;
    }

    let offset = display_offset.min(lines_above);
    let height = mark_height(pane, size);

    // Scrolled all the way up, the bar touches the top edge; at the bottom
    // of the history it touches the bottom edge. Each line of offset moves
    // it by the same step.
    let travel = pane.height - height;
    let lines_below_top = (lines_above - offset) as f32;
    let y = pane.y + travel * lines_below_top / lines_above as f32;

    let width = SCROLL_MARK_WIDTH.min(pane.width);

    Some(Rectangle {
        x: pane.x + pane.width - width,
        y,
        width,
        height,
    })
}

/// The scroll offset whose mark starts nearest to `mark_y`, the place of
/// the top edge of the mark in window coordinates: the reverse of
/// [`scroll_mark_rect`]. A place above the top of the way the mark can go
/// gives the top of the history, one below gives the bottom.
///
/// `None` when there is no mark, and when the mark fills the pane height,
/// so no place of it stands for an offset.
fn scroll_offset_for_mark(
    pane: Rectangle,
    size: SnapshotSize,
    mark_y: f32,
) -> Option<usize> {
    let lines_above = size.max_display_offset();
    if lines_above == 0 || size.screen_lines == 0 || pane.height <= 0.0 {
        return None;
    }

    let travel = pane.height - mark_height(pane, size);
    if travel <= 0.0 {
        return None;
    }

    let share_from_top = ((mark_y - pane.y) / travel).clamp(0.0, 1.0);
    let lines_below_top = (share_from_top * lines_above as f32).round();

    Some(lines_above - (lines_below_top as usize).min(lines_above))
}

/// Ask to scroll the history of the terminal `terminal_id` so that the top
/// edge of `mark` comes nearest to `mark_y`: to the offset of that place,
/// and at the top of the way of the mark to the top of the history and one
/// step past it, where the first block shows whole.
fn scroll_to_mark_at<T>(
    terminal_id: u64,
    mark: &ScrollMark,
    mark_y: f32,
    publisher: &mut impl FnMut(crate::Event<T>),
) {
    let Some(offset) = mark.offset_for_mark_at(mark_y) else {
        return;
    };
    let top_offset = mark.size.max_display_offset();

    let event = if offset >= top_offset {
        crate::Event::ScrollPastTheTop { id: terminal_id }
    } else {
        crate::Event::ScrollToOffset {
            id: terminal_id,
            offset,
        }
    };

    publisher(event);
}

/// How high the scroll mark is: the share of the history on screen, but
/// never under 48 px and never over the pane.
fn mark_height(pane: Rectangle, size: SnapshotSize) -> f32 {
    let share_on_screen = size.screen_lines as f32 / size.total_lines as f32;

    (pane.height * share_on_screen)
        .max(SCROLL_MARK_MIN_HEIGHT)
        .min(pane.height)
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};
    use otty_libterm::TerminalSize;
    use otty_libterm::escape::NamedPrivateMode;
    use otty_libterm::surface::{
        Surface, SurfaceActor, SurfaceConfig, SurfaceModel,
    };

    use super::*;

    /// A pane 320 px wide and 400 px high, away from the window corner.
    fn pane() -> Rectangle {
        Rectangle::new(Point::new(300.0, 60.0), Size::new(320.0, 400.0))
    }

    /// The text area of [`pane`]: 16 px in from the left and the right.
    fn text_area() -> Rectangle {
        Rectangle::new(Point::new(316.0, 60.0), Size::new(288.0, 400.0))
    }

    /// A history of `total_lines` lines on a screen of `screen_lines`
    /// rows.
    fn history(total_lines: usize, screen_lines: usize) -> SnapshotSize {
        SnapshotSize {
            columns: 36,
            screen_lines,
            total_lines,
        }
    }

    fn bottom(rect: Rectangle) -> f32 {
        rect.y + rect.height
    }

    /// A screen of 20 rows that printed `lines` lines.
    fn surface_with_lines(lines: usize) -> Surface {
        let size = TerminalSize {
            cell_width: 8,
            cell_height: 20,
            cols: 36,
            rows: 20,
        };
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        for _ in 1..lines {
            surface.line_feed();
        }

        surface
    }

    #[test]
    fn a_history_that_fits_on_screen_has_no_scroll_mark() {
        assert_eq!(scroll_mark_rect(pane(), history(20, 20), 0), None);
        assert_eq!(scroll_mark_rect(pane(), history(5, 20), 0), None);
    }

    #[test]
    fn the_scroll_mark_is_4_px_wide_at_the_right_edge_of_the_pane() {
        let mark = scroll_mark_rect(pane(), history(40, 20), 0)
            .expect("a scroll mark");

        assert_eq!(mark.x, 616.0);
        assert_eq!(mark.width, 4.0);
    }

    #[test]
    fn the_scroll_mark_is_as_high_as_the_share_of_the_history_on_screen() {
        let mark = scroll_mark_rect(pane(), history(40, 20), 0)
            .expect("a scroll mark");

        assert_eq!(mark.height, 200.0);
    }

    #[test]
    fn at_the_bottom_of_the_history_the_scroll_mark_ends_at_the_bottom_edge() {
        let mark = scroll_mark_rect(pane(), history(40, 20), 0)
            .expect("a scroll mark");

        assert_eq!(bottom(mark), bottom(pane()));
    }

    #[test]
    fn at_the_top_of_the_history_the_scroll_mark_starts_at_the_top_edge() {
        let mark = scroll_mark_rect(pane(), history(40, 20), 20)
            .expect("a scroll mark");

        assert_eq!(mark.y, pane().y);
    }

    #[test]
    fn a_very_long_history_gives_a_48_px_scroll_mark_inside_the_pane() {
        let at_bottom = scroll_mark_rect(pane(), history(100_000, 20), 0)
            .expect("a scroll mark");
        let at_top = scroll_mark_rect(pane(), history(100_000, 20), 99_980)
            .expect("a scroll mark");

        assert_eq!(at_bottom.height, 48.0);
        assert_eq!(bottom(at_bottom), bottom(pane()));
        assert_eq!(at_top.height, 48.0);
        assert_eq!(at_top.y, pane().y);
    }

    #[test]
    fn the_scroll_mark_moves_evenly_with_the_scroll_offset() {
        let tops: Vec<f32> = (0..=4)
            .map(|offset| {
                scroll_mark_rect(pane(), history(24, 20), offset)
                    .expect("a scroll mark")
                    .y
            })
            .collect();

        let steps: Vec<f32> =
            tops.windows(2).map(|pair| pair[0] - pair[1]).collect();

        // The bar is 333.3 px high, so it travels 66.7 px over 4 lines.
        for step in &steps {
            assert!((step - 400.0 / 24.0).abs() < 0.001, "steps: {steps:?}");
        }
    }

    #[test]
    fn in_a_pane_lower_than_48_px_the_scroll_mark_fills_the_pane_height() {
        let low = Rectangle::new(Point::new(0.0, 10.0), Size::new(320.0, 30.0));

        let mark =
            scroll_mark_rect(low, history(100, 2), 50).expect("a scroll mark");

        assert_eq!(mark.y, 10.0);
        assert_eq!(mark.height, 30.0);
    }

    #[test]
    fn an_offset_past_the_top_of_the_history_keeps_the_scroll_mark_in_the_pane()
    {
        let mark = scroll_mark_rect(pane(), history(40, 20), 35)
            .expect("a scroll mark");

        assert_eq!(mark.y, pane().y);
    }

    #[test]
    fn a_pane_with_no_height_or_no_rows_has_no_scroll_mark() {
        let flat = Rectangle::new(Point::new(0.0, 0.0), Size::new(320.0, 0.0));

        assert_eq!(scroll_mark_rect(flat, history(40, 20), 0), None);
        assert_eq!(scroll_mark_rect(pane(), history(40, 0), 0), None);
    }

    #[test]
    fn the_offset_for_the_place_of_the_mark_undoes_the_place_for_the_offset() {
        // A short history, where a line moves the mark by many pixels, and
        // a long one, where it moves it by less than a pixel.
        for total_lines in [24, 520] {
            let lines_above = total_lines - 20;
            let middle = lines_above / 2;

            for offset in [0, middle, lines_above] {
                let mark =
                    scroll_mark_rect(pane(), history(total_lines, 20), offset)
                        .expect("a scroll mark");

                let back = scroll_offset_for_mark(
                    pane(),
                    history(total_lines, 20),
                    mark.y,
                );

                assert_eq!(back, Some(offset), "{total_lines} lines");
            }
        }
    }

    #[test]
    fn a_place_between_two_lines_gives_the_nearest_line() {
        // The mark is 333.3 px high and travels 66.7 px over 4 lines, so a
        // line moves it by 16.7 px.
        let top_of_offset_2 = pane().y + 2.0 * 400.0 / 24.0;

        let a_bit_lower = scroll_offset_for_mark(
            pane(),
            history(24, 20),
            top_of_offset_2 + 6.0,
        );
        let a_bit_higher = scroll_offset_for_mark(
            pane(),
            history(24, 20),
            top_of_offset_2 - 6.0,
        );

        assert_eq!(a_bit_lower, Some(2));
        assert_eq!(a_bit_higher, Some(2));
    }

    #[test]
    fn a_place_above_the_way_of_the_mark_gives_the_top_and_below_it_the_bottom()
    {
        let above =
            scroll_offset_for_mark(pane(), history(40, 20), pane().y - 100.0);
        let below =
            scroll_offset_for_mark(pane(), history(40, 20), bottom(pane()));

        assert_eq!(above, Some(20));
        assert_eq!(below, Some(0));
    }

    #[test]
    fn no_place_gives_an_offset_when_there_is_no_mark_or_it_can_not_move() {
        let low = Rectangle::new(Point::new(0.0, 10.0), Size::new(320.0, 30.0));

        assert_eq!(
            scroll_offset_for_mark(pane(), history(20, 20), 100.0),
            None
        );
        assert_eq!(scroll_offset_for_mark(low, history(100, 2), 10.0), None);
    }

    #[test]
    fn the_strip_of_the_mark_is_the_right_inset_over_the_full_pane_height() {
        let snapshot = surface_with_lines(40).snapshot_owned();

        let mark = ScrollMark::of_view(pane(), text_area(), &snapshot.view())
            .expect("a scroll mark");

        assert_eq!(
            mark.strip(),
            Rectangle::new(Point::new(604.0, 60.0), Size::new(16.0, 400.0))
        );
    }

    #[test]
    fn the_active_mark_is_8_px_wide_at_the_right_edge_of_the_pane() {
        let snapshot = surface_with_lines(40).snapshot_owned();
        let mark = ScrollMark::of_view(pane(), text_area(), &snapshot.view())
            .expect("a scroll mark");

        let resting = mark.rect(false);
        let active = mark.rect(true);

        assert_eq!((resting.x, resting.width), (616.0, 4.0));
        assert_eq!((active.x, active.width), (612.0, 8.0));
        assert_eq!((active.y, active.height), (resting.y, resting.height));
    }

    #[test]
    fn a_history_that_fits_or_a_full_screen_program_has_no_mark_and_no_strip() {
        let fits = surface_with_lines(10).snapshot_owned();
        let mut full_screen = surface_with_lines(40);
        full_screen.set_private_mode(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        );
        let full_screen = full_screen.snapshot_owned();

        assert_eq!(
            ScrollMark::of_view(pane(), text_area(), &fits.view()),
            None
        );
        assert_eq!(
            ScrollMark::of_view(pane(), text_area(), &full_screen.view()),
            None
        );
    }
}
