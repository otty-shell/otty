//! The output of a block, read from a snapshot for copying: what came
//! after its command lines (the prompt and the typed command).

use crate::block::{BlockKey, BlockKind, BlockSnapshot};
use crate::cell::Flags;
use crate::snapshot::{SnapshotCell, SnapshotView};

impl<'a> SnapshotView<'a> {
    /// Return the output of the block with `key`: its text after its
    /// command lines, the prompt and the typed command. `None` when there
    /// is no such text.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_surface::{BlockKey, BlockSnapshot, SnapshotOwned};
    ///
    /// let mut snapshot = SnapshotOwned::default();
    /// snapshot.blocks.push(BlockSnapshot {
    ///     key: BlockKey::new(1),
    ///     cached_text: Some("~/otty\n$ ls\nCargo.toml\nREADME.md".into()),
    ///     command_line_count: 2,
    ///     ..BlockSnapshot::default()
    /// });
    ///
    /// let output = snapshot.view().block_output_text(BlockKey::new(1));
    ///
    /// assert_eq!(output.as_deref(), Some("Cargo.toml\nREADME.md"));
    /// ```
    pub fn block_output_text(&self, key: BlockKey) -> Option<String> {
        let block = self.block(key)?;
        let text = collect_block_text(block, self.cells)?;

        let output = text
            .split('\n')
            .skip(command_lines_in_text(block))
            .collect::<Vec<_>>()
            .join("\n");

        (!output.is_empty()).then_some(output)
    }
}

