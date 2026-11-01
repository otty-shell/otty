//! `ActionTile`: large row button of the empty card.

use iced::widget::{button, container, row, svg, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::kbd::kbd;
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Edge of the icon (`width: 16px; height: 16px`).
const ICON_SIZE: f32 = 16.0;
/// Gap and side padding of the tile (`gap: space-4`, `padding: 0 space-4`).
const GAP: f32 = spacing::SPACE_4;

/// Large button with an icon, a label and an optional shortcut hint.
///
/// The first tile of a list may use the accent color for its icon.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::action_tile::ActionTile;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     NewTerminal,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let tile: Element<'_, Message> =
///     ActionTile::new(icon, "New terminal", &palette)
///         .shortcut("Ctrl T")
///         .accent(true)
///         .on_press(Message::NewTerminal)
///         .into();
/// ```
pub struct ActionTile<Message> {
    icon: svg::Handle,
    label: String,
    palette: UiPalette,
    shortcut: Option<String>,
    note: Option<String>,
    is_accent: bool,
    on_press: Option<Message>,
}

impl<Message> ActionTile<Message> {
    /// Creates a tile with an `icon` and a `label`.
    pub fn new(
        icon: svg::Handle,
        label: impl Into<String>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            icon,
            label: label.into(),
            palette: palette.clone(),
            shortcut: None,
            note: None,
            is_accent: false,
            on_press: None,
        }
    }

    /// Shows a shortcut hint at the right edge.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Shows a muted mono note at the right edge, such as a site name.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Draws the icon in the accent color instead of the text color.
    pub fn accent(mut self, is_accent: bool) -> Self {
        self.is_accent = is_accent;
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<ActionTile<Message>>
    for Element<'a, Message>
{
    fn from(value: ActionTile<Message>) -> Self {
        let palette = value.palette;
        let strong = palette.text_strong();
        let icon_color = if value.is_accent {
            palette.accent()
        } else {
            strong
        };
        let body = typography::BODY;

        let mut content = row![
            svg(value.icon).width(ICON_SIZE).height(ICON_SIZE).style(
                move |_, _| svg::Style {
                    color: Some(icon_color),
                }
            ),
            container(
                text(value.label)
                    .size(body.size())
                    .font(fonts::UI)
                    .color(strong),
            )
            .width(Length::Fill),
        ]
        .spacing(GAP)
        .height(Length::Fill)
        .align_y(Alignment::Center);
        if let Some(note) = value.note {
            content = content.push(
                text(note)
                    .size(typography::SMALL.size())
                    .font(fonts::MONO)
                    .color(palette.text_muted()),
            );
        }
        if let Some(shortcut) = value.shortcut {
            content = content.push(kbd(shortcut, &palette));
        }

        button(content)
            .width(Length::Fill)
            .height(size::ACTION_TILE)
            .padding(Padding::from([0.0, GAP]))
            .on_press_maybe(value.on_press)
            .style(move |_, status| button::Style {
                background: Some(
                    if matches!(
                        status,
                        button::Status::Hovered | button::Status::Pressed
                    ) {
                        palette.surface_hover()
                    } else {
                        palette.surface_raised()
                    }
                    .into(),
                ),
                text_color: strong,
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
        NewTerminal,
    }

    #[test]
    fn given_action_tile_when_label_pressed_then_its_message_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            ActionTile::new(glyph::plus(), "New terminal", &palette)
                .shortcut("Ctrl T")
                .accent(true)
                .on_press(Message::NewTerminal),
        );

        ui.click("New terminal").expect("label should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::NewTerminal]);
    }

    #[test]
    fn given_action_tile_with_note_then_the_note_is_shown() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            ActionTile::<Message>::new(
                glyph::plus(),
                "Install Docker",
                &palette,
            )
            .note("docs.docker.com"),
        );

        assert!(ui.find("Install Docker").is_ok());
        assert!(ui.find("docs.docker.com").is_ok());
    }

    #[test]
    fn given_action_tile_when_laid_out_then_label_is_centered_in_44px_height() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            ActionTile::<Message>::new(glyph::plus(), "New terminal", &palette)
                .shortcut("Ctrl T"),
        );

        let bounds = ui
            .find("New terminal")
            .expect("label should be visible")
            .bounds();

        let center = bounds.y + bounds.height / 2.0;
        assert!(
            (center - size::ACTION_TILE / 2.0).abs() < 1.0,
            "label center {center} should be at half of {}",
            size::ACTION_TILE
        );
    }

    #[test]
    fn given_action_tile_without_message_when_pressed_then_nothing_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(ActionTile::<Message>::new(
            glyph::plus(),
            "New launch…",
            &palette,
        ));

        ui.click("New launch…").expect("label should be visible");

        assert_eq!(ui.into_messages().count(), 0);
    }
}
