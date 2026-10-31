//! `TitleSearch`: title bar button that looks like a search field.

use iced::widget::text::Wrapping;
use iced::widget::{button, container, row, svg, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::kbd::kbd;
use crate::palette::UiPalette;
use crate::{fonts, radius, size};

/// Edge of the search icon (`width: 13px; height: 13px`).
const ICON_SIZE: f32 = 13.0;
/// Gap between icon, title and shortcut (`gap: 10px`).
const GAP: f32 = 10.0;
/// Side padding of the button (`padding: 0 10px`).
const PADDING_X: f32 = 10.0;
/// Font size of the title (`font-size: 12.5px`).
const FONT_SIZE: f32 = 12.5;

/// Button that opens the command palette. It shows a search icon, the title
/// of the focused session and the palette shortcut. The whole button sends
/// one message.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::title_search::TitleSearch;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     OpenPalette,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let search: Element<'_, Message> =
///     TitleSearch::new("zsh  ~/projects/otty", "Ctrl K", icon, &palette)
///         .on_press(Message::OpenPalette)
///         .into();
/// ```
pub struct TitleSearch<Message> {
    title: String,
    shortcut: String,
    icon: svg::Handle,
    palette: UiPalette,
    on_press: Option<Message>,
}

impl<Message> TitleSearch<Message> {
    /// Creates the button with the session `title`, the palette `shortcut`
    /// text and the search `icon`.
    pub fn new(
        title: impl Into<String>,
        shortcut: impl Into<String>,
        icon: svg::Handle,
        palette: &UiPalette,
    ) -> Self {
        Self {
            title: title.into(),
            shortcut: shortcut.into(),
            icon,
            palette: palette.clone(),
            on_press: None,
        }
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<TitleSearch<Message>>
    for Element<'a, Message>
{
    fn from(value: TitleSearch<Message>) -> Self {
        let palette = value.palette;
        let muted = palette.text_muted();
        let background = palette.surface_raised();

        let content = row![
            svg(value.icon)
                .width(ICON_SIZE)
                .height(ICON_SIZE)
                .style(move |_, _| svg::Style { color: Some(muted) }),
            container(
                text(value.title)
                    .size(FONT_SIZE)
                    .font(fonts::UI)
                    .wrapping(Wrapping::None),
            )
            .width(Length::Fill)
            .clip(true),
            kbd(value.shortcut, &palette),
        ]
        .spacing(GAP)
        .align_y(Alignment::Center);

        button(content)
            .width(size::TITLE_SEARCH_WIDTH)
            .height(size::TITLE_SEARCH_HEIGHT)
            .padding(Padding::from([0.0, PADDING_X]))
            .on_press_maybe(value.on_press)
            .style(move |_, _| button::Style {
                background: Some(background.into()),
                text_color: muted,
                border: Border {
                    radius: radius::RADIUS_SM.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::components::glyph;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        OpenPalette,
    }

    #[test]
    fn given_title_search_when_title_pressed_then_one_message_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            TitleSearch::new(
                "zsh  ~/projects/otty",
                "Ctrl K",
                glyph::plus(),
                &palette,
            )
            .on_press(Message::OpenPalette),
        );

        ui.click("zsh  ~/projects/otty")
            .expect("title should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::OpenPalette]);
    }

    #[test]
    fn given_title_search_when_shortcut_hint_pressed_then_one_message_is_sent()
    {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            TitleSearch::new("zsh", "Ctrl K", glyph::plus(), &palette)
                .on_press(Message::OpenPalette),
        );

        ui.click("Ctrl K").expect("shortcut should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::OpenPalette]);
    }
}
