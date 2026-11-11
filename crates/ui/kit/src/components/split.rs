//! `Split`: the 1px `line` between resizable panes and its drag.
//!
//! `iced`'s `PaneGrid::on_resize` hands every press to the panes first and
//! then starts its own drag without asking whether a pane took the press, so
//! a press on a scrollbar beside a split both scrolls and resizes. There is no
//! way to turn that off inside `PaneGrid`, so [`ResizableGrid`] sits around
//! the grid and owns the drag: while the pointer belongs to a split, the panes
//! see it as hidden.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::widget::pane_grid::{self, Axis, ResizeEvent, Split};
use iced::{
    Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Size,
    Theme, Vector, mouse, touch,
};

use crate::palette::UiPalette;
use crate::size;

/// A `PaneGrid` whose splits can be dragged. A press in the grab zone of a
/// split only resizes, for the whole drag, and the panes do not see the
/// pointer until it ends. A press anywhere else belongs to the panes for the
/// whole drag, and no resize starts.
///
/// The grab zone is the line and `size::SPLIT_GRAB` to its right or below it,
/// so a thin scrollbar on the right or bottom edge of a pane stays usable right
/// up to the line. Where a bar starts at a split, the split wins.
///
/// Give the grid `.spacing(size::LINE)`, the same `min_size` as here and no
/// `.on_resize`. Without [`ResizableGrid::on_resize`] nothing can be dragged;
/// the panes keep their widget state when it comes and goes.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::pane_grid::{self, Content, PaneGrid};
/// use iced::widget::text;
/// use otty_ui_kit::components::split::ResizableGrid;
/// use otty_ui_kit::palette::UiPalette;
/// use otty_ui_kit::size;
///
/// #[derive(Clone)]
/// enum Message {
///     Resized(pane_grid::ResizeEvent),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let (panes, _) = pane_grid::State::new("Pane");
/// let grid = PaneGrid::new(&panes, |_, title, _| Content::new(text(*title)))
///     .spacing(size::LINE)
///     .min_size(50);
/// let view: Element<'_, Message> =
///     ResizableGrid::new(grid, &panes, 50.0, &palette)
///         .on_resize(Message::Resized)
///         .into();
/// ```
pub struct ResizableGrid<'a, Message> {
    grid: Element<'a, Message>,
    node: pane_grid::Node,
    min_size: f32,
    line: Color,
    on_resize: Option<Box<dyn Fn(ResizeEvent) -> Message + 'a>>,
}

impl<'a, Message> ResizableGrid<'a, Message> {
    /// Wraps `grid`, which shows `panes` with the given `min_size`.
    pub fn new<T>(
        grid: impl Into<Element<'a, Message>>,
        panes: &pane_grid::State<T>,
        min_size: f32,
        palette: &UiPalette,
    ) -> Self {
        Self {
            grid: grid.into(),
            node: panes.layout().clone(),
            min_size,
            line: palette.line(),
            on_resize: None,
        }
    }

    /// Sets the function that maps a drag of a split to a message.
    pub fn on_resize(
        mut self,
        on_resize: impl Fn(ResizeEvent) -> Message + 'a,
    ) -> Self {
        self.on_resize = Some(Box::new(on_resize));
        self
    }

