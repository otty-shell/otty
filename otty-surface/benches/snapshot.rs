use criterion::{Criterion, black_box, criterion_group, criterion_main};
use otty_surface::{
    Dimensions, Surface, SurfaceActor, SurfaceConfig, SurfaceModel,
};

struct BenchDimensions {
    columns: usize,
    lines: usize,
}

impl BenchDimensions {
    fn new(columns: usize, lines: usize) -> Self {
        Self { columns, lines }
    }
}

impl Dimensions for BenchDimensions {
    fn total_lines(&self) -> usize {
        self.lines
    }

    fn screen_lines(&self) -> usize {
        self.lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

fn bench_snapshot_owned(c: &mut Criterion) {
    let dims = BenchDimensions::new(80, 24);
    let mut surface = Surface::new(SurfaceConfig::default(), &dims);
    surface.reset_damage();

    c.bench_function("snapshot_owned_with_damage", |b| {
        b.iter(|| {
            surface.print('x');
            let frame = surface.snapshot_owned();
            surface.reset_damage();
            black_box(frame.view().visible_cell_count);
        });
    });
}

criterion_group!(surface, bench_snapshot_owned);
criterion_main!(surface);
