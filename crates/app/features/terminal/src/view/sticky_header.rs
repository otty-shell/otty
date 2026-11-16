//! The layer over a terminal pane with the sticky header: a kit
//! [`BlockHeader`] at the top of the pane for the command block whose first
//! line is above the top edge. The feature fills it with the command, how
//! it ended and how many of its lines are above; the kit draws it and
//! decides what gives way in a narrow pane.

use iced::Element;
use iced::widget::{Space, responsive};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::block_header::BlockHeader;
use otty_ui_kit::palette::UiPalette;
use otty_ui_term::{BlockKey, TEXT_INSET_X};

use crate::model::{StickyHeader, sticky_header};
use crate::types::Terminal;

/// The layer over `terminal`: the sticky header of the block at the top of
/// the pane, counted up to `now_ms` (milliseconds since 1970) for a running
/// command, or nothing when that block starts on screen or a full-screen
/// program is on. The block at the
/// top is the first one with any part of a row still in the pane; a row
/// the gaps between the blocks push over the top edge, even in part, is
/// above its start ([`Terminal::blocks_in_pane`]).
///
/// The bar lies over the output and does not push it down. A press on the
/// bar sends `on_press` and starts no text selection in the lines it
/// covers; the wheel goes through to the terminal. Its button sends
/// `on_go_to_start` with the key of the block.
pub(crate) fn view<'a, Message: Clone + 'a>(
    terminal: &'a Terminal,
    now_ms: i64,
    palette: &UiPalette,
    on_press: Message,
    on_go_to_start: impl Fn(BlockKey) -> Message + 'a,
) -> Element<'a, Message> {
    let palette = palette.clone();

    responsive(move |size| {
        let blocks = terminal.blocks_in_pane();
        let is_full_screen = terminal.snapshot().view().is_full_screen();

        match sticky_header(&blocks, is_full_screen, now_ms) {
            Some(header) => header_bar(
                &header,
                size.width,
                &palette,
                on_press.clone(),
                &on_go_to_start,
            ),
            None => Space::new().into(),
        }
    })
    .into()
}

/// The kit bar of `header` in a pane `pane_width` wide, its texts 16 px in
/// like the text of the terminal.
fn header_bar<'a, Message: Clone + 'a>(
    header: &StickyHeader,
    pane_width: f32,
    palette: &UiPalette,
    on_press: Message,
    on_go_to_start: &impl Fn(BlockKey) -> Message,
) -> Element<'a, Message> {
    let mut bar = BlockHeader::new(
        header.outcome(),
        header.command(),
        header.duration(),
        palette,
    )
    .go_to_start(
        i18n::t(Key::BlockGoToStart),
        on_go_to_start(header.block_key()),
    )
    .on_press(on_press)
    .width(pane_width)
    .text_inset(TEXT_INSET_X);

    if let Some(code) = header.exit_code() {
        bar = bar.exit_code(i18n::block_exit_code(code));
    }
    if header.lines_above() > 0 {
        bar = bar.lines_above(i18n::block_lines_above(header.lines_above()));
    }

    bar.into()
}

#[cfg(test)]
mod tests {
    use iced::widget::{Space, mouse_area, stack};
    use iced::{Event, Length, Point, Size, mouse};
    use iced_test::Simulator;
    use otty_app_testing::blocks::{command_block, finished, prompt_block};
    use otty_app_testing::terminal::ScriptedTerminals;
    use otty_libterm::TerminalSize;
    use otty_libterm::escape::NamedPrivateMode;
    use otty_libterm::surface::{
        Surface, SurfaceActor, SurfaceConfig, SurfaceModel,
    };
    use otty_ui_term::{BlockKey, BlockSnapshot};

    use super::*;
    use crate::testing::{screen_of, terminal_showing};

    const PANE: Size = Size::new(400.0, 200.0);
    /// A point over the bar, away from its texts and its button.
    const OVER_THE_BAR: Point = Point::new(300.0, 15.0);
    /// A point over the button at the right end of the bar: 8 px from the
    /// edge, 24 px wide.
    const OVER_THE_BUTTON: Point = Point::new(PANE.width - 8.0 - 12.0, 15.0);
    /// A point over the output under the bar.
    const UNDER_THE_BAR: Point = Point::new(300.0, 100.0);

