//! Remembers shaped terminal text runs so unchanged text is not shaped again.
//!
//! Shaping is the expensive part of drawing terminal text. Each run is
//! remembered by what its shape depends on, not by where it is on screen, so
//! a frame shapes only the runs whose text or style it has not seen yet.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use iced::{Color, Font};
use iced_graphics::text::cosmic_text;

use crate::render_runs::RenderRun;

/// How many cells of shaped text may stay remembered while the latest frame
/// does not draw all of it.
///
/// Runs that left the screen often come back (scrolling, a redrawn TUI), so
/// keeping them saves shaping. The limit is in cells, not in runs, because
/// one run can be a whole line: a plain text line is a single run as wide as
/// the terminal. Going by the sizes of the structures a shaped buffer holds,
/// a cell takes roughly 200 bytes. That is an estimate, not a measurement;
/// by it this budget, a few screens of text, is several megabytes per
/// terminal.
const CACHE_BUDGET_CELLS: usize = 32_768;

/// Smallest cost of one cached run, in cells.
///
/// It stands for the fixed size of a shaped buffer, so thousands of
/// single-cell runs do not count as almost nothing.
const MIN_RUN_COST_CELLS: usize = 8;

/// A shaped text run that is ready to draw.
#[derive(Clone)]
pub(crate) struct ShapedRun {
    buffer: Arc<cosmic_text::Buffer>,
    baseline_y: f32,
}

impl ShapedRun {
    /// Shaped text buffer, shared by every run with the same shape.
    pub(crate) fn buffer(&self) -> &Arc<cosmic_text::Buffer> {
        &self.buffer
    }

    /// Distance from the top of the buffer to its text baseline.
    pub(crate) fn baseline_y(&self) -> f32 {
        self.baseline_y
    }

    /// Wrap a shaped buffer, reading its baseline once.
    pub(crate) fn new(buffer: cosmic_text::Buffer) -> Self {
        let baseline_y = buffer
            .layout_runs()
            .next()
            .map_or(0.0, |layout_run| layout_run.line_y);

        Self {
            buffer: Arc::new(buffer),
            baseline_y,
        }
    }
}

/// Shaped runs remembered by their shaping inputs.
#[derive(Clone, Default)]
pub(crate) struct ShapedRunCache {
    runs: HashMap<RunShapeKey, CachedRun>,
    /// Summed cost of the remembered runs, in cells.
    cost_cells: usize,
    frame: u64,
}

impl ShapedRunCache {
    /// Number of remembered shaped runs.
    pub(crate) fn len(&self) -> usize {
        self.runs.len()
    }

    /// Begin a new frame; runs asked for from now on count as used by it.
    pub(crate) fn start_frame(&mut self) {
        self.frame += 1;
    }

    /// Return the shaped form of `run`, calling `shape` only when no run
    /// with the same text and style is remembered.
    pub(crate) fn get_or_shape(
        &mut self,
        run: &RenderRun,
        shape: impl FnOnce() -> ShapedRun,
    ) -> ShapedRun {
        let frame = self.frame;
        let cost_cells = &mut self.cost_cells;
        let cached =
            self.runs.entry(RunShapeKey::new(run)).or_insert_with(|| {
                *cost_cells += run_cost_cells(run.cell_columns());

                CachedRun {
                    shaped: shape(),
                    cell_columns: run.cell_columns(),
                    last_used_frame: frame,
                }
            });
        cached.last_used_frame = frame;

        cached.shaped.clone()
    }

    /// End the frame, dropping the runs it did not use once the cache holds
    /// more than [`CACHE_BUDGET_CELLS`].
    pub(crate) fn finish_frame(&mut self) {
        if self.cost_cells <= CACHE_BUDGET_CELLS {
            return;
        }

        let frame = self.frame;
        self.runs
            .retain(|_, cached| cached.last_used_frame == frame);
        self.cost_cells = self
            .runs
            .values()
            .map(|cached| run_cost_cells(cached.cell_columns))
            .sum();
    }

