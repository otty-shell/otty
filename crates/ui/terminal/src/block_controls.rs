use iced::Rectangle;

use crate::block_layout::BlockRect;

const ACTION_BUTTON_PADDING: f32 = 6.0;
const ACTION_BUTTON_MIN_SIZE: f32 = 14.0;
const ACTION_BUTTON_SIZE_FACTOR: f32 = 1.05;

#[derive(Clone, Debug)]
pub struct BlockActionButtonGeometry {
    pub block_id: String,
    pub rect: Rectangle<f32>,
}

/// Compute the on-screen rectangle for a block action button, if the block is
/// visible within the viewport.
pub fn compute_action_button_geometry(
    block_rect: &BlockRect,
    cell_height: f32,
) -> Option<BlockActionButtonGeometry> {
    let block_height = block_rect.rect.height;
    if block_height <= 0.0 || block_rect.rect.width <= 0.0 {
        return None;
    }

    let padding = ACTION_BUTTON_PADDING.min(block_height / 2.0);
    let available_height = (block_height - padding).max(0.0);
    if available_height <= 0.0 {
        return None;
    }

    let mut size = cell_height * ACTION_BUTTON_SIZE_FACTOR;
    let min_size = ACTION_BUTTON_MIN_SIZE.min(block_height);
    let clamp_min = min_size.min(available_height);
    size = size.clamp(clamp_min, available_height);

    let block_pixel_top = block_rect.rect.y;
    let max_y = block_pixel_top + block_height - size - padding;
    let mut y = block_pixel_top + padding;
    if y > max_y {
        y = max_y.max(block_pixel_top);
    }

    let x = block_rect.rect.x + block_rect.rect.width
        - size
        - ACTION_BUTTON_PADDING;

    Some(BlockActionButtonGeometry {
        block_id: block_rect.block_id.clone(),
        rect: Rectangle {
            x,
            y,
            width: size,
            height: size,
        },
    })
}

#[cfg(test)]
mod tests {
    use otty_libterm::surface::BlockKind;

    use super::*;

    fn make_block_rect(height: f32, width: f32) -> BlockRect {
        BlockRect {
            block_id: "block".into(),
            kind: BlockKind::Command,
            rect: Rectangle {
                x: 10.0,
                y: 20.0,
                width,
                height,
            },
        }
    }

    #[test]
    fn returns_none_for_invalid_geometry() {
        let flat_rect = make_block_rect(0.0, 100.0);
        assert!(compute_action_button_geometry(&flat_rect, 14.0).is_none());

        let zero_width = make_block_rect(10.0, 0.0);
        assert!(compute_action_button_geometry(&zero_width, 14.0).is_none());
    }

    #[test]
    fn clamps_button_to_available_height() {
        let rect = make_block_rect(12.0, 80.0);
        let button =
            compute_action_button_geometry(&rect, ACTION_BUTTON_MIN_SIZE)
                .expect("button");

        assert_eq!(button.block_id, "block");
        assert!((button.rect.height - 6.0).abs() < f32::EPSILON);
        assert!(button.rect.y >= rect.rect.y);
        assert!(
            button.rect.y + button.rect.height
                <= rect.rect.y + rect.rect.height
        );
    }

    #[test]
    fn aligns_button_to_right_edge() {
        let rect = make_block_rect(40.0, 120.0);
        let button =
            compute_action_button_geometry(&rect, 20.0).expect("button");

        let expected_x = rect.rect.x + rect.rect.width
            - button.rect.width
            - ACTION_BUTTON_PADDING;
        assert!((button.rect.x - expected_x).abs() < f32::EPSILON);
    }
}
