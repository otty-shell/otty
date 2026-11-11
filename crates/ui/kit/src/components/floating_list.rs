//! A field with a list that floats under it, as a [`TagSearchField`] shows
//! its suggestions.
//!
//! The list is an overlay, so it sits above the content under the field
//! without moving it. The widget also keeps the highlighted row in view: only
//! it knows the list's height, and doing it here spares every app a scroll
//! task after each ↑ and ↓.
//!
//! [`TagSearchField`]: super::tag_search_field::TagSearchField

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::Scrollable;
use iced::advanced::widget::operation::scrollable::AbsoluteOffset;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::widget::{Id, Space};
use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    mouse,
};

use crate::spacing;

/// Gap between the field and the list.
const LIST_GAP: f32 = spacing::SPACE_1;
/// Smallest width of the list; a narrow field still has a readable list.
const LIST_MIN_WIDTH: f32 = 260.0;

/// A field and, while it is open, the list under it, aligned to the field's
/// left edge. A press outside both sends `on_close` and still reaches what is
/// there; a press in the list stays in the list, so the field keeps the focus.
pub(super) struct FloatingList<'a, Message> {
    field: Element<'a, Message>,
    list: Element<'a, Message>,
    is_open: bool,
    highlighted: Option<usize>,
    row_height: f32,
    on_close: Option<Message>,
}

impl<'a, Message: 'a> FloatingList<'a, Message> {
    /// Wraps `field`, with no list.
    pub(super) fn new(field: impl Into<Element<'a, Message>>) -> Self {
        Self {
            field: field.into(),
            list: Space::new().into(),
            is_open: false,
            highlighted: None,
            row_height: 0.0,
            on_close: None,
        }
    }

    /// Opens `list`, whose rows are `row_height` high with no gaps, with the
    /// row at `highlighted` kept in view.
    pub(super) fn list(
        mut self,
        list: impl Into<Element<'a, Message>>,
        highlighted: Option<usize>,
        row_height: f32,
    ) -> Self {
        self.list = list.into();
        self.is_open = true;
        self.highlighted = highlighted;
        self.row_height = row_height;
        self
    }

    /// Sets the message sent on a press outside the field and the list.
    pub(super) fn on_close(mut self, message: Option<Message>) -> Self {
        self.on_close = message;
        self
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer>
    for FloatingList<'_, Message>
{
    fn size(&self) -> Size<Length> {
        self.field.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.field.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.field), Tree::new(&self.list)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.field.as_widget(), self.list.as_widget()]);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.field.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            limits,
        )
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
        self.field.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
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
        self.field.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.field.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.field.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        let [field_tree, list_tree] = children.as_mut_slice() else {
            return None;
        };

        if !self.is_open {
            state.shown_row = None;

            return self.field.as_widget_mut().overlay(
                field_tree,
                layout,
                renderer,
                viewport,
                translation,
            );
        }

        Some(overlay::Element::new(Box::new(ListOverlay {
            list: &mut self.list,
            tree: list_tree,
            anchor: layout.bounds() + translation,
            highlighted: self.highlighted,
            row_height: self.row_height,
            shown_row: &mut state.shown_row,
            on_close: self.on_close.as_ref(),
        })))
    }
}

impl<'a, Message: Clone + 'a> From<FloatingList<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: FloatingList<'a, Message>) -> Self {
        Element::new(value)
    }
}

/// What a [`FloatingList`] remembers between frames.
#[derive(Default)]
struct State {
    /// The highlighted row the list last scrolled to. The list scrolls only
    /// when this changes, so the mouse wheel can move it freely.
    shown_row: Option<usize>,
}

/// The open list, drawn over everything under the field at `anchor`.
struct ListOverlay<'a, 'b, Message> {
    list: &'a mut Element<'b, Message>,
    tree: &'a mut Tree,
    anchor: Rectangle,
    highlighted: Option<usize>,
    row_height: f32,
    shown_row: &'a mut Option<usize>,
    on_close: Option<&'a Message>,
}

impl<Message> ListOverlay<'_, '_, Message> {
    /// Scrolls the laid out list to the highlighted row when it changed.
    fn keep_highlight_in_view(
        &mut self,
        node: &layout::Node,
        renderer: &Renderer,
    ) {
        if *self.shown_row == self.highlighted {
            return;
        }
        *self.shown_row = self.highlighted;

        let Some(row) = self.highlighted else {
            return;
        };

        let top = row as f32 * self.row_height;
        let mut scroll = KeepRowInView {
            top,
            bottom: top + self.row_height,
        };
        self.list.as_widget_mut().operate(
            self.tree,
            Layout::new(node),
            renderer,
            &mut scroll,
        );
    }
}