    /// Forget every shaped run.
    pub(crate) fn clear(&mut self) {
        self.runs.clear();
        self.cost_cells = 0;
    }
}

#[derive(Clone)]
struct CachedRun {
    shaped: ShapedRun,
    cell_columns: usize,
    last_used_frame: u64,
}

/// Everything the shape of a run depends on.
///
/// The position on screen is left out on purpose: a run that moved keeps its
/// shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RunShapeKey {
    text: String,
    font: Font,
    cell_columns: usize,
    /// Colors baked into the glyphs. Empty for a one-color run, which takes
    /// its color when it is drawn.
    color_overrides: Vec<ColorSpanKey>,
}

impl RunShapeKey {
    fn new(run: &RenderRun) -> Self {
        let color_overrides = if run.has_color_overrides() {
            run.color_spans()
                .iter()
                .map(|span| {
                    ColorSpanKey::new(span.byte_range(), span.foreground())
                })
                .collect()
        } else {
            Vec::new()
        };

        Self {
            text: run.text().to_owned(),
            font: run.font(),
            cell_columns: run.cell_columns(),
            color_overrides,
        }
    }
}

/// One colored byte range of a run, comparable and hashable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ColorSpanKey {
    byte_range: Range<usize>,
    /// Bit patterns of the red, green, blue and alpha channels.
    color: [u32; 4],
}

impl ColorSpanKey {
    fn new(byte_range: &Range<usize>, color: Color) -> Self {
        Self {
            byte_range: byte_range.clone(),
            color: [
                color.r.to_bits(),
                color.g.to_bits(),
                color.b.to_bits(),
                color.a.to_bits(),
            ],
        }
    }
}

