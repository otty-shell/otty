//! `Menu` and `MenuItem`: context menus and dropdown lists.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Border, Element, Length, Padding, Size, Theme};

use super::tooltip::tooltip;
use super::{faded, kbd};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing};

/// Width of a menu in logical pixels.
pub const MENU_WIDTH: f32 = 240.0;
/// Padding around the items (`padding: 5px`).
pub const MENU_PADDING: f32 = 5.0;
/// Text size of items (`font-size: 12.5px`), also of the rows of the other
/// lists that look like a menu.
pub(super) const ITEM_TEXT_SIZE: f32 = 12.5;
/// Horizontal padding of an item (`padding: 0 10px`), also of the rows of
/// the other lists that look like a menu.
pub(super) const ITEM_PADDING_X: f32 = 10.0;
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
/// An item without a message is shown disabled. A disabled item may say
/// why in a tooltip while the pointer is over it; a menu has no notes of
/// its own for that.
pub struct MenuItem<Message> {
    label: String,
    shortcut: Option<String>,
    active: bool,
    danger: bool,
    disabled_reason: Option<String>,
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
            disabled_reason: None,
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

    /// Says why the item can not be used now, such as "A key can not be
    /// renamed yet". The reason shows as the kit's tooltip while the
    /// pointer is over the item, and only while the item has no message.
    ///
    /// ```
    /// use otty_ui_kit::components::menu::{Menu, MenuItem};
    /// use otty_ui_kit::palette::UiPalette;
    ///
    /// let menu: Menu<()> = Menu::new(&UiPalette::otty_dark()).item(
    ///     MenuItem::new("Rename…").disabled_reason("A key can not be renamed"),
    /// );
    /// ```
    pub fn disabled_reason(mut self, reason: impl Into<String>) -> Self {
        self.disabled_reason = Some(reason.into());
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
            disabled_reason: self.disabled_reason,
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

        container(list)
            .width(value.width)
            .padding(MENU_PADDING)
            .style(surface_style(&palette))
            .into()
    }
}

/// Style of the surface a menu floats on: `surface-overlay` with the
/// popover shadow and medium corners. Every list that floats as a menu,
/// such as the suggestions under a field, has this look.
pub(super) fn surface_style(
    palette: &UiPalette,
) -> impl Fn(&Theme) -> container::Style + 'static {
    let background = palette.surface_overlay();
    let shadow = palette.shadow_popover();

    move |_| container::Style {
        background: Some(background.into()),
        border: Border {
            radius: radius::RADIUS_MD.into(),
            ..Border::default()
        },
        shadow,
        ..container::Style::default()
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

/// Builds one pressable row. A disabled row with a reason shows it as a
/// tooltip under the pointer.
fn item_view<'a, Message: Clone + 'a>(
    item: MenuItem<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let reason = if item.on_press.is_none() {
        item.disabled_reason
    } else {
        None
    };

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

    let row = button(content)
        .width(Length::Fill)
        .height(size::ROW)
        .padding(Padding::from([0.0, ITEM_PADDING_X]))
        .on_press_maybe(item.on_press)
        .style(item_style_with(palette, item.active, item.danger));

    match reason {
        Some(reason) => tooltip(row, reason, palette),
        None => row.into(),
    }
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
    use iced::Point;
    use iced::widget::column;
    use iced_test::simulator;

    use super::*;
    use crate::contrast::contrast_ratio;
    use crate::testing::pictures::draws;
    use crate::testing::temp_dir::TempDir;

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

    /// A window for a menu and the tooltip of its first item, as
    /// [`draws`] places them.
    const MENU_WINDOW: Size = Size::new(400.0, 240.0);

    /// A point on the first item of a menu, from the menu's top left
    /// corner.
    const ON_FIRST_ITEM: Point =
        Point::new(MENU_PADDING + 20.0, MENU_PADDING + size::ROW / 2.0);

    /// A menu of one item, Rename, that sends a message when `is_on`,
    /// with `reason` as why it is off, if any.
    fn rename_menu(
        reason: Option<&'static str>,
        is_on: bool,
    ) -> impl Fn() -> Element<'static, ()> {
        move || {
            let mut item = MenuItem::new("Rename");
            if let Some(reason) = reason {
                item = item.disabled_reason(reason);
            }
            if is_on {
                item = item.on_press(());
            }

            Menu::new(&UiPalette::otty_dark()).item(item).into()
        }
    }

    /// Issue 42: a disabled item says why in the kit's tooltip while the
    /// pointer is over it. iced draws tooltips as overlays that widget
    /// queries do not reach, so the reason is checked on the drawing.
    #[test]
    fn given_disabled_item_with_reason_when_hovered_then_reason_shows() {
        let folder = TempDir::new("menu-reason");
        let hovered = folder.path().join("hovered");
        let over = Some(ON_FIRST_ITEM);

        assert!(
            draws(
                &rename_menu(Some("A key"), false),
                MENU_WINDOW,
                over,
                &hovered
            ),
            "save"
        );

        assert!(
            !draws(
                &rename_menu(Some("Another reason"), false),
                MENU_WINDOW,
                over,
                &hovered
            ),
            "the reason shows under the pointer"
        );
        assert!(
            !draws(
                &rename_menu(Some("A key"), false),
                MENU_WINDOW,
                None,
                &hovered
            ),
            "the tooltip shows only under the pointer"
        );
    }

    /// A disabled item with no reason, and an item that can be pressed,
    /// show no tooltip.
    #[test]
    fn given_no_reason_or_item_on_when_hovered_then_no_tooltip_shows() {
        let folder = TempDir::new("menu-no-reason");
        let idle = folder.path().join("idle");
        let lit = folder.path().join("lit");
        let over = Some(ON_FIRST_ITEM);

        assert!(draws(&rename_menu(None, false), MENU_WINDOW, None, &idle));
        assert!(draws(&rename_menu(None, true), MENU_WINDOW, over, &lit));

        assert!(
            draws(&rename_menu(None, false), MENU_WINDOW, over, &idle),
            "a disabled item with no reason does not change under the pointer"
        );
        assert!(
            draws(&rename_menu(Some("A key"), true), MENU_WINDOW, over, &lit),
            "an item that can be pressed shows no reason"
        );
    }

    #[test]
    fn given_disabled_item_with_reason_when_pressed_then_nothing_is_sent() {
        let menu = Menu::new(&UiPalette::otty_dark())
            .item(MenuItem::new("Rename").disabled_reason("A key"))
            .item(MenuItem::new("Paste").on_press(Message::Paste));
        let mut ui = simulator(Element::from(menu));

        ui.click("Rename").expect("item should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(messages.is_empty(), "{messages:?}");
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