    /// Returns the split whose grab zone is under `cursor`.
    fn split_under(
        &self,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<(Split, Axis)> {
        self.on_resize.as_ref()?;
        let position = cursor.position_over(bounds)?;
        let relative = Point::new(position.x - bounds.x, position.y - bounds.y);

        self.node
            .split_regions(size::LINE, self.min_size, bounds.size())
            .into_iter()
            .find(|(_, (axis, region, ratio))| {
                grab_zone(*axis, *region, *ratio).contains(relative)
            })
            .map(|(split, (axis, _, _))| (split, axis))
    }

    /// Returns the ratio of `split` that puts its line under `cursor`.
    fn ratio_under(
        &self,
        bounds: Rectangle,
        split: Split,
        cursor: mouse::Cursor,
    ) -> Option<f32> {
        // `land`: a window grip on a layer above must not stop the drag.
        let position = cursor.land().position()?;
        let splits =
            self.node
                .split_regions(size::LINE, self.min_size, bounds.size());
        let (axis, region, _) = splits.get(&split)?;

        let ratio = match axis {
            Axis::Horizontal => {
                (position.y - bounds.y - region.y) / region.height
            },
            Axis::Vertical => (position.x - bounds.x - region.x) / region.width,
        };

        Some(ratio.clamp(0.0, 1.0))
    }

    /// Returns the bounds of the line of `split`, in the window.
    fn line_bounds(
        &self,
        bounds: Rectangle,
        split: Split,
    ) -> Option<Rectangle> {
        let splits =
            self.node
                .split_regions(size::LINE, self.min_size, bounds.size());
        let (axis, region, ratio) = splits.get(&split)?;
        let line = axis.split_line_bounds(*region, *ratio, size::LINE);

        Some(line + Vector::new(bounds.x, bounds.y))
    }

    /// Returns the split the pointer belongs to now, if any.
    fn active_split(
        &self,
        state: &State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<(Split, Axis)> {
        match state.drag {
            Drag::Resizing { split, axis } => Some((split, axis)),
            Drag::Panes => None,
            Drag::None => self.split_under(bounds, cursor),
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for ResizableGrid<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.grid.as_widget().size()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.grid)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.grid.as_widget()]);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.grid.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            limits,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.grid.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let under = self.split_under(bounds, cursor);
        let state = tree.state.downcast_mut::<State>();

        if under != state.hovered {
            state.hovered = under;
            shell.request_redraw();
        }

        if is_press(event) {
            state.drag = match under {
                Some((split, axis)) => Drag::Resizing { split, axis },
                None if cursor.is_over(bounds) => Drag::Panes,
                None => Drag::None,
            };
        }

        let drag = state.drag;
        let panes_cursor = if hides_pointer(drag, under.is_some(), event) {
            cursor.levitate()
        } else {
            cursor
        };

        self.grid.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            panes_cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if let Drag::Resizing { split, .. } = drag {
            if is_press(event) {
                shell.capture_event();
            }

            if is_move(event)
                && let Some(on_resize) = &self.on_resize
                && let Some(ratio) = self.ratio_under(bounds, split, cursor)
            {
                shell.publish(on_resize(ResizeEvent { split, ratio }));
                shell.capture_event();
            }

            if is_release(event) {
                shell.request_redraw();
            }
        }

        if is_release(event) {
            tree.state.downcast_mut::<State>().drag = Drag::None;
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<State>();
        let active = self.active_split(state, bounds, cursor);
        let panes_cursor = if active.is_some() {
            cursor.levitate()
        } else {
            cursor
        };

        self.grid.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            panes_cursor,
            viewport,
        );

        if let Some((split, _)) = active
            && let Some(line) = self.line_bounds(bounds, split)
        {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: line,
                    ..renderer::Quad::default()
                },
                self.line,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();

        match self.active_split(state, layout.bounds(), cursor) {
            Some((_, Axis::Horizontal)) => {
                mouse::Interaction::ResizingVertically
            },
            Some((_, Axis::Vertical)) => {
                mouse::Interaction::ResizingHorizontally
            },
            None => self.grid.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            ),
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.grid.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<ResizableGrid<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: ResizableGrid<'a, Message>) -> Self {
        Element::new(value)
    }
}

/// Widget state of a [`ResizableGrid`].
#[derive(Default)]
struct State {
    drag: Drag,
    /// The split under the pointer at the last event, to redraw when it
    /// changes: the line and the pointer shape depend on it.
    hovered: Option<(Split, Axis)>,
}

/// Who the pointer belongs to while the left button is down.
#[derive(Debug, Clone, Copy, Default)]
enum Drag {
    /// The button is up.
    #[default]
    None,
    /// The press was in the grab zone of this split.
    Resizing { split: Split, axis: Axis },
    /// The press was on a pane: its drag goes on over any split.
    Panes,
}

