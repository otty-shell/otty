//! `Divider`: the 1px `line` rule between panes, panels and bars.

use iced::widget::container;
use iced::{Element, Length};

use crate::palette::UiPalette;
use crate::size;

/// Returns a horizontal rule: `size::LINE` high and as wide as its parent.
pub fn horizontal<'a, Message: 'a>(
    palette: &UiPalette,
) -> Element<'a, Message> {
    rule(palette, Length::Fill, Length::Fixed(size::LINE))
}

/// Returns a vertical rule: `size::LINE` wide and as high as its parent.
pub fn vertical<'a, Message: 'a>(palette: &UiPalette) -> Element<'a, Message> {
    rule(palette, Length::Fixed(size::LINE), Length::Fill)
}

fn rule<'a, Message: 'a>(
    palette: &UiPalette,
    width: Length,
    height: Length,
) -> Element<'a, Message> {
    let line = palette.line();

    container("")
        .width(width)
        .height(height)
        .style(move |_| container::Style {
            background: Some(line.into()),
            ..container::Style::default()
        })
        .into()
}
