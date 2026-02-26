use iced::Background;
use iced::widget::{container, scrollable};

use super::theme::IcedColorPalette;

/// Return a scrollbar style closure with thin rails and reduced alpha.
pub(crate) fn thin_scroll_style(
    palette: IcedColorPalette,
) -> impl Fn(&iced::Theme, scrollable::Status) -> scrollable::Style + 'static {
    move |theme, status| {
        let mut style = scrollable::default(theme, status);
        let radius = iced::border::Radius::from(0.0);

        style.vertical_rail.border.radius = radius;
        style.vertical_rail.scroller.border.radius = radius;
        style.horizontal_rail.border.radius = radius;
        style.horizontal_rail.scroller.border.radius = radius;

        let mut scroller_color = match style.vertical_rail.scroller.background {
            Background::Color(color) => color,
            _ => palette.dim_foreground,
        };
        scroller_color.a = (scroller_color.a * 0.7).min(1.0);
        style.vertical_rail.scroller.background =
            Background::Color(scroller_color);
        style.horizontal_rail.scroller.background =
            Background::Color(scroller_color);

        style
    }
}

/// Return a container style for tree rows with selection/hover highlights.
pub(crate) fn tree_row_style(
    palette: &IcedColorPalette,
    is_selected: bool,
    is_hovered: bool,
) -> container::Style {
    let background = if is_selected {
        let mut color = palette.dim_blue;
        color.a = 0.7;
        Some(color.into())
    } else if is_hovered {
        let mut color = palette.overlay;
        color.a = 0.6;
        Some(color.into())
    } else {
        None
    };

    container::Style {
        background,
        text_color: Some(palette.foreground),
        ..Default::default()
    }
}
