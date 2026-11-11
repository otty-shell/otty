//! `PanelSection`: titled block of a side panel that opens and closes.

use iced::widget::text::Alignment as TextAlignment;
use iced::widget::{button, column, container, row, svg};
use iced::{Alignment, Element, Length, Padding};

use super::ellipsis_text::EllipsisText;
use super::glyph;
use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Edge of the caret (`.sec-caret svg { width: 11px }`).
const CARET_SIZE: f32 = 11.0;
/// Gap between header items and between header and body (`gap: 1px`).
pub(crate) const BODY_GAP: f32 = 1.0;
/// Height of a closed section: the header row and the padding around it.
pub(crate) const HEADER_HEIGHT: f32 =
    spacing::SPACE_3 + size::ROW + spacing::SPACE_3;
/// Most actions a header can hold.
const MAX_ACTIONS: usize = 3;

/// Block of a side panel: header with caret, title, optional count or note
/// and up to three actions; the body shows only while the section is open.
///
/// A section marked with [`PanelSection::grow`] takes the height left in its
/// column and cuts its body at the edge.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::panel_section::PanelSection;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Toggle,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let section: Element<'_, Message> = PanelSection::new("Sessions", &palette)
///     .open(true)
///     .count(3)
///     .body(text("rows go here"))
///     .on_toggle(Message::Toggle)
///     .into();
/// ```
pub struct PanelSection<'a, Message> {
    title: String,
    palette: UiPalette,
    is_open: bool,
    is_grow: bool,
    note: Option<String>,
    actions: Vec<Element<'a, Message>>,
    body: Option<Element<'a, Message>>,
    on_toggle: Option<Message>,
}