impl<Message: Clone> overlay::Overlay<Message, Theme, Renderer>
    for ListOverlay<'_, '_, Message>
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let width = self.anchor.width.max(LIST_MIN_WIDTH).min(bounds.width);
        let limits = layout::Limits::new(
            Size::new(width, 0.0),
            Size::new(width, bounds.height),
        );

        let node = self
            .list
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);
        self.keep_highlight_in_view(&node, renderer);

        let position = list_position(self.anchor, node.size(), bounds);

        node.move_to(position)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.list.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.list
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let bounds = layout.bounds();

        self.list.as_widget_mut().update(
            self.tree, event, layout, cursor, renderer, clipboard, shell,
            &bounds,
        );

        if !matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) {
            return;
        }

        if cursor.is_over(bounds) {
            shell.capture_event();
        } else if !cursor.is_over(self.anchor)
            && let Some(message) = self.on_close
        {
            shell.publish(message.clone());
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        if !cursor.is_over(bounds) {
            return mouse::Interaction::None;
        }

        // The rows under the list must not light up: anything but `None`
        // hides the pointer from them.
        match self
            .list
            .as_widget()
            .mouse_interaction(self.tree, layout, cursor, &bounds, renderer)
        {
            mouse::Interaction::None => mouse::Interaction::Idle,
            interaction => interaction,
        }
    }
}

/// Scrolls a scrollable just enough to show the row spanning `top..bottom` of
/// its content.
struct KeepRowInView {
    top: f32,
    bottom: f32,
}

impl Operation for KeepRowInView {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        let offset =
            offset_showing(self.top, self.bottom, translation.y, bounds.height);

        if let Some(y) = offset {
            state.scroll_to(AbsoluteOffset {
                x: None,
                y: Some(y),
            });
        }
    }
}

/// Returns the top-left corner of a list of `list` size for a field at
/// `anchor` in a window of `bounds` size: under the field at its left edge,
/// or over the field when there is no room under it, never past the window's
/// edges.
fn list_position(anchor: Rectangle, list: Size, bounds: Size) -> Point {
    let x = anchor.x.min(bounds.width - list.width).max(0.0);

    let below = anchor.y + anchor.height + LIST_GAP;
    let y = if below + list.height <= bounds.height {
        below
    } else {
        (anchor.y - LIST_GAP - list.height).max(0.0)
    };

    Point::new(x, y)
}

/// Returns the scroll offset that shows the row spanning `top..bottom` in a
/// view `height` high now scrolled to `offset`; `None` when it already shows.
fn offset_showing(
    top: f32,
    bottom: f32,
    offset: f32,
    height: f32,
) -> Option<f32> {
    if top < offset {
        Some(top)
    } else if bottom > offset + height {
        Some(bottom - height)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Size = Size::new(800.0, 600.0);
    const LIST: Size = Size::new(260.0, 100.0);

    fn field(x: f32, y: f32) -> Rectangle {
        Rectangle::new(Point::new(x, y), Size::new(200.0, 34.0))
    }

    #[test]
    fn with_room_the_list_opens_under_the_field_at_its_left_edge() {
        let position = list_position(field(40.0, 50.0), LIST, WINDOW);

        assert_eq!(position, Point::new(40.0, 84.0 + LIST_GAP));
    }

    #[test]
    fn without_room_under_the_field_the_list_opens_over_it() {
        let position = list_position(field(40.0, 550.0), LIST, WINDOW);

        assert_eq!(position, Point::new(40.0, 550.0 - LIST_GAP - 100.0));
    }

    #[test]
    fn the_list_stays_inside_the_window() {
        let right = list_position(field(700.0, 50.0), LIST, WINDOW);
        let narrow = list_position(
            field(20.0, 50.0),
            LIST,
            Size::new(100.0, WINDOW.height),
        );

        assert_eq!(right.x, 800.0 - 260.0);
        assert_eq!(narrow.x, 0.0);
    }

    #[test]
    fn a_row_in_view_does_not_scroll() {
        assert_eq!(offset_showing(30.0, 60.0, 0.0, 240.0), None);
        assert_eq!(offset_showing(30.0, 60.0, 30.0, 240.0), None);
        assert_eq!(offset_showing(210.0, 240.0, 0.0, 240.0), None);
    }

    #[test]
    fn a_row_below_the_view_scrolls_to_the_view_bottom() {
        assert_eq!(offset_showing(240.0, 270.0, 0.0, 240.0), Some(30.0));
    }

    #[test]
    fn a_row_above_the_view_scrolls_to_the_view_top() {
        assert_eq!(offset_showing(60.0, 90.0, 120.0, 240.0), Some(60.0));
    }
}
