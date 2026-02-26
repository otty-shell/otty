use iced::widget::{container, scrollable};
use iced::{Background, Point, Size};

use crate::theme::{IcedColorPalette, ThemeProps};

/// Compute a context menu anchor position near the cursor, clamped to bounds.
///
/// ```rust,ignore
/// # use iced::{Point, Size};
/// # use crate::ui::widgets::services::anchor_position;
/// let cursor = Point::new(120.0, 80.0);
/// let grid = Size::new(800.0, 600.0);
/// let anchor = anchor_position(cursor, grid, 220.0, 160.0, 6.0);
/// assert!(anchor.x >= 6.0);
/// assert!(anchor.y >= 6.0);
/// ```
pub(super) fn anchor_position(
    cursor: Point,
    grid_size: Size,
    menu_width: f32,
    menu_height: f32,
    margin: f32,
) -> Point {
    let clamped_cursor = Point::new(
        cursor.x.clamp(0.0, grid_size.width),
        cursor.y.clamp(0.0, grid_size.height),
    );

    let fits_right = clamped_cursor.x + menu_width + margin <= grid_size.width;
    let x = if fits_right {
        (clamped_cursor.x + margin).min(grid_size.width - margin - menu_width)
    } else {
        (clamped_cursor.x - menu_width - margin).max(margin)
    };

    let fits_down = clamped_cursor.y + menu_height + margin <= grid_size.height;
    let y = if fits_down {
        (clamped_cursor.y + margin).min(grid_size.height - margin - menu_height)
    } else {
        (clamped_cursor.y - menu_height - margin).max(margin)
    };

    let max_x = (grid_size.width - menu_width - margin).max(margin);
    let max_y = (grid_size.height - menu_height - margin).max(margin);

    Point::new(x.clamp(margin, max_x), y.clamp(margin, max_y))
}

pub(super) fn thin_scroll_style(
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

pub(super) fn tree_row_style(
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

pub(super) fn menu_height_for_items(
    item_count: usize,
    item_height: f32,
    vertical_padding: f32,
) -> f32 {
    vertical_padding + item_height * item_count as f32
}

pub(super) fn menu_panel_style(
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

#[cfg(test)]
mod tests {
    use super::*;

    const MENU_WIDTH: f32 = 250.0;
    const MENU_MARGIN: f32 = 6.0;

    #[test]
    fn given_cursor_outside_bounds_when_anchor_position_then_clamps_inside_bounds()
     {
        let grid = Size::new(400.0, 300.0);
        let cursor = Point::new(390.0, 290.0);
        let menu_height = 140.0;
        let anchor =
            anchor_position(cursor, grid, MENU_WIDTH, menu_height, MENU_MARGIN);
        assert!(anchor.x >= MENU_MARGIN);
        assert!(anchor.y >= MENU_MARGIN);
        assert!(anchor.x + MENU_WIDTH <= grid.width - MENU_MARGIN + 0.1);
        assert!(anchor.y + menu_height <= grid.height - MENU_MARGIN + 0.1);
    }

    #[test]
    fn given_space_available_when_anchor_position_then_stays_near_cursor() {
        let grid = Size::new(800.0, 600.0);
        let cursor = Point::new(100.0, 120.0);
        let menu_height = 140.0;
        let anchor =
            anchor_position(cursor, grid, MENU_WIDTH, menu_height, MENU_MARGIN);
        assert!((anchor.x - (cursor.x + MENU_MARGIN)).abs() < 0.1);
        assert!((anchor.y - (cursor.y + MENU_MARGIN)).abs() < 0.1);
    }

    #[test]
    fn given_cursor_near_right_edge_when_anchor_position_then_flips_to_left() {
        let grid = Size::new(500.0, 400.0);
        let cursor = Point::new(490.0, 200.0);
        let menu_height = 140.0;
        let anchor =
            anchor_position(cursor, grid, MENU_WIDTH, menu_height, MENU_MARGIN);
        assert!(anchor.x < cursor.x);
        assert!(cursor.x - anchor.x >= MENU_WIDTH - MENU_MARGIN);
    }
}
