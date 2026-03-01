use iced::Background;
use iced::widget::{container, scrollable};

use super::theme::{ThemeProps, IcedColorPalette};

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

pub(crate) fn menu_panel_style(
    theme: ThemeProps<'_>,
) -> impl Fn(&iced::Theme) -> container::Style + 'static {
    let palette = theme.theme.iced_palette().clone();
    move |_theme: &iced::Theme| container::Style {
        background: Some(palette.overlay.into()),
        text_color: Some(palette.foreground),
        border: iced::Border {
            width: 0.25,
            color: palette.overlay,
            radius: iced::border::Radius::new(4.0),
        },
        ..Default::default()
    }
}
