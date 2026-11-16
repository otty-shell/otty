//! `HeadingRow`: a heading with its actions at the end of the line, and the
//! actions on a line of their own when both do not fit.

use iced::advanced::layout::{self, Layout, Limits};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget, overlay, renderer};
use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse,
};

use crate::spacing;

/// Place of the heading in [`HeadingRow::parts`].
const HEADING: usize = 0;
/// Place of the actions in [`HeadingRow::parts`].
const ACTIONS: usize = 1;

/// A heading, such as a title with its badge, and the actions that go with
/// it, such as a row of buttons. While both fit on one line the actions end
/// the line at the right edge; when they do not, the actions move to a line
/// of their own under the heading, at the left edge. The actions keep their
/// size and never cover the heading.
///
/// Give it a heading as wide as its content: a heading that fills its width
/// never leaves room for the actions. A part of the heading that may give
/// way, such as a note that ends with "…", gets the room the heading has
/// once the actions moved.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::{row, text};
/// use otty_ui_kit::components::button::Button;
/// use otty_ui_kit::components::heading_row::HeadingRow;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Edit,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let heading = row![text("Claude Code · Work"), text("connected")];
/// let edit = Button::new("Edit", &palette).on_press(Message::Edit);
/// let line: Element<'_, Message> = HeadingRow::new(heading, edit).into();
/// ```
pub struct HeadingRow<'a, Message> {
    /// The heading at [`HEADING`] and the actions at [`ACTIONS`], kept in
    /// one array so the events, the drawing and the menus go to both alike.
    parts: [Element<'a, Message>; 2],
    spacing: f32,
}

impl<'a, Message> HeadingRow<'a, Message> {
    /// Puts `heading` at the start of the line and `actions` at its end.
    pub fn new(
        heading: impl Into<Element<'a, Message>>,
        actions: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            parts: [heading.into(), actions.into()],
            spacing: spacing::SPACE_4,
        }
    }

    /// Sets the least gap between the heading and the actions on one line,
    /// and the gap between the two lines.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

impl<Message> Widget<Message, Theme, Renderer> for HeadingRow<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.parts.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.parts);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits,
    ) -> layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Shrink);
        let room = limits.max().width;
        let part_limits = limits.loose();

        let [heading, actions] = &mut self.parts;
        let heading = heading.as_widget_mut().layout(
            &mut tree.children[HEADING],
            renderer,
            &part_limits,
        );
        let actions = actions.as_widget_mut().layout(
            &mut tree.children[ACTIONS],
            renderer,
            &part_limits,
        );
        let heading_size = heading.size();
        let actions_size = actions.size();

        let one_line_width =
            heading_size.width + self.spacing + actions_size.width;
        let (heading, actions, needed) = if one_line_width <= room {
            let height = heading_size.height.max(actions_size.height);
            // With no limit to the width, the actions follow the heading.
            let end = if room.is_finite() {
                room
            } else {
                one_line_width
            };

            (
                heading.move_to((0.0, (height - heading_size.height) / 2.0)),
                actions.move_to((
                    end - actions_size.width,
                    (height - actions_size.height) / 2.0,
                )),
                Size::new(one_line_width, height),
            )
        } else {
            let under = heading_size.height + self.spacing;
            let width = heading_size.width.max(actions_size.width);

            (
                heading,
                actions.move_to((0.0, under)),
                Size::new(width, under + actions_size.height),
            )
        };

        let size = limits.resolve(Length::Fill, Length::Shrink, needed);
        layout::Node::with_children(size, vec![heading, actions])
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
        for ((part, tree), layout) in
            self.parts.iter().zip(&tree.children).zip(layout.children())
        {
            part.as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((part, tree), layout) in self
                .parts
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                part.as_widget_mut()
                    .operate(tree, layout, renderer, operation);
            }
        });
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
        for ((part, tree), layout) in self
            .parts
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            part.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell,
                viewport,
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
        self.parts
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((part, tree), layout)| {
                part.as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.parts,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<HeadingRow<'a, Message>> for Element<'a, Message> {
    fn from(value: HeadingRow<'a, Message>) -> Self {
        Element::new(value)
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::{button, container, row, text};
    use iced::{Alignment, Element, Rectangle};
    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Edit,
    }

    /// Gap between the parts in the tests.
    const GAP: f32 = 12.0;

    /// A heading row `width` wide: a name and a state, then Edit.
    fn line(width: f32) -> Element<'static, Message> {
        let heading = row![text("Claude Code · Work"), text("connected")]
            .spacing(GAP)
            .align_y(Alignment::Center);
        let edit = button(text("Edit")).padding(0).on_press(Message::Edit);

        container(HeadingRow::new(heading, edit).spacing(GAP))
            .width(width)
            .into()
    }

    /// Where the name, the state and Edit are in a row `width` wide.
    fn parts(width: f32) -> (Rectangle, Rectangle, Rectangle) {
        let mut ui = simulator(line(width));
        let name = ui.find("Claude Code · Work").expect("name").bounds();
        let state = ui.find("connected").expect("state").bounds();
        let edit = ui.find("Edit").expect("Edit").bounds();

        (name, state, edit)
    }

    /// The width the heading and Edit need on one line.
    fn one_line_width() -> f32 {
        let (name, state, edit) = parts(800.0);

        (state.x + state.width - name.x) + GAP + edit.width
    }

    #[test]
    fn given_room_for_both_when_laid_out_then_the_actions_end_the_line() {
        let width = one_line_width() + 40.0;

        let (name, state, edit) = parts(width);

        assert!(
            (edit.x + edit.width - width).abs() < 0.5,
            "Edit should end the line: {edit:?} in {width}"
        );
        assert!(
            (edit.center_y() - name.center_y()).abs() < 1.0,
            "Edit should be on the line of the name: {edit:?} and {name:?}"
        );
        assert!(edit.x > state.x + state.width, "{edit:?} after {state:?}");
    }

    #[test]
    fn given_too_little_room_when_laid_out_then_the_actions_move_under_the_heading_and_keep_their_size()
     {
        let (_, _, full) = parts(800.0);
        let width = one_line_width() - 1.0;

        let (name, state, edit) = parts(width);

        assert!(
            (state.center_y() - name.center_y()).abs() < 1.0,
            "the heading should stay on one line: {name:?} and {state:?}"
        );
        assert!(
            edit.y >= name.y + name.height + GAP - 0.5,
            "Edit should be under the heading: {edit:?} under {name:?}"
        );
        assert!(edit.x.abs() < 0.5, "Edit should start the line: {edit:?}");
        assert!(
            (edit.width - full.width).abs() < 0.5,
            "Edit should keep its size: {edit:?} against {full:?}"
        );
    }

    #[test]
    fn given_actions_on_their_own_line_when_pressed_then_they_send_their_message()
     {
        let mut ui = simulator(line(one_line_width() - 1.0));

        ui.click("Edit").expect("Edit should show");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Edit]);
    }
}
