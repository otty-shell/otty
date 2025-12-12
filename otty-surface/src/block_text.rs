use crate::block::{BlockKind, BlockSnapshot};
use crate::cell::Flags;
use crate::snapshot::{SnapshotCell, SnapshotOwned, SnapshotView};

/// Trim trailing ASCII spaces from a buffer and return an owned string.
fn trim_trailing_spaces(buffer: &str) -> String {
    buffer.trim_end_matches(' ').to_string()
}

/// Aggregate the textual content for the provided block from snapshot cells.
///
/// Returns `None` for prompt blocks or entries with zero visible lines.
pub fn collect_block_text(
    block: &BlockSnapshot,
    cells: &[SnapshotCell],
) -> Option<String> {
    if let Some(text) = &block.cached_text {
        if matches!(block.meta.kind, BlockKind::Prompt) {
            return None;
        }
        let text = text.as_ref();
        if text.is_empty() {
            return None;
        }
        return Some(text.to_string());
    }

    if block.line_count == 0 || matches!(block.meta.kind, BlockKind::Prompt) {
        return None;
    }

    let start = block.start_line;
    let end = start + block.line_count as i32;
    let mut lines = Vec::with_capacity(block.line_count);
    let mut current_line = None;
    let mut buffer = String::new();

    for cell in cells {
        let line_value = cell.point.line.0;
        if line_value < start || line_value >= end {
            continue;
        }

        if current_line != Some(line_value) {
            if current_line.is_some() {
                lines.push(trim_trailing_spaces(&buffer));
                buffer.clear();
            }
            current_line = Some(line_value);
        }

        if !cell.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            buffer.push(cell.cell.c);
        }
    }

    if current_line.is_some() {
        lines.push(trim_trailing_spaces(&buffer));
    }

    Some(lines.join("\n"))
}

impl<'a> SnapshotView<'a> {
    /// Return the concatenated text for a block identified by its id.
    pub fn block_text(&self, block_id: &str) -> Option<String> {
        let block = self.blocks.iter().find(|b| b.meta.id == block_id)?;
        self.block_text_from_snapshot(block)
    }

    /// Return the concatenated text for the provided block snapshot.
    pub fn block_text_from_snapshot(
        &self,
        block: &BlockSnapshot,
    ) -> Option<String> {
        collect_block_text(block, self.cells)
    }

    /// Return only the prompt/input line for the provided block id.
    ///
    /// The prompt text is derived from the first visible line of a command
    /// block and excludes any subsequent output lines. Returns `None` for
    /// non-command blocks or entries without visible lines.
    pub fn block_prompt_text(&self, block_id: &str) -> Option<String> {
        let block = self.blocks.iter().find(|b| b.meta.id == block_id)?;
        self.block_prompt_text_from_snapshot(block)
    }

    /// Return only the prompt/input line for the provided block snapshot.
    ///
    /// The returned string contains the first line of the block, trimmed of
    /// trailing ASCII spaces, and omits any output lines that belong to the
    /// same block.
    pub fn block_prompt_text_from_snapshot(
        &self,
        block: &BlockSnapshot,
    ) -> Option<String> {
        if let Some(text) = &block.cached_text {
            if block.meta.kind != BlockKind::Command {
                return None;
            }

            let first = text.lines().next().unwrap_or_default();
            if first.is_empty() {
                return None;
            }

            return Some(first.to_string());
        }

        if block.line_count == 0 || block.meta.kind != BlockKind::Command {
            return None;
        }

        let start = block.start_line;
        let end = start + block.line_count as i32;
        let mut current_line = None;
        let mut buffer = String::new();

        for cell in self.cells {
            let line_value = cell.point.line.0;
            if line_value < start || line_value >= end {
                continue;
            }

            if current_line.is_none() {
                current_line = Some(line_value);
            } else if current_line != Some(line_value) {
                break;
            }

            if !cell.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                buffer.push(cell.cell.c);
            }
        }

        if current_line.is_some() && !buffer.is_empty() {
            Some(trim_trailing_spaces(&buffer))
        } else {
            None
        }
    }
}

impl SnapshotOwned {
    /// Return the concatenated text for a block identified by its id.
    pub fn block_text(&self, block_id: &str) -> Option<String> {
        self.view().block_text(block_id)
    }

    /// Return the concatenated text for the provided block snapshot.
    pub fn block_text_from_snapshot(
        &self,
        block: &BlockSnapshot,
    ) -> Option<String> {
        self.view().block_text_from_snapshot(block)
    }

    /// Return only the prompt/input line for the provided block id.
    pub fn block_prompt_text(&self, block_id: &str) -> Option<String> {
        self.view().block_prompt_text(block_id)
    }

    /// Return only the prompt/input line for the provided block snapshot.
    pub fn block_prompt_text_from_snapshot(
        &self,
        block: &BlockSnapshot,
    ) -> Option<String> {
        self.view().block_prompt_text_from_snapshot(block)
    }
}

#[cfg(test)]
mod tests {
    use crate::block::{BlockKind, BlockMeta, BlockSnapshot};
    use crate::cell::{Cell, Flags};
    use crate::hyperlink::HyperlinkMap;
    use crate::index::{Column, Line, Point};
    use crate::mode::SurfaceMode;
    use crate::snapshot::{
        CursorSnapshot, SnapshotCell, SnapshotDamage, SnapshotOwned,
        SnapshotSize,
    };
    use crate::{Colors, SelectionRange};

