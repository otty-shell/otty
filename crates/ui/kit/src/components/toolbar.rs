//! `Toolbar`: a field and icon buttons in one row; buttons that do not fit
//! move into a trailing "⋯" menu.

use iced::widget::{Space, container, responsive, row, svg};
use iced::{Alignment, Element, Length};

use super::icon_button::IconButton;
use super::menu::{Menu, MenuItem};
use super::overflow_menu::OverflowMenu;
use crate::palette::UiPalette;
use crate::{size, spacing};

/// Width of a toolbar button (a medium [`IconButton`]).
///
/// [`IconButton`]: super::icon_button::IconButton
const BUTTON: f32 = size::CONTROL_MD;
/// Gap between toolbar buttons (`space-2`).
const BUTTON_GAP: f32 = spacing::SPACE_2;
/// Smallest gap between the field and the first button (`space-4`).
const FIELD_GAP: f32 = spacing::SPACE_4;

/// Builder of the field, called each time the toolbar is laid out.
type FieldView<'a, Message> = Box<dyn Fn() -> Element<'a, Message> + 'a>;

/// One action of a [`Toolbar`]: an icon button whose label is its tooltip
/// and its menu item text once it moves into the "⋯" menu.
pub struct ToolbarItem<Message> {
    icon: svg::Handle,
    label: String,
    on_press: Option<Message>,
}

impl<Message> ToolbarItem<Message> {
    /// Creates a disabled item with `icon` and `label`.
    pub fn new(icon: svg::Handle, label: impl Into<String>) -> Self {
        Self {
            icon,
            label: label.into(),
            on_press: None,
        }
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the message sent on press, or none for a disabled item.
    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }
}

/// The flexible field of a [`Toolbar`] and its width range.
struct ToolbarField<'a, Message> {
    min_width: f32,
    max_width: f32,
    view: FieldView<'a, Message>,
}

/// Row of controls: an optional field on the left, icon buttons on the
/// right. When the row is too narrow the field shrinks first, down to its
/// smallest width; then the last buttons move, in order, into a "⋯" menu at
/// the end of the row. Pressing a menu item sends that item's message and
/// closes the menu; a press outside or Escape closes it too.
///
/// The toolbar fills the width it gets and is as tall as its tallest
/// control. It is built with `responsive`, so it decides what fits from the
/// width it is given; the caller passes every item every time.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::{svg, text_input};
/// use otty_ui_kit::components::toolbar::{Toolbar, ToolbarItem};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Filter(String),
///     Prune,
///     Refresh,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let filter = String::new();
/// let toolbar: Element<'_, Message> = Toolbar::new("More actions", &palette)
///     .field(120.0, 240.0, move || {
///         text_input("Filter", &filter)
///             .on_input(Message::Filter)
///             .into()
///     })
///     .item(ToolbarItem::new(icon.clone(), "Prune").on_press(Message::Prune))
///     .item(ToolbarItem::new(icon, "Refresh").on_press(Message::Refresh))
///     .into();
/// ```
pub struct Toolbar<'a, Message> {
    palette: UiPalette,
    more_label: String,
    field: Option<ToolbarField<'a, Message>>,
    items: Vec<ToolbarItem<Message>>,
}

impl<'a, Message> Toolbar<'a, Message> {
    /// Creates an empty toolbar. `more_label` is the tooltip of the "⋯"
    /// button.
    pub fn new(more_label: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            more_label: more_label.into(),
            field: None,
            items: Vec::new(),
        }
    }

    /// Sets the field at the start of the row. It is `max_width` wide when
    /// there is room and shrinks down to `min_width` before any button
    /// moves into the menu. `view` builds it and is called on each layout,
    /// so the field should fill the width it gets.
    pub fn field(
        mut self,
        min_width: f32,
        max_width: f32,
        view: impl Fn() -> Element<'a, Message> + 'a,
    ) -> Self {
        self.field = Some(ToolbarField {
            min_width,
            max_width,
            view: Box::new(view),
        });
        self
    }

    /// Appends a button after the others.
    pub fn item(mut self, item: ToolbarItem<Message>) -> Self {
        self.items.push(item);
        self
    }
}

