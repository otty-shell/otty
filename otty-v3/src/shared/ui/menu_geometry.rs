use iced::{Point, Size};

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

/// Total menu height for a given number of items.
pub(crate) fn menu_height_for_items(
    item_count: usize,
    item_height: f32,
    vertical_padding: f32,
) -> f32 {
    vertical_padding + item_height * item_count as f32
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