/// Returns the `PaneGrid` style: the hovered region is drawn in `line`. The
/// split itself is drawn by [`ResizableGrid`].
pub fn style(
    palette: &UiPalette,
) -> impl Fn(&Theme) -> pane_grid::Style + use<> {
    let line = palette.line();

    move |_| pane_grid::Style {
        hovered_region: pane_grid::Highlight {
            background: line.into(),
            border: Border::default(),
        },
        picked_split: pane_grid::Line {
            color: line,
            width: size::LINE,
        },
        hovered_split: pane_grid::Line {
            color: line,
            width: size::LINE,
        },
    }
}

/// Returns the zone of a split that takes a press, relative to the grid: its
/// line and `size::SPLIT_GRAB` to the right of it or below it.
fn grab_zone(axis: Axis, region: Rectangle, ratio: f32) -> Rectangle {
    let line = axis.split_line_bounds(region, ratio, size::LINE);

    match axis {
        Axis::Horizontal => Rectangle {
            height: line.height + size::SPLIT_GRAB,
            ..line
        },
        Axis::Vertical => Rectangle {
            width: line.width + size::SPLIT_GRAB,
            ..line
        },
    }
}

/// Returns whether the panes must not see the pointer for `event`. Hovering a
/// grab zone hides it only from what would act like a press there: the wheel
/// and the other buttons still reach the pane under the pointer.
fn hides_pointer(drag: Drag, is_over_zone: bool, event: &Event) -> bool {
    match drag {
        Drag::Resizing { .. } => true,
        Drag::Panes => false,
        Drag::None => {
            is_over_zone
                && !matches!(
                    event,
                    Event::Mouse(
                        mouse::Event::WheelScrolled { .. }
                            | mouse::Event::ButtonPressed(
                                mouse::Button::Right
                                    | mouse::Button::Middle
                                    | mouse::Button::Back
                                    | mouse::Button::Forward
                                    | mouse::Button::Other(_)
                            )
                            | mouse::Event::ButtonReleased(
                                mouse::Button::Right
                                    | mouse::Button::Middle
                                    | mouse::Button::Back
                                    | mouse::Button::Forward
                                    | mouse::Button::Other(_)
                            )
                    )
                )
        },
    }
}

fn is_press(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. })
    )
}

fn is_move(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(touch::Event::FingerMoved { .. })
    )
}

fn is_release(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(
                touch::Event::FingerLifted { .. }
                    | touch::Event::FingerLost { .. }
            )
    )
}

#[cfg(test)]
mod tests {
    use iced::widget::pane_grid::{Configuration, Content, PaneGrid};
    use iced::widget::{Space, button, column, mouse_area, scrollable};
    use iced_test::Simulator;

    use super::*;
    use crate::components::scrollbar;

    const WINDOW: Size = Size::new(400.0, 300.0);
    /// Where the 1 px line of a split at the middle of [`WINDOW`] starts.
    const VERTICAL_LINE: f32 = 200.0;
    const HORIZONTAL_LINE: f32 = 150.0;

    #[derive(Debug, Clone, PartialEq)]
    enum Sent {
        Resized,
        Scrolled,
        Pressed,
        RightPressed,
        Moved,
    }

    #[derive(Clone, Copy)]
    enum Slot {
        /// A scrollable taller than the pane, with its bar on the right edge.
        Tall,
        /// A scrollable wider than the pane, with its bar on the bottom edge.
        Wide,
        /// A button that fills the pane.
        Press,
        /// A box that reports the pointer moving over it and a right press.
        Sensor,
    }