    /// The key of `cargo build`.
    const CARGO: BlockKey = BlockKey::new(7);

    #[derive(Debug, Clone, PartialEq)]
    enum Reached {
        TerminalPressed,
        TerminalScrolled,
        BarPressed,
        GoToStart(BlockKey),
    }

    /// `cargo build`, failed with 101 after 48.2 s, with 412 lines above
    /// the pane.
    fn failed_cargo_above_the_pane() -> BlockSnapshot {
        let mut block = BlockSnapshot {
            key: CARGO,
            line_count: 10,
            is_status_line_visible: false,
            text_lines_above: 412,
            ..finished(command_block("cmd-7", 1_000), Some(101), 48_200)
        };
        block.meta.cmd = Some(String::from("cargo build"));

        block
    }

    /// A stand-in for the terminal that reports what reaches it, with the
    /// bar of `header` over it in a pane of [`PANE`].
    fn header_over_a_terminal<'a>(
        header: &StickyHeader,
        palette: &UiPalette,
    ) -> Element<'a, Reached> {
        let terminal =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_press(Reached::TerminalPressed)
                .on_scroll(|_| Reached::TerminalScrolled);
        let bar = header_bar(
            header,
            PANE.width,
            palette,
            Reached::BarPressed,
            &Reached::GoToStart,
        );

        stack![terminal, bar]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn press_and_scroll_at(ui: &mut Simulator<'_, Reached>, position: Point) {
        ui.point_at(position);
        let _ = ui.simulate([
            Event::Mouse(mouse::Event::CursorMoved { position }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 },
            }),
        ]);
    }

    #[test]
    fn given_failed_block_above_the_pane_when_shown_then_the_bar_has_its_command_lines_above_exit_code_and_duration()
     {
        let blocks = [failed_cargo_above_the_pane()];
        let header = sticky_header(&blocks, false, 60_000).expect("a header");
        let palette = UiPalette::otty_dark();

        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            PANE,
            header_over_a_terminal(&header, &palette),
        );

        assert!(ui.find("cargo build").is_ok());
        assert!(ui.find("412 lines above").is_ok());
        assert!(ui.find("exit 101").is_ok());
        assert!(ui.find("48.2 s").is_ok());
    }

    #[test]
    fn given_bar_over_the_terminal_when_its_button_is_pressed_then_it_goes_to_the_start_of_its_block()
     {
        let blocks = [failed_cargo_above_the_pane()];
        let header = sticky_header(&blocks, false, 60_000).expect("a header");
        let palette = UiPalette::otty_dark();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            PANE,
            header_over_a_terminal(&header, &palette),
        );

        press_and_scroll_at(&mut ui, OVER_THE_BUTTON);

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert!(
            reached.contains(&Reached::GoToStart(CARGO)),
            "got {reached:?}"
        );
        assert!(
            !reached.contains(&Reached::TerminalPressed),
            "got {reached:?}"
        );
        assert!(!reached.contains(&Reached::BarPressed), "got {reached:?}");
    }

    #[test]
    fn given_bar_over_the_terminal_when_pressed_and_scrolled_on_it_then_the_press_stays_with_the_bar_and_the_wheel_reaches_the_terminal()
     {
        let blocks = [failed_cargo_above_the_pane()];
        let header = sticky_header(&blocks, false, 60_000).expect("a header");
        let palette = UiPalette::otty_dark();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            PANE,
            header_over_a_terminal(&header, &palette),
        );

        press_and_scroll_at(&mut ui, OVER_THE_BAR);

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert_eq!(reached, [Reached::BarPressed, Reached::TerminalScrolled]);
    }

    #[test]
    fn given_bar_over_the_terminal_when_pressed_under_it_then_the_terminal_gets_the_press()
     {
        let blocks = [failed_cargo_above_the_pane()];
        let header = sticky_header(&blocks, false, 60_000).expect("a header");
        let palette = UiPalette::otty_dark();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            PANE,
            header_over_a_terminal(&header, &palette),
        );

        press_and_scroll_at(&mut ui, UNDER_THE_BAR);

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert_eq!(
            reached,
            [Reached::TerminalPressed, Reached::TerminalScrolled]
        );
    }

    /// Issue 07: `ls -la` holds the six top rows of 10 rows of 20 px, its
    /// status line first, then `pwd` and the prompt. The gaps between the
    /// blocks push the status row and the command row of `ls -la` over the
    /// top edge, so its start is above the pane and it gets the bar, with
    /// the command row counted above.
    #[test]
    fn given_the_gaps_push_the_status_line_over_the_top_edge_when_shown_then_the_bar_is_on()
     {
        let grid = TerminalSize {
            cell_width: 8,
            cell_height: 20,
            cols: 40,
            rows: 10,
        };
        let mut ls = BlockSnapshot {
            line_count: 6,
            ..finished(command_block("ls", 1_000), Some(0), 100)
        };
        ls.meta.cmd = Some(String::from("ls -la"));
        let pwd = BlockSnapshot {
            start_line: 6,
            line_count: 3,
            ..finished(command_block("pwd", 2_000), Some(0), 100)
        };
        let prompt = BlockSnapshot {
            start_line: 9,
            ..prompt_block()
        };
        let terminals = ScriptedTerminals::default();
        let terminal = terminal_showing(
            &terminals,
            3,
            grid,
            screen_of(grid, vec![ls, pwd, prompt]),
        );
        let palette = UiPalette::otty_dark();

        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            PANE,
            view(&terminal, 60_000, &palette, Reached::BarPressed, |key| {
                Reached::GoToStart(key)
            }),
        );

        assert!(ui.find("ls -la").is_ok());
        assert!(ui.find("1 line above").is_ok());
    }

    /// 10 rows of 20 px, as above.
    const GRID: TerminalSize = TerminalSize {
        cell_width: 8,
        cell_height: 20,
        cols: 40,
        rows: 10,
    };

    /// `ls -la` finished, from the top row, `line_count` rows high.
    fn ls_at_the_top(line_count: usize) -> BlockSnapshot {
        let mut ls = BlockSnapshot {
            line_count,
            ..finished(command_block("ls", 1_000), Some(0), 100)
        };
        ls.meta.cmd = Some(String::from("ls -la"));

        ls
    }

    /// The sticky header over the real terminal widget that shows
    /// `blocks` in a pane of [`GRID`].
    fn header_over(blocks: Vec<BlockSnapshot>) -> Vec<String> {
        let terminals = ScriptedTerminals::default();
        let terminal =
            terminal_showing(&terminals, 3, GRID, screen_of(GRID, blocks));
        let palette = UiPalette::otty_dark();

        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            PANE,
            view(&terminal, 60_000, &palette, Reached::BarPressed, |key| {
                Reached::GoToStart(key)
            }),
        );

        ["ls -la", "1 line above"]
            .into_iter()
            .filter(|text| ui.find(*text).is_ok())
            .map(String::from)
            .collect()
    }

    /// `ls -la` holds its status row and its last row, then `cargo test`
    /// (7 rows) and the prompt. The gaps push the status row over the top
    /// edge and the last row of `ls -la` in part: the row still shows at
    /// the top, so the bar is on, with that row counted above.
    #[test]
    fn given_the_last_row_of_a_block_partly_over_the_top_edge_when_shown_then_the_bar_is_on()
     {
        let cargo = BlockSnapshot {
            start_line: 2,
            line_count: 7,
            ..finished(command_block("cargo", 2_000), Some(0), 100)
        };
        let prompt = BlockSnapshot {
            start_line: 9,
            ..prompt_block()
        };

        let shown = header_over(vec![ls_at_the_top(2), cargo, prompt]);

        assert_eq!(shown, ["ls -la", "1 line above"]);
    }

    /// `cd` on the top row, `ls -la` (its status row and its command row)
    /// under it, then `cargo test` of `cargo_rows` rows, `date` when
    /// `with_date`, and the prompt on the bottom row of [`GRID`].
    fn ls_under_cd(cargo_rows: usize, with_date: bool) -> Vec<BlockSnapshot> {
        let cd = BlockSnapshot {
            line_count: 1,
            ..finished(command_block("cd", 500), Some(0), 100)
        };
        let ls = BlockSnapshot {
            start_line: 1,
            ..ls_at_the_top(2)
        };
        let cargo = BlockSnapshot {
            start_line: 3,
            line_count: cargo_rows,
            ..finished(command_block("cargo", 2_000), Some(0), 100)
        };
        let date = BlockSnapshot {
            start_line: 8,
            line_count: 1,
            ..finished(command_block("date", 3_000), Some(0), 100)
        };
        let prompt = BlockSnapshot {
            start_line: 9,
            ..prompt_block()
        };

        let mut blocks = vec![cd, ls, cargo];
        if with_date {
            blocks.push(date);
        }
        blocks.push(prompt);

        blocks
    }

    /// Issue 18: the gaps push `cd` wholly over the top edge and 6 px of
    /// the 20 px status row of `ls -la`, less than half of it: the status
    /// line of `ls -la` still shows, so it gets no bar.
    #[test]
    fn given_less_than_half_of_the_status_row_over_the_top_edge_when_shown_then_the_bar_is_off()
     {
        let shown = header_over(ls_under_cd(6, false));

        assert!(shown.is_empty(), "got {shown:?}");
    }

    /// Issue 18: with `date` under `cargo test` the gaps push 19 px of the
    /// status row of `ls -la` over the top edge, more than half of it: the
    /// bar is on.
    #[test]
    fn given_more_than_half_of_the_status_row_over_the_top_edge_when_shown_then_the_bar_is_on()
     {
        let shown = header_over(ls_under_cd(5, true));

        assert_eq!(shown, ["ls -la"]);
    }

    /// `ls -la` holds the three top rows, then four blocks of one row and
    /// the prompt. The gaps push every row of `ls -la` over the top edge;
    /// only its gap and the line under it are left, so it gets no bar.
    #[test]
    fn given_only_the_gap_of_a_block_left_at_the_top_when_shown_then_the_bar_is_off()
     {
        let mut blocks = vec![ls_at_the_top(3)];
        for (row, name) in [(3, "cd"), (4, "pwd"), (5, "true"), (6, "date")] {
            blocks.push(BlockSnapshot {
                start_line: row,
                line_count: 1,
                ..finished(command_block(name, 2_000), Some(0), 100)
            });
        }
        blocks.push(BlockSnapshot {
            start_line: 7,
            line_count: 3,
            ..prompt_block()
        });

        let shown = header_over(blocks);

        assert!(shown.is_empty(), "got {shown:?}");
    }

    /// Issue 17: `vim` runs on the full screen. The snapshot is in the mode
    /// of a full-screen program, so `vim` gets no bar, whatever its status
    /// line.
    #[test]
    fn given_a_full_screen_program_when_shown_then_the_bar_is_off() {
        let mut surface = Surface::new(SurfaceConfig::default(), &GRID);
        surface.set_private_mode(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        );
        let mut snapshot = surface.snapshot_owned();
        let mut vim = BlockSnapshot {
            line_count: 10,
            is_status_line_visible: false,
            text_lines_above: 3,
            ..command_block("vim", 1_000)
        };
        vim.meta.cmd = Some(String::from("vim"));
        snapshot.blocks = vec![vim];
        let terminals = ScriptedTerminals::default();
        let terminal = terminal_showing(&terminals, 3, GRID, snapshot);
        let palette = UiPalette::otty_dark();

        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            PANE,
            view(&terminal, 60_000, &palette, Reached::BarPressed, |key| {
                Reached::GoToStart(key)
            }),
        );

        assert!(ui.find("vim").is_err());
    }
}
