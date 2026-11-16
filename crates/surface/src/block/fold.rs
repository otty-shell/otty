//! Folding a block, and the map of its lines that folding needs: which
//! rows of the block hold its command lines, which its output, and what of
//! it the stitched history shows.

use std::cmp::min;

use super::surface::Block;
use super::{BlockKey, BlockKind, BlockSurface};
use crate::cell::Cell;
use crate::grid::Grid;
use crate::index::{Column, Line};
use crate::{Dimensions, Flags};

/// Where the text of a block lies in its own grid.
pub(super) struct BlockLines {
    /// First line of the text.
    top_line: Line,
    /// How many lines the text has: the trimmed screen and the scrollback.
    text_lines: usize,
    /// How many of them, from the first one, hold the command lines.
    command_rows: usize,
}

impl BlockLines {
    /// First line of the text.
    pub(super) fn top_line(&self) -> Line {
        self.top_line
    }

    /// How many rows of the text, from the first one, hold the command
    /// lines.
    pub(super) fn command_rows(&self) -> usize {
        self.command_rows
    }
}

impl Block {
    /// Where the text of the block lies in its own grid now, and which of
    /// its rows hold the command lines. A closed start block with no text
    /// has none.
    pub(super) fn lines(&self) -> BlockLines {
        let (top_line, text_lines) = BlockSurface::block_visible_extent(self);
        let grid = self.surface.grid();

        if self.is_closed_start() && is_blank(grid, top_line, text_lines) {
            return BlockLines {
                top_line,
                text_lines: 0,
                command_rows: 0,
            };
        }

        let command_lines = self.command_lines.unwrap_or(0);

        // A command line takes its own row and every row it wraps over.
        let mut command_rows = 0;
        let mut ended_lines = 0;
        while ended_lines < command_lines && command_rows < text_lines {
            if !is_wrapped(grid, top_line + command_rows) {
                ended_lines += 1;
            }
            command_rows += 1;
        }

        BlockLines {
            top_line,
            text_lines,
            command_rows,
        }
    }

    /// How many lines of text the stitched history shows. A folded block
    /// shows only its command lines. A command block that the next prompt
    /// followed (`has_next_block`) shows its command lines and its output,
    /// without the empty line the cursor waited on: the gap between the
    /// blocks gives the space under it. Any other block shows all its
    /// lines; while a command runs, or until its next prompt comes, the
    /// cursor is on that line.
    pub(super) fn shown_text_line_count(
        &self,
        lines: &BlockLines,
        has_next_block: bool,
    ) -> usize {
        if self.is_folded {
            return lines.command_rows;
        }

        if has_next_block && self.meta.kind == BlockKind::Command {
            return lines.command_rows + self.output_line_count(lines);
        }

        lines.text_lines
    }

    /// How many lines of output the block has under its command lines. The
    /// empty line the cursor waited on when the next prompt came is not
    /// output.
    pub(super) fn output_line_count(&self, lines: &BlockLines) -> usize {
        let output = lines.text_lines.saturating_sub(lines.command_rows);

        output - usize::from(self.has_waiting_line(lines))
    }

    /// Whether the block can be folded: a finished command block with
    /// command lines and output under them.
    pub(super) fn is_foldable(&self, lines: &BlockLines) -> bool {
        self.meta.kind == BlockKind::Command
            && self.meta.is_finished
            && lines.command_rows > 0
            && self.output_line_count(lines) > 0
    }

    /// Remember how many lines the block holds now that its command starts:
    /// from now on the block has a status line.
    /// The shells move the cursor to the line under the command before they
    /// send `preexec`, so the line of the cursor counts only when there is
    /// text before the cursor.
    pub(super) fn remember_command_lines(&mut self) {
        let (top_line, text_lines) = BlockSurface::block_visible_extent(self);
        let grid = self.surface.grid();
        let cursor = grid.cursor.point;

        let rows_above_cursor =
            usize::try_from(cursor.line.0 - top_line.0).unwrap_or(0);
        let cursor_row = usize::from(cursor.column.0 > 0);
        let rows = min(rows_above_cursor + cursor_row, text_lines);

        // Rows that go on in the next row are parts of one line.
        let wrapped_rows = (0..rows)
            .filter(|&row| is_wrapped(grid, top_line + row))
            .count();
        let ends_wrapped = rows > 0 && is_wrapped(grid, top_line + (rows - 1));

        self.command_lines =
            Some(rows - wrapped_rows + usize::from(ends_wrapped));
    }