impl<'a, Message> PanelSection<'a, Message> {
    /// Creates a closed section with `title`.
    pub fn new(title: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            title: title.into(),
            palette: palette.clone(),
            is_open: false,
            is_grow: false,
            note: None,
            actions: Vec::new(),
            body: None,
            on_toggle: None,
        }
    }

    /// Opens or closes the section.
    pub fn open(mut self, is_open: bool) -> Self {
        self.is_open = is_open;
        self
    }

    /// Makes the section take the remaining height of its column.
    pub fn grow(mut self, is_grow: bool) -> Self {
        self.is_grow = is_grow;
        self
    }

    /// Shows a number of items after the title.
    pub fn count(mut self, count: usize) -> Self {
        self.note = Some(count.to_string());
        self
    }

    /// Shows a short muted note after the title.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Adds an action to the right of the header, usually a small
    /// `IconButton`. A header holds three actions; more are ignored.
    pub fn action(mut self, action: impl Into<Element<'a, Message>>) -> Self {
        if self.actions.len() < MAX_ACTIONS {
            self.actions.push(action.into());
        }
        self
    }

    /// Sets the content shown while the section is open.
    pub fn body(mut self, body: impl Into<Element<'a, Message>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Sets the message sent when the header title is pressed.
    pub fn on_toggle(mut self, message: Message) -> Self {
        self.on_toggle = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<PanelSection<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: PanelSection<'a, Message>) -> Self {
        let palette = value.palette;
        let heading = typography::HEADING;
        let caption = typography::CAPTION;
        let muted = palette.text_muted();

        let mut title = row![
            svg(glyph::caret(value.is_open))
                .width(CARET_SIZE)
                .height(CARET_SIZE)
                .style(move |_, _| svg::Style { color: Some(muted) }),
            // The title takes the room it needs first; the note on the right
            // takes what is left and ends with "…" when that is too little.
            EllipsisText::new(
                value.title,
                heading.size(),
                fonts::UI_SEMIBOLD,
                palette.text_strong(),
            )
            .shrink(0.0),
        ]
        .spacing(spacing::SPACE_2)
        .height(Length::Fill)
        .align_y(Alignment::Center);
        if let Some(note) = value.note {
            title = title.push(
                EllipsisText::new(note, caption.size(), fonts::UI, muted)
                    .align_x(TextAlignment::Right),
            );
        }

        let toggle = button(title)
            .width(Length::Fill)
            .height(size::ROW)
            .padding(0)
            .on_press_maybe(value.on_toggle)
            .style(|_, _| button::Style::default());

        let mut header = row![toggle]
            .spacing(spacing::SPACE_2)
            .height(size::ROW)
            .align_y(Alignment::Center);
        for action in value.actions {
            header = header.push(action);
        }
        let header = container(header).padding(Padding {
            top: 0.0,
            right: spacing::SPACE_1,
            bottom: 0.0,
            left: spacing::SPACE_2,
        });

        let mut section = column![header].spacing(BODY_GAP);
        if let Some(body) = value.body.filter(|_| value.is_open) {
            section = section.push(if value.is_grow {
                container(body).height(Length::Fill).clip(true)
            } else {
                container(body)
            });
        }

        let section = container(section).padding(spacing::SPACE_3);
        if value.is_grow {
            section.height(Length::Fill).clip(true).into()
        } else {
            section.into()
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced::widget::text;
    use iced_test::simulator;

    use super::*;
    use crate::components::icon_button::{IconButton, IconButtonSize};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Toggle,
        Add,
    }

    fn section(is_open: bool) -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();
        let add = IconButton::new(glyph::plus(), "Add", &palette)
            .size(IconButtonSize::Small)
            .on_press(Message::Add);

        container(
            PanelSection::new("Sessions", &palette)
                .open(is_open)
                .count(2)
                .action(add)
                .body(text("first row"))
                .on_toggle(Message::Toggle),
        )
        .width(260)
        .into()
    }

    #[test]
    fn given_header_when_title_pressed_then_toggle_is_sent() {
        let mut ui = simulator(section(true));

        ui.click("Sessions").expect("title should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Toggle]);
    }

    #[test]
    fn given_header_when_action_pressed_then_only_the_action_is_sent() {
        let mut ui = simulator(section(true));

        // Section padding 8, header right padding 4, action 24 wide.
        ui.point_at(Point::new(260.0 - 8.0 - 4.0 - 12.0, 8.0 + 15.0));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Add]);
    }

    #[test]
    fn given_open_state_when_viewed_then_body_shows_only_while_open() {
        let mut open = simulator(section(true));
        let mut closed = simulator(section(false));

        assert!(open.find("first row").is_ok());
        assert!(closed.find("first row").is_err());
    }

    /// Width of the action used by the narrow header tests.
    const ACTION_WIDTH: f32 = 30.0;
    /// Padding on the left and right of the header row of the section.
    const HEADER_PADDING: f32 = 8.0 + 4.0;

    /// A fixed width action whose "+" sits at the left edge of the button.
    fn plus_action() -> Element<'static, Message> {
        iced::widget::button(container(text("+")).width(Length::Fill))
            .width(ACTION_WIDTH)
            .padding(0)
            .on_press(Message::Add)
            .into()
    }

    /// Bounds of the title, of the note and of the action of a header in a
    /// space `width` wide.
    fn header_bounds(
        width: f32,
    ) -> (iced::Rectangle, iced::Rectangle, iced::Rectangle) {
        let palette = UiPalette::otty_dark();
        let section: Element<'_, Message> =
            PanelSection::new("Sessions", &palette)
                .note("this computer")
                .action(plus_action())
                .on_toggle(Message::Toggle)
                .into();
        let mut ui = simulator(container(section).width(width));
        let title = ui.find("Sessions").expect("title").bounds();
        let note = ui.find("this computer").expect("note").bounds();
        let action = ui.find("+").expect("action").bounds();

        (title, note, action)
    }

    #[test]
    fn given_narrow_header_when_laid_out_then_the_note_gives_way_before_the_title()
     {
        let (full_title, full_note, _) = header_bounds(600.0);

        let (title, note, action) = header_bounds(190.0);

        assert!(
            (title.width - full_title.width).abs() < 0.5,
            "the title should keep its width: {title:?} against {full_title:?}"
        );
        assert!(
            note.width < full_note.width,
            "the note should shrink: {note:?} against {full_note:?}"
        );
        assert!(
            (note.height - full_note.height).abs() < 0.5,
            "the note should stay one line: {note:?}"
        );
        assert!(note.x >= title.x + title.width, "the note must not overlap");
        assert!(
            note.x + note.width <= action.x + 0.5,
            "the note must not reach the action: {note:?} and {action:?}"
        );
    }

    #[test]
    fn given_minimum_panel_width_when_laid_out_then_title_stays_clear_of_action()
     {
        let (full_title, _, _) = header_bounds(600.0);

        let (title, note, action) = header_bounds(128.0);

        assert!(
            (title.height - full_title.height).abs() < 0.5,
            "the title should stay one line: {title:?}"
        );
        assert!(
            title.x + title.width <= action.x + 0.5,
            "the title must not reach the action: {title:?} and {action:?}"
        );
        assert!(
            note.x + note.width <= action.x + 0.5,
            "note clear of action"
        );
        assert!(
            (action.x - (128.0 - HEADER_PADDING - ACTION_WIDTH)).abs() < 0.5,
            "the action keeps its size and place: {action:?}"
        );
    }

    #[test]
    fn given_very_narrow_header_when_laid_out_then_the_title_ends_with_an_ellipsis()
     {
        let (full_title, _, _) = header_bounds(600.0);

        let (title, _, action) = header_bounds(100.0);

        assert!(
            title.width < full_title.width,
            "the title should be cut: {title:?} against {full_title:?}"
        );
        assert!(title.x + title.width <= action.x + 0.5, "title clear");
    }

    #[test]
    fn given_narrow_header_when_action_pressed_then_only_the_action_is_sent() {
        let palette = UiPalette::otty_dark();
        let section: Element<'_, Message> =
            PanelSection::new("Sessions", &palette)
                .action(plus_action())
                .on_toggle(Message::Toggle)
                .into();
        let mut ui = simulator(container(section).width(128));

        ui.click("+").expect("action should show");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Add]);
    }

    #[test]
    fn given_four_actions_when_added_then_only_three_are_kept() {
        let palette = UiPalette::otty_dark();
        let mut section = PanelSection::<Message>::new("Files", &palette);
        for _ in 0..4 {
            section = section.action(text("x"));
        }

        assert_eq!(section.actions.len(), 3);
    }
}
