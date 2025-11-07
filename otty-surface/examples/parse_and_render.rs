use log::trace;
use otty_escape::{Action, EscapeActor, EscapeParser, Parser, vte};
use otty_surface::{Surface, SurfaceConfig, SurfaceController};

fn main() {
    println!("=== otty-escape → otty-surface Integration Example ===\n");

    // Create a small surface for demonstration.
    let mut surface = Surface::new(SurfaceConfig {
        columns: 40,
        rows: 10,
        ..SurfaceConfig::default()
    });

    let mut parser = Parser::new();

    // Example 1: Basic printing and cursor movement.
    println!("Example 1: Basic text printing");
    advance(&mut parser, &mut surface, b"Hello, World!\r\n");
    advance(&mut parser, &mut surface, b"Line 2\r\n");

    // Example 2: SGR attributes.
    println!("\nExample 2: SGR (colors and styles)");
    advance(&mut parser, &mut surface, b"\x1b[1mBold\x1b[0m ");
    advance(&mut parser, &mut surface, b"\x1b[3mItalic\x1b[0m ");
    advance(&mut parser, &mut surface, b"\x1b[31mRed\x1b[0m\r\n");

    // Example 3: Cursor positioning.
    println!("\nExample 3: Cursor positioning");
    advance(&mut parser, &mut surface, b"\x1b[5;10HPOS(5,10)");
    advance(&mut parser, &mut surface, b"\x1b[6;1H");

    // Example 4: Clearing.
    println!("\nExample 4: Clearing");
    advance(&mut parser, &mut surface, b"Before clear");
    advance(&mut parser, &mut surface, b"\x1b[2K"); // Clear line
    advance(&mut parser, &mut surface, b"After clear\r\n");

    // Example 5: Scrolling.
    println!("\nExample 5: Scrolling (fill screen)");
    for i in 0..12 {
        let line = format!("Line {}\r\n", i);
        advance(&mut parser, &mut surface, line.as_bytes());
    }

    println!("\nScrollback history size: {}", surface.history_size());

    // Example 6: Tabs.
    println!("\nExample 6: Tabs");
    advance(&mut parser, &mut surface, b"A\tB\tC\r\n");

    // Print final grid state.
    println!("\n=== Final Grid State (visible area) ===");
    print_grid(&surface);

    println!("\n=== Display Metrics ===");
    println!("Width: {}", surface.grid().width());
    println!("Height: {}", surface.grid().height());
    println!("History: {}", surface.history_size());
    println!("Cursor: {:?}", surface.cursor_position());
}

