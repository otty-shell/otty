//! `HeadingRow`: a heading with its actions at the end of the line and an
//! optional lead before the heading.

use iced::advanced::layout::{self, Layout, Limits};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget, overlay, renderer};
use iced::widget::Space;
use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse,
};

use crate::spacing;

/// Place of the lead in [`HeadingRow::parts`]: an empty space while there
/// is none.
const LEAD: usize = 0;
/// Place of the heading in [`HeadingRow::parts`].
const HEADING: usize = 1;
/// Place of the actions in [`HeadingRow::parts`].
const ACTIONS: usize = 2;

/// A heading, such as a name, the actions that go with it, such as a row
/// of buttons, and an optional lead before the heading, such as a logo.
///
/// The lead, the heading and the actions share one vertical centre. The
/// actions always end the line at the right edge and keep their size. The
/// heading gets the room the lead and the actions leave, so a part of it
/// that may give way, such as a name that ends with "…", is cut before an
/// action is covered.
///
/// Give it a heading as wide as its content: a heading that fills its
/// width leaves no room for the actions.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::button::Button;
/// use otty_ui_kit::components::heading_row::HeadingRow;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Refresh,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let refresh = Button::new("Refresh", &palette).on_press(Message::Refresh);
/// let line: Element<'_, Message> =
///     HeadingRow::new(text("Build server · Work"), refresh)
///         .lead(text("*"))
///         .into();
/// ```
pub struct HeadingRow<'a, Message> {
    /// The lead at [`LEAD`], the heading at [`HEADING`] and the actions at
    /// [`ACTIONS`], kept in one list so the events, the drawing and the
    /// menus go to all alike.
    parts: Vec<Element<'a, Message>>,
    has_lead: bool,
    spacing: f32,
}

impl<'a, Message: 'a> HeadingRow<'a, Message> {
    /// Puts `heading` at the start of the line and `actions` at its end.
    pub fn new(
        heading: impl Into<Element<'a, Message>>,
        actions: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            parts: vec![Space::new().into(), heading.into(), actions.into()],
            has_lead: false,
            spacing: spacing::SPACE_4,
        }
    }

    /// Puts `lead`, such as a logo, before the heading, on the line of the
    /// heading.
    pub fn lead(mut self, lead: impl Into<Element<'a, Message>>) -> Self {
        self.parts[LEAD] = lead.into();
        self.has_lead = true;
        self
    }

    /// Sets the least gap between the parts on one line.
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

        let actions = self.parts[ACTIONS].as_widget_mut().layout(
            &mut tree.children[ACTIONS],
            renderer,
            &part_limits,
        );
        let actions_size = actions.size();
        let lead = self.parts[LEAD].as_widget_mut().layout(
            &mut tree.children[LEAD],
            renderer,
            &part_limits,
        );
        let lead_size = lead.size();
        // Where the heading starts: after the lead and a gap, if any.
        let start = if self.has_lead {
            lead_size.width + self.spacing
        } else {
            0.0
        };
        let heading_room =
            (room - start - self.spacing - actions_size.width).max(0.0);
        let heading = self.parts[HEADING].as_widget_mut().layout(
            &mut tree.children[HEADING],
            renderer,
            &part_limits.max_width(heading_room),
        );
        let heading_size = heading.size();
        let line_width =
            start + heading_size.width + self.spacing + actions_size.width;

        // With no limit to the width, the actions follow the heading.
        let end = if room.is_finite() { room } else { line_width };
        let line_height = lead_size
            .height
            .max(heading_size.height)
            .max(actions_size.height);
        let middle = |height: f32| (line_height - height) / 2.0;

        let lead = lead.move_to((0.0, middle(lead_size.height)));
        let heading = heading.move_to((start, middle(heading_size.height)));
        let actions = actions
            .move_to((end - actions_size.width, middle(actions_size.height)));

        let needed = Size::new(line_width, line_height);
        let size = limits.resolve(Length::Fill, Length::Shrink, needed);
        layout::Node::with_children(size, vec![lead, heading, actions])
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
    use iced::widget::{button, container, text};
    use iced::{Element, Rectangle};
    use iced_test::simulator;

    use super::*;
    use crate::components::ellipsis_text::EllipsisText;
    use crate::fonts;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Edit,
    }

    /// Gap between the parts in the tests.
    const GAP: f32 = 12.0;
    /// The name of the heading.
    const NAME: &str = "Build server · Work";
    /// The lead before the name, such as a logo.
    const LEAD: &str = "Logo";

    /// A name that ends with "…" when it has less room than it needs.
    fn name() -> EllipsisText {
        EllipsisText::new(NAME, 20.0, fonts::UI, iced::Color::WHITE).shrink(0.0)
    }

    /// A heading row `width` wide: the lead when `has_lead`, the name, then
    /// Edit.
    fn line(width: f32, has_lead: bool) -> Element<'static, Message> {
        let edit = button(text("Edit")).padding(0).on_press(Message::Edit);
        let mut heading = HeadingRow::new(name(), edit).spacing(GAP);
        if has_lead {
            heading = heading.lead(text(LEAD).size(30));
        }

        container(heading).width(width).into()
    }

    /// Where the name and Edit are in a row `width` wide with no lead.
    fn parts(width: f32) -> (Rectangle, Rectangle) {
        let mut ui = simulator(line(width, false));
        let name = ui.find(NAME).expect("name").bounds();
        let edit = ui.find("Edit").expect("Edit").bounds();

        (name, edit)
    }

    #[test]
    fn given_room_for_all_when_laid_out_then_the_actions_end_the_line_of_the_heading()
     {
        let (name, edit) = parts(400.0);

        assert!((edit.x + edit.width - 400.0).abs() < 0.5, "{edit:?}");
        assert!((edit.center_y() - name.center_y()).abs() < 1.0, "{edit:?}");
    }

    #[test]
    fn given_less_room_than_the_heading_needs_when_laid_out_then_the_heading_gives_way_to_the_actions()
     {
        let (full_name, full_edit) = parts(800.0);
        let width = full_name.width / 2.0 + GAP + full_edit.width;

        let (name, edit) = parts(width);

        assert!(
            name.x + name.width <= edit.x - GAP + 0.5,
            "the name should end before Edit: {name:?} and {edit:?}"
        );
        assert!(
            (edit.width - full_edit.width).abs() < 0.5,
            "Edit should keep its size: {edit:?} against {full_edit:?}"
        );
        assert!(
            (edit.x + edit.width - width).abs() < 0.5,
            "Edit should end the line: {edit:?} in {width}"
        );
    }

    #[test]
    fn given_a_lead_when_laid_out_then_the_lead_starts_the_line_and_shares_the_centre_of_the_heading()
     {
        let mut ui = simulator(line(800.0, true));

        let lead = ui.find(LEAD).expect("lead").bounds();
        let name = ui.find(NAME).expect("name").bounds();

        assert!(lead.x.abs() < 0.5, "the lead starts the line: {lead:?}");
        assert!(
            (name.x - (lead.x + lead.width + GAP)).abs() < 0.5,
            "the name follows the lead: {name:?} after {lead:?}"
        );
        assert!(
            (lead.center_y() - name.center_y()).abs() < 1.0,
            "{lead:?} should share the centre of the name {name:?}"
        );
    }

    #[test]
    fn given_a_narrow_row_when_the_actions_are_pressed_then_they_send_their_message()
     {
        let (full_name, _) = parts(800.0);
        let mut ui = simulator(line(full_name.width / 2.0, true));

        ui.click("Edit").expect("Edit should show");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Edit]);
    }
}
