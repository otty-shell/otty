//! `context_menu`: a [`Menu`] opened at the pointer, and [`MenuPlace`],
//! where it opens.

use iced::widget::{container, opaque};
use iced::{Element, Length, Padding, Point, Size};

use super::menu::Menu;
use crate::geometry::anchor_position;
use crate::spacing;

/// Gap between the pointer and the menu, and between the menu and the edges
/// of its area.
const MARGIN: f32 = spacing::SPACE_2;

/// Where a context menu opens: the pointer, measured from the top-left
/// corner of an area, and the size of that area, which the menu stays
/// inside.
///
/// A feature keeps it from the right click that opened the menu until the
/// menu is drawn.
///
/// # Example
///
/// ```
/// use iced::{Point, Size};
/// use otty_ui_kit::components::context_menu::MenuPlace;
///
/// let place =
///     MenuPlace::new(Point::new(120.0, 80.0), Size::new(800.0, 600.0));
///
/// assert_eq!(place.cursor(), Point::new(120.0, 80.0));
/// assert_eq!(place.area(), Size::new(800.0, 600.0));
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MenuPlace {
    cursor: Point,
    area: Size,
}

impl MenuPlace {
    /// The pointer, measured from the top-left corner of the area.
    pub fn cursor(&self) -> Point {
        self.cursor
    }

    /// The size of the area the menu stays inside.
    pub fn area(&self) -> Size {
        self.area
    }

    /// A place at `cursor` in an area of `area`.
    pub fn new(cursor: Point, area: Size) -> Self {
        Self { cursor, area }
    }
}