fn advance(
    parser: &mut Parser<vte::Parser>,
    surface: &mut Surface,
    bytes: &[u8],
) {
    struct SurfaceActor<'a> {
        surface: &'a mut Surface,
    }

    impl<'a> EscapeActor for SurfaceActor<'a> {
        fn handle(&mut self, action: Action) {
            use Action::*;

            match action {
                Print(ch) => self.surface.print(ch),
                Bell => self.surface.bell(),
                InsertBlank(count) => self.surface.insert_blank(count),
                InsertBlankLines(count) => {
                    self.surface.insert_blank_lines(count)
                },
                DeleteLines(count) => self.surface.delete_lines(count),
                DeleteChars(count) => self.surface.delete_chars(count),
                EraseChars(count) => self.surface.erase_chars(count),
                Backspace => self.surface.backspace(),
                CarriageReturn => self.surface.carriage_return(),
                LineFeed => self.surface.line_feed(),
                NextLine | NewLine => self.surface.new_line(),
                Substitute => self.surface.print('�'),
                SetHorizontalTab => self.surface.set_horizontal_tab(),
                ReverseIndex => self.surface.reverse_index(),
                ResetState => self.surface.reset(),
                ClearScreen(mode) => self.surface.clear_screen(mode),
                ClearLine(mode) => self.surface.clear_line(mode),
                InsertTabs(count) => self.surface.insert_tabs(count as usize),
                SetTabs(mask) => self.surface.set_tabs(mask),
                ClearTabs(mode) => self.surface.clear_tabs(mode),
                ScreenAlignmentDisplay => {
                    self.surface.screen_alignment_display()
                },
                MoveForwardTabs(count) => {
                    self.surface.move_forward_tabs(count as usize);
                },
                MoveBackwardTabs(count) => {
                    self.surface.move_backward_tabs(count as usize);
                },
                SetActiveCharsetIndex(_) | ConfigureCharset(_, _) => {
                    trace!("Charset handling not implemented yet");
                },
                SetColor { index, color } => {
                    self.surface.set_color(index, color)
                },
                QueryColor(index) => self.surface.query_color(index),
                ResetColor(index) => self.surface.reset_color(index),
                SetScrollingRegion(top, bottom) => {
                    self.surface.set_scrolling_region(top, bottom);
                },
                ScrollUp(count) => self.surface.scroll_up(count),
                ScrollDown(count) => self.surface.scroll_down(count),
                SetHyperlink(link) => self.surface.set_hyperlink(link),
                SGR(attribute) => self.surface.sgr(attribute),
                SetCursorShape(shape) => self.surface.set_cursor_shape(shape),
                SetCursorIcon(icon) => self.surface.set_cursor_icon(icon),
                SetCursorStyle(style) => self.surface.set_cursor_style(style),
                SaveCursorPosition => self.surface.save_cursor(),
                RestoreCursorPosition => self.surface.restore_cursor(),
                MoveUp {
                    rows,
                    carrage_return_needed,
                } => self.surface.move_up(rows, carrage_return_needed),
                MoveDown {
                    rows,
                    carrage_return_needed,
                } => self.surface.move_down(rows, carrage_return_needed),
                MoveForward(cols) => self.surface.move_forward(cols),
                MoveBackward(cols) => self.surface.move_backward(cols),
                Goto(row, col) => self.surface.goto(row, col),
                GotoRow(row) => self.surface.goto_row(row),
                GotoColumn(col) => self.surface.goto_column(col),
                IdentifyTerminal(response) => {
                    trace!("Identify terminal {:?}", response);
                },
                ReportDeviceStatus(status) => {
                    trace!("Report device status {}", status);
                },
                SetKeypadApplicationMode => {
                    self.surface.set_keypad_application_mode(true);
                },
                UnsetKeypadApplicationMode => {
                    self.surface.set_keypad_application_mode(false);
                },
                SetModifyOtherKeysState(state) => {
                    trace!("modifyOtherKeys => {:?}", state);
                },
                ReportModifyOtherKeysState => trace!("Report modifyOtherKeys"),
                ReportKeyboardMode => trace!("Report keyboard mode"),
                SetKeyboardMode(mode, behavior) => {
                    trace!("Set keyboard mode {:?} {:?}", mode, behavior);
                },
                PushKeyboardMode(_) => self.surface.push_keyboard_mode(),
                PopKeyboardModes(amount) => {
                    self.surface.pop_keyboard_modes(amount)
                },
                SetMode(mode) => self.surface.set_mode(mode, true),
                SetPrivateMode(mode) => {
                    self.surface.set_private_mode(mode, true)
                },
                UnsetMode(mode) => self.surface.set_mode(mode, false),
                UnsetPrivateMode(mode) => {
                    self.surface.set_private_mode(mode, false)
                },
                ReportMode(mode) => trace!("Report mode {:?}", mode),
                ReportPrivateMode(mode) => {
                    trace!("Report private mode {:?}", mode);
                },
                RequestTextAreaSizeByPixels => {
                    trace!("Request text area size (pixels)");
                },
                RequestTextAreaSizeByChars => {
                    trace!("Request text area size (chars)");
                },
                PushWindowTitle => self.surface.push_window_title(),
                PopWindowTitle => self.surface.pop_window_title(),
                SetWindowTitle(title) => self.surface.set_window_title(title),
            }
        }

        fn begin_sync(&mut self) {
            self.surface.begin_sync();
        }

        fn end_sync(&mut self) {
            self.surface.end_sync();
        }
    }

    let mut actor = SurfaceActor { surface };
    parser.advance(bytes, &mut actor);
}

fn print_grid(surface: &Surface) {
    let grid = surface.grid();
    let width = grid.width();
    let height = grid.height();

    println!("┌{}┐", "─".repeat(width));
    for row_idx in 0..height {
        let row = grid.row(row_idx);
        print!("│");
        for col_idx in 0..width {
            let cell = &row.cells[col_idx];
            print!("{}", cell.ch);
        }
        println!("│");
    }
    println!("└{}┘", "─".repeat(width));
}
