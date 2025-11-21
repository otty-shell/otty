use otty_surface::{
    Dimensions, Surface, SurfaceActor, SurfaceConfig, SurfaceModel,
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
    let size = Size {
        cols: 80,
        lines: 24,
    };
    let mut surface = Surface::new(SurfaceConfig::default(), &size);

    // Drive the surface using the `SurfaceActor` API.
    surface.print('H');
    surface.print('i');
    surface.new_line();
    surface.print('>');

    // Capture a snapshot and iterate over visible cells for rendering.
    let snapshot = surface.snapshot_owned();
    for indexed in snapshot.view().cells {
        let ch = indexed.cell.c;
        print!("{ch}");
    }
}
