//! `Menu` and `MenuItem`: context menus and dropdown lists.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::{faded, kbd};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing};

/// Width of a menu in logical pixels.
pub const MENU_WIDTH: f32 = 240.0;
/// Padding around the items (`padding: 5px`).
pub const MENU_PADDING: f32 = 5.0;
/// Text size of items (`font-size: 12.5px`).
const ITEM_TEXT_SIZE: f32 = 12.5;
/// Horizontal padding of an item (`padding: 0 10px`).
const ITEM_PADDING_X: f32 = 10.0;
/// Text size of a heading row (`font-size: 12px`).
const HEADING_TEXT_SIZE: f32 = 12.0;
/// Space around a separator (`margin: 4px 2px`).
const SEPARATOR_MARGIN: Padding = Padding {
    top: 4.0,
    right: 2.0,
    bottom: 4.0,
    left: 2.0,
};

/// One row of a [`Menu`] that the user can press.
///
/// An item without a message is shown disabled.
pub struct MenuItem<Message> {
    label: String,
    shortcut: Option<String>,
    active: bool,
    on_press: Option<Message>,
}

impl<Message> MenuItem<Message> {
    /// Creates an item with a verb `label` in sentence case.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            active: false,
            on_press: None,
        }
    }

    /// Shows a shortcut hint on the right.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Draws the item as hovered (for example the keyboard-selected item).
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

/// Entry of a [`Menu`].
enum Entry<Message> {
    Item(MenuItem<Message>),
    Separator,
    Heading(String),
}

/// Overlay surface with items, separators and headings.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::menu::{Menu, MenuItem};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Copy,
///     Paste,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let menu: Element<'_, Message> = Menu::new(&palette)
///     .item(
///         MenuItem::new("Copy")
///             .shortcut("Ctrl Shift C")
///             .on_press(Message::Copy),
///     )
///     .separator()
///     .item(MenuItem::new("Paste").on_press(Message::Paste))
///     .into();
/// ```
pub struct Menu<Message> {
    palette: UiPalette,
    entries: Vec<Entry<Message>>,
}

impl<Message> Menu<Message> {
    /// Creates an empty menu.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            entries: Vec::new(),
        }
    }

    /// Appends an item.
    pub fn item(mut self, item: MenuItem<Message>) -> Self {
        self.entries.push(Entry::Item(item));
        self
    }

    /// Appends a separator line between groups.
    pub fn separator(mut self) -> Self {
        self.entries.push(Entry::Separator);
        self
    }

    /// Appends a heading row. Use only when groups need a name.
    pub fn heading(mut self, label: impl Into<String>) -> Self {
        self.entries.push(Entry::Heading(label.into()));
        self
    }
}

impl<'a, Message: Clone + 'a> From<Menu<Message>> for Element<'a, Message> {
    fn from(value: Menu<Message>) -> Self {
        let palette = value.palette;

        let mut list = column![];
        for entry in value.entries {
            list = list.push(match entry {
                Entry::Item(item) => item_view(item, &palette),
                Entry::Separator => separator_view(&palette),
                Entry::Heading(label) => heading_view(label, &palette),
            });
        }

        let background = palette.surface_overlay();
        let shadow = palette.shadow_popover();
        container(list)
            .width(MENU_WIDTH)
            .padding(MENU_PADDING)
            .style(move |_| container::Style {
                background: Some(background.into()),
                border: Border {
                    radius: radius::RADIUS_MD.into(),
                    ..Border::default()
                },
                shadow,
                ..container::Style::default()
            })
            .into()
    }
}

/// Builds one pressable row.
fn item_view<'a, Message: Clone + 'a>(
    item: MenuItem<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut content = row![
        text(item.label)
            .size(ITEM_TEXT_SIZE)
            .font(fonts::UI)
            .width(Length::Fill)
    ]
    .spacing(spacing::SPACE_3)
    .height(Length::Fill)
    .align_y(Alignment::Center);
    if let Some(shortcut) = item.shortcut {
        content = content.push(kbd::kbd(shortcut, palette));
    }

    let palette = palette.clone();
    let active = item.active;
    button(content)
        .width(Length::Fill)
        .height(size::ROW)
        .padding(Padding::from([0.0, ITEM_PADDING_X]))
        .on_press_maybe(item.on_press)
        .style(move |_, status| {
            let lit = active
                || matches!(
                    status,
                    button::Status::Hovered | button::Status::Pressed
                );
            let color = if lit {
                palette.text_strong()
            } else {
                palette.text_body()
            };

            button::Style {
                background: lit.then(|| palette.surface_hover().into()),
                text_color: if status == button::Status::Disabled {
                    faded(color)
                } else {
                    color
                },
                border: Border {
                    radius: radius::RADIUS_XS.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            }
        })
        .into()
}

/// Builds a 1px separator line.
fn separator_view<'a, Message: 'a>(
    palette: &UiPalette,
) -> Element<'a, Message> {
    let line = palette.line();

    container(container(Space::new()).width(Length::Fill).height(1).style(
        move |_| container::Style {
            background: Some(line.into()),
            ..container::Style::default()
        },
    ))
    .padding(SEPARATOR_MARGIN)
    .into()
}

/// Builds a heading row.
fn heading_view<'a, Message: 'a>(
    label: String,
    palette: &UiPalette,
) -> Element<'a, Message> {
    container(
        text(label)
            .size(HEADING_TEXT_SIZE)
            .font(fonts::UI)
            .color(palette.text_muted()),
    )
    .padding(Padding {
        top: 6.0,
        right: ITEM_PADDING_X,
        bottom: 4.0,
        left: ITEM_PADDING_X,
    })
    .into()
}
