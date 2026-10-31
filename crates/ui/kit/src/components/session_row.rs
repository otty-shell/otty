//! `SessionRow`: one session under a group label.

use iced::widget::text::Wrapping;
use iced::widget::{button, container, mouse_area, row, text};
use iced::{Alignment, Border, Color, Element, Length, Padding};

use super::glyph;
use super::icon_button::{IconButton, IconButtonSize};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Gap between title, state and close (`gap: 8px`).
const GAP: f32 = spacing::SPACE_3;
/// Right padding of the row (`padding: 0 8px`).
const PADDING_RIGHT: f32 = spacing::SPACE_3;

/// Color of the state word of a [`SessionRow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTone {
    /// `text-muted`: "connecting", "editor".
    Muted,
    /// `status-warn`: the session waits for the user.
    Waiting,
    /// `status-ok`: the work is done.
    Done,
    /// `status-error`: the session failed.
    Failed,
}

impl SessionTone {
    /// Returns the color of this tone in `palette`.
    pub const fn color(self, palette: &UiPalette) -> Color {
        match self {
            Self::Muted => palette.text_muted(),
            Self::Waiting => palette.status_warn(),
            Self::Done => palette.status_ok(),
            Self::Failed => palette.status_error(),
        }
    }
}

/// Session row, indented to line up under its [`SessionGroup`] name.
///
/// The title is cut at the row edge: `iced` has no text ellipsis. The close
/// cross shows whenever `on_close` is set, so the caller decides for which
/// rows (for example the selected one).
///
/// [`SessionGroup`]: super::session_group::SessionGroup
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::session_row::{SessionRow, SessionTone};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Open,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let row: Element<'_, Message> = SessionRow::new("claude  Review", &palette)
///     .state("needs you", SessionTone::Waiting)
///     .on_press(Message::Open)
///     .into();
/// ```
pub struct SessionRow<Message> {
    title: String,
    palette: UiPalette,
    is_selected: bool,
    state: Option<(String, SessionTone)>,
    on_press: Option<Message>,
    on_right_press: Option<Message>,
    on_close: Option<Message>,
}

impl<Message> SessionRow<Message> {
    /// Creates a row with `title` only.
    pub fn new(title: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            title: title.into(),
            palette: palette.clone(),
            is_selected: false,
            state: None,
            on_press: None,
            on_right_press: None,
            on_close: None,
        }
    }

    /// Marks the row as the selected one.
    pub fn selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }

    /// Shows a state word after the title, such as "connecting".
    pub fn state(mut self, word: impl Into<String>, tone: SessionTone) -> Self {
        self.state = Some((word.into(), tone));
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the message sent on right press.
    pub fn on_right_press(mut self, message: Message) -> Self {
        self.on_right_press = Some(message);
        self
    }

    /// Shows a close cross that sends `message`.
    pub fn on_close(mut self, message: Message) -> Self {
        self.on_close = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<SessionRow<Message>>
    for Element<'a, Message>
{
    fn from(value: SessionRow<Message>) -> Self {
        let palette = value.palette;
        let is_selected = value.is_selected;
        let body = typography::BODY;
        let caption = typography::CAPTION;

        let mut content = row![
            container(
                text(value.title)
                    .size(body.size())
                    .font(fonts::UI)
                    .wrapping(Wrapping::None),
            )
            .width(Length::Fill)
            .clip(true),
        ]
        .spacing(GAP)
        .height(Length::Fill)
        .align_y(Alignment::Center);
        if let Some((word, tone)) = value.state {
            content = content.push(
                text(word)
                    .size(caption.size())
                    .font(fonts::UI)
                    .color(tone.color(&palette)),
            );
        }
        let has_close = value.on_close.is_some();
        if let Some(message) = value.on_close {
            content = content.push(
                IconButton::new(glyph::cross(), "Close session", &palette)
                    .size(IconButtonSize::Small)
                    .on_press(message),
            );
        }

        // The close cross sits inside the row button and catches its own
        // press, so it keeps a smaller right padding.
        let control = button(content)
            .width(Length::Fill)
            .height(size::ROW)
            .padding(Padding {
                top: 0.0,
                right: if has_close { 0.0 } else { PADDING_RIGHT },
                bottom: 0.0,
                left: size::SESSION_INDENT,
            })
            .on_press_maybe(value.on_press)
            .style(move |_, status| button::Style {
                background: if is_selected {
                    Some(palette.surface_selected().into())
                } else {
                    (status == button::Status::Hovered)
                        .then(|| palette.surface_raised().into())
                },
                text_color: if is_selected {
                    palette.text_strong()
                } else {
                    palette.text_body()
                },
                border: Border {
                    radius: radius::RADIUS_SM.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            });

        let mut area = mouse_area(control);
        if let Some(message) = value.on_right_press {
            area = area.on_right_press(message);
        }

        area.into()
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Open,
        Menu,
        Close,
    }

    fn row_view() -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();

        container(
            SessionRow::new("htop", &palette)
                .state("editor", SessionTone::Muted)
                .on_press(Message::Open)
                .on_right_press(Message::Menu)
                .on_close(Message::Close),
        )
        .width(260)
        .into()
    }

    #[test]
    fn given_row_when_title_pressed_then_press_is_sent() {
        let mut ui = simulator(row_view());

        ui.click("htop").expect("title should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Open]);
    }

    #[test]
    fn given_row_when_close_pressed_then_only_close_is_sent() {
        let mut ui = simulator(row_view());

        // Close cross is the last 24px of the 260px wide row.
        ui.point_at(Point::new(260.0 - 12.0, size::ROW / 2.0));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Close]);
    }

    #[test]
    fn given_row_when_right_pressed_then_menu_is_sent() {
        let mut ui = simulator(row_view());

        ui.point_at(Point::new(40.0, size::ROW / 2.0));
        let _ = ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Right,
            )),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Right,
            )),
        ]);

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Menu]);
    }

    #[test]
    fn given_each_tone_when_color_read_then_it_is_its_token() {
        let palette = UiPalette::otty_dark();

        assert_eq!(SessionTone::Muted.color(&palette), palette.text_muted());
        assert_eq!(SessionTone::Waiting.color(&palette), palette.status_warn());
        assert_eq!(SessionTone::Done.color(&palette), palette.status_ok());
        assert_eq!(SessionTone::Failed.color(&palette), palette.status_error());
    }
}