    /// Whether the block is the start block of the terminal and the first
    /// prompt or command has closed it. While it is open, it holds the
    /// cursor, so its cursor line stays on screen.
    fn is_closed_start(&self) -> bool {
        self.meta.kind == BlockKind::Start && self.meta.is_finished
    }

    /// Whether the block ends, under its command lines, with the empty line
    /// the cursor waited on when the next prompt came: its last line holds
    /// the cursor and no text. A block whose output ended with a new line
    /// has one; a block whose output ended without one has none, and blank
    /// lines the output printed above that line are output.
    fn has_waiting_line(&self, lines: &BlockLines) -> bool {
        if lines.text_lines <= lines.command_rows {
            return false;
        }

        let grid = self.surface.grid();
        let last_line = lines.top_line + (lines.text_lines - 1);

        grid.cursor.point.line == last_line && grid[last_line].is_clear()
    }
}

impl BlockSurface {
    /// Fold the block with `key`, or unfold it when it is folded. Only a
    /// finished command block with output can be folded; for any other
    /// block nothing changes. The selection is cleared, since the lines
    /// under it move. While the history is scrolled up, the first line of
    /// the view stays in place.
    pub(super) fn toggle_fold(&mut self, key: BlockKey) {
        let Some(index) = self.block_index(key) else {
            return;
        };
        let block = &self.blocks[index];
        if !block.is_foldable(&block.lines()) {
            return;
        }

        let slices = self.block_slices();
        let context = self.viewport_context(&slices);
        let Some(slice) = slices.iter().find(|slice| slice.index == index)
        else {
            return;
        };
        let block = &self.blocks[index];
        let lines = block.lines();
        // The lines under the command lines leave the history or come back.
        let hidden_start = slice.text_start + lines.command_rows;
        let lines_before = slice.end - slice.start;
        let has_next_block = index + 1 < self.blocks.len();
        let is_folding = !block.is_folded;

        self.blocks[index].is_folded = is_folding;
        let lines_after =
            Self::block_history_line_count(&self.blocks[index], has_next_block);
        self.clear_selection();
        self.keep_view_start(
            context.start,
            hidden_start,
            lines_before.abs_diff(lines_after),
            is_folding,
        );
    }

    /// Scroll so that the first line of the view stays where it is after
    /// `hidden_lines` lines from the global line `hidden_start` left the
    /// history (`is_folding`) or came back. `view_start` is the global line
    /// the view began at before. A view at the bottom stays at the bottom,
    /// with the prompt.
    fn keep_view_start(
        &mut self,
        view_start: usize,
        hidden_start: usize,
        hidden_lines: usize,
        is_folding: bool,
    ) {
        if self.display_offset == 0 {
            return;
        }

        // Lines that change below the start of the view do not move it;
        // lines that change above it move it by as many.
        let new_start = if hidden_start >= view_start {
            view_start
        } else if is_folding {
            view_start - min(hidden_lines, view_start - hidden_start)
        } else {
            view_start + hidden_lines
        };

        let max_offset = self.total_lines().saturating_sub(self.screen_lines());
        self.display_offset = max_offset.saturating_sub(new_start);
    }
}

/// Whether the `count` lines of `grid` from `top_line` hold no text.
fn is_blank(grid: &Grid<Cell>, top_line: Line, count: usize) -> bool {
    (0..count).all(|row| grid[top_line + row].is_clear())
}

