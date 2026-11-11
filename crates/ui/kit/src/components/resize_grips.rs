//! `ResizeGrips`: invisible strips along the window edges that start a window
//! resize when pressed. For windows without system decorations.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Space, container, mouse_area, stack};
use iced::window::Direction;
use iced::{Element, Length, mouse};

/// Thickness of the grip along each window edge. No size token: the grip is
/// not drawn.
const EDGE_THICKNESS: f32 = 6.0;
/// Width and height of the grip in each window corner. No size token: the
/// grip is not drawn.
const CORNER_SIZE: f32 = 12.0;

/// Returns the grips for all four edges and corners. A press on one sends
/// `on_resize` with its direction, and the pointer shows a resize cursor over
/// it. Put it in a `stack` layer above the window content.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::window::Direction;
/// use otty_ui_kit::components::resize_grips::resize_grips;
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     ResizeWindow(Direction),
/// }
///
/// let grips: Element<'_, Message> = resize_grips(Message::ResizeWindow);
/// ```
pub fn resize_grips<'a, Message: Clone + 'a>(
    on_resize: fn(Direction) -> Message,
) -> Element<'a, Message> {
    let edge = Length::Fixed(EDGE_THICKNESS);
    let corner = Length::Fixed(CORNER_SIZE);

    // Corners come last so they lie above the edges where the two meet.
    stack![
        grip(Length::Fill, edge, Direction::North, on_resize),
        grip(Length::Fill, edge, Direction::South, on_resize),
        grip(edge, Length::Fill, Direction::East, on_resize),
        grip(edge, Length::Fill, Direction::West, on_resize),
        grip(corner, corner, Direction::NorthWest, on_resize),
        grip(corner, corner, Direction::NorthEast, on_resize),
        grip(corner, corner, Direction::SouthWest, on_resize),
        grip(corner, corner, Direction::SouthEast, on_resize),
    ]
    .into()
}

/// One grip of `width` by `height`, pushed to the window side or corner that
/// `direction` names.
fn grip<'a, Message: Clone + 'a>(
    width: Length,
    height: Length,
    direction: Direction,
    on_resize: fn(Direction) -> Message,
) -> Element<'a, Message> {
    let (align_x, align_y, cursor) = match direction {
        Direction::North => (
            Horizontal::Left,
            Vertical::Top,
            mouse::Interaction::ResizingVertically,
        ),
        Direction::South => (
            Horizontal::Left,
            Vertical::Bottom,
            mouse::Interaction::ResizingVertically,
        ),
        Direction::East => (
            Horizontal::Right,
            Vertical::Top,
            mouse::Interaction::ResizingHorizontally,
        ),
        Direction::West => (
            Horizontal::Left,
            Vertical::Top,
            mouse::Interaction::ResizingHorizontally,
        ),
        Direction::NorthWest => (
            Horizontal::Left,
            Vertical::Top,
            mouse::Interaction::ResizingDiagonallyDown,
        ),
        Direction::NorthEast => (
            Horizontal::Right,
            Vertical::Top,
            mouse::Interaction::ResizingDiagonallyUp,
        ),
        Direction::SouthWest => (
            Horizontal::Left,
            Vertical::Bottom,
            mouse::Interaction::ResizingDiagonallyUp,
        ),
        Direction::SouthEast => (
            Horizontal::Right,
            Vertical::Bottom,
            mouse::Interaction::ResizingDiagonallyDown,
        ),
    };

    let strip = mouse_area(Space::new().width(width).height(height))
        .on_press(on_resize(direction))
        .interaction(cursor);

    container(strip)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(align_x)
        .align_y(align_y)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;

    use super::*;

    /// The size of the simulator window, see `iced_test::Simulator::new`.
    const WIDTH: f32 = 1024.0;
    const HEIGHT: f32 = 768.0;

    /// Presses the mouse at `x`, `y` and returns the names of the directions
    /// sent. `Direction` has no `PartialEq`, so the tests compare names.
    fn press_at(x: f32, y: f32) -> Vec<String> {
        let mut ui =
            simulator(resize_grips(|direction| format!("{direction:?}")));

        ui.point_at(Point::new(x, y));
        let _ = ui.simulate(simulator::click());

        ui.into_messages().collect()
    }

    #[test]
    fn given_grips_when_an_edge_is_pressed_then_the_window_resizes_that_way() {
        let middle_x = WIDTH / 2.0;
        let middle_y = HEIGHT / 2.0;

        assert_eq!(press_at(middle_x, 2.0), ["North"]);
        assert_eq!(press_at(middle_x, HEIGHT - 2.0), ["South"]);
        assert_eq!(press_at(WIDTH - 2.0, middle_y), ["East"]);
        assert_eq!(press_at(2.0, middle_y), ["West"]);
    }

    #[test]
    fn given_grips_when_a_corner_is_pressed_then_the_window_resizes_diagonally()
    {
        assert_eq!(press_at(8.0, 8.0), ["NorthWest"]);
        assert_eq!(press_at(WIDTH - 8.0, 8.0), ["NorthEast"]);
        assert_eq!(press_at(8.0, HEIGHT - 8.0), ["SouthWest"]);
        assert_eq!(press_at(WIDTH - 8.0, HEIGHT - 8.0), ["SouthEast"]);
    }

    #[test]
    fn given_grips_when_the_middle_is_pressed_then_nothing_is_sent() {
        assert!(press_at(WIDTH / 2.0, HEIGHT / 2.0).is_empty());
    }
}
