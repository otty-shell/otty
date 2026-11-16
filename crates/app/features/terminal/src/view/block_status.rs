//! The layer over a terminal pane that puts a kit [`BlockStatusLine`] on
//! the status line of every command block: the empty line the terminal
//! adds above the text of a command. The feature fills it with the status
//! of the block, its words and its fold arrow; the kit draws it and decides
//! what gives way in a narrow pane.

use iced::widget::{Stack, pin, responsive};
use iced::{Element, Length, Point};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::block_outcome::BlockOutcome;
use otty_ui_kit::components::block_status_line::BlockStatusLine;
use otty_ui_kit::palette::UiPalette;
use otty_ui_term::{BlockKey, BlockRect, TEXT_INSET_X};

use crate::model::{
    BlockFold, BlockStatus, FoldAction, visible_block_folds,
    visible_block_statuses,
};
use crate::types::Terminal;

/// The layer over `terminal`: on every status line on screen, the status
/// of its command block, counted up to `now_ms` (milliseconds since 1970)
/// for a running command, the badge of a folded block, and the fold arrow
/// of a folded block or of the block under the pointer. Pressing an arrow
/// sends `on_toggle` with the key of its block.
///
/// The places of the status lines come from the terminal widget
/// ([`Terminal::block_rects`]), so every line sits where the widget keeps
/// it: over the gap above its block and its status row, also past the top
/// of the history. Which blocks show a status line is what the pane shows
/// of them ([`Terminal::blocks_in_pane`]), and the block under the pointer
/// is the one the widget reported last
/// ([`Terminal::hovered_block_key`]). Only an arrow takes the pointer;
/// clicks, drags and the wheel elsewhere go to the terminal under the
/// layer.
pub(crate) fn view<'a, Message: Clone + 'a>(
    terminal: &'a Terminal,
    now_ms: i64,
    palette: &UiPalette,
    on_toggle: impl Fn(BlockKey) -> Message + 'a,
) -> Element<'a, Message> {
    let palette = palette.clone();

    responsive(move |size| {
        let rects = terminal.block_rects(Point::ORIGIN, size);
        let blocks = terminal.blocks_in_pane();
        let statuses = visible_block_statuses(&blocks, now_ms);
        let folds = visible_block_folds(&blocks, terminal.hovered_block_key());

        status_line_layer(
            &statuses, &folds, &rects, size.width, &palette, &on_toggle,
        )
    })
    .into()
}

/// Put the status line of every status, with the fold arrow of its block,
/// where the rectangle of its block places it (see
/// [`BlockRect::status_line`]), `pane_width` wide. A status whose block
/// shows no status line is left out.
fn status_line_layer<'a, Message: Clone + 'a>(
    statuses: &[BlockStatus],
    folds: &[BlockFold],
    rects: &[BlockRect],
    pane_width: f32,
    palette: &UiPalette,
    on_toggle: &impl Fn(BlockKey) -> Message,
) -> Element<'a, Message> {
    let lines = statuses.iter().filter_map(|status| {
        let block_key = status.block_key();
        let place = rects
            .iter()
            .find(|rect| rect.block_key == block_key)?
            .status_line()?;
        let fold = folds.iter().find(|fold| fold.block_key() == block_key);

        let line = status_line(status, fold, palette, on_toggle)
            .line_height(place.height)
            .width(pane_width)
            .text_inset(TEXT_INSET_X);

        Some(pin(line).y(place.y).into())
    });

    Stack::with_children(lines)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The kit status line of `status`: its outcome, the exit code of a failed
/// command or `running`, the duration, the number of lines a fold hides,
/// and the arrow of `fold`, named by its tooltip.
fn status_line<Message>(
    status: &BlockStatus,
    fold: Option<&BlockFold>,
    palette: &UiPalette,
    on_toggle: &impl Fn(BlockKey) -> Message,
) -> BlockStatusLine<Message> {
    let mut line =
        BlockStatusLine::new(status.outcome(), status.duration(), palette);

    if let Some(code) = status.exit_code() {
        line = line.word(i18n::block_exit_code(code));
    }
    if status.outcome() == BlockOutcome::Running {
        line = line.word(i18n::t(Key::BlockRunning));
    }
    if let Some(count) = status.hidden_line_count() {
        line = line.hidden_lines(i18n::block_lines_hidden(count));
    }
    if let Some(fold) = fold {
        let (is_folded, name) = match fold.arrow() {
            FoldAction::Collapse => (false, Key::BlockCollapse),
            FoldAction::Expand => (true, Key::BlockExpand),
        };
        line = line.fold_arrow(
            is_folded,
            i18n::t(name),
            on_toggle(fold.block_key()),
        );
    }

    line
}

