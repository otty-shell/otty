use iced::{Point, Rectangle, Size};
use otty_libterm::surface::{BlockKind, BlockSnapshot, SnapshotView};

const ACTION_BUTTON_PADDING: f32 = 6.0;
const ACTION_BUTTON_MIN_SIZE: f32 = 14.0;
const ACTION_BUTTON_SIZE_FACTOR: f32 = 1.05;

#[derive(Clone, Debug)]
pub(crate) struct BlockActionButtonGeometry {
    pub block_id: String,
    pub rect: Rectangle<f32>,
}

fn block_for_id<'a>(
    snapshot: &'a SnapshotView<'_>,
    block_id: &str,
) -> Option<&'a BlockSnapshot> {
    snapshot.blocks().iter().find(|block| {
        block.id == block_id && block.meta.kind != BlockKind::Prompt
    })
}

/// Compute the on-screen rectangle for a block action button, if the block is
/// visible within the viewport.
pub(crate) fn compute_action_button_geometry(
    snapshot: &SnapshotView<'_>,
    block_id: &str,
    layout_position: Point,
    layout_size: Size<f32>,
    cell_height: f32,
) -> Option<BlockActionButtonGeometry> {
    let block = block_for_id(snapshot, block_id)?;
    if block.line_count == 0 {
        return None;
    }

    let block_top = (block.start_line as f32 + snapshot.display_offset as f32)
        * cell_height;
    let block_height = block.line_count as f32 * cell_height;
    if block_height <= 0.0 {
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

    let block_pixel_top = layout_position.y + block_top;
    let max_y = block_pixel_top + block_height - size - padding;
    let mut y = block_pixel_top + padding;
    if y > max_y {
        y = max_y.max(block_pixel_top);
    }

    let x =
        layout_position.x + layout_size.width - size - ACTION_BUTTON_PADDING;

    Some(BlockActionButtonGeometry {
        block_id: block_id.to_string(),
        rect: Rectangle {
            x,
            y,
            width: size,
            height: size,
        },
    })
}
