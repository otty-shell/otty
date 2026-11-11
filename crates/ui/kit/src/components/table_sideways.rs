//! Sideways scrolling of a [`DataTable`] that is wider than its space.
//!
//! The header and the rows scroll together inside one horizontal
//! `scrollable`. A scrollable hands its content a pointer moved by the scroll
//! offset, and the rows deeper inside can only take back the vertical part
//! of it (see [`TableBody`]). [`SidewaysScroll`] sits outside the scrollable,
//! reads how far it is scrolled sideways before each event, and keeps that
//! number where [`TableBody`] can read it.
//!
//! [`DataTable`]: super::data_table::DataTable
//! [`TableBody`]: super::table_body::TableBody

use std::cell::Cell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::Scrollable;
use iced::advanced::widget::{Id, Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse,
};

/// How far a table is scrolled sideways, in pixels, shared between
/// [`SidewaysScroll`], which writes it, and the table body, which reads it.
pub(super) type SidewaysOffset = Rc<Cell<f32>>;

/// Wraps a horizontal `scrollable` and records its horizontal offset.
pub(super) struct SidewaysScroll<'a, Message> {
    scrollable: Element<'a, Message>,
    offset: SidewaysOffset,
}

impl<'a, Message> SidewaysScroll<'a, Message> {
    /// Wraps `scrollable`, which must be a horizontal `scrollable` itself,
    /// and writes its offset to `offset`.
    pub(super) fn new(
        scrollable: impl Into<Element<'a, Message>>,
        offset: SidewaysOffset,
    ) -> Self {
        Self {
            scrollable: scrollable.into(),
            offset,
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for SidewaysScroll<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.scrollable.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.scrollable.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.scrollable.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            limits,
        )
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
        self.scrollable.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.scrollable)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.scrollable));
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.scrollable.as_widget_mut().operate(
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
        let mut probe = OffsetProbe { x: 0.0 };
        self.scrollable.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            &mut probe,
        );
        self.offset.set(probe.x);

        self.scrollable.as_widget_mut().update(
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

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.scrollable.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.scrollable.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<SidewaysScroll<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: SidewaysScroll<'a, Message>) -> Self {
        Element::new(value)
    }
}

/// Reads the horizontal offset of the first scrollable it meets, which is
/// the wrapped one, and does not look inside it.
struct OffsetProbe {
    x: f32,
}

impl Operation for OffsetProbe {
    fn traverse(&mut self, _operate: &mut dyn FnMut(&mut dyn Operation)) {}

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        _bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.x = translation.x;
    }
}