/// Aggregate the textual content for the provided block from snapshot cells.
/// The status line of the block is not text and is left out.
///
/// Returns `None` for prompt blocks or entries with zero visible lines.
pub(crate) fn collect_block_text(
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

    let (start, line_count) = block.text_lines();
    if line_count == 0 || matches!(block.meta.kind, BlockKind::Prompt) {
        return None;
    }

    let end = start + line_count as i32;
    let mut lines = Vec::with_capacity(line_count);
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

/// How many of the first lines of the text of `block` are its command
/// lines. The cached text starts at the first command line. Text read from
/// the screen starts at the top edge of the pane, so the command lines
/// above that edge are not in it.
fn command_lines_in_text(block: &BlockSnapshot) -> usize {
    if block.cached_text.is_some() {
        return block.command_line_count;
    }

    block
        .command_line_count
        .saturating_sub(block.text_lines_above)
}

/// Trim trailing ASCII spaces from a buffer and return an owned string.
fn trim_trailing_spaces(buffer: &str) -> String {
    buffer.trim_end_matches(' ').to_string()
}

#[cfg(test)]
mod tests {
    use super::collect_block_text;
    use crate::block::testing::{
        TestDimensions, block_of_kind, copied_output, end_command,
        surface_running_typed_command, surface_with_wrapped_command,
    };
    use crate::block::{BlockKey, BlockKind, BlockSnapshot};
    use crate::cell::{Cell, Flags};
    use crate::hyperlink::HyperlinkMap;
    use crate::index::{Column, Line, Point};
    use crate::mode::SurfaceMode;
    use crate::snapshot::{
        CursorSnapshot, SnapshotCell, SnapshotDamage, SnapshotOwned,
        SnapshotSize,
    };
    use crate::{Colors, SelectionRange, SurfaceActor};

    /// A block of `kind` with the key `key`, `line_count` lines high from
    /// the line `start_line`.
    fn block_snapshot(
        key: u64,
        kind: BlockKind,
        start_line: i32,
        line_count: usize,
    ) -> BlockSnapshot {
        BlockSnapshot {
            start_line,
            line_count,
            ..block_of_kind(key, kind)
        }
    }

    /// A command block whose first line on screen is its status line.
    fn block_with_status_line(
        key: u64,
        start_line: i32,
        line_count: usize,
    ) -> BlockSnapshot {
        BlockSnapshot {
            has_status_line: true,
            is_status_line_visible: true,
            ..block_snapshot(key, BlockKind::Command, start_line, line_count)
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
            Vec::new(),
        )
    }

    #[test]
    fn collects_multiple_lines_for_command_block() {
        let block = block_snapshot(1, BlockKind::Command, 0, 2);
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
        let block = block_snapshot(3, BlockKind::Prompt, 0, 1);
        let cells = vec![cell(0, 0, '$')];

        assert!(collect_block_text(&block, &cells).is_none());
    }

    #[test]
    fn returns_none_for_empty_block() {
        let block = block_snapshot(4, BlockKind::Command, 0, 0);
        let cells = vec![cell(0, 0, 'a')];

        assert!(collect_block_text(&block, &cells).is_none());
    }

    #[test]
    fn keeps_fullscreen_blocks() {
        let block = block_snapshot(5, BlockKind::FullScreen, 0, 1);
        let cells = vec![cell(0, 0, 'Z')];

        assert_eq!(collect_block_text(&block, &cells), Some("Z".into()));
    }

    #[test]
    fn skips_wide_char_spacers() {
        let mut wide_cell = Cell {
            c: 'あ',
            ..Cell::default()
        };
        wide_cell.flags.insert(Flags::WIDE_CHAR);

        let mut spacer = Cell {
            c: ' ',
            ..Cell::default()
        };
        spacer.flags.insert(Flags::WIDE_CHAR_SPACER);

        let block = block_snapshot(6, BlockKind::Command, 0, 1);
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

        assert_eq!(collect_block_text(&block, &cells), Some("あ".to_string()));
    }

    #[test]
    fn given_status_line_on_screen_when_text_collected_then_it_is_left_out() {
        let block = BlockSnapshot {
            command_line_count: 1,
            ..block_with_status_line(1, 0, 3)
        };
        let cells = vec![
            cell(0, 0, ' '),
            cell(1, 0, '$'),
            cell(1, 1, 'l'),
            cell(2, 0, 'o'),
            cell(2, 1, 'k'),
        ];
        let snapshot = build_snapshot(cells.clone(), vec![block.clone()]);

        assert_eq!(
            collect_block_text(&block, &cells).as_deref(),
            Some("$l\nok")
        );
        assert_eq!(
            snapshot
                .view()
                .block_output_text(BlockKey::new(1))
                .as_deref(),
            Some("ok")
        );
    }

    #[test]
    fn given_status_line_on_screen_when_selection_copied_then_its_line_is_left_out()
     {
        let block = block_with_status_line(2, 1, 2);
        let cells = vec![
            cell(0, 0, 'a'),
            cell(0, 1, ' '),
            cell(1, 0, ' '),
            cell(1, 1, ' '),
            cell(2, 0, 'b'),
            cell(2, 1, ' '),
        ];
        let selection = SelectionRange::new(
            Point::new(Line(0), Column(0)),
            Point::new(Line(2), Column(0)),
            false,
        );
        let snapshot = SnapshotOwned::from_parts(
            cells,
            Some(selection),
            HyperlinkMap::default(),
            CursorSnapshot::default(),
            0,
            Colors::default(),
            SurfaceMode::default(),
            SnapshotSize {
                columns: 2,
                screen_lines: 3,
                total_lines: 3,
            },
            SnapshotDamage::Full,
            6,
            vec![block],
            // The block surface lists the status line among the lines it
            // adds.
            vec![Line(1)],
        );

        assert_eq!(snapshot.view().selectable_content(), "a b");
    }

    #[test]
    fn given_block_without_command_lines_when_its_output_is_copied_then_it_is_all_its_text()
     {
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks.push(BlockSnapshot {
            cached_text: Some("a\nb".into()),
            ..block_of_kind(1, BlockKind::FullScreen)
        });

        let output = snapshot.view().block_output_text(BlockKey::new(1));

        assert_eq!(output.as_deref(), Some("a\nb"));
    }

    #[test]
    fn given_one_line_command_when_it_ended_then_the_copies_split_its_text_after_the_first_line()
     {
        let mut surface =
            surface_running_typed_command(20, 12, &["$ ls"], "ls", &["a", "b"]);
        end_command(&mut surface, "cmd-1", "prompt-2");

        let copied = copied_output(&mut surface, "cmd-1");

        assert_eq!(copied, Some(String::from("a\nb\n")));
    }

    #[test]
    fn given_two_line_prompt_when_it_ended_then_no_prompt_line_is_in_the_output()
     {
        let mut surface = surface_running_typed_command(
            20,
            12,
            &["~/otty", "$ ls"],
            "ls",
            &["a", "b"],
        );
        end_command(&mut surface, "cmd-1", "prompt-2");

        let copied = copied_output(&mut surface, "cmd-1");

        assert_eq!(copied, Some(String::from("a\nb\n")));
    }

    #[test]
    fn given_two_line_prompt_while_the_command_runs_then_no_prompt_line_is_in_the_output()
     {
        let mut surface = surface_running_typed_command(
            20,
            12,
            &["~/otty", "$ ls"],
            "ls",
            &["a"],
        );

        let copied = copied_output(&mut surface, "cmd-1");

        assert_eq!(copied, Some(String::from("a\n")));
    }

    #[test]
    fn given_command_lines_above_the_pane_while_the_command_runs_then_the_output_is_its_text_on_screen()
     {
        let mut surface = surface_running_typed_command(
            12,
            4,
            &["~/otty", "$ seq 6"],
            "seq 6",
            &["1", "2", "3", "4", "5", "6"],
        );

        let copied = copied_output(&mut surface, "cmd-1");

        assert_eq!(copied, Some(String::from("4\n5\n6\n")));
    }

    #[test]
    fn given_command_wrapped_at_the_pane_edge_when_it_ended_then_no_part_of_it_is_in_the_output()
     {
        let mut surface = surface_with_wrapped_command();
        end_command(&mut surface, "cmd-1", "prompt-2");

        let copied = copied_output(&mut surface, "cmd-1");

        assert_eq!(copied, Some(String::from("aaaa bbbb\n")));
    }

    #[test]
    fn given_command_of_two_lines_when_it_ended_then_neither_line_is_in_the_output()
     {
        let mut surface = surface_running_typed_command(
            20,
            12,
            &["$ echo a \\", "> b"],
            "echo a b",
            &["a b"],
        );
        end_command(&mut surface, "cmd-1", "prompt-2");

        let copied = copied_output(&mut surface, "cmd-1");

        assert_eq!(copied, Some(String::from("a b\n")));
    }

    #[test]
    fn given_ended_wrapped_command_when_the_pane_gets_wider_or_narrower_then_the_copies_keep_the_rows_of_its_text()
     {
        let mut surface = surface_with_wrapped_command();
        end_command(&mut surface, "cmd-1", "prompt-2");
        let expected = Some(String::from("aaaa bbbb\n"));

        surface.resize(TestDimensions::new(20, 12));
        let wide = copied_output(&mut surface, "cmd-1");
        surface.resize(TestDimensions::new(6, 12));
        let narrow = copied_output(&mut surface, "cmd-1");

        // The text of an ended block was kept when it ended: a resize
        // wraps the grid again, but not that text.
        assert_eq!(wide, expected);
        assert_eq!(narrow, expected);
    }

    #[test]
    fn given_running_wrapped_command_when_the_pane_gets_wider_then_the_command_takes_one_line_before_and_after_it_ends()
     {
        let mut surface = surface_with_wrapped_command();
        let expected = Some(String::from("aaaa bbbb\n"));

        surface.resize(TestDimensions::new(20, 12));
        let running = copied_output(&mut surface, "cmd-1");
        end_command(&mut surface, "cmd-1", "prompt-2");
        let ended = copied_output(&mut surface, "cmd-1");

        assert_eq!(running, expected);
        assert_eq!(ended, expected);
    }
}
