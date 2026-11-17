//! `HeadingRow`: a heading with its actions at the end of the line, an
//! optional lead before the heading, and a note after the heading that
//! moves under it when the line is too short.

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
/// Place of the note in [`HeadingRow::parts`], when there is one.
const NOTE: usize = 3;
/// Gap between the line of the heading and the note under it.
const NOTE_GAP: f32 = spacing::SPACE_1;

/// A heading, such as a name, the actions that go with it, such as a row
/// of buttons, an optional lead before the heading, such as a logo, and an
/// optional note, such as a muted plan and account.
///
/// The lead, the heading, the note and the actions of the first line share
/// one vertical centre. The actions always end the first line at the right
/// edge and keep their size. The note follows the heading on that line
/// while all of them fit; when they do not, the note moves to a line of
/// its own under the heading, starting where the heading starts, and the
/// lead and the actions stay on the line of the heading. The heading gets
/// the room the lead and the actions leave, so a part of it that may give
/// way, such as a name that ends with "…", is cut before an action is
/// covered.
///
/// Give it a heading and a note as wide as their content: a part that
/// fills its width leaves no room for the others.
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
///     HeadingRow::new(text("Claude Code · Work"), refresh)
///         .lead(text("*"))
///         .note(text("Max · user@example.com"))
///         .into();
/// ```
pub struct HeadingRow<'a, Message> {
    /// The lead at [`LEAD`], the heading at [`HEADING`], the actions at
    /// [`ACTIONS`] and the note at [`NOTE`], kept in one list so the
    /// events, the drawing and the menus go to all alike.
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

    /// Puts `note` after the heading, or under it when the line is too
    /// short for the heading, the note and the actions.
    pub fn note(mut self, note: impl Into<Element<'a, Message>>) -> Self {
        self.parts.truncate(NOTE);
        self.parts.push(note.into());
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
        let note = self.parts.get_mut(NOTE).map(|note| {
            note.as_widget_mut().layout(
                &mut tree.children[NOTE],
                renderer,
                &part_limits,
            )
        });

        let heading_size = heading.size();
        let note_size = note.as_ref().map_or(Size::ZERO, layout::Node::size);
        let note_room = match &note {
            Some(_) => note_size.width + self.spacing,
            None => 0.0,
        };
        let one_line_width = start
            + heading_size.width
            + note_room
            + self.spacing
            + actions_size.width;
        let is_one_line = one_line_width <= room;

        // With no limit to the width, the actions follow the heading.
        let end = if room.is_finite() {
            room
        } else {
            one_line_width
        };
        let first_line_height = lead_size
            .height
            .max(heading_size.height)
            .max(actions_size.height);
        let line_height = if is_one_line {
            first_line_height.max(note_size.height)
        } else {
            first_line_height
        };
        let middle = |height: f32| (line_height - height) / 2.0;

        let lead = lead.move_to((0.0, middle(lead_size.height)));
        let heading = heading.move_to((start, middle(heading_size.height)));
        let actions = actions
            .move_to((end - actions_size.width, middle(actions_size.height)));
        let mut children = vec![lead, heading, actions];
        let needed = match note {
            Some(note) if is_one_line => {
                let x = start + heading_size.width + self.spacing;
                children.push(note.move_to((x, middle(note_size.height))));
                Size::new(one_line_width, line_height)
            },
            Some(note) => {
                let under = line_height + NOTE_GAP;
                children.push(note.move_to((start, under)));
                let width =
                    (one_line_width - note_room).max(start + note_size.width);
                Size::new(width, under + note_size.height)
            },
            None => Size::new(one_line_width, line_height),
        };

        let size = limits.resolve(Length::Fill, Length::Shrink, needed);
        layout::Node::with_children(size, children)
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
    const NAME: &str = "Claude Code · Work";
    /// The note after the name.
    const NOTE: &str = "Max · user@example.com";

    /// A name that ends with "…" when it has less room than it needs.
    fn name() -> EllipsisText {
        EllipsisText::new(NAME, 20.0, fonts::UI, iced::Color::WHITE).shrink(0.0)
    }

    /// A heading row `width` wide: the name, the note when `has_note`, then
    /// Edit.
    fn line(width: f32, has_note: bool) -> Element<'static, Message> {
        let edit = button(text("Edit")).padding(0).on_press(Message::Edit);
        let mut heading = HeadingRow::new(name(), edit).spacing(GAP);
        if has_note {
            heading = heading.note(text(NOTE));
        }

        container(heading).width(width).into()
    }

    /// Where the name, the note and Edit are in a row `width` wide.
    fn parts(width: f32) -> (Rectangle, Rectangle, Rectangle) {
        let mut ui = simulator(line(width, true));
        let name = ui.find(NAME).expect("name").bounds();
        let note = ui.find(NOTE).expect("note").bounds();
        let edit = ui.find("Edit").expect("Edit").bounds();

        (name, note, edit)
    }

    /// The width the name, the note and Edit need on one line.
    fn one_line_width() -> f32 {
        let (name, note, edit) = parts(800.0);

        name.width + GAP + note.width + GAP + edit.width
    }

    #[test]
    fn given_room_for_all_when_laid_out_then_the_note_follows_the_heading_and_the_actions_end_the_line()
     {
        let width = one_line_width() + 40.0;

        let (name, note, edit) = parts(width);

        assert!(
            (edit.x + edit.width - width).abs() < 0.5,
            "Edit should end the line: {edit:?} in {width}"
        );
        assert!(
            (note.x - (name.x + name.width + GAP)).abs() < 0.5,
            "the note should follow the name: {note:?} after {name:?}"
        );
        for part in [note, edit] {
            assert!(
                (part.center_y() - name.center_y()).abs() < 1.0,
                "{part:?} should be on the line of the name {name:?}"
            );
        }
    }

    #[test]
    fn given_too_little_room_for_the_note_when_laid_out_then_the_note_moves_under_the_heading_and_the_actions_stay()
     {
        let (_, _, full) = parts(800.0);
        let width = one_line_width() - 1.0;

        let (name, note, edit) = parts(width);

        assert!(
            note.y >= name.y + name.height - 0.5,
            "the note should be under the name: {note:?} under {name:?}"
        );
        assert!(
            note.x.abs() < 0.5,
            "the note should start the line: {note:?}"
        );
        assert!(
            (edit.center_y() - name.center_y()).abs() < 1.0,
            "Edit should stay on the line of the name: {edit:?} and {name:?}"
        );
        assert!(
            (edit.x + edit.width - width).abs() < 0.5,
            "Edit should still end the line: {edit:?} in {width}"
        );
        assert!(
            (edit.width - full.width).abs() < 0.5,
            "Edit should keep its size: {edit:?} against {full:?}"
        );
    }

    #[test]
    fn given_less_room_than_the_heading_needs_when_laid_out_then_the_heading_gives_way_to_the_actions()
     {
        let (full_name, _, full_edit) = parts(800.0);
        let width = full_name.width / 2.0 + GAP + full_edit.width;

        let (name, _, edit) = parts(width);

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
    fn given_no_note_when_laid_out_then_the_actions_end_the_line_of_the_heading()
     {
        let mut ui = simulator(line(400.0, false));

        let name = ui.find(NAME).expect("name").bounds();
        let edit = ui.find("Edit").expect("Edit").bounds();

        assert!((edit.x + edit.width - 400.0).abs() < 0.5, "{edit:?}");
        assert!((edit.center_y() - name.center_y()).abs() < 1.0, "{edit:?}");
    }

    /// The lead before the name, such as a logo.
    const LEAD: &str = "Logo";

    /// Where the lead, the name and the note are in a row `width` wide with
    /// the lead, the name, the note and Edit.
    fn parts_with_lead(width: f32) -> (Rectangle, Rectangle, Rectangle) {
        let edit = button(text("Edit")).padding(0).on_press(Message::Edit);
        let heading = HeadingRow::new(name(), edit)
            .lead(text(LEAD).size(30))
            .note(text(NOTE))
            .spacing(GAP);
        let mut ui = simulator(container(heading).width(width));

        let lead = ui.find(LEAD).expect("lead").bounds();
        let name = ui.find(NAME).expect("name").bounds();
        let note = ui.find(NOTE).expect("note").bounds();

        (lead, name, note)
    }

    #[test]
    fn given_a_lead_and_room_for_all_when_laid_out_then_the_lead_the_heading_and_the_note_share_one_centre()
     {
        let (lead, name, note) = parts_with_lead(800.0);

        assert!(lead.x.abs() < 0.5, "the lead starts the line: {lead:?}");
        assert!(
            (name.x - (lead.x + lead.width + GAP)).abs() < 0.5,
            "the name follows the lead: {name:?} after {lead:?}"
        );
        for part in [lead, note] {
            assert!(
                (part.center_y() - name.center_y()).abs() < 1.0,
                "{part:?} should share the centre of the name {name:?}"
            );
        }
    }

    #[test]
    fn given_a_lead_and_too_little_room_for_the_note_when_laid_out_then_the_note_starts_under_the_heading_and_the_lead_stays_by_it()
     {
        let (lead, name, wide_note) = parts_with_lead(800.0);
        let (_, _, edit) = parts(800.0);
        let width = lead.width
            + GAP
            + name.width
            + GAP
            + wide_note.width
            + GAP
            + edit.width
            - 1.0;

        let (lead, name, note) = parts_with_lead(width);

        assert!(
            note.y >= name.y + name.height - 0.5,
            "the note should be under the name: {note:?} under {name:?}"
        );
        assert!(
            (note.x - name.x).abs() < 0.5,
            "the note should start where the name starts: {note:?} {name:?}"
        );
        assert!(
            (lead.center_y() - name.center_y()).abs() < 1.0,
            "the lead stays on the line of the name: {lead:?} {name:?}"
        );
    }

    #[test]
    fn given_a_narrow_row_when_the_actions_are_pressed_then_they_send_their_message()
     {
        let mut ui = simulator(line(one_line_width() - 1.0, true));

        ui.click("Edit").expect("Edit should show");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Edit]);
    }
}
