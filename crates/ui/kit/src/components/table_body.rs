//! Body rows of a [`DataTable`] that report right clicks with their place.
//!
//! A context menu opens where the pointer is, so a right click must send
//! the row index and the pointer position. `iced` 0.14 `mouse_area` sends
//! neither, and inside a `scrollable` every child sees the pointer moved by
//! the scroll offset. [`TableBody`] sits directly inside the table's
//! `scrollable` and takes that offset back out.
//!
//! [`DataTable`]: super::data_table::DataTable

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    mouse,
};

/// Message builder for a right click: row index and pointer position.
pub(super) type RightPress<'a, Message> =
    Box<dyn Fn(usize, Point) -> Message + 'a>;

/// Wraps the column of table rows. Each child of the column is one row.
pub(super) struct TableBody<'a, Message> {
    rows: Element<'a, Message>,
    on_right_press: Option<RightPress<'a, Message>>,
}

impl<'a, Message> TableBody<'a, Message> {
    /// Wraps `rows`, which must be the direct content of a `scrollable`.
    pub(super) fn new(
        rows: impl Into<Element<'a, Message>>,
        on_right_press: Option<RightPress<'a, Message>>,
    ) -> Self {
        Self {
            rows: rows.into(),
            on_right_press,
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for TableBody<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.rows.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.rows.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.rows.as_widget_mut().layout(
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
        self.rows.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.rows)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.rows));
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.rows.as_widget_mut().operate(
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
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) =
            event
            && let Some(on_right_press) = &self.on_right_press
            && let Some(position) = cursor.position_over(layout.bounds())
            && let Some(index) = layout
                .children()
                .position(|row| row.bounds().contains(position))
        {
            // The scrollable lays the rows out from its own top-left corner
            // and hands them a viewport and pointer moved by the scroll
            // offset; the difference of the two corners is that offset.
            let scroll = viewport.position() - layout.bounds().position();

            shell.publish(on_right_press(index, position - scroll));
            shell.capture_event();

            return;
        }

        self.rows.as_widget_mut().update(
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
        self.rows.as_widget().mouse_interaction(
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
        self.rows.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<TableBody<'a, Message>> for Element<'a, Message> {
    fn from(value: TableBody<'a, Message>) -> Self {
        Element::new(value)
    }
}
