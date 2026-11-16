use iced::Rectangle;
use otty_libterm::surface::{BlockKey, BlockKind};

use crate::block_layout::BlockRect;
use crate::row_layout::BLOCK_LINE_HEIGHT;

/// Width of the stripe at the left edge of a failed block, in pixels.
const FAILED_STRIPE_WIDTH: f32 = 2.0;

/// The background of a block, named after the state it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockBackground {
    /// The block is selected.
    Selected,
    /// The pointer is over the block.
    Hovered,
    /// The command of the block failed.
    Failed,
}

/// What the terminal view paints for one visible block, under its text.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BlockPaint {
    rect: Rectangle,
    background: Option<BlockBackground>,
    has_failed_stripe: bool,
    has_line: bool,
}

impl BlockPaint {
    /// The place of the block, from the left to the right edge of the pane.
    pub(crate) fn rect(&self) -> Rectangle {
        self.rect
    }

    /// The background of the block, if it has one.
    pub(crate) fn background(&self) -> Option<BlockBackground> {
        self.background
    }

    /// The place of the stripe at the left edge of a failed block.
    pub(crate) fn failed_stripe(&self) -> Option<Rectangle> {
        if !self.has_failed_stripe {
            return None;
        }

        Some(Rectangle {
            width: FAILED_STRIPE_WIDTH.min(self.rect.width),
            ..self.rect
        })
    }

    /// The place of the line under the block: its lowest row of pixels.
    pub(crate) fn line(&self) -> Option<Rectangle> {
        if !self.has_line {
            return None;
        }

        let height = BLOCK_LINE_HEIGHT.min(self.rect.height);

        Some(Rectangle {
            y: self.rect.y + self.rect.height - height,
            height,
            ..self.rect
        })
    }
}

