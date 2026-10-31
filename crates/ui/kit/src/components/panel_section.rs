//! `PanelSection`: titled block of a side panel that opens and closes.

use iced::widget::{button, column, container, row, svg, text};
use iced::{Alignment, Element, Length, Padding};

use super::glyph;
use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Edge of the caret (`.sec-caret svg { width: 11px }`).
const CARET_SIZE: f32 = 11.0;
/// Gap between header items and between header and body (`gap: 1px`).
const BODY_GAP: f32 = 1.0;
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
            container(
                text(value.title)
                    .size(heading.size())
                    .font(fonts::UI_SEMIBOLD)
                    .color(palette.text_strong()),
            )
            .width(Length::Fill),
        ]
        .spacing(spacing::SPACE_2)
        .height(Length::Fill)
        .align_y(Alignment::Center);
        if let Some(note) = value.note {
            title = title.push(
                text(note).size(caption.size()).font(fonts::UI).color(muted),
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
