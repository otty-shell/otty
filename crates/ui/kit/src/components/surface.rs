//! `Surface`: the filled container that backs a panel, bar or canvas.

use iced::widget::{Container, container};
use iced::{Element, Length};

use crate::palette::UiPalette;

/// The background layer a [`surface`] paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// Window chrome: title bar and menu rail (`surface_chrome`).
    Chrome,
    /// Side panels and their headers (`surface_panel`).
    Panel,
    /// The working area behind content (`surface_canvas`).
    Canvas,
}

/// Returns `content` in a container that fills its parent, clips overflow,
/// and paints the background of `surface` with the body text color.
///
/// The container is returned unfinished, so a caller can still set a fixed
/// size, padding or alignment.
pub fn surface<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    surface: Surface,
    palette: &UiPalette,
) -> Container<'a, Message> {
    let background = match surface {
        Surface::Chrome => palette.surface_chrome(),
        Surface::Panel => palette.surface_panel(),
        Surface::Canvas => palette.surface_canvas(),
    };
    let text_color = palette.text_body();

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(background.into()),
            text_color: Some(text_color),
            ..container::Style::default()
        })
}
