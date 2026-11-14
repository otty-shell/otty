//! `context_menu`: a [`Menu`] opened at the pointer.

use iced::widget::{Space, Stack, container, mouse_area, opaque};
use iced::{Element, Length, Padding, Point, Size};

use super::menu::Menu;
use crate::geometry::anchor_position;
use crate::spacing;

/// Gap between the pointer and the menu, and between the menu and the edges
/// of its area.
const MARGIN: f32 = spacing::SPACE_2;

/// Places `menu` next to `cursor` and inside `area`, over a layer that fills
/// the area and sends `on_dismiss` on a press of any mouse button away from
/// the menu.
///
/// `cursor` is measured from the top-left corner of the area. A press on the
/// menu itself, between its items or on a disabled item, does not close it.
/// Draw the result over the whole area, for example as a layer of a `stack`.
///
/// # Example
///
/// ```
/// use iced::{Element, Point, Size};
/// use otty_ui_kit::components::context_menu::context_menu;
/// use otty_ui_kit::components::menu::{Menu, MenuItem};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Rename,
///     CloseMenu,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let menu = Menu::new(&palette)
///     .item(MenuItem::new("Rename").on_press(Message::Rename));
/// let layer: Element<'_, Message> = context_menu(
///     menu,
///     Point::new(120.0, 80.0),
///     Size::new(800.0, 600.0),
///     Message::CloseMenu,
/// );
/// ```
pub fn context_menu<'a, Message: Clone + 'a>(
    menu: Menu<Message>,
    cursor: Point,
    area: Size,
    on_dismiss: Message,
) -> Element<'a, Message> {
    let menu_size = menu.size();
    let anchor = anchor_position(
        cursor,
        area,
        menu_size.width,
        menu_size.height,
        MARGIN,
    );

    // `opaque` keeps a press on the menu that no item takes from reaching
    // the dismiss layer under it.
    let placed_menu = container(opaque(menu)).padding(Padding {
        top: anchor.y,
        left: anchor.x,
        ..Padding::ZERO
    });
    let dismiss_layer = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(on_dismiss.clone())
    .on_right_press(on_dismiss.clone())
    .on_middle_press(on_dismiss);

    Stack::with_children(vec![dismiss_layer.into(), placed_menu.into()])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::mouse::{self, Button};
    use iced::{Event, Rectangle};
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
        Dismiss,
    }

    /// Open, a separator, a disabled Rename and Delete.
    fn sample_menu() -> Menu<Message> {
        Menu::new(&UiPalette::otty_dark())
            .item(MenuItem::new("Open").on_press(Message::Open))
            .separator()
            .item(MenuItem::new("Rename"))
            .item(MenuItem::new("Delete").on_press(Message::Delete))
    }

    fn layer<'a>(cursor: Point) -> Element<'a, Message> {
        context_menu(sample_menu(), cursor, AREA, Message::Dismiss)
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

    #[test]
    fn given_open_menu_when_any_button_pressed_away_then_it_is_dismissed() {
        let away = Point::new(900.0, 700.0);

        for button in [Button::Left, Button::Right, Button::Middle] {
            assert_eq!(press_at(away, button), [Message::Dismiss]);
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
