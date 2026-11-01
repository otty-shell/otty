//! Body rows of a [`DataTable`] that report right clicks and "⋯" presses
//! with their place.
//!
//! A context menu opens where the pointer is, so a right click must send
//! the row index and the pointer position. `iced` 0.14 `mouse_area` sends
//! neither, and inside a `scrollable` every child sees the pointer moved by
//! the scroll offset. [`TableBody`] sits directly inside the table's
//! `scrollable` and takes that offset back out.
//!
//! The "⋯" button of a row opens the same menu under the button, so its
//! press needs the button's place too, which a plain `button` cannot send.
//! [`TableBody`] finds the button in the row layout and sends its corner.
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
/// Message builder for a "⋯" press: row index and the bottom-left corner of
/// the button.
pub(super) type MenuPress<'a, Message> =
    Box<dyn Fn(usize, Point) -> Message + 'a>;

/// Wraps the column of table rows. Each child of the column is one row.
pub(super) struct TableBody<'a, Message> {
    rows: Element<'a, Message>,
    on_right_press: Option<RightPress<'a, Message>>,
    menu_rows: Vec<bool>,
    on_menu_press: Option<MenuPress<'a, Message>>,
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
            menu_rows: Vec::new(),
            on_menu_press: None,
        }
    }

    /// Sends `on_menu_press` when the "⋯" button of a row is pressed.
    /// `menu_rows` says, per row index, whether that row has the button.
    pub(super) fn row_menus(
        mut self,
        menu_rows: Vec<bool>,
        on_menu_press: MenuPress<'a, Message>,
    ) -> Self {
        self.menu_rows = menu_rows;
        self.on_menu_press = Some(on_menu_press);
        self
    }

    /// Returns the index and bounds of the "⋯" button under `position`.
    fn menu_button_at(
        &self,
        layout: Layout<'_>,
        position: Point,
    ) -> Option<(usize, Rectangle)> {
        self.on_menu_press.as_ref()?;

        layout
            .children()
            .enumerate()
            .filter(|(index, _)| self.menu_rows.get(*index) == Some(&true))
            .find_map(|(index, row)| {
                menu_button_bounds(row)
                    .filter(|bounds| bounds.contains(position))
                    .map(|bounds| (index, bounds))
            })
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
            let scroll = scroll_offset(layout, viewport);

            shell.publish(on_right_press(index, position - scroll));
            shell.capture_event();

            return;
        }

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) =
            event
            && let Some(on_menu_press) = &self.on_menu_press
            && let Some(position) = cursor.position_over(layout.bounds())
            && let Some((index, button)) = self.menu_button_at(layout, position)
        {
            let scroll = scroll_offset(layout, viewport);
            let corner = Point::new(button.x, button.y + button.height);

            shell.publish(on_menu_press(index, corner - scroll));
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
        if let Some(position) = cursor.position_over(layout.bounds())
            && self.menu_button_at(layout, position).is_some()
        {
            return mouse::Interaction::Pointer;
        }

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

/// Returns the scroll offset of the rows. The scrollable lays the rows out
/// from its own top-left corner and hands them a viewport and pointer moved
/// by the scroll offset; the difference of the two corners is that offset.
fn scroll_offset(layout: Layout<'_>, viewport: &Rectangle) -> Vector {
    viewport.position() - layout.bounds().position()
}

/// Returns the bounds of the "⋯" button in the layout of one table `row`.
/// The row is a column of the row button and its rule; the button holds the
/// row of cells, and the last cell holds the "⋯" button (see
/// `data_table::row_view`).
fn menu_button_bounds(row: Layout<'_>) -> Option<Rectangle> {
    let control = row.children().next()?;
    let cells = control.children().next()?;
    let menu_cell = cells.children().last()?;
    let button = menu_cell.children().next()?;

    Some(button.bounds())
}
