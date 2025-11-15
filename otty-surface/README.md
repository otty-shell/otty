# otty-surface

`otty-surface` is a grid-based terminal surface library. It provides the
in-memory model of a terminal screen: cells, scrollback history, cursor,
selection, damage tracking, colors and modes – everything a renderer needs to
draw a terminal, independent of any specific escape-sequence parser or UI
framework.

The crate is used by higher-level components (like terminal frontends or TUI libraries) to:

- Apply high-level terminal actions (print, scroll, clear, set modes, etc.).
- Maintain scrollback and primary/alternate screen buffers.
- Track selections and semantic ranges.
- Compute minimal "damage" regions for efficient redraw.
- Capture snapshots suitable for rendering.

## Basic usage example

```rust
use otty_surface::{
    Surface, SurfaceConfig,
    Dimensions, SurfaceActor,
};

/// Simple static terminal size.
struct Size {
    cols: usize,
    lines: usize,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.lines
    }

    fn screen_lines(&self) -> usize {
        self.lines
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

fn main() {
    // Create a 80x24 surface with default configuration.
    let size = Size { cols: 80, lines: 24 };
    let mut surface = Surface::new(SurfaceConfig::default(), &size);

    // Drive the surface using the `SurfaceActor` API.
    surface.print('H');
    surface.print('i');
    surface.new_line();
    surface.print('>');

    // Capture a snapshot and iterate over visible cells for rendering.
    let snapshot = surface.snapshot();
    for indexed in snapshot.display_iter {
        let ch = indexed.cell.c;
        print!("{ch}");
    }
}
```