/// Whether the text of `line` goes on in the next line: the terminal
/// wrapped it at the right edge.
fn is_wrapped(grid: &Grid<Cell>, line: Line) -> bool {
    let columns = grid.columns();
    if columns == 0 {
        return false;
    }

    grid[line][Column(columns - 1)]
        .flags
        .contains(Flags::WRAPLINE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::testing::*;
    use crate::grid::Scroll;
    use crate::index::Point;
    use crate::snapshot::SurfaceModel;
    use crate::{SurfaceActor, SurfaceConfig};

    #[test]
    fn given_block_with_2_command_lines_and_20_output_lines_when_folded_then_its_status_line_and_command_lines_stay_and_20_are_hidden()
     {
        let mut surface = surface_with_seq_20(30);
        let before = history_line_count(&surface);

        surface.toggle_block_fold(key_of(&surface, "cmd-1"));
        let snapshot = surface.snapshot_owned();

        let block = block_snapshot(&snapshot, "cmd-1");
        assert_eq!(block.hidden_line_count, Some(20));
        assert_eq!(block.line_count, 1 + 2);
        assert_eq!(block_line_text(&snapshot, "cmd-1", 0), "");
        assert_eq!(block_line_text(&snapshot, "cmd-1", 1), "~/otty");
        assert_eq!(block_line_text(&snapshot, "cmd-1", 2), "$ seq 20");
        // The block ends with its command lines: the prompt follows.
        assert_eq!(block_line_text(&snapshot, "cmd-1", 3), "$ ls");
        assert_eq!(history_line_count(&surface), before - 20);
    }

    #[test]
    fn given_folded_block_when_unfolded_then_its_status_line_and_all_its_lines_come_back()
     {
        let mut surface = surface_with_seq_20(30);
        let before = history_line_count(&surface);
        surface.toggle_block_fold(key_of(&surface, "cmd-1"));

        surface.toggle_block_fold(key_of(&surface, "cmd-1"));
        let snapshot = surface.snapshot_owned();

        let block = block_snapshot(&snapshot, "cmd-1");
        assert_eq!(block.hidden_line_count, None);
        assert_eq!(block.line_count, 1 + 2 + 20);
        assert_eq!(block_line_text(&snapshot, "cmd-1", 0), "");
        assert_eq!(block_line_text(&snapshot, "cmd-1", 2), "$ seq 20");
        assert_eq!(block_line_text(&snapshot, "cmd-1", 3), "1");
        assert_eq!(block_line_text(&snapshot, "cmd-1", 22), "20");
        assert_eq!(history_line_count(&surface), before);
    }

    #[test]
    fn given_finished_command_block_when_shown_then_it_can_be_folded() {
        let mut surface = surface_with_seq_20(30);

        let snapshot = surface.snapshot_owned();

        let block = block_snapshot(&snapshot, "cmd-1");
        assert!(block.is_foldable);
        assert!(!block.is_folded());
    }

    #[test]
    fn given_running_block_and_prompt_block_when_folded_then_nothing_changes() {
        let mut running = surface_running_ls();
        next_line(&mut running);
        print_text(&mut running, "a b");
        let running_before = history_line_count(&running);
        let mut prompt = surface_with_seq_20(30);
        let prompt_before = history_line_count(&prompt);

        running.toggle_block_fold(key_of(&running, "cmd-1"));
        prompt.toggle_block_fold(key_of(&prompt, "prompt-2"));

        let running_snapshot = running.snapshot_owned();
        let command = block_snapshot(&running_snapshot, "cmd-1");
        assert!(!command.is_foldable);
        assert!(!command.is_folded());
        assert_eq!(history_line_count(&running), running_before);
        let prompt_snapshot = prompt.snapshot_owned();
        let prompt_block = block_snapshot(&prompt_snapshot, "prompt-2");
        assert!(!prompt_block.is_foldable);
        assert!(!prompt_block.is_folded());
        assert_eq!(history_line_count(&prompt), prompt_before);
    }

    #[test]
    fn given_command_without_output_when_shown_then_it_cannot_be_folded() {
        let dims = TestDimensions::new(12, 10);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ cd /tmp");
        start_command(&mut surface, "cmd-1", "cd /tmp");
        end_command(&mut surface, "cmd-1", "prompt-2");

        surface.toggle_block_fold(key_of(&surface, "cmd-1"));
        let snapshot = surface.snapshot_owned();

        let block = block_snapshot(&snapshot, "cmd-1");
        assert!(!block.is_foldable);
        assert!(!block.is_folded());
    }

    #[test]
    fn given_folded_block_in_the_middle_when_shown_then_the_next_block_follows_its_command_lines()
     {
        let mut surface = surface_with_seq_20(30);
        surface.toggle_block_fold(key_of(&surface, "cmd-1"));

        let snapshot = surface.snapshot_owned();

        let folded = block_snapshot(&snapshot, "cmd-1");
        let prompt = block_snapshot(&snapshot, "prompt-2");
        // The status line and 2 command lines.
        assert_eq!(prompt.start_line, folded.start_line + 3);
        assert_eq!(line_text(&snapshot, prompt.start_line), "$ ls");
        let point = Point::new(Line(prompt.start_line), Column(2));
        assert_eq!(
            snapshot
                .view()
                .block_at_point(point)
                .map(|block| block.meta.id.as_str()),
            Some("prompt-2")
        );
        let slices = surface.block_slices();
        let context = surface.viewport_context(&slices);
        let (index, local_point, _) = surface
            .resolve_block_point_with(&slices, &context, point)
            .expect("the point is in the history");
        assert_eq!(surface.blocks[index].meta.id, "prompt-2");
        assert_eq!(local_point.column, Column(2));
    }

    #[test]
    fn given_text_selected_below_a_folded_block_when_copied_then_it_is_the_text_on_screen()
     {
        let mut surface = surface_with_seq_20(30);
        surface.toggle_block_fold(key_of(&surface, "cmd-1"));
        let before = surface.snapshot_owned();
        let command_line = block_snapshot(&before, "cmd-1").start_line + 2;
        let prompt_line = block_snapshot(&before, "prompt-2").start_line;

        surface.start_selection(
            crate::SelectionType::Simple,
            Point::new(Line(command_line), Column(2)),
            crate::Side::Left,
        );
        surface.update_selection(
            Point::new(Line(prompt_line), Column(3)),
            crate::Side::Right,
        );
        let snapshot = surface.snapshot_owned();

        // The selection holds the cells on screen, without line breaks: the
        // rest of the command line and the prompt.
        assert_eq!(snapshot.view().selectable_content(), "seq 20    $ ls");
    }

    #[test]
    fn given_folded_block_when_its_text_is_copied_then_the_whole_output_is_in_it()
     {
        let mut surface = surface_with_seq_20(30);
        let unfolded =
            block_text(&surface.snapshot_owned(), key_of(&surface, "cmd-1"));

        surface.toggle_block_fold(key_of(&surface, "cmd-1"));
        let folded = surface.snapshot_owned();

        let text = block_text(&folded, block_snapshot(&folded, "cmd-1").key)
            .expect("the block has text");
        assert_eq!(Some(text.clone()), unfolded);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "~/otty");
        assert_eq!(lines[1], "$ seq 20");
        assert_eq!(lines[2], "1");
        assert_eq!(lines[21], "20");
        let output = lines[2..].join("\n");
        assert_eq!(
            folded
                .view()
                .block_output_text(block_snapshot(&folded, "cmd-1").key),
            Some(format!("{output}\n"))
        );
    }

    #[test]
    fn given_history_scrolled_to_the_top_when_a_block_is_folded_then_the_scroll_offset_stays_inside_the_shorter_history()
     {
        let mut surface = surface_with_seq_20(6);
        surface.scroll_display(Scroll::Top);
        assert!(surface.display_offset > 0);

        surface.toggle_block_fold(key_of(&surface, "cmd-1"));
        let snapshot = surface.snapshot_owned();

        let max_offset =
            surface.total_lines().saturating_sub(surface.screen_lines());
        assert!(
            surface.display_offset <= max_offset,
            "offset {} is beyond {max_offset}",
            surface.display_offset
        );
        assert_eq!(snapshot.view().display_offset, surface.display_offset);
        let block = block_snapshot(&snapshot, "cmd-1");
        assert_eq!(line_text(&snapshot, block.start_line + 2), "$ seq 20");
    }

    #[test]
    fn given_view_scrolled_to_a_block_when_it_is_folded_and_unfolded_then_the_block_stays_at_the_top_of_the_view()
     {
        let mut surface = surface_with_seq_20(6);
        start_command(&mut surface, "cmd-2", "ls");
        print_lines(&mut surface, &["1", "2", "3", "4", "5", "6", "7", "8"]);
        end_command(&mut surface, "cmd-2", "prompt-3");
        let start_of_cmd_1 = surface
            .block_slices()
            .iter()
            .find(|slice| surface.blocks[slice.index].meta.id == "cmd-1")
            .map(|slice| slice.start)
            .expect("cmd-1 is in the history");
        let max_offset =
            surface.total_lines().saturating_sub(surface.screen_lines());
        surface.display_offset = max_offset - start_of_cmd_1;

        surface.toggle_block_fold(key_of(&surface, "cmd-1"));
        let folded = surface.snapshot_owned();
        surface.toggle_block_fold(key_of(&surface, "cmd-1"));
        let unfolded = surface.snapshot_owned();

        // Lines on screen are counted from the bottom of the history, so the
        // top line of the view is minus the scroll offset.
        let top = -(folded.view().display_offset as i32);
        assert_eq!(block_snapshot(&folded, "cmd-1").start_line, top);
        assert_eq!(line_text(&folded, top + 2), "$ seq 20");
        // The status line of `ls` follows the command lines.
        assert_eq!(block_snapshot(&folded, "cmd-2").start_line, top + 3);
        let top = -(unfolded.view().display_offset as i32);
        assert_eq!(block_snapshot(&unfolded, "cmd-1").start_line, top);
        assert_eq!(line_text(&unfolded, top + 3), "1");
    }

    #[test]
    fn given_command_typed_on_the_line_of_the_cursor_when_folded_then_that_line_stays()
     {
        let mut surface = surface_running_ls();
        next_line(&mut surface);
        print_text(&mut surface, "a b");
        surface.handle_block_event(exit_event("cmd-1", 0), 5_100);
        surface.handle_block_event(precmd_event("prompt-2"), 5_100);

        surface.toggle_block_fold(key_of(&surface, "cmd-1"));
        let snapshot = surface.snapshot_owned();

        let block = block_snapshot(&snapshot, "cmd-1");
        assert_eq!(block.hidden_line_count, Some(1));
        assert_eq!(block_line_text(&snapshot, "cmd-1", 1), "$ ls");
        // The status line and the command line.
        assert_eq!(block.line_count, 2);
    }

    /// `$ echo aaaa bbbb`, 16 characters, typed in a pane 10 columns wide,
    /// so the command wraps over two lines, then folded.
    fn surface_with_folded_wrapped_command() -> BlockSurface {
        let dims = TestDimensions::new(10, 12);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ echo aaaa bbbb");
        start_command(&mut surface, "cmd-1", "echo aaaa bbbb");
        print_lines(&mut surface, &["aaaa bbbb"]);
        end_command(&mut surface, "cmd-1", "prompt-2");
        print_text(&mut surface, "$");
        surface.toggle_block_fold(key_of(&surface, "cmd-1"));

        surface
    }

    #[test]
    fn given_folded_block_whose_command_wrapped_when_the_pane_gets_wider_then_the_whole_command_stays_on_one_line()
     {
        let mut surface = surface_with_folded_wrapped_command();
        let narrow = surface.snapshot_owned();
        assert_eq!(block_snapshot(&narrow, "cmd-1").line_count, 1 + 2);

        surface.resize(TestDimensions::new(20, 12));
        let wide = surface.snapshot_owned();

        let block = block_snapshot(&wide, "cmd-1");
        assert_eq!(block.line_count, 1 + 1);
        assert_eq!(block.hidden_line_count, Some(1));
        assert_eq!(block_line_text(&wide, "cmd-1", 1), "$ echo aaaa bbbb");
        assert_eq!(line_text(&wide, block.start_line + 2), "$");
    }

    #[test]
    fn given_folded_block_when_the_pane_gets_narrower_then_its_command_wraps_and_stays_whole()
     {
        let mut surface = surface_with_folded_wrapped_command();

        surface.resize(TestDimensions::new(6, 12));
        let narrow = surface.snapshot_owned();

        let block = block_snapshot(&narrow, "cmd-1");
        assert_eq!(block.line_count, 1 + 3);
        assert_eq!(block.hidden_line_count, Some(2));
        assert_eq!(block_line_text(&narrow, "cmd-1", 1), "$ echo");
        assert_eq!(block_line_text(&narrow, "cmd-1", 3), "bbbb");
        assert_eq!(line_text(&narrow, block.start_line + 4), "$");
    }

    #[test]
    fn given_selection_in_a_block_when_it_is_folded_then_the_selection_is_cleared()
     {
        let mut surface = surface_with_seq_20(30);
        let before = surface.snapshot_owned();
        let output_line = block_snapshot(&before, "cmd-1").start_line + 3;
        surface.start_selection(
            crate::SelectionType::Simple,
            Point::new(Line(output_line), Column(0)),
            crate::Side::Left,
        );
        surface.update_selection(
            Point::new(Line(output_line + 2), Column(1)),
            crate::Side::Right,
        );

        surface.toggle_block_fold(key_of(&surface, "cmd-1"));
        let snapshot = surface.snapshot_owned();

        assert_eq!(snapshot.view().selection, None);
        assert_eq!(snapshot.view().selectable_content(), "");
    }
}
