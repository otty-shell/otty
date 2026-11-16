//! How far to scroll the history to bring a block into view, worked out
//! once from the lines every block has in the history and the gaps the row
//! layout puts around them.

use otty_libterm::surface::{BlockKey, BlockSnapshot, SnapshotView};

use crate::row_layout::{BLOCK_GAP, BLOCK_GAP_BELOW, RowLayout};

/// Where a jump to a block takes the view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScrollPlace {
    /// This many lines up from the bottom of the history, with the bottom
    /// row at the bottom edge.
    Offset(usize),
    /// The top of the history and one step past it, with the rows laid out
    /// from the top edge (see [`RowLayout`]).
    PastTheTop,
}

/// The place that brings the block `block_key` of `view` into view, for
/// the rows `rows` the pane shows now: the start of the block, the gap
/// above it and its status line, at the top edge of the pane.
///
/// It walks from the block down and adds what the pane shows under its top
/// edge: the gap above the block and its rows, then for every next block
/// the gap and the line under the one before, the gap above it and its
/// rows, until the pane is full. The bottom row of the pane stays at the
/// bottom edge, so the last of these rows becomes the bottom row. Nothing
/// of the block before is left in the pane: when a part of its last row
/// would show at the top edge, one more row comes at the bottom. When the
/// history below the block is too short to fill the pane, the block goes as
/// high as it can, and the view goes to the bottom of the history. A block
/// taller than the pane starts at the top row, where no gap is drawn. A
/// block whose start would need more history above it than there is, such
/// as the first block, or any block of a history that can not scroll, is
/// shown past the top of the history, where the rows are laid out from the
/// top edge and the first block starts there with its whole gap.
///
/// `None` when nothing moves: the pane shows all of the block already, the
/// view is where the block would bring it, the block is not in `view` or
/// has no lines, or a full-screen program fills the pane.
pub(crate) fn place_showing_block(
    view: &SnapshotView<'_>,
    rows: &RowLayout,
    block_key: BlockKey,
) -> Option<ScrollPlace> {
    let cell_height = rows.cell_height();
    if view.is_full_screen() || cell_height <= 0.0 {
        return None;
    }

    let blocks = view.blocks();
    let index = blocks.iter().position(|block| block.key == block_key)?;
    let block = &blocks[index];
    if block.history_line_count == 0 || rows.shows_whole_block(block) {
        return None;
    }

    let start: usize = blocks[..index]
        .iter()
        .map(|block| block.history_line_count)
        .sum();
    let bottom_row = bottom_row_under(
        &blocks[index..],
        start,
        view.size.screen_lines,
        cell_height,
    );
    let history_lines: usize =
        blocks.iter().map(|block| block.history_line_count).sum();
    let offset = history_lines.saturating_sub(bottom_row + 1);

    let place = if offset > view.size.max_display_offset() {
        ScrollPlace::PastTheTop
    } else {
        ScrollPlace::Offset(offset)
    };
    let current = if rows.is_past_the_top() {
        ScrollPlace::PastTheTop
    } else {
        ScrollPlace::Offset(view.display_offset)
    };

    (place != current).then_some(place)
}

/// The line of the history that is the bottom row of a pane of
/// `screen_lines` rows, `cell_height` high, whose top edge is at the start
/// of the first of `blocks`, the line `start` of the history. The blocks
/// follow each other from there.
///
/// Whole rows seldom fill the pane to the pixel: some space is left above
/// the first block. When it is more than the gap and the line under the
/// block before, a part of the last row of that block would show at the
/// top edge, and the sticky header of that block would lie over the status
/// line. Then one more row comes at the bottom, which pushes the block
/// before wholly over the top edge, even though the top edge then cuts a
/// part of the gap above the first block, or of its status row.
fn bottom_row_under(
    blocks: &[BlockSnapshot],
    start: usize,
    screen_lines: usize,
    cell_height: f32,
) -> usize {
    let pane_height = screen_lines as f32 * cell_height;
    let Some((first, below)) = blocks.split_first() else {
        return start;
    };

    let mut height = BLOCK_GAP + first.history_line_count as f32 * cell_height;
    if height > pane_height {
        return start + screen_lines.saturating_sub(1);
    }

    let mut bottom_row = (start + first.history_line_count).saturating_sub(1);
    let space_between = BLOCK_GAP_BELOW + BLOCK_GAP;
    for block in below.iter().filter(|block| block.history_line_count > 0) {
        let room = pane_height - height - space_between;
        let rows_that_fit = (room / cell_height).floor().max(0.0) as usize;
        let rows = block.history_line_count.min(rows_that_fit);
        if rows == 0 {
            break;
        }

        height += space_between + rows as f32 * cell_height;
        bottom_row += rows;
        if rows < block.history_line_count {
            break;
        }
    }

    let space_above = pane_height - height;
    let shows_block_before = start > 0 && space_above > BLOCK_GAP_BELOW;

    if shows_block_before {
        bottom_row + 1
    } else {
        bottom_row
    }
}