    fn slot_view<'a>(slot: Slot) -> Element<'a, Sent> {
        match slot {
            Slot::Tall => scrollable(column![
                Space::new().width(Length::Fill).height(2000)
            ])
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollbar::vertical())
            .on_scroll(|_| Sent::Scrolled)
            .into(),
            Slot::Wide => scrollable(Space::new().width(2000).height(2000))
                .width(Length::Fill)
                .height(Length::Fill)
                .direction(scrollbar::horizontal())
                .on_scroll(|_| Sent::Scrolled)
                .into(),
            Slot::Press => {
                button(Space::new().width(Length::Fill).height(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .on_press(Sent::Pressed)
                    .into()
            },
            Slot::Sensor => mouse_area(
                Space::new().width(Length::Fill).height(Length::Fill),
            )
            .on_move(|_| Sent::Moved)
            .on_right_press(Sent::RightPressed)
            .into(),
        }
    }

    fn panes(axis: Axis, a: Slot, b: Slot) -> pane_grid::State<Slot> {
        pane_grid::State::with_configuration(Configuration::Split {
            axis,
            ratio: 0.5,
            a: Box::new(Configuration::Pane(a)),
            b: Box::new(Configuration::Pane(b)),
        })
    }

    /// The grid the way the app builds it, without anything to resize it.
    fn fixed_grid(panes: &pane_grid::State<Slot>) -> ResizableGrid<'_, Sent> {
        let palette = UiPalette::otty_dark();
        let grid =
            PaneGrid::new(panes, |_, slot, _| Content::new(slot_view(*slot)))
                .width(Length::Fill)
                .height(Length::Fill)
                .spacing(size::LINE)
                .min_size(0);

        ResizableGrid::new(grid, panes, 0.0, &palette)
    }