impl<'a, Message: Clone + 'a> Toolbar<'a, Message> {
    /// Builds the row for a toolbar `width` wide: the field, a gap that
    /// takes the free space, the buttons that fit and the "⋯" menu with
    /// the others.
    fn view(&self, width: f32) -> Element<'a, Message> {
        let field_range = self
            .field
            .as_ref()
            .map(|field| (field.min_width, field.max_width));
        let fit = fit(width, field_range, self.items.len());
        let (shown, hidden) = self.items.split_at(fit.shown);

        let mut buttons = row![].spacing(BUTTON_GAP);
        for item in shown {
            buttons = buttons.push(
                IconButton::new(
                    item.icon.clone(),
                    item.label.as_str(),
                    &self.palette,
                )
                .on_press_maybe(item.on_press.clone()),
            );
        }
        if !hidden.is_empty() {
            let mut menu = Menu::new(&self.palette);
            for item in hidden {
                let mut entry = MenuItem::new(item.label.as_str());
                if let Some(message) = &item.on_press {
                    entry = entry.on_press(message.clone());
                }
                menu = menu.item(entry);
            }
            buttons = buttons.push(OverflowMenu::new(
                self.more_label.clone(),
                menu,
                &self.palette,
            ));
        }

        // `fit` leaves at least `FIELD_GAP` free, so the gap that takes the
        // free space keeps the field and the buttons apart.
        let mut bar = row![].align_y(Alignment::Center);
        if let Some(field) = &self.field {
            bar = bar.push(container((field.view)()).width(fit.field_width));
        }

        bar.push(Space::new().width(Length::Fill))
            .push(buttons)
            .into()
    }
}

impl<'a, Message: Clone + 'a> From<Toolbar<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: Toolbar<'a, Message>) -> Self {
        responsive(move |size| value.view(size.width))
            .height(Length::Shrink)
            .into()
    }
}

/// How a toolbar fills its width: the field width and how many buttons
/// show before the "⋯" button.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ToolbarFit {
    field_width: f32,
    shown: usize,
}

/// Returns how a toolbar `width` wide holds a field with the width range
/// `field` (none without a field) and `count` buttons. Every button shows
/// while the field can keep its smallest width; otherwise as many buttons
/// show as fit next to the "⋯" button and the smallest field. The field
/// then takes the rest, up to its largest width; it goes below its
/// smallest width only when not even the "⋯" button fits.
fn fit(width: f32, field: Option<(f32, f32)>, count: usize) -> ToolbarFit {
    let field_part = field.map_or(0.0, |(min_width, _)| min_width + FIELD_GAP);
    let fits = |buttons: usize| field_part + buttons_width(buttons) <= width;

    let shown = if fits(count) {
        count
    } else {
        (0..count).rev().find(|&shown| fits(shown + 1)).unwrap_or(0)
    };
    let drawn = if shown < count { shown + 1 } else { shown };

    let field_width = match field {
        Some((_, max_width)) => (width - FIELD_GAP - buttons_width(drawn))
            .min(max_width)
            .max(0.0),
        None => 0.0,
    };

    ToolbarFit { field_width, shown }
}