#[cfg(test)]
mod tests {
    use otty_libterm::surface::SurfaceMode;

    use super::*;
    use crate::testing::{HistoryBlock, history_snapshot};

    const CELL_HEIGHT: f32 = 16.0;
    /// The pane of the tests: 6 rows, 96 px.
    const SCREEN_LINES: usize = 6;

    const START: BlockKey = BlockKey::new(0);
    const LS: BlockKey = BlockKey::new(1);
    const PWD: BlockKey = BlockKey::new(2);
    const SEQ: BlockKey = BlockKey::new(3);
    const DATE: BlockKey = BlockKey::new(4);
    const FOLDED: BlockKey = BlockKey::new(5);
    const CAT: BlockKey = BlockKey::new(6);
    const TOP: BlockKey = BlockKey::new(7);
    const PROMPT: BlockKey = BlockKey::new(8);

    /// A history of 40 lines: the start block (a message of the day, no
    /// status line), `ls`, `pwd`, `seq` (taller than the pane), `date`, a
    /// folded block, `cat`, `top` and the prompt (no status line).
    fn history() -> Vec<HistoryBlock> {
        vec![
            HistoryBlock::plain(START, 2),
            HistoryBlock::command(LS, 3),
            HistoryBlock::command(PWD, 2),
            HistoryBlock::command(SEQ, 20),
            HistoryBlock::command(DATE, 2),
            HistoryBlock::folded(FOLDED, 3, 20),
            HistoryBlock::command(CAT, 4),
            HistoryBlock::command(TOP, 3),
            HistoryBlock::plain(PROMPT, 1),
        ]
    }

    /// Where the view of `history` scrolled up by `offset` lines goes to
    /// show `key`.
    fn offset_for(
        history: &[HistoryBlock],
        offset: usize,
        key: BlockKey,
    ) -> Option<ScrollPlace> {
        let snapshot = history_snapshot(SCREEN_LINES, history, offset);
        let view = snapshot.view();
        let rows = RowLayout::of_view(&view, CELL_HEIGHT, false);

        place_showing_block(&view, &rows, key)
    }

    /// Whether the pane of `history` at `place` starts with `key`: the
    /// first block in the pane is `key` and its first row, the status row
    /// of a command, starts inside the pane. Nothing of a block before it
    /// is in the pane, so no sticky header shows.
    fn starts_with(
        history: &[HistoryBlock],
        place: ScrollPlace,
        key: BlockKey,
    ) -> bool {
        let offset = match place {
            ScrollPlace::Offset(offset) => offset,
            ScrollPlace::PastTheTop => {
                history_snapshot(SCREEN_LINES, history, 0)
                    .view()
                    .size
                    .max_display_offset()
            },
        };
        let snapshot = history_snapshot(SCREEN_LINES, history, offset);
        let view = snapshot.view();
        let rows = RowLayout::of_view(
            &view,
            CELL_HEIGHT,
            place == ScrollPlace::PastTheTop,
        );

        let in_pane = rows.blocks_in_pane(view.blocks());

        in_pane.first().is_some_and(|first| {
            first.key == key && rows.line_top(first.start_line) >= 0.0
        })
    }

