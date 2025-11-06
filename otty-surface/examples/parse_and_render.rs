use otty_escape::Parser;
use otty_surface::{Surface, SurfaceConfig};

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

fn advance(parser: &mut Parser, surface: &mut Surface, bytes: &[u8]) {
    parser.advance(bytes, surface);
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