#[cfg(test)]
mod tests {
    use iced::widget::{Space, mouse_area, stack};
    use iced::{Event, Size, mouse};
    use iced_test::Simulator;
    use otty_app_testing::blocks::{command_block, finished, prompt_block};
    use otty_app_testing::terminal::ScriptedTerminals;
    use otty_libterm::TerminalSize;
    use otty_ui_term::{BlockKey, BlockSnapshot, SnapshotOwned, TerminalView};

    use super::*;
    use crate::testing::{screen_of, terminal_showing};
    use crate::types::TerminalEvent;

    const LINE_HEIGHT: f32 = 20.0;
    const PANE: Size = Size::new(400.0, 200.0);
    /// A point over the status of the block whose status line is the
    /// second line of the pane.
    const OVER_THE_STATUS: Point = Point::new(40.0, 30.0);
    const TERMINAL_ID: u64 = 7;
    /// A grid of 40 columns and 10 lines of 8 by 20 px cells, in a pane
    /// ([`TERMINAL_PANE`]) 16 px wider at each side.
    const GRID: TerminalSize = TerminalSize {
        cell_width: 8,
        cell_height: 20,
        cols: 40,
        rows: 10,
    };
    const TERMINAL_PANE: Size = Size::new(352.0, 200.0);
    /// A point over the text of the first command line of `ls`.
    const OVER_THE_COMMAND: Point = Point::new(100.0, 30.0);
    /// A point over the fold arrow of `ls`, in the left inset of its
    /// status line, the first line of the pane.
    const OVER_THE_ARROW: Point = Point::new(8.0, 10.0);
    /// The key of `ls`.
    const LS: BlockKey = BlockKey::new(1);

    #[derive(Debug, Clone, PartialEq)]
    enum Under {
        Pressed,
        Released,
        Scrolled,
    }

    #[derive(Debug, Clone)]
    enum Reached {
        Terminal(TerminalEvent),
        Toggle(BlockKey),
    }

    fn running_block() -> BlockSnapshot {
        BlockSnapshot {
            start_line: 1,
            ..command_block("sleep", 1_000)
        }
    }

    /// `ls`, finished well after 0.1 s and folded: it hides 3 lines.
    fn folded_ls() -> BlockSnapshot {
        BlockSnapshot {
            start_line: 1,
            line_count: 3,
            is_foldable: true,
            hidden_line_count: Some(3),
            ..finished(command_block("ls", 1_000), Some(0), 100)
        }
    }