    fn simulator(panes: &pane_grid::State<Slot>) -> Simulator<'_, Sent> {
        Simulator::with_size(
            iced::Settings::default(),
            WINDOW,
            fixed_grid(panes).on_resize(|_| Sent::Resized),
        )
    }

    fn move_to(ui: &mut Simulator<'_, Sent>, position: Point) {
        ui.point_at(position);
        let _ =
            ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    }

    fn press(ui: &mut Simulator<'_, Sent>) {
        let _ = ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ))]);
    }

    fn release(ui: &mut Simulator<'_, Sent>) {
        let _ = ui.simulate([Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        ))]);
    }

    /// Presses at `from`, moves through `path` and releases; returns what was
    /// sent.
    fn drag(
        panes: &pane_grid::State<Slot>,
        from: Point,
        path: &[Point],
    ) -> Vec<Sent> {
        let mut ui = simulator(panes);

        move_to(&mut ui, from);
        press(&mut ui);
        for position in path {
            move_to(&mut ui, *position);
        }
        release(&mut ui);

        ui.into_messages().collect()
    }

    fn count(sent: &[Sent], kind: &Sent) -> usize {
        sent.iter().filter(|item| *item == kind).count()
    }

    fn assert_only_resizes(sent: &[Sent]) {
        assert!(
            sent.contains(&Sent::Resized),
            "a resize should happen: {sent:?}"
        );
        assert!(
            sent.iter().all(|item| *item == Sent::Resized),
            "only a resize should happen: {sent:?}"
        );
    }

    fn assert_only_scrolls(sent: &[Sent]) {
        assert!(
            sent.contains(&Sent::Scrolled),
            "a scroll should happen: {sent:?}"
        );
        assert!(
            sent.iter().all(|item| *item == Sent::Scrolled),
            "only a scroll should happen: {sent:?}"
        );
    }

    #[test]
    fn a_press_on_the_line_beside_a_bar_only_resizes() {
        let panes = panes(Axis::Vertical, Slot::Tall, Slot::Press);

        let sent = drag(
            &panes,
            Point::new(VERTICAL_LINE + 0.5, 150.0),
            &[Point::new(150.0, 250.0), Point::new(120.0, 280.0)],
        );

        assert_only_resizes(&sent);
    }

    #[test]
    fn a_press_on_a_bar_beside_the_line_only_scrolls() {
        let panes = panes(Axis::Vertical, Slot::Tall, Slot::Press);

        // The bar is the last 4 px of the left pane, right up to the line.
        for x in [VERTICAL_LINE - 3.5, VERTICAL_LINE - 0.5] {
            let sent = drag(
                &panes,
                Point::new(x, 150.0),
                &[Point::new(x, 200.0), Point::new(x, 280.0)],
            );

            assert_only_scrolls(&sent);
        }
    }

    #[test]
    fn a_press_where_a_vertical_bar_meets_a_split_only_resizes() {
        let panes = panes(Axis::Horizontal, Slot::Press, Slot::Tall);

        let sent = drag(
            &panes,
            Point::new(398.0, HORIZONTAL_LINE + 3.0),
            &[Point::new(398.0, 200.0), Point::new(398.0, 280.0)],
        );

        assert_only_resizes(&sent);
    }

    #[test]
    fn a_press_on_a_vertical_bar_away_from_the_split_only_scrolls() {
        let panes = panes(Axis::Horizontal, Slot::Press, Slot::Tall);

        let sent = drag(
            &panes,
            Point::new(398.0, 220.0),
            &[Point::new(398.0, 250.0), Point::new(398.0, 280.0)],
        );

        assert_only_scrolls(&sent);
    }

    #[test]
    fn a_press_where_a_sideways_bar_meets_a_split_only_resizes() {
        let panes = panes(Axis::Vertical, Slot::Press, Slot::Wide);

        let sent = drag(
            &panes,
            Point::new(VERTICAL_LINE + 3.0, 298.0),
            &[Point::new(260.0, 298.0), Point::new(300.0, 298.0)],
        );

        assert_only_resizes(&sent);
    }

    #[test]
    fn a_press_on_a_sideways_bar_away_from_the_split_only_scrolls() {
        let panes = panes(Axis::Vertical, Slot::Press, Slot::Wide);

        let sent = drag(
            &panes,
            Point::new(300.0, 298.0),
            &[Point::new(340.0, 298.0), Point::new(380.0, 298.0)],
        );

        assert_only_scrolls(&sent);
    }

    #[test]
    fn a_resize_passing_over_a_scrollbar_does_not_scroll() {
        let panes = panes(Axis::Vertical, Slot::Press, Slot::Tall);

        let sent = drag(
            &panes,
            Point::new(VERTICAL_LINE + 0.5, 150.0),
            &[
                Point::new(300.0, 150.0),
                Point::new(398.0, 150.0),
                Point::new(398.0, 280.0),
            ],
        );

        assert_only_resizes(&sent);
    }

    #[test]
    fn nothing_under_the_pointer_reacts_while_a_resize_passes_over_it() {
        let panes = panes(Axis::Vertical, Slot::Sensor, Slot::Press);

        // Starts on the line, crosses the sensor and is let go over the
        // button, which must not count as a press.
        let sent = drag(
            &panes,
            Point::new(VERTICAL_LINE + 0.5, 150.0),
            &[
                Point::new(100.0, 150.0),
                Point::new(300.0, 150.0),
                Point::new(310.0, 150.0),
            ],
        );

        assert_only_resizes(&sent);
    }

    #[test]
    fn a_press_in_the_grab_zone_does_not_reach_the_content_under_it() {
        let panes = panes(Axis::Vertical, Slot::Sensor, Slot::Press);

        let sent = drag(
            &panes,
            Point::new(VERTICAL_LINE + 3.0, 150.0),
            &[Point::new(VERTICAL_LINE + 3.0, 150.0)],
        );

        assert_eq!(count(&sent, &Sent::Pressed), 0, "{sent:?}");
    }

    #[test]
    fn a_press_right_of_the_grab_zone_reaches_the_content() {
        let panes = panes(Axis::Vertical, Slot::Sensor, Slot::Press);

        let sent = drag(
            &panes,
            Point::new(VERTICAL_LINE + 20.0, 150.0),
            &[Point::new(VERTICAL_LINE + 20.0, 150.0)],
        );

        assert_eq!(sent, vec![Sent::Pressed]);
    }

    #[test]
    fn a_scrollbar_drag_over_a_split_keeps_scrolling_and_starts_no_resize() {
        let panes = panes(Axis::Horizontal, Slot::Press, Slot::Tall);
        let x = 398.0;

        // Up from the bar, over the split and into the other pane.
        let sent = drag(
            &panes,
            Point::new(x, 280.0),
            &[
                Point::new(x, 200.0),
                Point::new(x, HORIZONTAL_LINE + 2.0),
                Point::new(x, HORIZONTAL_LINE - 20.0),
            ],
        );

        assert_only_scrolls(&sent);
    }

    #[test]
    fn a_content_drag_passing_through_the_grab_zone_keeps_reaching_the_content()
    {
        let panes = panes(Axis::Vertical, Slot::Press, Slot::Sensor);

        // The first move comes before the press, the second one is in the
        // zone: a text selection in a terminal has to keep following it.
        let sent = drag(
            &panes,
            Point::new(300.0, 150.0),
            &[Point::new(VERTICAL_LINE + 3.0, 150.0)],
        );

        assert_eq!(sent, vec![Sent::Moved, Sent::Moved]);
    }

    #[test]
    fn without_a_resize_message_the_zone_belongs_to_the_panes() {
        let panes = panes(Axis::Vertical, Slot::Press, Slot::Press);
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            WINDOW,
            fixed_grid(&panes),
        );

        move_to(&mut ui, Point::new(VERTICAL_LINE + 3.0, 150.0));
        press(&mut ui);
        move_to(&mut ui, Point::new(300.0, 150.0));
        move_to(&mut ui, Point::new(VERTICAL_LINE + 3.0, 150.0));
        release(&mut ui);

        assert_eq!(ui.into_messages().collect::<Vec<_>>(), vec![Sent::Pressed]);
    }

    #[test]
    fn the_zone_follows_where_the_panes_are_in_the_window() {
        let panes = panes(Axis::Vertical, Slot::Press, Slot::Press);
        let padded = iced::widget::container(
            fixed_grid(&panes).on_resize(|_| Sent::Resized),
        )
        .padding(iced::Padding {
            left: 100.0,
            ..iced::Padding::ZERO
        });
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            Size::new(WINDOW.width + 100.0, WINDOW.height),
            padded,
        );

        move_to(&mut ui, Point::new(100.0 + VERTICAL_LINE + 0.5, 150.0));
        press(&mut ui);
        move_to(&mut ui, Point::new(350.0, 150.0));
        release(&mut ui);

        assert_only_resizes(&ui.into_messages().collect::<Vec<_>>());
    }

    #[test]
    fn a_drag_of_a_split_reports_the_ratio_under_the_pointer() {
        let panes = panes(Axis::Vertical, Slot::Press, Slot::Press);
        let palette = UiPalette::otty_dark();
        let grid =
            PaneGrid::new(&panes, |_, slot, _| Content::new(slot_view(*slot)))
                .width(Length::Fill)
                .height(Length::Fill)
                .spacing(size::LINE)
                .min_size(0);
        let resizable: ResizableGrid<'_, f32> = ResizableGrid::new(
            Element::from(grid).map(|_| 0.0),
            &panes,
            0.0,
            &palette,
        )
        .on_resize(|event| event.ratio);
        let mut ui =
            Simulator::with_size(iced::Settings::default(), WINDOW, resizable);
        let position = Point::new(VERTICAL_LINE + 0.5, 150.0);
        let target = Point::new(100.0, 150.0);

        ui.point_at(position);
        let _ =
            ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
        let _ = ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ))]);
        ui.point_at(target);
        let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
            position: target,
        })]);

        assert_eq!(ui.into_messages().collect::<Vec<_>>(), vec![0.25]);
    }

    #[test]
    fn hovering_the_grab_zone_hides_the_pointer_from_the_content() {
        let panes = panes(Axis::Vertical, Slot::Press, Slot::Sensor);
        let mut ui = simulator(&panes);

        move_to(&mut ui, Point::new(VERTICAL_LINE + 3.0, 150.0));

        assert!(ui.into_messages().next().is_none());
    }

    #[test]
    fn a_wheel_over_the_grab_zone_still_scrolls_the_content() {
        let panes = panes(Axis::Vertical, Slot::Press, Slot::Tall);
        let mut ui = simulator(&panes);

        move_to(&mut ui, Point::new(VERTICAL_LINE + 3.0, 150.0));
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -40.0 },
        })]);

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![Sent::Scrolled]
        );
    }

    #[test]
    fn a_right_press_in_the_grab_zone_still_reaches_the_content() {
        let panes = panes(Axis::Vertical, Slot::Press, Slot::Sensor);
        let mut ui = simulator(&panes);

        move_to(&mut ui, Point::new(VERTICAL_LINE + 3.0, 150.0));
        let _ = ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Right,
        ))]);

        assert!(ui.into_messages().any(|sent| sent == Sent::RightPressed));
    }
}