    /// `pwd` (2 lines from line 5) is above the pane at the bottom of the
    /// history. Its gap and its 2 rows take 38 px; the next block, `seq`,
    /// adds the gap, the line, the gap and 2 of its rows (45 px). That
    /// leaves 13 px above `pwd`, where 6 px of the last row of `ls` would
    /// show over the gap and the line under it (issue 17). So `seq` gives
    /// one more row: the view shows the lines 4 to 9 of the history, 30
    /// lines up from its bottom, and the top edge cuts 3 px of the gap
    /// above `pwd`.
    #[test]
    fn a_short_block_in_the_middle_comes_to_the_top_with_the_rows_that_fit_below_it()
     {
        let history = history();

        let offset = offset_for(&history, 0, PWD);

        assert_eq!(offset, Some(ScrollPlace::Offset(30)));
        assert!(starts_with(&history, ScrollPlace::Offset(30), PWD));
        assert!(!starts_with(&history, ScrollPlace::Offset(31), PWD));
    }

    /// Issue 17: after a jump nothing of the block before the target is in
    /// the pane, so the pane starts with the target and no sticky header
    /// shows over its status line.
    #[test]
    fn a_jump_leaves_nothing_of_the_block_before_in_the_pane() {
        let history = history();

        for (from, key) in [(0, PWD), (0, SEQ), (0, DATE), (34, CAT), (0, LS)] {
            let place = offset_for(&history, from, key).expect("a jump");

            assert!(starts_with(&history, place, key), "{key:?} at {place:?}");
        }
    }

    /// Issue 17: `pwd` and `date` of 2 lines, then `cat` of 3, in a pane of
    /// 6 rows. `pwd` and `date` fill 83 px; `cat` does not fit, and 13 px
    /// are left above `pwd`, 6 px of them on the last row of `ls`. The
    /// first row of `cat` comes with 29 px (the gap, the line, the gap and
    /// the row), more than the 13 px and the gap above `pwd`, so whole rows
    /// can not show the status line of `pwd` without a part of `ls`. Then
    /// `ls` goes wholly over the top edge and the status row of `pwd` is
    /// cut: the header that shows is the one of `pwd`, not of `ls`.
    #[test]
    fn when_whole_rows_can_not_keep_the_status_line_the_block_before_still_leaves_the_pane()
     {
        let history = vec![
            HistoryBlock::command(LS, 4),
            HistoryBlock::command(PWD, 2),
            HistoryBlock::command(DATE, 2),
            HistoryBlock::command(CAT, 3),
            HistoryBlock::command(TOP, 3),
            HistoryBlock::plain(PROMPT, 1),
        ];

        let place = offset_for(&history, 0, PWD);

        // `pwd` from line 4: the bottom row is the first line of `cat`, 8.
        assert_eq!(place, Some(ScrollPlace::Offset(15 - 9)));
        let snapshot = history_snapshot(SCREEN_LINES, &history, 6);
        let view = snapshot.view();
        let rows = RowLayout::of_view(&view, CELL_HEIGHT, false);
        let in_pane = rows.blocks_in_pane(view.blocks());
        let first = in_pane.first().expect("a block in the pane");
        assert_eq!(first.key, PWD);
        assert!(!first.is_status_line_visible);
    }

    /// Issue 18: `pwd`, `date` and `cat` of one line each, then `top`, in
    /// a pane of 6 rows. They fill 80 px; 16 px are left above `pwd`, 6 px
    /// of them on the last row of `ls`, so the first row of `top` comes
    /// too (issue 17). That pushes `ls` wholly over the top edge and 7 px
    /// of the 16 px status row of `pwd`: less than half, so the status line
    /// of `pwd` still shows, and no sticky header lies over it.
    #[test]
    fn when_the_jump_cuts_less_than_half_of_the_status_row_the_status_line_stays()
     {
        let history = vec![
            HistoryBlock::command(LS, 4),
            HistoryBlock::command(PWD, 1),
            HistoryBlock::command(DATE, 1),
            HistoryBlock::command(CAT, 1),
            HistoryBlock::command(TOP, 3),
            HistoryBlock::plain(PROMPT, 1),
        ];

        let place = offset_for(&history, 0, PWD);

        // `pwd` at line 4: the bottom row is the first line of `top`, 7.
        assert_eq!(place, Some(ScrollPlace::Offset(11 - 8)));
        let snapshot = history_snapshot(SCREEN_LINES, &history, 3);
        let view = snapshot.view();
        let rows = RowLayout::of_view(&view, CELL_HEIGHT, false);
        let in_pane = rows.blocks_in_pane(view.blocks());
        let first = in_pane.first().expect("a block in the pane");
        assert_eq!(first.key, PWD);
        assert_eq!(rows.line_top(first.start_line), -7.0);
        assert!(first.is_status_line_visible);
    }