/// Decide what to paint for each visible block.
///
/// The background shows one state: selected, else hovered, else failed. A
/// failed block keeps its stripe under any background. A block has a line
/// under it, in the gap under its last row. A block that ends on the bottom
/// row has no gap there, so it gets no line, which would lie over its text;
/// the last block of the history always ends there. Only command blocks are
/// painted: the prompt block has no background and no stripe, and the
/// block a terminal starts with (output before the first prompt) has no
/// paint at all.
pub(crate) fn block_paints(
    rects: &[BlockRect],
    selected_block: Option<BlockKey>,
    hovered_block: Option<BlockKey>,
) -> Vec<BlockPaint> {
    rects
        .iter()
        .filter(|rect| rect.rect.height > 0.0)
        .map(|rect| {
            let block = Some(rect.block_key);
            let is_command = rect.kind == BlockKind::Command;

            let background = if !is_command {
                None
            } else if block == selected_block {
                Some(BlockBackground::Selected)
            } else if block == hovered_block {
                Some(BlockBackground::Hovered)
            } else if rect.is_failed {
                Some(BlockBackground::Failed)
            } else {
                None
            };

            BlockPaint {
                rect: rect.rect,
                background,
                has_failed_stripe: rect.is_failed && is_command,
                has_line: rect.has_gap_below && rect.kind != BlockKind::Start,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A block 100 px wide and 20 px high at the top left corner.
    fn block_rect(key: u64, kind: BlockKind) -> BlockRect {
        BlockRect {
            block_key: BlockKey::new(key),
            kind,
            rect: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            is_failed: false,
            has_gap_below: true,
            status_line: None,
        }
    }

    fn failed_block_rect(key: u64) -> BlockRect {
        BlockRect {
            is_failed: true,
            ..block_rect(key, BlockKind::Command)
        }
    }

    #[test]
    fn a_plain_block_gets_no_background_and_no_stripe() {
        let rects = [block_rect(1, BlockKind::Command)];

        let paints = block_paints(&rects, None, None);

        assert_eq!(paints.len(), 1);
        assert_eq!(paints[0].background(), None);
        assert_eq!(paints[0].failed_stripe(), None);
    }

    #[test]
    fn a_hovered_block_gets_the_hovered_background() {
        let rects = [
            block_rect(1, BlockKind::Command),
            block_rect(2, BlockKind::Command),
        ];

        let paints = block_paints(&rects, None, Some(BlockKey::new(2)));

        assert_eq!(paints[0].background(), None);
        assert_eq!(paints[1].background(), Some(BlockBackground::Hovered));
    }

    /// Owner, issue 13: a shell started inside another one counts its
    /// blocks from 1 again, so two blocks can have one shell id. Only the
    /// block under the pointer gets the hovered background, and only the
    /// selected one the selected background.
    #[test]
    fn of_two_blocks_with_one_shell_id_only_the_one_acted_on_is_painted() {
        use iced::{Point, Size};
        use otty_libterm::surface::{BlockSnapshot, SnapshotOwned};

        use crate::testing::block_rects_of;

        let mut snapshot = SnapshotOwned::default();
        for (key, start_line) in [(1, 0), (2, 2)] {
            let mut block = BlockSnapshot {
                key: BlockKey::new(key),
                start_line,
                line_count: 2,
                ..BlockSnapshot::default()
            };
            block.meta.id = String::from("cmd-1");
            block.meta.kind = BlockKind::Command;
            snapshot.blocks.push(block);
        }
        let rects = block_rects_of(
            &snapshot.view(),
            Point::ORIGIN,
            Size::new(100.0, 100.0),
            10.0,
        );

        let hovered = block_paints(&rects, None, Some(BlockKey::new(2)));
        let selected = block_paints(&rects, Some(BlockKey::new(1)), None);

        assert_eq!(hovered[0].background(), None);
        assert_eq!(hovered[1].background(), Some(BlockBackground::Hovered));
        assert_eq!(selected[0].background(), Some(BlockBackground::Selected));
        assert_eq!(selected[1].background(), None);
    }

    #[test]
    fn a_selected_and_hovered_block_gets_the_selected_background() {
        let rects = [block_rect(1, BlockKind::Command)];

        let paints = block_paints(
            &rects,
            Some(BlockKey::new(1)),
            Some(BlockKey::new(1)),
        );

        assert_eq!(paints[0].background(), Some(BlockBackground::Selected));
    }

    #[test]
    fn a_failed_block_gets_the_failed_background_and_the_stripe() {
        let rects = [failed_block_rect(3)];

        let paints = block_paints(&rects, None, None);

        assert_eq!(paints[0].background(), Some(BlockBackground::Failed));
        assert_eq!(
            paints[0].failed_stripe(),
            Some(Rectangle {
                x: 0.0,
                y: 0.0,
                width: 2.0,
                height: 20.0,
            })
        );
    }

    #[test]
    fn a_failed_and_hovered_block_gets_the_hovered_background_and_keeps_the_stripe()
     {
        let rects = [failed_block_rect(3)];

        let paints = block_paints(&rects, None, Some(BlockKey::new(3)));

        assert_eq!(paints[0].background(), Some(BlockBackground::Hovered));
        assert!(paints[0].failed_stripe().is_some());
    }

    #[test]
    fn a_failed_and_selected_block_gets_the_selected_background_and_keeps_the_stripe()
     {
        let rects = [failed_block_rect(3)];

        let paints = block_paints(
            &rects,
            Some(BlockKey::new(3)),
            Some(BlockKey::new(3)),
        );

        assert_eq!(paints[0].background(), Some(BlockBackground::Selected));
        assert!(paints[0].failed_stripe().is_some());
    }

    /// The last block of the history ends on the bottom row, which has no
    /// gap under it.
    #[test]
    fn the_last_block_gets_no_line_and_the_one_before_it_does() {
        let before_last = BlockRect {
            rect: Rectangle {
                x: 4.0,
                y: 10.0,
                width: 100.0,
                height: 20.0,
            },
            ..block_rect(1, BlockKind::Command)
        };
        let last = BlockRect {
            has_gap_below: false,
            ..block_rect(4, BlockKind::Prompt)
        };

        let paints = block_paints(&[before_last, last], None, None);

        assert_eq!(
            paints[0].line(),
            Some(Rectangle {
                x: 4.0,
                y: 29.0,
                width: 100.0,
                height: 1.0,
            })
        );
        assert_eq!(paints[1].line(), None);
    }

    #[test]
    fn the_prompt_block_gets_no_background_when_hovered() {
        let rects = [block_rect(4, BlockKind::Prompt)];

        let paints = block_paints(&rects, None, Some(BlockKey::new(4)));

        assert_eq!(paints[0].background(), None);
        assert_eq!(paints[0].failed_stripe(), None);
    }

    /// The block every terminal starts with is no block for the user,
    /// whatever it holds: a shell without the integration has only it.
    #[test]
    fn the_start_block_gets_no_background_no_stripe_and_no_line() {
        let start = BlockRect {
            is_failed: true,
            ..block_rect(0, BlockKind::Start)
        };
        let ls = block_rect(1, BlockKind::Command);

        let selected = block_paints(
            &[start.clone(), ls.clone()],
            Some(BlockKey::new(0)),
            None,
        );
        let hovered = block_paints(&[start, ls], None, Some(BlockKey::new(0)));

        for paints in [selected, hovered] {
            assert_eq!(paints[0].background(), None);
            assert_eq!(paints[0].failed_stripe(), None);
            assert_eq!(paints[0].line(), None);
            assert!(paints[1].line().is_some());
        }
    }

    /// Issue 07: a failed block of one row over the prompt, in a pane of 4
    /// rows of 10 px. Its background and its stripe reach over the 6 px
    /// above and under its row, and its line lies under the gap below it,
    /// 6 px above the gap of the prompt.
    #[test]
    fn the_paint_of_a_block_covers_the_gaps_around_its_rows() {
        use iced::{Point, Size};
        use otty_libterm::surface::{BlockSnapshot, SnapshotOwned};

        use crate::testing::block_rects_of;

        let mut failed = BlockSnapshot {
            key: BlockKey::new(1),
            start_line: 2,
            line_count: 1,
            ..BlockSnapshot::default()
        };
        failed.meta.kind = BlockKind::Command;
        failed.meta.is_finished = true;
        failed.meta.exit_code = Some(1);
        let mut prompt = BlockSnapshot {
            key: BlockKey::new(2),
            start_line: 3,
            line_count: 1,
            ..BlockSnapshot::default()
        };
        prompt.meta.kind = BlockKind::Prompt;
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks = vec![failed, prompt];
        let mut view = snapshot.view();
        view.size.screen_lines = 4;
        let rects =
            block_rects_of(&view, Point::ORIGIN, Size::new(100.0, 40.0), 10.0);

        let paints = block_paints(&rects, None, None);

        // The prompt row is 30..40, the failed row 7..17.
        let expected = Rectangle {
            x: 0.0,
            y: 7.0 - 6.0,
            width: 100.0,
            height: 6.0 + 10.0 + 7.0,
        };
        assert_eq!(paints[0].rect(), expected);
        assert_eq!(paints[0].background(), Some(BlockBackground::Failed));
        let stripe = paints[0].failed_stripe().expect("a stripe");
        assert_eq!((stripe.y, stripe.height), (expected.y, expected.height));
        let line = paints[0].line().expect("a line");
        assert_eq!((line.y, line.height), (17.0 + 6.0, 1.0));
        assert_eq!(paints[1].rect().y, 24.0);
    }

    /// Issue 17: scrolled up, `pwd` ends on the bottom row of a pane of 4
    /// rows of 10 px and the prompt under it is below the pane. The bottom
    /// row has no gap under it, so `pwd` gets no line, which would lie over
    /// its text; `ls` above it keeps its line in the gap under it.
    #[test]
    fn a_block_that_ends_on_the_bottom_row_gets_no_line_over_its_text() {
        use iced::{Point, Size};
        use otty_libterm::surface::{BlockSnapshot, SnapshotOwned};

        use crate::testing::block_rects_of;

        let mut snapshot = SnapshotOwned::default();
        for (key, kind, start_line, line_count) in [
            (1, BlockKind::Command, 0, 2),
            (2, BlockKind::Command, 2, 2),
            (3, BlockKind::Prompt, 4, 0),
        ] {
            let mut block = BlockSnapshot {
                key: BlockKey::new(key),
                start_line,
                line_count,
                ..BlockSnapshot::default()
            };
            block.meta.kind = kind;
            snapshot.blocks.push(block);
        }
        let mut view = snapshot.view();
        view.size.screen_lines = 4;
        let rects =
            block_rects_of(&view, Point::ORIGIN, Size::new(100.0, 40.0), 10.0);

        let paints = block_paints(&rects, None, None);

        // The rows of `pwd` are 20..40, the gap above it 14..20 and the
        // gap and the line under `ls` 7..14.
        assert_eq!(paints.len(), 2);
        let line = paints[0].line().expect("a line under ls");
        assert_eq!((line.y, line.height), (13.0, 1.0));
        assert_eq!(paints[1].line(), None);
    }

    #[test]
    fn a_block_without_height_is_not_painted() {
        let flat = BlockRect {
            rect: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 0.0,
            },
            ..block_rect(1, BlockKind::Command)
        };

        let paints = block_paints(&[flat], Some(BlockKey::new(1)), None);

        assert!(paints.is_empty());
    }

    #[test]
    fn a_block_keeps_its_place() {
        let rects = [block_rect(1, BlockKind::Command)];

        let paints = block_paints(&rects, None, None);

        assert_eq!(paints[0].rect(), rects[0].rect);
    }
}