    use super::collect_block_text;

    fn block_snapshot(
        id: &str,
        kind: BlockKind,
        start_line: i32,
        line_count: usize,
    ) -> BlockSnapshot {
        BlockSnapshot {
            meta: BlockMeta {
                id: id.into(),
                kind,
                ..BlockMeta::default()
            },
            start_line,
            line_count,
            cached_text: None,
            is_alt_screen: false,
        }
    }

    fn cell(line: i32, column: usize, ch: char) -> SnapshotCell {
        let cell = Cell {
            c: ch,
            ..Cell::default()
        };
        SnapshotCell {
            point: Point::new(Line(line), Column(column)),
            cell,
        }
    }

    fn build_snapshot(
        cells: Vec<SnapshotCell>,
        blocks: Vec<BlockSnapshot>,
    ) -> SnapshotOwned {
        SnapshotOwned::from_parts(
            cells,
            None::<SelectionRange>,
            HyperlinkMap::default(),
            CursorSnapshot::default(),
            0,
            Colors::default(),
            SurfaceMode::default(),
            SnapshotSize {
                columns: 80,
                screen_lines: 2,
                total_lines: 2,
            },
            SnapshotDamage::Full,
            160,
            blocks,
        )
    }

    #[test]
    fn collects_multiple_lines_for_command_block() {
        let block = block_snapshot("block-1", BlockKind::Command, 0, 2);
        let cells = vec![
            cell(0, 0, 'e'),
            cell(0, 1, 'c'),
            cell(0, 2, 'h'),
            cell(0, 3, 'o'),
            cell(0, 4, ' '),
            cell(0, 5, ' '),
            cell(0, 6, 'h'),
            cell(0, 7, 'i'),
            cell(1, 0, 'o'),
            cell(1, 1, 'k'),
        ];

        let content = collect_block_text(&block, &cells);

        assert_eq!(content.as_deref(), Some("echo  hi\nok"));
    }

    #[test]
    fn returns_none_for_prompt_block() {
        let block = block_snapshot("prompt", BlockKind::Prompt, 0, 1);
        let cells = vec![cell(0, 0, '$')];

        assert!(collect_block_text(&block, &cells).is_none());
    }

    #[test]
    fn returns_none_for_empty_block() {
        let block = block_snapshot("empty", BlockKind::Command, 0, 0);
        let cells = vec![cell(0, 0, 'a')];

        assert!(collect_block_text(&block, &cells).is_none());
    }

    #[test]
    fn keeps_fullscreen_blocks() {
        let block = block_snapshot("fs", BlockKind::FullScreen, 0, 1);
        let cells = vec![cell(0, 0, 'Z')];

        assert_eq!(collect_block_text(&block, &cells), Some("Z".into()));
    }

    #[test]
    fn skips_wide_char_spacers() {
        let mut wide_cell = Cell {
            c: '漢',
            ..Cell::default()
        };
        wide_cell.flags.insert(Flags::WIDE_CHAR);

        let mut spacer = Cell {
            c: ' ',
            ..Cell::default()
        };
        spacer.flags.insert(Flags::WIDE_CHAR_SPACER);

        let block = block_snapshot("wide", BlockKind::Command, 0, 1);
        let cells = vec![
            SnapshotCell {
                point: Point::new(Line(0), Column(0)),
                cell: wide_cell,
            },
            SnapshotCell {
                point: Point::new(Line(0), Column(1)),
                cell: spacer,
            },
        ];

        assert_eq!(collect_block_text(&block, &cells), Some("漢".to_string()));
    }

    #[test]
    fn snapshot_view_exposes_block_text_lookup() {
        let block = block_snapshot("block-1", BlockKind::Command, 0, 1);
        let snapshot = build_snapshot(vec![cell(0, 0, 'x')], vec![block]);
        let view = snapshot.view();

        assert_eq!(view.block_text("block-1"), Some("x".into()));
    }

    #[test]
    fn snapshot_owned_proxies_block_text_lookup() {
        let block = block_snapshot("block-2", BlockKind::Command, 0, 1);
        let snapshot = build_snapshot(vec![cell(0, 0, 'y')], vec![block]);

        assert_eq!(snapshot.block_text("block-2"), Some("y".into()));
    }

    #[test]
    fn block_prompt_text_returns_first_line_for_command_block() {
        let block = block_snapshot("block-1", BlockKind::Command, 0, 2);
        let cells = vec![
            cell(0, 0, 'p'),
            cell(0, 1, 's'),
            cell(0, 2, '1'),
            cell(0, 3, ' '),
            cell(0, 4, 'c'),
            cell(0, 5, 'm'),
            cell(1, 0, 'o'),
            cell(1, 1, 'k'),
        ];

        let snapshot = build_snapshot(cells, vec![block]);
        let view = snapshot.view();

        assert_eq!(
            view.block_prompt_text("block-1").as_deref(),
            Some("ps1 cm")
        );
    }

    #[test]
    fn block_prompt_text_returns_none_for_non_command_block() {
        let block = block_snapshot("prompt", BlockKind::Prompt, 0, 1);
        let snapshot = build_snapshot(vec![cell(0, 0, '$')], vec![block]);
        let view = snapshot.view();

        assert!(view.block_prompt_text("prompt").is_none());
    }
}
