//! `TabStrip` and `Tab`: the top strip of open sessions.

use iced::widget::text::Wrapping;
use iced::widget::{button, container, row, scrollable, stack, text};
use iced::{Alignment, Element, Length, Padding};

use super::glyph;
use super::icon_button::{IconButton, IconButtonSize, IconTone};
use crate::palette::UiPalette;
use crate::{fonts, size, spacing};

/// Largest width of one tab (`max-width: 220px`).
const TAB_MAX_WIDTH: f32 = 220.0;
/// Title size (`font-size: 12.5px`).
const TITLE_SIZE: f32 = 12.5;
/// Height of the accent line on top of the active tab (`inset 0 2px 0`).
const ACTIVE_LINE: f32 = 2.0;
/// Left padding of a tab title (`padding: 0 6px 0 12px`).
const TITLE_PADDING_LEFT: f32 = spacing::SPACE_4;
/// Width taken by the close cross: 24px button and 6px padding on each side.
const CLOSE_WIDTH: f32 = size::CONTROL_SM + 2.0 * spacing::SPACE_2;
/// Right padding of a tab without a close cross.
const TITLE_PADDING_RIGHT: f32 = spacing::SPACE_4;

/// One tab of a [`TabStrip`].
///
/// A tab without a press message is shown static.
pub struct Tab<Message> {
    title: String,
    is_active: bool,
    on_press: Option<Message>,
    on_close: Option<Message>,
}

impl<Message> Tab<Message> {
    /// Creates an inactive tab.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            is_active: false,
            on_press: None,
            on_close: None,
        }
    }

    /// Marks the tab as the current one.
    pub fn active(mut self, is_active: bool) -> Self {
        self.is_active = is_active;
        self
    }

    /// Sets the message sent when the tab is pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Shows a close cross that sends `message`.
    pub fn on_close(mut self, message: Message) -> Self {
        self.on_close = Some(message);
        self
    }
}

/// Strip of tabs with a plus right after the last one.
///
/// A title that does not fit in 220px is cut at the end; `iced` has no text
/// ellipsis, so no `...` is drawn. Tabs that do not fit in the strip are
/// clipped at its right edge together with the plus.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::tab_strip::{Tab, TabStrip};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Select(usize),
///     New,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let strip: Element<'_, Message> = TabStrip::new(&palette)
///     .tab(Tab::new("main").active(true).on_press(Message::Select(0)))
///     .tab(Tab::new("logs").on_press(Message::Select(1)))
///     .on_new(Message::New)
///     .into();
/// ```
pub struct TabStrip<Message> {
    palette: UiPalette,
    tabs: Vec<Tab<Message>>,
    on_new: Option<Message>,
    scroll_id: Option<iced::widget::Id>,
}

impl<Message> TabStrip<Message> {
    /// Creates an empty strip.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            tabs: Vec::new(),
            on_new: None,
            scroll_id: None,
        }
    }

    /// Appends a tab.
    pub fn tab(mut self, tab: Tab<Message>) -> Self {
        self.tabs.push(tab);
        self
    }

    /// Sets the message sent by the plus button.
    pub fn on_new(mut self, message: Message) -> Self {
        self.on_new = Some(message);
        self
    }

    /// Makes the strip scroll sideways and names the scrollable, so the app
    /// can scroll it with `iced::widget::operation`.
    pub fn scroll_id(mut self, id: impl Into<iced::widget::Id>) -> Self {
        self.scroll_id = Some(id.into());
        self
    }
}

impl<'a, Message: Clone + 'a> From<TabStrip<Message>> for Element<'a, Message> {
    fn from(value: TabStrip<Message>) -> Self {
        let palette = value.palette;

        let mut strip = row![].height(Length::Fill);
        for tab in value.tabs {
            strip = strip.push(tab_view(tab, &palette));
        }
        let mut plus = IconButton::new(glyph::plus(), "New session", &palette)
            .tone(IconTone::Accent);
        if let Some(message) = value.on_new {
            plus = plus.on_press(message);
        }
        strip = strip.push(
            container(plus)
                .padding(Padding::from([0.0, spacing::SPACE_1]))
                .align_y(Alignment::Center)
                .height(Length::Fill),
        );

        let content: Element<'a, Message> = match value.scroll_id {
            Some(id) => scrollable::Scrollable::with_direction(
                strip,
                scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new()
                        .width(0)
                        .scroller_width(0)
                        .margin(0),
                ),
            )
            .id(id)
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
            None => strip.into(),
        };

        let background = palette.surface_panel();
        container(content)
            .width(Length::Fill)
            .height(size::TAB)
            .clip(true)
            .style(move |_| container::Style {
                background: Some(background.into()),
                ..container::Style::default()
            })
            .into()
    }
}

/// Builds one tab: accent line on top of the active one, title, close cross.
fn tab_view<'a, Message: Clone + 'a>(
    tab: Tab<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let is_active = tab.is_active;
    let canvas = palette.surface_canvas();
    let accent = palette.accent();
    let strong = palette.text_strong();
    let body = palette.text_body();
    let muted = palette.text_muted();
    let has_close = tab.on_close.is_some();
    let title_max = if has_close {
        TAB_MAX_WIDTH - TITLE_PADDING_LEFT - CLOSE_WIDTH
    } else {
        TAB_MAX_WIDTH - TITLE_PADDING_LEFT - TITLE_PADDING_RIGHT
    };

    let title = button(
        container(
            text(tab.title)
                .size(TITLE_SIZE)
                .font(fonts::UI)
                .wrapping(Wrapping::None),
        )
        .height(Length::Fill)
        .max_width(title_max)
        .align_y(Alignment::Center)
        .clip(true),
    )
    .height(Length::Fill)
    .padding(Padding {
        top: 0.0,
        right: if has_close { 0.0 } else { TITLE_PADDING_RIGHT },
        bottom: 0.0,
        left: TITLE_PADDING_LEFT,
    })
    .on_press_maybe(tab.on_press)
    .style(move |_, status| {
        let hovered = status == button::Status::Hovered;

        button::Style {
            background: (hovered && !is_active).then(|| canvas.into()),
            text_color: if is_active {
                strong
            } else if hovered {
                body
            } else {
                muted
            },
            ..button::Style::default()
        }
    });

    let mut content = row![title].align_y(Alignment::Center);
    if let Some(message) = tab.on_close {
        content = content.push(
            container(
                IconButton::new(glyph::cross(), "Close tab", palette)
                    .size(IconButtonSize::Small)
                    .on_press(message),
            )
            .padding(Padding::from([0.0, spacing::SPACE_2])),
        );
    }

    let line = container("").width(Length::Fill).height(ACTIVE_LINE).style(
        move |_| container::Style {
            background: is_active.then(|| accent.into()),
            ..container::Style::default()
        },
    );
    let body_view = container(content).height(Length::Fill).style(move |_| {
        container::Style {
            background: is_active.then(|| canvas.into()),
            ..container::Style::default()
        }
    });

    container(stack![body_view, line])
        .max_width(TAB_MAX_WIDTH)
        .height(Length::Fill)
        .into()
}