/// What one cached run counts for against the budget, in cells.
fn run_cost_cells(cell_columns: usize) -> usize {
    cell_columns.max(MIN_RUN_COST_CELLS)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use iced::Color;
    use iced::font::Weight as FontWeight;

    use super::*;
    use crate::render_runs::RenderTextStyle;

    const SELECTED: Color = Color::from_rgb(0.2, 0.7, 0.9);

    fn run_at(text: &str, line: i32, start_column: usize) -> RenderRun {
        RenderRun::new_empty_color_spans(
            text,
            line,
            start_column,
            text.chars().count(),
            RenderTextStyle::from_fg_and_font(Color::WHITE, Font::MONOSPACE),
        )
    }

    fn run(text: &str) -> RenderRun {
        run_at(text, 0, 0)
    }

    fn wide_run(text: &str, cell_columns: usize) -> RenderRun {
        RenderRun::new_empty_color_spans(
            text,
            0,
            0,
            cell_columns,
            RenderTextStyle::from_fg_and_font(Color::WHITE, Font::MONOSPACE),
        )
    }

    fn two_color_run(second: Color) -> RenderRun {
        RenderRun::new_with_color_spans(
            "ab",
            0,
            0,
            2,
            Font::MONOSPACE,
            vec![(0..1, 0, 1, Color::WHITE), (1..2, 1, 1, second)],
        )
    }

    fn shaped() -> ShapedRun {
        ShapedRun::new(cosmic_text::Buffer::new_empty(
            cosmic_text::Metrics::new(14.0, 16.0),
        ))
    }

    /// Run one frame over `runs` and return the shaped run of each.
    fn frame(
        cache: &mut ShapedRunCache,
        runs: &[RenderRun],
        shape_count: &Cell<usize>,
    ) -> Vec<ShapedRun> {
        cache.start_frame();
        let shaped_runs = runs
            .iter()
            .map(|run| {
                cache.get_or_shape(run, || {
                    shape_count.set(shape_count.get() + 1);
                    shaped()
                })
            })
            .collect();
        cache.finish_frame();

        shaped_runs
    }

    fn same_buffer(left: &ShapedRun, right: &ShapedRun) -> bool {
        Arc::ptr_eq(left.buffer(), right.buffer())
    }

    #[test]
    fn unchanged_run_is_shaped_once() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);
        let runs = [run("abc")];

        let first = frame(&mut cache, &runs, &shape_count);
        let second = frame(&mut cache, &runs, &shape_count);

        assert_eq!(shape_count.get(), 1);
        assert!(same_buffer(&first[0], &second[0]));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn changing_one_run_shapes_only_that_run() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);

        let first = frame(
            &mut cache,
            &[run_at("abc", 0, 0), run_at("def", 1, 0)],
            &shape_count,
        );
        let second = frame(
            &mut cache,
            &[run_at("abc", 0, 0), run_at("xyz", 1, 0)],
            &shape_count,
        );

        assert_eq!(shape_count.get(), 3);
        assert!(same_buffer(&first[0], &second[0]));
        assert!(!same_buffer(&first[1], &second[1]));
    }

    #[test]
    fn run_at_another_position_reuses_its_buffer() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);

        let first = frame(&mut cache, &[run_at("abc", 0, 0)], &shape_count);
        let second = frame(&mut cache, &[run_at("abc", 7, 12)], &shape_count);

        assert_eq!(shape_count.get(), 1);
        assert!(same_buffer(&first[0], &second[0]));
    }

    #[test]
    fn equal_runs_in_one_frame_share_one_buffer() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);

        let shaped_runs = frame(
            &mut cache,
            &[run_at("中", 0, 0), run_at("中", 0, 2), run_at("中", 3, 4)],
            &shape_count,
        );

        assert_eq!(shape_count.get(), 1);
        assert!(same_buffer(&shaped_runs[0], &shaped_runs[1]));
        assert!(same_buffer(&shaped_runs[0], &shaped_runs[2]));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn color_change_on_a_one_color_run_does_not_reshape() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);
        let selected = RenderRun::new_empty_color_spans(
            "abc",
            0,
            0,
            3,
            RenderTextStyle::from_fg_and_font(SELECTED, Font::MONOSPACE),
        );

        let first = frame(&mut cache, &[run("abc")], &shape_count);
        let second = frame(&mut cache, &[selected], &shape_count);

        assert_eq!(shape_count.get(), 1);
        assert!(same_buffer(&first[0], &second[0]));
    }

    #[test]
    fn color_change_on_a_multi_color_run_reshapes() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);

        let first = frame(&mut cache, &[two_color_run(SELECTED)], &shape_count);
        let second =
            frame(&mut cache, &[two_color_run(Color::BLACK)], &shape_count);
        let third = frame(&mut cache, &[two_color_run(SELECTED)], &shape_count);

        assert_eq!(shape_count.get(), 2);
        assert!(!same_buffer(&first[0], &second[0]));
        assert!(same_buffer(&first[0], &third[0]));
    }

    #[test]
    fn one_color_and_multi_color_runs_do_not_share_a_buffer() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);

        let shaped_runs = frame(
            &mut cache,
            &[run("ab"), two_color_run(SELECTED)],
            &shape_count,
        );

        assert_eq!(shape_count.get(), 2);
        assert!(!same_buffer(&shaped_runs[0], &shaped_runs[1]));
    }

    #[test]
    fn font_and_cell_columns_are_part_of_the_key() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);
        let bold = Font {
            weight: FontWeight::Bold,
            ..Font::MONOSPACE
        };
        let bold_run = RenderRun::new_empty_color_spans(
            "中",
            0,
            0,
            2,
            RenderTextStyle::from_fg_and_font(Color::WHITE, bold),
        );
        let wide_run = RenderRun::new_empty_color_spans(
            "中",
            0,
            0,
            2,
            RenderTextStyle::from_fg_and_font(Color::WHITE, Font::MONOSPACE),
        );

        frame(&mut cache, &[run("中"), wide_run, bold_run], &shape_count);

        assert_eq!(shape_count.get(), 3);
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn unused_runs_stay_while_the_cache_is_within_the_budget() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);

        frame(&mut cache, &[run("abc"), run("def")], &shape_count);
        frame(&mut cache, &[run("abc")], &shape_count);
        assert_eq!(cache.len(), 2);

        frame(&mut cache, &[run("def")], &shape_count);
        assert_eq!(shape_count.get(), 2);
    }

    #[test]
    fn unused_runs_are_dropped_when_the_cache_is_over_the_budget() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);
        let narrow_runs_over_budget =
            CACHE_BUDGET_CELLS / MIN_RUN_COST_CELLS + 1;
        let many_runs = (0..narrow_runs_over_budget)
            .map(|index| run(&format!("{index}")))
            .collect::<Vec<_>>();

        // Runs the current frame draws stay, however many there are.
        frame(&mut cache, &many_runs, &shape_count);
        assert_eq!(cache.len(), narrow_runs_over_budget);

        let kept = frame(&mut cache, &many_runs[..1], &shape_count);
        assert_eq!(cache.len(), 1);

        let again = frame(&mut cache, &many_runs[..1], &shape_count);
        assert!(same_buffer(&kept[0], &again[0]));
        assert_eq!(shape_count.get(), narrow_runs_over_budget);
    }

    #[test]
    fn wide_runs_fill_the_budget_with_far_fewer_entries_than_narrow_ones() {
        let shape_count = Cell::new(0);
        let wide_columns = 200;
        let wide_runs_over_budget = CACHE_BUDGET_CELLS / wide_columns + 1;
        let wide_runs = (0..wide_runs_over_budget)
            .map(|index| wide_run(&format!("{index}"), wide_columns))
            .collect::<Vec<_>>();
        let narrow_runs = (0..wide_runs_over_budget)
            .map(|index| run(&format!("{index}")))
            .collect::<Vec<_>>();

        let mut wide_cache = ShapedRunCache::default();
        frame(&mut wide_cache, &wide_runs, &shape_count);
        frame(&mut wide_cache, &wide_runs[..1], &shape_count);

        let mut narrow_cache = ShapedRunCache::default();
        frame(&mut narrow_cache, &narrow_runs, &shape_count);
        frame(&mut narrow_cache, &narrow_runs[..1], &shape_count);

        assert!(wide_runs_over_budget < 200);
        assert_eq!(wide_cache.len(), 1);
        assert_eq!(narrow_cache.len(), wide_runs_over_budget);
    }

    #[test]
    fn dropped_runs_free_their_share_of_the_budget() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);
        let wide_columns = 200;
        let wide_runs_over_budget = CACHE_BUDGET_CELLS / wide_columns + 1;
        let wide_runs = (0..wide_runs_over_budget)
            .map(|index| wide_run(&format!("{index}"), wide_columns))
            .collect::<Vec<_>>();

        frame(&mut cache, &wide_runs, &shape_count);
        frame(&mut cache, &wide_runs[..1], &shape_count);
        assert_eq!(cache.len(), 1);

        // The budget is free again, so unused runs may stay once more.
        frame(&mut cache, &wide_runs[..3], &shape_count);
        frame(&mut cache, &wide_runs[..1], &shape_count);
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn clear_drops_every_run() {
        let mut cache = ShapedRunCache::default();
        let shape_count = Cell::new(0);

        let first = frame(&mut cache, &[run("abc")], &shape_count);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.cost_cells, 0);

        let second = frame(&mut cache, &[run("abc")], &shape_count);
        assert_eq!(shape_count.get(), 2);
        assert!(!same_buffer(&first[0], &second[0]));
    }

    #[test]
    fn shaped_run_without_layout_has_a_zero_baseline() {
        assert_eq!(shaped().baseline_y(), 0.0);
    }
}