/// Returns the width of `count` buttons side by side.
fn buttons_width(count: usize) -> f32 {
    match count {
        0 => 0.0,
        _ => count as f32 * BUTTON + (count - 1) as f32 * BUTTON_GAP,
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::{Space, container};
    use iced::{Event, Point, keyboard, mouse};
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::components::glyph;
    use crate::test_support::TempDir;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Run,
        Prune,
        Refresh,
    }

    /// Smallest and largest width of the test field.
    const FIELD: (f32, f32) = (120.0, 190.0);
    /// Width that holds the smallest field, Run and the "⋯" button.
    const NARROW: f32 = FIELD.0 + FIELD_GAP + 2.0 * BUTTON + BUTTON_GAP;

    /// Toolbar `width` wide with a field and Run, Prune (disabled when
    /// `can_prune` is off) and Refresh.
    fn toolbar(width: f32, can_prune: bool) -> Element<'static, Message> {
        let palette = UiPalette::otty_dark();
        let toolbar = Toolbar::new("More actions", &palette)
            .field(FIELD.0, FIELD.1, || {
                Space::new().width(Length::Fill).height(BUTTON).into()
            })
            .item(ToolbarItem::new(glyph::plus(), "Run").on_press(Message::Run))
            .item(
                ToolbarItem::new(glyph::plus(), "Prune")
                    .on_press_maybe(can_prune.then_some(Message::Prune)),
            )
            .item(
                ToolbarItem::new(glyph::plus(), "Refresh")
                    .on_press(Message::Refresh),
            );

        container(toolbar).width(width).into()
    }

    /// Center of the button `from_end` places before the last one, in a
    /// toolbar `width` wide.
    fn button_center(width: f32, from_end: usize) -> Point {
        Point::new(
            width - BUTTON / 2.0 - from_end as f32 * (BUTTON + BUTTON_GAP),
            BUTTON / 2.0,
        )
    }

    fn click_at(ui: &mut Simulator<'_, Message>, point: Point) {
        ui.point_at(point);
        let _ = ui.simulate(simulator::click());
    }

    #[test]
    fn given_room_for_everything_then_field_is_widest_and_every_button_shows() {
        let fit = fit(400.0, Some(FIELD), 3);

        assert_eq!(
            fit,
            ToolbarFit {
                field_width: 190.0,
                shown: 3
            }
        );
    }

    #[test]
    fn given_less_room_then_field_shrinks_before_any_button_moves() {
        let buttons = 3.0 * BUTTON + 2.0 * BUTTON_GAP;
        let width = FIELD.0 + FIELD_GAP + buttons + 10.0;

        let fit = fit(width, Some(FIELD), 3);

        assert_eq!(
            fit,
            ToolbarFit {
                field_width: FIELD.0 + 10.0,
                shown: 3
            }
        );
    }

    #[test]
    fn given_field_at_its_smallest_then_last_buttons_move_into_the_menu() {
        let fit = fit(NARROW, Some(FIELD), 3);

        assert_eq!(
            fit,
            ToolbarFit {
                field_width: FIELD.0,
                shown: 1
            }
        );
    }

    #[test]
    fn given_no_room_even_for_the_menu_then_field_goes_below_its_smallest() {
        let fit = fit(100.0, Some(FIELD), 3);

        assert_eq!(
            fit,
            ToolbarFit {
                field_width: 100.0 - FIELD_GAP - BUTTON,
                shown: 0
            }
        );
    }

    #[test]
    fn given_no_field_then_buttons_use_the_whole_width() {
        let all = 3.0 * BUTTON + 2.0 * BUTTON_GAP;

        assert_eq!(fit(all, None, 3).shown, 3);
        assert_eq!(fit(all - 1.0, None, 3).shown, 1);
        assert_eq!(fit(all - 1.0, None, 3).field_width, 0.0);
    }

    #[test]
    fn given_wide_toolbar_when_a_button_pressed_then_its_message_is_sent() {
        let mut ui = simulator(toolbar(400.0, true));

        click_at(&mut ui, button_center(400.0, 0));
        click_at(&mut ui, button_center(400.0, 2));

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Refresh, Message::Run]);
    }

    /// Draws a wide toolbar whose middle button is labeled `label`, with
    /// the pointer over that button, and checks it against the picture
    /// saved in `reference`; the first call saves the picture. Tooltips are
    /// overlays that widget queries do not reach.
    fn hovered_button_matches(
        label: &str,
        reference: &std::path::Path,
    ) -> bool {
        let palette = UiPalette::otty_dark();
        let toolbar = Toolbar::new("More actions", &palette)
            .item(ToolbarItem::new(glyph::plus(), "Run").on_press(Message::Run))
            .item(
                ToolbarItem::new(glyph::plus(), label).on_press(Message::Prune),
            )
            .item(
                ToolbarItem::new(glyph::plus(), "Refresh")
                    .on_press(Message::Refresh),
            );
        let mut ui = simulator(container(toolbar).width(400.0).height(100.0));
        let middle = button_center(400.0, 1);
        ui.point_at(middle);
        let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
            position: middle,
        })]);

        ui.snapshot(&iced::Theme::Dark)
            .expect("toolbar should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    #[test]
    fn given_wide_toolbar_when_a_button_hovered_then_its_label_shows() {
        let folder = TempDir::new("toolbar");
        let reference = folder.path().join("tooltip");

        assert!(hovered_button_matches("Prune", &reference));
        assert!(!hovered_button_matches("Prune unused", &reference));
        assert!(hovered_button_matches("Prune", &reference));
    }

    #[test]
    fn given_narrow_toolbar_when_more_pressed_then_the_last_buttons_show_in_its_menu()
     {
        let mut ui = simulator(toolbar(NARROW, true));
        assert!(ui.find("Prune").is_err());

        click_at(&mut ui, button_center(NARROW, 0));

        assert!(ui.find("Prune").is_ok());
        assert!(ui.find("Refresh").is_ok());
        assert!(ui.find("Run").is_err(), "Run still shows as a button");
    }

    #[test]
    fn given_open_menu_when_an_item_pressed_then_its_message_is_sent_and_menu_closes()
     {
        let mut ui = simulator(toolbar(NARROW, true));

        click_at(&mut ui, button_center(NARROW, 0));
        ui.click("Refresh").expect("menu item should show");

        assert!(ui.find("Prune").is_err(), "menu should close");
        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Refresh]);
    }

    #[test]
    fn given_open_menu_when_disabled_item_pressed_then_nothing_is_sent() {
        let mut ui = simulator(toolbar(NARROW, false));

        click_at(&mut ui, button_center(NARROW, 0));
        ui.click("Prune").expect("menu item should show");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_open_menu_when_pressed_outside_or_escape_then_it_closes() {
        let mut ui = simulator(toolbar(NARROW, true));

        click_at(&mut ui, button_center(NARROW, 0));
        click_at(&mut ui, Point::new(10.0, BUTTON / 2.0));
        assert!(ui.find("Prune").is_err(), "outside press closes");

        click_at(&mut ui, button_center(NARROW, 0));
        let _ = ui.simulate([simulator::press_key(
            keyboard::Key::Named(keyboard::key::Named::Escape),
            None,
        )]);
        assert!(ui.find("Prune").is_err(), "Escape closes");

        click_at(&mut ui, button_center(NARROW, 0));
        click_at(&mut ui, button_center(NARROW, 0));
        assert!(ui.find("Prune").is_err(), "a second ⋯ press closes");

        assert_eq!(ui.into_messages().count(), 0);
    }
}