/// Places `menu` next to the pointer of `place` and inside its area.
///
/// A press on the menu itself, between its items or on a disabled item,
/// stays on it. The menu has no layer of its own that takes a press away
/// from it: such a press reaches what lies under it, the trigger of
/// another menu too, and the app closes the menu (in OTTY the frame does,
/// for every menu). Draw the result over the whole area, for example as a
/// layer of a `stack`.
///
/// # Example
///
/// ```
/// use iced::{Element, Point, Size};
/// use otty_ui_kit::components::context_menu::{MenuPlace, context_menu};
/// use otty_ui_kit::components::menu::{Menu, MenuItem};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Rename,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let menu = Menu::new(&palette)
///     .item(MenuItem::new("Rename").on_press(Message::Rename));
/// let layer: Element<'_, Message> = context_menu(
///     menu,
///     MenuPlace::new(Point::new(120.0, 80.0), Size::new(800.0, 600.0)),
/// );
/// ```
pub fn context_menu<'a, Message: Clone + 'a>(
    menu: Menu<Message>,
    place: MenuPlace,
) -> Element<'a, Message> {
    let menu_size = menu.size();
    let anchor = anchor_position(
        place.cursor(),
        place.area(),
        menu_size.width,
        menu_size.height,
        MARGIN,
    );

    // `opaque` keeps a press on the menu that no item takes from reaching
    // what lies under it.
    container(opaque(menu))
        .padding(Padding {
            top: anchor.y,
            left: anchor.x,
            ..Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::mouse::{self, Button};
    use iced::widget::mouse_area;
    use iced::{Event, Length, Rectangle};
    use iced_test::simulator;

    use super::*;
    use crate::components::menu::{self, MenuItem};
    use crate::palette::UiPalette;
    use crate::size;

    /// Size of the simulated window, which is also the area of the menu.
    const AREA: Size = Size::new(1024.0, 768.0);
    /// Where the menu opens in most tests: far from every edge.
    const CURSOR: Point = Point::new(40.0, 40.0);

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Open,
        Delete,
        Under,
    }

    /// Open, a separator, a disabled Rename that says why and Delete.
    fn sample_menu() -> Menu<Message> {
        Menu::new(&UiPalette::otty_dark())
            .item(MenuItem::new("Open").on_press(Message::Open))
            .separator()
            .item(MenuItem::new("Rename").disabled_reason("Not yet"))
            .item(MenuItem::new("Delete").on_press(Message::Delete))
    }

    /// The menu at `cursor` over a layer that sends `Under` on a press of
    /// any button, as a widget under the menu does.
    fn layer<'a>(cursor: Point) -> Element<'a, Message> {
        let under = mouse_area(
            container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::Under)
        .on_right_press(Message::Under)
        .on_middle_press(Message::Under);
        let menu = context_menu(sample_menu(), MenuPlace::new(cursor, AREA));

        iced::widget::stack![under, menu]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Press and release `button` at `position` and return what was sent.
    fn press_at(position: Point, button: Button) -> Vec<Message> {
        let mut ui = simulator(layer(CURSOR));

        ui.point_at(position);
        let _ = ui.simulate([
            Event::Mouse(mouse::Event::ButtonPressed(button)),
            Event::Mouse(mouse::Event::ButtonReleased(button)),
        ]);

        ui.into_messages().collect()
    }

    #[test]
    fn given_open_menu_when_item_pressed_then_its_message_is_sent() {
        let mut ui = simulator(layer(CURSOR));

        ui.click("Delete").expect("item should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, [Message::Delete]);
    }

    /// Issue 42: a press away from the menu reaches what lies under it,
    /// such as the trigger of another menu.
    #[test]
    fn given_open_menu_when_any_button_pressed_away_then_it_reaches_under() {
        let away = Point::new(900.0, 700.0);

        for button in [Button::Left, Button::Right, Button::Middle] {
            assert_eq!(press_at(away, button), [Message::Under]);
        }
    }

    /// The separator is the row under the first item: a press on it is a
    /// press on the menu, not away from it.
    #[test]
    fn given_open_menu_when_separator_pressed_then_menu_stays_open() {
        let on_separator = Point::new(
            CURSOR.x + MARGIN + menu::MENU_WIDTH / 2.0,
            CURSOR.y + MARGIN + menu::MENU_PADDING + size::ROW + 4.0,
        );

        assert!(press_at(on_separator, Button::Left).is_empty());
        assert!(press_at(on_separator, Button::Right).is_empty());
    }

    #[test]
    fn given_open_menu_when_disabled_item_pressed_then_menu_stays_open() {
        let mut ui = simulator(layer(CURSOR));

        ui.click("Rename").expect("item should be visible");

        assert_eq!(ui.into_messages().count(), 0);
    }

    /// Issue 42: the pointer moving over a disabled item, which shows its
    /// reason, and a press on it send nothing: the menu stays open.
    #[test]
    fn given_open_menu_when_disabled_item_hovered_and_pressed_then_it_stays() {
        let mut ui = simulator(layer(CURSOR));
        let item = ui.find("Rename").expect("item should be visible").bounds();

        for position in [item.center(), Point::new(item.x + 2.0, item.y + 2.0)]
        {
            ui.point_at(position);
            let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
                position,
            })]);
        }
        let _ = ui.simulate(iced_test::simulator::click());

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_room_when_placed_then_menu_opens_right_under_the_pointer() {
        let mut ui = simulator(layer(CURSOR));

        let item = ui.find("Open").expect("item should be visible").bounds();

        assert!(item.x >= CURSOR.x + MARGIN);
        assert!(item.y >= CURSOR.y + MARGIN);
        assert!(item.x < CURSOR.x + MARGIN + menu::MENU_WIDTH);
    }

    #[test]
    fn given_pointer_in_the_corner_when_placed_then_menu_stays_inside_area() {
        let corner = Point::new(AREA.width - 1.0, AREA.height - 1.0);
        let mut ui = simulator(layer(corner));

        let first = ui.find("Open").expect("item should be visible").bounds();
        let last = ui.find("Delete").expect("item should be visible").bounds();

        let area = Rectangle::with_size(AREA);
        assert!(
            first.is_within(&area),
            "{first:?} should be inside {AREA:?}"
        );
        assert!(last.is_within(&area), "{last:?} should be inside {AREA:?}");
    }
}