    /// A stand-in for the terminal that reports what reaches it, with the
    /// status layer over it.
    fn status_over_a_terminal<'a>(
        block: &'a BlockSnapshot,
        palette: &UiPalette,
    ) -> Element<'a, Under> {
        let statuses: Vec<BlockStatus> =
            BlockStatus::of_block(block, 13_000).into_iter().collect();
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks = vec![block.clone()];
        let terminals = ScriptedTerminals::default();
        let rects = terminal_showing(&terminals, TERMINAL_ID, GRID, snapshot)
            .block_rects(Point::ORIGIN, PANE);
        let terminal =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_press(Under::Pressed)
                .on_release(Under::Released)
                .on_scroll(|_| Under::Scrolled);
        let palette = palette.clone();
        let layer = responsive(move |size| {
            status_line_layer(
                &statuses,
                &[],
                &rects,
                size.width,
                &palette,
                &|_| Under::Pressed,
            )
        });

        stack![terminal, layer]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// `ls`, finished: its status line, one command line and 3 lines of
    /// output from the top of the pane, then the prompt.
    fn snapshot_with_ls() -> SnapshotOwned {
        let ls = BlockSnapshot {
            key: LS,
            line_count: 5,
            is_foldable: true,
            ..finished(command_block("ls", 1_000), Some(0), 100)
        };
        let prompt = BlockSnapshot {
            start_line: 5,
            ..prompt_block()
        };
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks = vec![ls, prompt];

        snapshot
    }

    /// A terminal that shows `ls` with the pointer reported over it, so
    /// the arrow of `ls` is on screen.
    fn terminal_with_ls_under_the_pointer(
        terminals: &ScriptedTerminals,
    ) -> Terminal {
        let mut terminal =
            terminal_showing(terminals, TERMINAL_ID, GRID, snapshot_with_ls());
        terminal.handle(otty_ui_term::Event::BlockHovered {
            id: TERMINAL_ID,
            block_key: Some(LS),
        });

        terminal
    }

    /// `ls` folded: its status line and its command line stay, and the 3
    /// lines of output are hidden.
    fn snapshot_with_folded_ls() -> SnapshotOwned {
        let mut snapshot = snapshot_with_ls();
        let ls = &mut snapshot.blocks[0];
        ls.line_count = 2;
        ls.hidden_line_count = Some(3);
        snapshot.blocks[1].start_line = 2;

        snapshot
    }

    /// The real terminal widget with the status line layer, which holds
    /// the fold arrows, over it, as the pane stacks them.
    fn folds_over_the_terminal<'a>(
        terminal: &'a Terminal,
        palette: &UiPalette,
    ) -> Element<'a, Reached> {
        let widget = TerminalView::show(terminal).map(Reached::Terminal);
        let layer = view(terminal, 0, palette, Reached::Toggle);

        stack![widget, layer]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Move the pointer to `position` and press and release it there when
    /// `press` is set.
    fn move_to(ui: &mut Simulator<'_, Reached>, position: Point, press: bool) {
        ui.point_at(position);
        let _ =
            ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
        if press {
            let _ = ui.simulate([
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ]);
        }
    }

    #[test]
    fn given_status_over_the_terminal_when_pressed_and_scrolled_on_it_then_the_terminal_gets_it()
     {
        let block = running_block();
        let palette = UiPalette::otty_dark();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            PANE,
            status_over_a_terminal(&block, &palette),
        );
        assert!(ui.find("running").is_ok());
        assert!(ui.find("12 s").is_ok());

        ui.point_at(OVER_THE_STATUS);
        let _ = ui.simulate([
            Event::Mouse(mouse::Event::CursorMoved {
                position: OVER_THE_STATUS,
            }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 },
            }),
        ]);

        let reached: Vec<Under> = ui.into_messages().collect();
        assert_eq!(
            reached,
            vec![Under::Pressed, Under::Released, Under::Scrolled]
        );
    }

    #[test]
    fn given_folded_block_when_shown_then_its_badge_is_on_the_status_line_and_lets_presses_through()
     {
        let block = folded_ls();
        let palette = UiPalette::otty_dark();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            PANE,
            status_over_a_terminal(&block, &palette),
        );
        let badge = ui
            .find("3 lines hidden")
            .expect("the badge is shown")
            .bounds();

        // The status line is the second line of the pane.
        assert!(
            badge.y >= LINE_HEIGHT
                && badge.y + badge.height <= 2.0 * LINE_HEIGHT
        );
        ui.point_at(badge.center());
        let _ = ui.simulate([
            Event::Mouse(mouse::Event::CursorMoved {
                position: badge.center(),
            }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ]);

        let reached: Vec<Under> = ui.into_messages().collect();
        assert_eq!(reached, vec![Under::Pressed, Under::Released]);
    }

    #[test]
    fn given_pointer_moved_from_a_block_onto_its_arrow_when_pressed_then_the_block_stays_hovered_and_folds()
     {
        let terminals = ScriptedTerminals::default();
        let terminal = terminal_with_ls_under_the_pointer(&terminals);
        let palette = UiPalette::otty_dark();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            TERMINAL_PANE,
            folds_over_the_terminal(&terminal, &palette),
        );

        move_to(&mut ui, OVER_THE_COMMAND, false);
        move_to(&mut ui, OVER_THE_ARROW, true);

        let reached: Vec<Reached> = ui.into_messages().collect();
        let hovered: Vec<Option<BlockKey>> = reached
            .iter()
            .filter_map(|message| match message {
                Reached::Terminal(otty_ui_term::Event::BlockHovered {
                    block_key,
                    ..
                }) => Some(*block_key),
                _ => None,
            })
            .collect();
        assert_eq!(hovered, [Some(LS)], "got {reached:?}");
        assert!(
            reached
                .iter()
                .any(|message| matches!(message, Reached::Toggle(LS))),
            "got {reached:?}"
        );
        // The press stays with the arrow: no text selection starts.
        assert!(
            !reached.iter().any(|message| matches!(
                message,
                Reached::Terminal(otty_ui_term::Event::SelectStart { .. })
            )),
            "got {reached:?}"
        );
    }

    #[test]
    fn given_text_selection_dragged_onto_the_arrow_when_released_there_then_the_drag_ends_and_the_block_does_not_fold()
     {
        let terminals = ScriptedTerminals::default();
        let terminal = terminal_with_ls_under_the_pointer(&terminals);
        let palette = UiPalette::otty_dark();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            TERMINAL_PANE,
            folds_over_the_terminal(&terminal, &palette),
        );
        // Over the output of `ls`, and over the prompt.
        let over_the_output = Point::new(140.0, 70.0);
        let over_the_prompt = Point::new(200.0, 110.0);

        move_to(&mut ui, OVER_THE_COMMAND, false);
        let _ = ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ))]);
        move_to(&mut ui, over_the_output, false);
        move_to(&mut ui, OVER_THE_ARROW, false);
        let _ = ui.simulate([Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        ))]);
        // With the button up, a move selects nothing any more.
        move_to(&mut ui, over_the_prompt, false);

        let reached: Vec<Reached> = ui.into_messages().collect();
        // The positions are measured from the left edge of the text,
        // 16 px in from the left edge of the pane.
        let selection_moves: Vec<(f32, f32)> = reached
            .iter()
            .filter_map(|message| match message {
                Reached::Terminal(otty_ui_term::Event::SelectUpdate {
                    position,
                    ..
                }) => Some(*position),
                _ => None,
            })
            .collect();
        assert_eq!(
            selection_moves,
            [(124.0, 70.0), (-8.0, 10.0)],
            "got {reached:?}"
        );
        assert!(
            !reached.iter().any(|message| matches!(
                message,
                Reached::Toggle(_)
                    | Reached::Terminal(
                        otty_ui_term::Event::BlockSelected { .. }
                    )
            )),
            "got {reached:?}"
        );
    }

    /// Issue 09: the wheel did nothing over the arrow.
    #[test]
    fn given_pointer_on_the_arrow_when_the_wheel_turns_then_the_terminal_scrolls_its_history()
     {
        let terminals = ScriptedTerminals::default();
        let terminal = terminal_with_ls_under_the_pointer(&terminals);
        let palette = UiPalette::otty_dark();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            TERMINAL_PANE,
            folds_over_the_terminal(&terminal, &palette),
        );

        move_to(&mut ui, OVER_THE_COMMAND, false);
        move_to(&mut ui, OVER_THE_ARROW, false);
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 2.0 },
        })]);

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert!(
            reached.iter().any(|message| matches!(
                message,
                Reached::Terminal(otty_ui_term::Event::Scroll { delta: 2, .. })
            )),
            "got {reached:?}"
        );
    }

    #[test]
    fn given_folded_block_away_from_the_pointer_when_its_arrow_on_the_status_line_is_pressed_then_it_unfolds()
     {
        let terminals = ScriptedTerminals::default();
        let terminal = terminal_showing(
            &terminals,
            TERMINAL_ID,
            GRID,
            snapshot_with_folded_ls(),
        );
        let palette = UiPalette::otty_dark();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            TERMINAL_PANE,
            folds_over_the_terminal(&terminal, &palette),
        );

        move_to(&mut ui, OVER_THE_ARROW, true);

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert!(
            reached
                .iter()
                .any(|message| matches!(message, Reached::Toggle(LS))),
            "got {reached:?}"
        );
    }

    #[test]
    fn given_folded_block_when_its_command_line_is_pressed_then_the_terminal_gets_the_press()
     {
        let terminals = ScriptedTerminals::default();
        let terminal = terminal_showing(
            &terminals,
            TERMINAL_ID,
            GRID,
            snapshot_with_folded_ls(),
        );
        let palette = UiPalette::otty_dark();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            TERMINAL_PANE,
            folds_over_the_terminal(&terminal, &palette),
        );
        // In the left inset of the command line, under the arrow.
        let under_the_arrow = Point::new(8.0, 30.0);

        move_to(&mut ui, under_the_arrow, true);

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert!(
            reached.iter().any(|message| matches!(
                message,
                Reached::Terminal(otty_ui_term::Event::SelectStart { .. })
            )),
            "got {reached:?}"
        );
        assert!(
            !reached
                .iter()
                .any(|message| matches!(message, Reached::Toggle(_))),
            "got {reached:?}"
        );
    }

    /// Issue 07: `ls` holds rows 6 to 8 of the 10 rows of 20 px, under its
    /// status, and the prompt the bottom row. The gaps put the status row
    /// of `ls` at 107..127 px, and its status line takes the 6 px gap above
    /// it too: 101..127 px, with the status in its middle.
    #[test]
    fn given_gaps_around_the_blocks_when_shown_then_the_status_line_takes_the_gap_above_its_block()
     {
        let ls = BlockSnapshot {
            key: LS,
            start_line: 6,
            line_count: 3,
            ..finished(command_block("ls", 1_000), Some(0), 100)
        };
        let prompt = BlockSnapshot {
            start_line: 9,
            ..prompt_block()
        };
        let terminals = ScriptedTerminals::default();
        let terminal = terminal_showing(
            &terminals,
            TERMINAL_ID,
            GRID,
            screen_of(GRID, vec![ls, prompt]),
        );
        let palette = UiPalette::otty_dark();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            TERMINAL_PANE,
            folds_over_the_terminal(&terminal, &palette),
        );

        let duration = ui.find("0.1 s").expect("the duration").bounds();

        let middle = duration.y + duration.height / 2.0;
        assert!((middle - 114.0).abs() < 0.5, "{duration:?}");
    }
}
