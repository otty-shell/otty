use iced::{Point, Size};
use otty_ui_kit::components::menu;
use otty_ui_kit::size;

/// Compute a context menu anchor position near the cursor, clamped to bounds.
pub(crate) fn anchor_position(
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

/// Total height of a kit `Menu` with `item_count` items (no separators).
pub(crate) fn menu_height_for_items(item_count: usize) -> f32 {
    2.0 * menu::MENU_PADDING + size::ROW * item_count as f32
}

#[cfg(test)]
mod tests {
    use otty_ui_kit::components::menu;
    use otty_ui_kit::spacing;

    use super::*;

    const MENU_WIDTH: f32 = menu::MENU_WIDTH;

    #[test]
    fn given_three_items_when_menu_height_then_rows_plus_kit_padding() {
        let height = menu_height_for_items(3);
        assert_eq!(height, 3.0 * size::ROW + 2.0 * menu::MENU_PADDING);
        assert_eq!(height, 100.0);
    }

    #[test]
    fn given_no_items_when_menu_height_then_only_padding() {
        assert_eq!(menu_height_for_items(0), 2.0 * menu::MENU_PADDING);
    }

    #[test]
    fn given_menu_at_bottom_right_when_placed_then_kit_menu_fits_window() {
        let grid = Size::new(600.0, 400.0);
        let height = menu_height_for_items(6);
        let anchor = anchor_position(
            Point::new(599.0, 399.0),
            grid,
            MENU_WIDTH,
            height,
            spacing::SPACE_2,
        );
        assert!(anchor.x >= 0.0 && anchor.y >= 0.0);
        assert!(anchor.x + MENU_WIDTH <= grid.width);
        assert!(anchor.y + height <= grid.height);
    }

    #[test]
    fn given_cursor_outside_bounds_when_anchor_position_then_clamps_inside_bounds()
     {
        let grid = Size::new(400.0, 300.0);
        let cursor = Point::new(390.0, 290.0);
        let menu_height = 140.0;
        let anchor = anchor_position(
            cursor,
            grid,
            MENU_WIDTH,
            menu_height,
            spacing::SPACE_2,
        );
        assert!(anchor.x >= spacing::SPACE_2);
        assert!(anchor.y >= spacing::SPACE_2);
        assert!(anchor.x + MENU_WIDTH <= grid.width - spacing::SPACE_2 + 0.1);
        assert!(anchor.y + menu_height <= grid.height - spacing::SPACE_2 + 0.1);
    }

    #[test]
    fn given_space_available_when_anchor_position_then_stays_near_cursor() {
        let grid = Size::new(800.0, 600.0);
        let cursor = Point::new(100.0, 120.0);
        let menu_height = 140.0;
        let anchor = anchor_position(
            cursor,
            grid,
            MENU_WIDTH,
            menu_height,
            spacing::SPACE_2,
        );
        assert!((anchor.x - (cursor.x + spacing::SPACE_2)).abs() < 0.1);
        assert!((anchor.y - (cursor.y + spacing::SPACE_2)).abs() < 0.1);
    }

    #[test]
    fn given_cursor_near_right_edge_when_anchor_position_then_flips_to_left() {
        let grid = Size::new(500.0, 400.0);
        let cursor = Point::new(490.0, 200.0);
        let menu_height = 140.0;
        let anchor = anchor_position(
            cursor,
            grid,
            MENU_WIDTH,
            menu_height,
            spacing::SPACE_2,
        );
        assert!(anchor.x < cursor.x);
        assert!(cursor.x - anchor.x >= MENU_WIDTH - spacing::SPACE_2);
    }
}