    /// `seq` (20 lines from line 7) does not fit in the pane: its start is
    /// the top row, which has no gap, and its rows fill the pane.
    #[test]
    fn a_block_taller_than_the_pane_starts_at_the_top_row() {
        let history = history();

        let offset = offset_for(&history, 0, SEQ);

        assert_eq!(offset, Some(ScrollPlace::Offset(40 - 7 - SCREEN_LINES)));
        let snapshot = history_snapshot(SCREEN_LINES, &history, 27);
        let view = snapshot.view();
        let rows = RowLayout::of_view(&view, CELL_HEIGHT, false);
        let seq = view.block(SEQ).expect("seq is on screen");
        assert_eq!(rows.status_line_span(seq), Some(0.0..CELL_HEIGHT));
    }

    /// `top` is the last command: the prompt under it is all the history
    /// has, so the view goes to the bottom of the history. Its status line
    /// is whole there; nothing lower can push `cat` above it further up.
    #[test]
    fn the_last_block_brings_the_view_to_the_bottom_of_the_history() {
        let history = history();

        let to_top = offset_for(&history, 34, TOP);
        let to_prompt = offset_for(&history, 34, PROMPT);

        assert_eq!(to_top, Some(ScrollPlace::Offset(0)));
        assert_eq!(to_prompt, Some(ScrollPlace::Offset(0)));
        let snapshot = history_snapshot(SCREEN_LINES, &history, 0);
        let view = snapshot.view();
        let rows = RowLayout::of_view(&view, CELL_HEIGHT, false);
        let top = view.block(TOP).expect("top is on screen");
        assert!(rows.status_line_span(top).is_some());
    }

    /// The start block has no status line and gets the gap above it too.
    /// Under it nothing is left to push over the top edge, and at the top
    /// of the history the gaps still cut its top: the view goes one step
    /// past the top, where the rows are laid out from the top edge and the
    /// start block begins there with its whole gap.
    #[test]
    fn the_first_block_brings_the_view_past_the_top_of_the_history() {
        let history = history();

        let place = offset_for(&history, 0, START);

        assert_eq!(place, Some(ScrollPlace::PastTheTop));
        let snapshot = history_snapshot(SCREEN_LINES, &history, 34);
        let view = snapshot.view();
        let rows = RowLayout::of_view(&view, CELL_HEIGHT, true);
        let start = view.block(START).expect("the start block is on screen");
        assert_eq!(rows.block_span(start).map(|span| span.start), Some(0.0));
        assert_eq!(rows.line_top(start.start_line), BLOCK_GAP);
    }

    /// `ls`, the first command, is 2 lines below the top of the history.
    /// At the top of the history 6 px of the last row of the start block
    /// would show over the gap above `ls`, so the view goes one line lower
    /// (issue 17): the start block goes wholly over the top edge, the edge
    /// cuts 3 px of the gap above `ls`, and its status row is whole.
    #[test]
    fn the_first_command_comes_to_the_top_with_the_start_block_over_the_top_edge()
     {
        let history = history();

        let place = offset_for(&history, 0, LS);

        assert_eq!(place, Some(ScrollPlace::Offset(33)));
        let snapshot = history_snapshot(SCREEN_LINES, &history, 33);
        let view = snapshot.view();
        let rows = RowLayout::of_view(&view, CELL_HEIGHT, false);
        let ls = view.block(LS).expect("ls is on screen");
        let span = rows.status_line_span(ls).expect("its status line shows");
        assert_eq!(span, 0.0..BLOCK_GAP - 3.0 + CELL_HEIGHT);
        assert!(starts_with(&history, ScrollPlace::Offset(33), LS));
    }

