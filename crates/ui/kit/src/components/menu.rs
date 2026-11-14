//! `Menu` and `MenuItem`: context menus and dropdown lists.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Border, Element, Length, Padding, Size, Theme};

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
/// Space around the text of a heading row (`padding: 6px 10px 4px`).
const HEADING_PADDING: Padding = Padding {
    top: 6.0,
    right: ITEM_PADDING_X,
    bottom: 4.0,
    left: ITEM_PADDING_X,
};
/// Height of a heading row: its text line, 1.3 times the text size as iced
/// draws it by default, and the padding around it.
const HEADING_HEIGHT: f32 =
    HEADING_PADDING.top + HEADING_TEXT_SIZE * 1.3 + HEADING_PADDING.bottom;
/// Thickness of a separator line.
const SEPARATOR_LINE: f32 = 1.0;
/// Space around a separator (`margin: 4px 2px`).
const SEPARATOR_MARGIN: Padding = Padding {
    top: 4.0,
    right: 2.0,
    bottom: 4.0,
    left: 2.0,
};
/// Height of a separator: the line and the space above and under it.
const SEPARATOR_HEIGHT: f32 =
    SEPARATOR_MARGIN.top + SEPARATOR_LINE + SEPARATOR_MARGIN.bottom;

/// One row of a [`Menu`] that the user can press.
///
/// An item without a message is shown disabled.
pub struct MenuItem<Message> {
    label: String,
    shortcut: Option<String>,
    active: bool,
    danger: bool,
    on_press: Option<Message>,
}

impl<Message> MenuItem<Message> {
    /// Creates an item with a verb `label` in sentence case.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            active: false,
            danger: false,
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

    /// Draws the label in the danger color, for destructive actions.
    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Turns the message of the item into another one.
    fn map<Other>(
        self,
        to_other: impl Fn(Message) -> Other,
    ) -> MenuItem<Other> {
        MenuItem {
            label: self.label,
            shortcut: self.shortcut,
            active: self.active,
            danger: self.danger,
            on_press: self.on_press.map(to_other),
        }
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
    width: f32,
    entries: Vec<Entry<Message>>,
}

impl<Message> Menu<Message> {
    /// Returns the size the menu takes when drawn, so a caller can place it
    /// without counting its rows.
    pub fn size(&self) -> Size {
        let rows: f32 = self
            .entries
            .iter()
            .map(|entry| match entry {
                Entry::Item(_) => size::ROW,
                Entry::Separator => SEPARATOR_HEIGHT,
                Entry::Heading(_) => HEADING_HEIGHT,
            })
            .sum();

        Size::new(self.width, 2.0 * MENU_PADDING + rows)
    }

    /// Creates an empty menu.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            width: MENU_WIDTH,
            entries: Vec::new(),
        }
    }

    /// Sets the menu width; the default is `MENU_WIDTH`.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
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

    /// Turns the message of every item into another one, as `Element::map`
    /// does, so a menu built by one part of the app can be placed by
    /// another.
    pub fn map<Other>(
        self,
        to_other: impl Fn(Message) -> Other,
    ) -> Menu<Other> {
        let entries = self
            .entries
            .into_iter()
            .map(|entry| match entry {
                Entry::Item(item) => Entry::Item(item.map(&to_other)),
                Entry::Separator => Entry::Separator,
                Entry::Heading(label) => Entry::Heading(label),
            })
            .collect();

        Menu {
            palette: self.palette,
            width: self.width,
            entries,
        }
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

        let width = value.width;
        let background = palette.surface_overlay();
        let shadow = palette.shadow_popover();
        container(list)
            .width(width)
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

/// Style of a menu item: `surface-hover` and `text-strong` while hovered,
/// pressed or `active` (keyboard-selected), `text-body` otherwise.
pub(super) fn item_style(
    palette: &UiPalette,
    active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style + 'static {
    item_style_with(palette, active, false)
}

/// Builds a heading row (`.ot-menu-label`). The label may be borrowed, so
/// a caller that redraws every frame does not copy it.
pub(super) fn heading_view<'a, Message: 'a>(
    label: impl text::IntoFragment<'a>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    container(
        text(label)
            .size(HEADING_TEXT_SIZE)
            .font(fonts::UI)
            .color(palette.text_muted()),
    )
    .padding(HEADING_PADDING)
    .into()
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

    button(content)
        .width(Length::Fill)
        .height(size::ROW)
        .padding(Padding::from([0.0, ITEM_PADDING_X]))
        .on_press_maybe(item.on_press)
        .style(item_style_with(palette, item.active, item.danger))
        .into()
}

