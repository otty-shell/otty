use iced::widget::{container, mouse_area, stack, text};
use iced::window::Direction;
use iced::{Element, Length, Theme, mouse};

const RESIZE_EDGE_THICKNESS: f32 = 6.0;
const RESIZE_CORNER_THICKNESS: f32 = 12.0;

/// Events emitted by the window resize grips.
#[derive(Debug, Clone)]
pub(crate) enum ResizeGripEvent {
    Resize(Direction),
}

/// Render the eight-directional window-resize grip overlays.
pub(crate) fn view() -> Element<'static, ResizeGripEvent, Theme, iced::Renderer>
{
    let n_grip = mouse_area(
        container(text(""))
            .width(Length::Fill)
            .height(Length::Fixed(RESIZE_EDGE_THICKNESS)),
    )
    .on_press(ResizeGripEvent::Resize(Direction::North))
    .interaction(mouse::Interaction::ResizingVertically);

    let s_grip = mouse_area(
        container(text(""))
            .width(Length::Fill)
            .height(Length::Fixed(RESIZE_EDGE_THICKNESS)),
    )
    .on_press(ResizeGripEvent::Resize(Direction::South))
    .interaction(mouse::Interaction::ResizingVertically);

    let e_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_EDGE_THICKNESS))
            .height(Length::Fill),
    )
    .on_press(ResizeGripEvent::Resize(Direction::East))
    .interaction(mouse::Interaction::ResizingHorizontally);

    let w_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_EDGE_THICKNESS))
            .height(Length::Fill),
    )
    .on_press(ResizeGripEvent::Resize(Direction::West))
    .interaction(mouse::Interaction::ResizingHorizontally);

    let nw_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_THICKNESS)),
    )
    .on_press(ResizeGripEvent::Resize(Direction::NorthWest))
    .interaction(mouse::Interaction::ResizingDiagonallyDown);

    let ne_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_THICKNESS)),
    )
    .on_press(ResizeGripEvent::Resize(Direction::NorthEast))
    .interaction(mouse::Interaction::ResizingDiagonallyUp);

    let sw_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_THICKNESS)),
    )
    .on_press(ResizeGripEvent::Resize(Direction::SouthWest))
    .interaction(mouse::Interaction::ResizingDiagonallyUp);

    let se_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_THICKNESS)),
    )
    .on_press(ResizeGripEvent::Resize(Direction::SouthEast))
    .interaction(mouse::Interaction::ResizingDiagonallyDown);

    stack!(
        container(n_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Top),
        container(s_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Bottom),
        container(e_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right),
        container(w_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left),
        container(nw_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top),
        container(ne_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Top),
        container(sw_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Bottom),
        container(se_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Bottom),
    )
    .into()
}
