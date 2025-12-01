use iced::{Point, Rectangle, Size};
use otty_libterm::surface::{BlockKind, SnapshotView};

/// Rectangle describing the absolute geometry of a terminal block.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockRect {
    /// Identifier of the block the rectangle belongs to.
    pub block_id: String,
    /// Semantic kind of the block (prompt, command, fullscreen).
    pub kind: BlockKind,
    /// Absolute rectangle in layout coordinates.
    pub rect: Rectangle<f32>,
}

/// Compute layout rectangles for every visible block in the snapshot.
pub fn block_rects(
    view: &SnapshotView<'_>,
    layout_pos: Point,
    layout_size: Size<f32>,
    cell_height: f32,
) -> Vec<BlockRect> {
    if layout_size.width <= 0.0 || cell_height <= 0.0 {
        return Vec::new();
    }

    let display_offset = view.display_offset as f32;
    view.blocks()
        .iter()
        .filter_map(|block| {
            if block.line_count == 0 {
                return None;
            }

            let block_height = block.line_count as f32 * cell_height;
            if block_height <= 0.0 {
                return None;
            }

            let block_top = block.start_line as f32;
            let y = layout_pos.y + ((block_top + display_offset) * cell_height);

            Some(BlockRect {
                block_id: block.meta.id.clone(),
                kind: block.meta.kind.clone(),
                rect: Rectangle {
                    x: layout_pos.x,
                    y,
                    width: layout_size.width,
                    height: block_height,
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use otty_libterm::surface::{BlockMeta, BlockSnapshot, SnapshotOwned};

    fn block(
        id: &str,
        kind: BlockKind,
        start_line: i32,
        line_count: usize,
    ) -> BlockSnapshot {
        BlockSnapshot {
            meta: BlockMeta {
                id: id.to_string(),
                kind,
                ..BlockMeta::default()
            },
            start_line,
            line_count,
            is_alt_screen: false,
        }
    }

    fn snapshot_with_blocks(blocks: Vec<BlockSnapshot>) -> SnapshotOwned {
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks = blocks;
        snapshot
    }

    #[test]
    fn skips_blocks_without_height() {
        let snapshot = snapshot_with_blocks(vec![
            block("command", BlockKind::Command, 0, 2),
            block("empty", BlockKind::Command, 2, 0),
        ]);
        let view = snapshot.view();

        let rects = block_rects(
            &view,
            Point::new(10.0, 20.0),
            Size::new(120.0, 300.0),
            4.0,
        );

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].block_id, "command");
        assert_eq!(rects[0].rect.height, 8.0);
    }

    #[test]
    fn accounts_for_display_offset_and_layout_origin() {
        let snapshot = snapshot_with_blocks(vec![
            block("a", BlockKind::Command, 0, 2),
            block("prompt", BlockKind::Prompt, 2, 1),
        ]);
        let mut view = snapshot.view();
        view.display_offset = 3;

        let rects = block_rects(
            &view,
            Point::new(5.0, 7.0),
            Size::new(80.0, 200.0),
            5.0,
        );

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].rect.y, 7.0 + ((0.0 + 3.0) * 5.0));
        assert_eq!(rects[0].rect.height, 10.0);
        assert_eq!(rects[0].rect.width, 80.0);
        assert_eq!(rects[1].rect.y, 7.0 + ((2.0 + 3.0) * 5.0));
    }
}