    /// Past the top the pane shows the first block whole: nothing moves.
    #[test]
    fn past_the_top_the_first_block_stays_where_it_is() {
        let history = history();
        let snapshot = history_snapshot(SCREEN_LINES, &history, 34);
        let view = snapshot.view();
        let rows = RowLayout::of_view(&view, CELL_HEIGHT, true);

        let place = place_showing_block(&view, &rows, START);

        assert_eq!(place, None);
    }

    /// A new terminal after `ls`, `pwd` and `date`: 6 lines, as many as
    /// the pane has rows, so the history can not scroll, and the gaps push
    /// the start of `ls` over the top edge. Only the step past the top
    /// shows it.
    #[test]
    fn in_a_short_history_cut_at_the_top_the_first_block_is_shown_past_the_top()
    {
        let history = vec![
            HistoryBlock::command(LS, 2),
            HistoryBlock::command(PWD, 2),
            HistoryBlock::command(DATE, 1),
            HistoryBlock::plain(PROMPT, 1),
        ];
        assert!(!starts_with(&history, ScrollPlace::Offset(0), LS));

        let to_ls = offset_for(&history, 0, LS);
        let to_pwd = offset_for(&history, 0, PWD);

        assert_eq!(to_ls, Some(ScrollPlace::PastTheTop));
        assert_eq!(to_pwd, None);
    }

    /// The folded block shows 3 lines (its status line and the two command
    /// lines of a prompt of two lines), and only those take room under
    /// `date`.
    #[test]
    fn a_folded_block_below_takes_only_the_lines_it_shows() {
        let history = history();

        let offset = offset_for(&history, 0, DATE);

        // `date` from line 27, 2 lines; 2 of the 3 folded lines fit, and
        // the third one comes too, so that nothing of `seq` shows.
        assert_eq!(offset, Some(ScrollPlace::Offset(40 - 32)));
        assert!(starts_with(&history, ScrollPlace::Offset(8), DATE));
        assert!(!starts_with(&history, ScrollPlace::Offset(9), DATE));
    }

    #[test]
    fn a_folded_block_above_leaves_the_start_of_the_next_one_where_it_is() {
        let history = history();

        let offset = offset_for(&history, 34, CAT);

        // `cat` from line 32, 4 lines: `top` under it does not fit, but its
        // first row comes, so that nothing of the folded block shows.
        assert_eq!(offset, Some(ScrollPlace::Offset(40 - 37)));
        assert!(starts_with(&history, ScrollPlace::Offset(3), CAT));
    }

    #[test]
    fn a_block_the_pane_shows_whole_stays_where_it_is() {
        let history = history();

        let top = offset_for(&history, 0, TOP);
        let prompt = offset_for(&history, 0, PROMPT);

        assert_eq!(top, None);
        assert_eq!(prompt, None);
    }

    /// `cat` holds the top rows at the bottom of the history, but the gaps
    /// push its start over the top edge.
    #[test]
    fn a_block_on_screen_with_its_start_cut_comes_down_to_show_it() {
        let history = history();
        assert!(!starts_with(&history, ScrollPlace::Offset(0), CAT));

        let offset = offset_for(&history, 0, CAT);

        assert_eq!(offset, Some(ScrollPlace::Offset(3)));
    }

    #[test]
    fn a_block_that_is_not_in_the_history_moves_nothing() {
        let history = history();

        let offset = offset_for(&history, 0, BlockKey::new(99));

        assert_eq!(offset, None);
    }

    /// A short history fits in the pane: it can not scroll.
    #[test]
    fn in_a_history_shorter_than_the_pane_nothing_moves() {
        let history =
            vec![HistoryBlock::command(LS, 3), HistoryBlock::plain(PROMPT, 1)];

        let offset = offset_for(&history, 0, LS);

        assert_eq!(offset, None);
    }

    #[test]
    fn a_full_screen_program_moves_nothing() {
        let history = history();
        let snapshot = history_snapshot(SCREEN_LINES, &history, 0);
        let mut view = snapshot.view();
        view.mode = SurfaceMode::ALT_SCREEN;
        let rows = RowLayout::of_view(&view, CELL_HEIGHT, false);

        let place = place_showing_block(&view, &rows, LS);

        assert_eq!(place, None);
    }
}