/// Like [`item_style`]; a `danger` item uses `text-danger` on
/// `surface-danger-hover` when lit, because status colors never sit on `surface-hover`.
fn item_style_with(
    palette: &UiPalette,
    active: bool,
    danger: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style + 'static {
    let palette = palette.clone();

    move |_, status| {
        let lit = active
            || matches!(
                status,
                button::Status::Hovered | button::Status::Pressed
            );
        let color = if danger {
            palette.text_danger()
        } else if lit {
            palette.text_strong()
        } else {
            palette.text_body()
        };

        button::Style {
            background: lit.then(|| {
                if danger {
                    palette.surface_danger_hover().into()
                } else {
                    palette.surface_hover().into()
                }
            }),
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
    }
}

/// Builds a 1px separator line.
fn separator_view<'a, Message: 'a>(
    palette: &UiPalette,
) -> Element<'a, Message> {
    let line = palette.line();

    container(
        container(Space::new())
            .width(Length::Fill)
            .height(SEPARATOR_LINE)
            .style(move |_| container::Style {
                background: Some(line.into()),
                ..container::Style::default()
            }),
    )
    .padding(SEPARATOR_MARGIN)
    .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::column;
    use iced_test::simulator;

    use super::*;
    use crate::contrast::contrast_ratio;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Copy,
        Paste,
    }

    /// The height `menu` really takes: where a widget under it starts.
    fn drawn_height(menu: Menu<Message>) -> f32 {
        let content: Element<'_, Message> =
            column![Element::from(menu), text("below")].into();
        let mut ui = simulator(content);

        ui.find("below")
            .expect("marker should be visible")
            .bounds()
            .y
    }

    #[test]
    fn given_empty_menu_when_size_then_height_is_only_the_padding() {
        let menu: Menu<Message> = Menu::new(&UiPalette::otty_dark());

        assert_eq!(menu.size(), Size::new(MENU_WIDTH, 2.0 * MENU_PADDING));
    }

    #[test]
    fn given_items_when_size_then_height_is_the_drawn_height() {
        let menu = Menu::new(&UiPalette::otty_dark())
            .item(MenuItem::new("Copy").on_press(Message::Copy))
            .item(MenuItem::new("Paste").on_press(Message::Paste))
            .item(MenuItem::new("Disabled"));

        let height = menu.size().height;

        assert_eq!(height, 2.0 * MENU_PADDING + 3.0 * size::ROW);
        assert!((drawn_height(menu) - height).abs() < 0.5);
    }

    #[test]
    fn given_separators_and_heading_when_size_then_height_is_the_drawn_height()
    {
        let menu = Menu::new(&UiPalette::otty_dark())
            .item(MenuItem::new("Copy").on_press(Message::Copy))
            .separator()
            .heading("Launches")
            .item(MenuItem::new("Paste").on_press(Message::Paste))
            .separator()
            .item(MenuItem::new("Close"));

        let height = menu.size().height;
        let drawn = drawn_height(menu);

        assert!(
            (drawn - height).abs() < 0.5,
            "size says {height}, the menu is drawn {drawn} high"
        );
    }

    #[test]
    fn given_custom_width_when_size_then_width_is_the_custom_one() {
        let menu: Menu<Message> =
            Menu::new(&UiPalette::otty_dark()).width(280.0);

        assert_eq!(menu.size().width, 280.0);
    }

    #[test]
    fn given_mapped_menu_when_item_pressed_then_mapped_message_is_sent() {
        let menu = Menu::new(&UiPalette::otty_dark())
            .item(MenuItem::new("Copy").on_press(Message::Copy))
            .separator()
            .item(MenuItem::new("Paste").on_press(Message::Paste));
        let size = menu.size();

        let mapped: Menu<Option<Message>> = menu.map(Some);
        assert_eq!(mapped.size(), size);

        let mut ui = simulator(Element::from(mapped));
        ui.click("Paste").expect("item should be visible");
        let messages: Vec<Option<Message>> = ui.into_messages().collect();
        assert_eq!(messages, [Some(Message::Paste)]);
    }

    #[test]
    fn given_danger_item_when_hovered_then_background_is_danger_hover() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let style = item_style_with(&palette, false, true);

            let idle = style(&Theme::Dark, button::Status::Active);
            let hovered = style(&Theme::Dark, button::Status::Hovered);

            assert_eq!(idle.background, None);
            assert_eq!(
                hovered.background,
                Some(palette.surface_danger_hover().into())
            );
            assert!(
                contrast_ratio(
                    palette.text_danger(),
                    palette.surface_danger_hover()
                ) >= 4.5
            );
        }
    }
}
