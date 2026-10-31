//! `StatusBar`: thin bar at the bottom of the window.

use iced::widget::{Space, button, container, row, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::target_dot::{TargetKind, target_dot};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Horizontal padding of the bar (`padding: 0 12px`).
const PADDING_X: f32 = spacing::SPACE_4;
/// Gap between items.
const ITEM_GAP: f32 = spacing::SPACE_4;
/// Gap between a dot and its name.
const DOT_GAP: f32 = spacing::SPACE_2;
/// Horizontal padding of a button item.
const BUTTON_PADDING_X: f32 = spacing::SPACE_2;

/// One entry of a [`StatusBar`].
pub enum StatusItem<Message> {
    /// Muted plain text.
    Text(String),
    /// Muted text in the mono font, for paths and values.
    Mono(String),
    /// Target dot followed by the target name.
    Target(TargetKind, String),
    /// Text button that sends `Message` on press.
    Button(String, Message),
}

impl<Message> StatusItem<Message> {
    /// Creates a plain text item.
    pub fn text(label: impl Into<String>) -> Self {
        Self::Text(label.into())
    }

    /// Creates a mono text item.
    pub fn mono(label: impl Into<String>) -> Self {
        Self::Mono(label.into())
    }

    /// Creates a target item: dot and name.
    pub fn target(kind: TargetKind, name: impl Into<String>) -> Self {
        Self::Target(kind, name.into())
    }

    /// Creates a button item.
    pub fn button(label: impl Into<String>, on_press: Message) -> Self {
        Self::Button(label.into(), on_press)
    }
}

/// Status bar: items on the left, a gap, items on the right.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};
/// use otty_ui_kit::components::target_dot::TargetKind;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     OpenSettings,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let bar: Element<'_, Message> = StatusBar::new(&palette)
///     .left(StatusItem::target(TargetKind::Local, "fedora"))
///     .right(StatusItem::button("Settings", Message::OpenSettings))
///     .into();
/// ```
pub struct StatusBar<Message> {
    palette: UiPalette,
    left: Vec<StatusItem<Message>>,
    right: Vec<StatusItem<Message>>,
}

impl<Message> StatusBar<Message> {
    /// Creates an empty bar.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    /// Adds an item to the left group.
    pub fn left(mut self, item: StatusItem<Message>) -> Self {
        self.left.push(item);
        self
    }

    /// Adds an item to the right group.
    pub fn right(mut self, item: StatusItem<Message>) -> Self {
        self.right.push(item);
        self
    }
}

impl<'a, Message: Clone + 'a> From<StatusBar<Message>>
    for Element<'a, Message>
{
    fn from(value: StatusBar<Message>) -> Self {
        let palette = value.palette;
        let background = palette.surface_chrome();

        let left = value
            .left
            .into_iter()
            .map(|item| item_view(item, &palette))
            .collect::<Vec<_>>();
        let right = value
            .right
            .into_iter()
            .map(|item| item_view(item, &palette))
            .collect::<Vec<_>>();

        let content = row![
            row(left).spacing(ITEM_GAP).align_y(Alignment::Center),
            Space::new().width(Length::Fill),
            row(right).spacing(ITEM_GAP).align_y(Alignment::Center),
        ]
        .align_y(Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(size::STATUSBAR)
            .padding(Padding::from([0.0, PADDING_X]))
            .align_y(Alignment::Center)
            .style(move |_| container::Style {
                background: Some(background.into()),
                ..container::Style::default()
            })
            .into()
    }
}

/// Draws one status item.
fn item_view<'a, Message: Clone + 'a>(
    item: StatusItem<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let muted = palette.text_muted();

    match item {
        StatusItem::Text(label) => caption(label, false, muted),
        StatusItem::Mono(label) => caption(label, true, muted),
        StatusItem::Target(kind, name) => row![
            target_dot(kind, palette),
            caption(name, false, palette.text_body()),
        ]
        .spacing(DOT_GAP)
        .align_y(Alignment::Center)
        .into(),
        StatusItem::Button(label, on_press) => {
            let hover = palette.surface_hover();
            let strong = palette.text_strong();

            button(caption(label, false, muted))
                .padding(Padding::from([0.0, BUTTON_PADDING_X]))
                .on_press(on_press)
                .style(move |_, status| {
                    let is_hovered = status == button::Status::Hovered;

                    button::Style {
                        background: is_hovered.then(|| hover.into()),
                        text_color: if is_hovered { strong } else { muted },
                        border: Border {
                            radius: radius::RADIUS_XS.into(),
                            ..Border::default()
                        },
                        ..button::Style::default()
                    }
                })
                .into()
        },
    }
}

/// Draws caption-size text in `color`.
fn caption<'a, Message: 'a>(
    content: String,
    is_mono: bool,
    color: iced::Color,
) -> Element<'a, Message> {
    let style = typography::CAPTION;

    text(content)
        .size(style.size())
        .line_height(iced::Pixels(style.line_height()))
        .font(if is_mono { fonts::MONO } else { fonts::UI })
        .color(color)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        OpenSettings,
    }

    #[test]
    fn given_button_item_when_clicked_then_its_message_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            StatusBar::new(&palette)
                .left(StatusItem::target(TargetKind::Local, "fedora"))
                .right(StatusItem::mono("~/projects"))
                .right(StatusItem::button("Settings", Message::OpenSettings)),
        );

        ui.click("Settings")
            .expect("button label should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::OpenSettings]);
    }
}
