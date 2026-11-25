//! `StatusBar`: thin bar at the bottom of the window.

use iced::widget::{Space, button, container, row, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::ellipsis_text::text_width;
use super::kbd::{kbd, kbd_width};
use super::target_dot::{TargetKind, target_dot};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Horizontal padding of the bar (`padding: 0 12px`).
const PADDING_X: f32 = spacing::SPACE_4;
/// Gap between items (`gap: 18px`).
const ITEM_GAP: f32 = size::STATUS_GAP;
/// Gap between a dot and its name (`.ot-status-target { gap: 6px }`), and
/// between a text and its key hints.
const DOT_GAP: f32 = spacing::SPACE_2;
/// Horizontal padding of a button item (`padding: 0 8px`).
const BUTTON_PADDING_X: f32 = spacing::SPACE_3;

/// One entry of a [`StatusBar`].
pub enum StatusItem<Message> {
    /// Muted plain text.
    Text(String),
    /// Muted text in the mono font, for paths and values.
    Mono(String),
    /// Target dot followed by the target name; `bool` is true for the plain
    /// variant that uses the body text color.
    Target(TargetKind, String, bool),
    /// Text button that sends `Message` on press.
    Button(String, Message),
    /// Text in the body text color followed by the keys that act on it,
    /// each as a `Kbd` hint, such as `Block 5 of 6` with `Ctrl Shift ↑`
    /// and `Ctrl Shift ↓`.
    WithKeys(String, Vec<String>),
}

impl<Message> StatusItem<Message> {
    /// Returns the width the item takes on one line, measured with the
    /// font and the size it is drawn with.
    ///
    /// # Example
    ///
    /// ```
    /// use otty_ui_kit::components::status_bar::StatusItem;
    ///
    /// let counter: StatusItem<()> =
    ///     StatusItem::with_keys("Block 5 of 6", ["Ctrl Shift ↑"]);
    /// let text: StatusItem<()> = StatusItem::text("Block 5 of 6");
    ///
    /// assert!(counter.width() > text.width());
    /// ```
    pub fn width(&self) -> f32 {
        match self {
            Self::Text(label) => caption_width(label, false),
            Self::Mono(label) => caption_width(label, true),
            Self::Target(_, name, _) => {
                size::DOT + DOT_GAP + caption_width(name, false)
            },
            Self::Button(label, _) => {
                caption_width(label, false) + 2.0 * BUTTON_PADDING_X
            },
            Self::WithKeys(label, keys) => {
                let keys: f32 =
                    keys.iter().map(|key| DOT_GAP + kbd_width(key)).sum();

                caption_width(label, false) + keys
            },
        }
    }

    /// Creates a plain text item.
    pub fn text(label: impl Into<String>) -> Self {
        Self::Text(label.into())
    }

    /// Creates a mono text item.
    pub fn mono(label: impl Into<String>) -> Self {
        Self::Mono(label.into())
    }

    /// Creates a target item: dot and muted name.
    pub fn target(kind: TargetKind, name: impl Into<String>) -> Self {
        Self::Target(kind, name.into(), false)
    }

    /// Creates a target item whose name uses the body text color.
    pub fn plain_target(kind: TargetKind, name: impl Into<String>) -> Self {
        Self::Target(kind, name.into(), true)
    }

    /// Creates a button item.
    pub fn button(label: impl Into<String>, on_press: Message) -> Self {
        Self::Button(label.into(), on_press)
    }

    /// Creates an item of body text followed by the hints of `keys`.
    ///
    /// # Example
    ///
    /// ```
    /// use otty_ui_kit::components::status_bar::StatusItem;
    ///
    /// let item: StatusItem<()> = StatusItem::with_keys(
    ///     "Block 5 of 6",
    ///     ["Ctrl Shift ↑", "Ctrl Shift ↓"],
    /// );
    ///
    /// assert!(matches!(item, StatusItem::WithKeys(_, keys) if keys.len() == 2));
    /// ```
    pub fn with_keys<Key: Into<String>>(
        label: impl Into<String>,
        keys: impl IntoIterator<Item = Key>,
    ) -> Self {
        Self::WithKeys(label.into(), keys.into_iter().map(Into::into).collect())
    }

    /// Turns the message of a button item into another message, so an item
    /// built with the messages of one part of an app can go into a bar of
    /// the whole app. Other items stay as they are.
    ///
    /// # Example
    ///
    /// ```
    /// use otty_ui_kit::components::status_bar::StatusItem;
    ///
    /// enum AppMessage {
    ///     Jobs(u32),
    /// }
    ///
    /// let item = StatusItem::button("Jobs", 3).map(AppMessage::Jobs);
    ///
    /// assert!(matches!(
    ///     item,
    ///     StatusItem::Button(label, AppMessage::Jobs(3)) if label == "Jobs"
    /// ));
    /// ```
    pub fn map<Other>(
        self,
        f: impl FnOnce(Message) -> Other,
    ) -> StatusItem<Other> {
        match self {
            Self::Text(label) => StatusItem::Text(label),
            Self::Mono(label) => StatusItem::Mono(label),
            Self::Target(kind, name, is_plain) => {
                StatusItem::Target(kind, name, is_plain)
            },
            Self::Button(label, message) => {
                StatusItem::Button(label, f(message))
            },
            Self::WithKeys(label, keys) => StatusItem::WithKeys(label, keys),
        }
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
    /// Returns the width the bar needs to show all its items on one line,
    /// so a caller can decide which items fit.
    ///
    /// # Example
    ///
    /// ```
    /// use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};
    /// use otty_ui_kit::palette::UiPalette;
    ///
    /// let palette = UiPalette::otty_dark();
    /// let bar: StatusBar<()> = StatusBar::new(&palette)
    ///     .left(StatusItem::text("~/otty"))
    ///     .right(StatusItem::text("OTTY 0.4.0"));
    ///
    /// assert!(bar.width() > 0.0);
    /// ```
    pub fn width(&self) -> f32 {
        2.0 * PADDING_X + group_width(&self.left) + group_width(&self.right)
    }

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

    /// Adds `items` to the left group, from the first, while the bar fits
    /// in `available` width with a gap between its groups. The first item
    /// that does not fit and all after it are left out, whole, so in a
    /// narrow bar the left side gives way and the right side keeps its
    /// size. Add the right items first, so their room is kept.
    ///
    /// # Example
    ///
    /// ```
    /// use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};
    /// use otty_ui_kit::palette::UiPalette;
    ///
    /// let palette = UiPalette::otty_dark();
    /// let bar: StatusBar<()> = StatusBar::new(&palette)
    ///     .right(StatusItem::text("OTTY 0.4.0"))
    ///     .left_that_fit([StatusItem::text("~/otty")], 200.0);
    ///
    /// assert!(bar.width() <= 200.0);
    /// ```
    pub fn left_that_fit(
        mut self,
        items: impl IntoIterator<Item = StatusItem<Message>>,
        available: f32,
    ) -> Self {
        for item in items {
            if self.width() + ITEM_GAP + item.width() > available {
                break;
            }
            self.left.push(item);
        }

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
        StatusItem::Target(kind, name, is_plain) => row![
            target_dot(kind, palette),
            caption(
                name,
                false,
                if is_plain { palette.text_body() } else { muted }
            ),
        ]
        .spacing(DOT_GAP)
        .align_y(Alignment::Center)
        .into(),
        StatusItem::WithKeys(label, keys) => {
            let mut parts = vec![caption(label, false, palette.text_body())];
            parts.extend(keys.into_iter().map(|key| kbd(key, palette)));

            row(parts)
                .spacing(DOT_GAP)
                .align_y(Alignment::Center)
                .into()
        },
        StatusItem::Button(label, on_press) => {
            let overlay = palette.surface_overlay();
            let hover = palette.surface_hover();
            let strong = palette.text_strong();

            button(
                container(caption(label, false, strong)).center_y(Length::Fill),
            )
            .height(size::STATUS_BUTTON)
            .padding(Padding::from([0.0, BUTTON_PADDING_X]))
            .on_press(on_press)
            .style(move |_, status| {
                let is_hovered = status == button::Status::Hovered;

                button::Style {
                    background: Some(
                        if is_hovered { hover } else { overlay }.into(),
                    ),
                    text_color: strong,
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

/// Returns the width of the items of one group, with the gaps between them.
fn group_width<Message>(items: &[StatusItem<Message>]) -> f32 {
    let gaps = items.len().saturating_sub(1) as f32 * ITEM_GAP;

    items.iter().map(StatusItem::width).sum::<f32>() + gaps
}

/// Returns the width of caption-size text, as [`caption`] draws it.
fn caption_width(content: &str, is_mono: bool) -> f32 {
    let font = if is_mono { fonts::MONO } else { fonts::UI };
    text_width(content, typography::CAPTION.size(), font)
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
    fn an_item_with_keys_shows_its_text_and_every_key() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(StatusBar::<Message>::new(&palette).left(
            StatusItem::with_keys(
                "Block 5 of 6",
                ["Ctrl Shift ↑", "Ctrl Shift ↓"],
            ),
        ));

        for label in ["Block 5 of 6", "Ctrl Shift ↑", "Ctrl Shift ↓"] {
            ui.find(label).expect("the text and the keys should show");
        }
    }

    /// The width a bar measures is the width its items take when it is
    /// laid out: the padding, the groups and the gaps in them.
    #[test]
    fn the_width_of_a_bar_is_what_its_items_take_on_one_line() {
        let palette = UiPalette::otty_dark();
        let bar = StatusBar::<Message>::new(&palette)
            .left(StatusItem::target(TargetKind::Local, "fedora"))
            .left(StatusItem::mono("~/otty"))
            .left(StatusItem::with_keys(
                "Block 5 of 6",
                ["Ctrl Shift ↑", "Ctrl Shift ↓"],
            ))
            .right(StatusItem::text("2 panes"))
            .right(StatusItem::button("Settings", Message::OpenSettings));
        let width = bar.width();
        let mut ui = simulator(bar);

        let name = ui.find("fedora").expect("the target shows").bounds();
        let last_key = ui.find("Ctrl Shift ↓").expect("the key shows").bounds();
        let panes = ui.find("2 panes").expect("the count shows").bounds();
        let button = ui.find("Settings").expect("the button shows").bounds();
        let left =
            last_key.x + last_key.width + 6.0 - (name.x - DOT_GAP - size::DOT);
        let right = button.x + button.width + BUTTON_PADDING_X - panes.x;

        let laid_out = PADDING_X + left + right + PADDING_X;
        assert!((laid_out - width).abs() < 0.5, "{laid_out} != {width}");
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

    /// The bar of the fitting tests: the version on the right, then the
    /// left texts that fit in `room`.
    fn fitted_bar(room: f32) -> StatusBar<Message> {
        let palette = UiPalette::otty_dark();
        let left = [
            StatusItem::text("2 projects"),
            StatusItem::text("5 sessions"),
        ];

        StatusBar::new(&palette)
            .right(StatusItem::text("OTTY 0.4.0"))
            .left_that_fit(left, room)
    }

    /// In a narrow bar the left side gives way first: its last texts are
    /// left out, whole, and the right side stays.
    #[test]
    fn the_left_items_that_do_not_fit_are_left_out_from_the_last() {
        let palette = UiPalette::otty_dark();
        let right = StatusBar::<Message>::new(&palette)
            .right(StatusItem::text("OTTY 0.4.0"))
            .width();
        let first = StatusItem::<Message>::text("2 projects").width();
        let room = right + ITEM_GAP + first + 1.0;

        let mut ui = simulator(fitted_bar(room));

        assert!(ui.find("2 projects").is_ok());
        assert!(ui.find("5 sessions").is_err());
        assert!(ui.find("OTTY 0.4.0").is_ok());
    }

    #[test]
    fn with_room_for_all_every_left_item_shows() {
        let mut ui = simulator(fitted_bar(1000.0));

        assert!(ui.find("2 projects").is_ok());
        assert!(ui.find("5 sessions").is_ok());
    }
}
