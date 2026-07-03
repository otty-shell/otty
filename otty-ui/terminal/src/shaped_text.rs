use std::cell::RefCell;
use std::sync::Arc;

use iced::{Color, Point, Rectangle};
use iced_core::Pixels;
use iced_core::text::LineHeight;
use iced_graphics::text::{self, Renderer as TextRenderer, cosmic_text};

use crate::render_runs::RenderRun;

pub(crate) struct TextRunDrawConfig {
    layout_position: Point,
    display_offset: f32,
    cell_width: f32,
    cell_height: f32,
    font_size: f32,
    font_scale_factor: f32,
}

impl TextRunDrawConfig {
    pub(crate) fn new(
        layout_position: Point,
        display_offset: f32,
        cell_width: f32,
        cell_height: f32,
        font_size: f32,
        font_scale_factor: f32,
    ) -> Self {
        Self {
            layout_position,
            display_offset,
            cell_width,
            cell_height,
            font_size,
            font_scale_factor,
        }
    }
}

/// Keeps raw shaped text buffers alive until the renderer consumes them.
pub(crate) struct TextRunBufferStore {
    buffers: RefCell<Vec<Arc<cosmic_text::Buffer>>>,
}

impl TextRunBufferStore {
    pub(crate) fn new() -> Self {
        Self {
            buffers: RefCell::new(Vec::new()),
        }
    }

    fn replace(&self, buffers: Vec<Arc<cosmic_text::Buffer>>) {
        self.buffers.replace(buffers);
    }

    fn buffer_count(&self) -> usize {
        self.buffers.borrow().len()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.buffer_count()
    }
}

impl Clone for TextRunBufferStore {
    fn clone(&self) -> Self {
        Self {
            buffers: RefCell::new(self.buffers.borrow().clone()),
        }
    }
}

impl std::fmt::Debug for TextRunBufferStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextRunBufferStore")
            .field("buffer_count", &self.buffer_count())
            .finish()
    }
}

struct ShapedRenderRun {
    buffer: Arc<cosmic_text::Buffer>,
    origin: Point,
    color: Color,
}

pub(crate) fn draw_render_runs<Renderer>(
    renderer: &mut Renderer,
    runs: &[RenderRun],
    config: &TextRunDrawConfig,
    clip_bounds: Rectangle,
    retained_buffers: &TextRunBufferStore,
) where
    Renderer: TextRenderer,
{
    let mut font_system =
        text::font_system().write().expect("Write font system");
    let font_system = font_system.raw();

    let shaped_runs = runs
        .iter()
        .map(|run| shape_render_run(run, config, font_system))
        .collect::<Vec<_>>();
    let line_baselines = collect_line_baselines(runs, &shaped_runs);

    for (run, shaped) in runs.iter().zip(&shaped_runs) {
        let line_baseline =
            baseline_for_line(&line_baselines, run.line(), shaped.baseline_y());
        let position = Point::new(
            shaped.origin.x,
            shaped.origin.y + line_baseline - shaped.baseline_y(),
        );

        draw_shaped_run(renderer, shaped, position, clip_bounds);
    }

    retained_buffers.replace(
        shaped_runs
            .into_iter()
            .map(|shaped| shaped.buffer)
            .collect(),
    );
}

fn shape_render_run(
    run: &RenderRun,
    config: &TextRunDrawConfig,
    font_system: &mut cosmic_text::FontSystem,
) -> ShapedRenderRun {
    let size = Pixels(config.font_size);
    let line_height: f32 = LineHeight::Relative(config.font_scale_factor)
        .to_absolute(size)
        .into();
    let metrics = cosmic_text::Metrics::new(config.font_size, line_height);
    let run_width = run.cell_columns() as f32 * config.cell_width;

    let mut buffer = cosmic_text::Buffer::new(font_system, metrics);
    buffer.set_size(font_system, Some(run_width), Some(config.cell_height));
    buffer.set_wrap(font_system, cosmic_text::Wrap::None);
    buffer.set_monospace_width(font_system, Some(config.cell_width));
    let mut raw_color = set_buffer_text(font_system, &mut buffer, run, None);
    if let Some(letter_spacing) = grid_fit_letter_spacing(&buffer, run, config)
    {
        raw_color = set_buffer_text(
            font_system,
            &mut buffer,
            run,
            Some(letter_spacing),
        );
    }

    let origin = Point::new(
        config.layout_position.x
            + (run.start_column() as f32 * config.cell_width),
        config.layout_position.y
            + ((run.line() as f32 + config.display_offset)
                * config.cell_height),
    );

    ShapedRenderRun {
        buffer: Arc::new(buffer),
        origin,
        color: raw_color,
    }
}

fn draw_shaped_run<Renderer>(
    renderer: &mut Renderer,
    shaped: &ShapedRenderRun,
    position: Point,
    clip_bounds: Rectangle,
) where
    Renderer: TextRenderer,
{
    fill_raw_run(renderer, shaped, position, shaped.color, clip_bounds);
}

fn fill_raw_run<Renderer>(
    renderer: &mut Renderer,
    shaped: &ShapedRenderRun,
    position: Point,
    color: Color,
    clip_bounds: Rectangle,
) where
    Renderer: TextRenderer,
{
    renderer.fill_raw(text::Raw {
        buffer: Arc::downgrade(&shaped.buffer),
        position,
        color,
        clip_bounds,
    });
}

fn set_buffer_text(
    font_system: &mut cosmic_text::FontSystem,
    buffer: &mut cosmic_text::Buffer,
    run: &RenderRun,
    letter_spacing: Option<f32>,
) -> Color {
    let default_attrs = text_attributes(run.font(), letter_spacing);
    let fallback_foreground = run.fallback_foreground();
    let has_color_overrides = run
        .color_spans()
        .iter()
        .any(|span| span.foreground() != fallback_foreground);

    if !has_color_overrides {
        buffer.set_text(
            font_system,
            run.text(),
            &default_attrs,
            cosmic_text::Shaping::Advanced,
            Some(cosmic_text::Align::Left),
        );
        return fallback_foreground;
    }

    buffer.set_rich_text(
        font_system,
        run.color_spans().iter().map(|span| {
            let attrs = default_attrs
                .clone()
                .color(text::to_color(span.foreground()));

            (&run.text()[span.byte_range().clone()], attrs)
        }),
        &default_attrs,
        cosmic_text::Shaping::Advanced,
        Some(cosmic_text::Align::Left),
    );
    Color::WHITE
}

fn text_attributes(
    font: iced::Font,
    letter_spacing: Option<f32>,
) -> cosmic_text::Attrs<'static> {
    let attrs = text::to_attributes(font);

    if let Some(letter_spacing) = letter_spacing {
        attrs.letter_spacing(letter_spacing)
    } else {
        attrs
    }
}

fn grid_fit_letter_spacing(
    buffer: &cosmic_text::Buffer,
    run: &RenderRun,
    config: &TextRunDrawConfig,
) -> Option<f32> {
    if run.text().chars().count() != run.cell_columns() {
        return None;
    }

    if config.font_size <= 0.0 {
        return None;
    }

    let glyph_count = buffer
        .layout_runs()
        .map(|layout_run| layout_run.glyphs.len())
        .sum::<usize>();
    if glyph_count == 0 {
        return None;
    }

    let actual_width = buffer
        .layout_runs()
        .map(|layout_run| layout_run.line_w)
        .fold(0.0_f32, f32::max);
    let target_width = run.cell_columns() as f32 * config.cell_width;
    let width_delta = target_width - actual_width;
    if width_delta.abs() < 0.01 {
        return None;
    }

    Some(width_delta / glyph_count as f32 / config.font_size)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LineBaseline {
    line: i32,
    baseline: f32,
}

impl ShapedRenderRun {
    fn baseline_y(&self) -> f32 {
        self.buffer
            .layout_runs()
            .next()
            .map_or(0.0, |layout_run| layout_run.line_y)
    }
}

fn collect_line_baselines(
    runs: &[RenderRun],
    shaped_runs: &[ShapedRenderRun],
) -> Vec<LineBaseline> {
    let mut baselines: Vec<LineBaseline> = Vec::new();

    for (run, shaped) in runs.iter().zip(shaped_runs) {
        let baseline = shaped.baseline_y();
        if let Some(existing) =
            baselines.iter_mut().find(|entry| entry.line == run.line())
        {
            existing.baseline = existing.baseline.max(baseline);
        } else {
            baselines.push(LineBaseline {
                line: run.line(),
                baseline,
            });
        }
    }

    baselines
}

fn baseline_for_line(
    baselines: &[LineBaseline],
    line: i32,
    fallback: f32,
) -> f32 {
    baselines
        .iter()
        .find(|entry| entry.line == line)
        .map_or(fallback, |entry| entry.baseline)
}

#[cfg(test)]
mod tests {
    use iced::{Font, Size};

    use super::*;
    use crate::render_runs::{RenderRun, RenderTextStyle};

    fn run(text: &str) -> RenderRun {
        RenderRun::for_test(
            text,
            2,
            3,
            5,
            RenderTextStyle::for_test(Color::WHITE, Font::MONOSPACE),
        )
    }

    fn config() -> TextRunDrawConfig {
        TextRunDrawConfig::new(
            Point::new(10.0, 20.0),
            1.0,
            8.0,
            16.0,
            14.0,
            1.2,
        )
    }

    #[test]
    fn shape_render_run_uses_terminal_grid_constraints() {
        let run = run("ab cd");
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");

        let shaped = shape_render_run(&run, &config, font_system.raw());
        let first_glyph_x = shaped
            .buffer
            .layout_runs()
            .next()
            .and_then(|layout_run| layout_run.glyphs.first())
            .map(|glyph| glyph.x);

        assert_eq!(shaped.buffer.wrap(), cosmic_text::Wrap::None);
        assert_eq!(shaped.buffer.monospace_width(), Some(8.0));
        assert_eq!(shaped.buffer.size(), (Some(40.0), Some(16.0)));
        assert_eq!(shaped.origin.x, 34.0);
        assert_eq!(first_glyph_x, Some(0.0));
        assert!(shaped.origin.y.is_finite());
    }

    #[test]
    fn same_line_runs_keep_stable_vertical_origin_when_split() {
        let plain_run = run("abc");
        let complex_run = run("ที่นี่");
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");

        let plain = shape_render_run(&plain_run, &config, font_system.raw());
        let complex =
            shape_render_run(&complex_run, &config, font_system.raw());

        assert_eq!(plain.origin.y, complex.origin.y);
    }

    #[test]
    fn same_line_runs_share_one_draw_baseline_when_split() {
        let plain_run = run("abc");
        let complex_run = run("ที่นี่");
        let runs = [plain_run, complex_run];
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");
        let shaped_runs = runs
            .iter()
            .map(|run| shape_render_run(run, &config, font_system.raw()))
            .collect::<Vec<_>>();

        let baselines = collect_line_baselines(&runs, &shaped_runs);
        let plain_baseline = baseline_for_line(&baselines, runs[0].line(), 0.0);
        let complex_baseline =
            baseline_for_line(&baselines, runs[1].line(), 0.0);

        assert_eq!(plain_baseline, complex_baseline);
    }

    #[test]
    fn shape_render_run_uses_monospace_width_for_combining_marks() {
        let text = "e\u{0301}x";
        let run = RenderRun::for_test(
            text,
            2,
            3,
            2,
            RenderTextStyle::for_test(Color::WHITE, Font::MONOSPACE),
        );
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");
        let shaped = shape_render_run(&run, &config, font_system.raw());

        assert_eq!(shaped.buffer.monospace_width(), Some(8.0));
    }

    #[test]
    fn shape_render_run_keeps_hebrew_niqqud_in_layout_text() {
        let text = "ב\u{05b0}\u{05bc}";
        let run = RenderRun::for_test(
            text,
            2,
            3,
            1,
            RenderTextStyle::for_test(Color::WHITE, Font::MONOSPACE),
        );
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");
        let shaped = shape_render_run(&run, &config, font_system.raw());
        let layout_text = shaped
            .buffer
            .layout_runs()
            .map(|layout_run| layout_run.text)
            .collect::<Vec<_>>()
            .join("");
        let glyph_ranges = shaped
            .buffer
            .layout_runs()
            .flat_map(|layout_run| {
                layout_run.glyphs.iter().map(|glyph| glyph.start..glyph.end)
            })
            .collect::<Vec<_>>();
        let covered_end = glyph_ranges.iter().map(|range| range.end).max();

        assert_eq!(layout_text, text);
        assert_eq!(covered_end, Some(text.len()));
    }

    #[test]
    fn color_spans_keep_glyph_geometry_stable() {
        let selected_color = Color::from_rgb(0.2, 0.7, 0.9);
        let plain = RenderRun::for_test(
            "abc",
            2,
            3,
            3,
            RenderTextStyle::for_test(Color::WHITE, Font::MONOSPACE),
        );
        let colored = RenderRun::for_test_with_color_spans(
            "abc",
            2,
            3,
            3,
            Font::MONOSPACE,
            vec![
                (0..1, 0, 1, Color::WHITE),
                (1..2, 1, 1, selected_color),
                (2..3, 2, 1, Color::WHITE),
            ],
        );
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");

        let plain = shape_render_run(&plain, &config, font_system.raw());
        let colored = shape_render_run(&colored, &config, font_system.raw());
        let plain_geometry = glyph_geometry(&plain.buffer);
        let colored_geometry = glyph_geometry(&colored.buffer);

        assert_eq!(plain_geometry, colored_geometry);
    }

    #[test]
    fn shape_render_run_fits_monospace_advances_to_terminal_grid() {
        let run = RenderRun::for_test(
            "otty git:(chars)x",
            0,
            0,
            17,
            RenderTextStyle::for_test(Color::WHITE, Font::MONOSPACE),
        );
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");
        let shaped = shape_render_run(&run, &config, font_system.raw());
        let glyphs = shaped
            .buffer
            .layout_runs()
            .flat_map(|layout_run| {
                layout_run.glyphs.iter().map(|glyph| (glyph.x, glyph.w))
            })
            .collect::<Vec<_>>();

        assert_eq!(glyphs.len(), run.cell_columns());
        for (index, (x, width)) in glyphs.into_iter().enumerate() {
            let expected_x = index as f32 * config.cell_width;
            assert!((x - expected_x).abs() < 0.01);
            assert!((width - config.cell_width).abs() < 0.01);
        }
    }

    #[test]
    fn draw_render_runs_submits_raw_text_and_retains_buffers() {
        let runs = [run("e\u{0301} ที่นี่ ن\u{064f} ש\u{05c1}")];
        let config = config();
        let clip_bounds = Rectangle::new(Point::ORIGIN, Size::new(200.0, 80.0));
        let retained_buffers = TextRunBufferStore::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_render_runs(
            &mut renderer,
            &runs,
            &config,
            clip_bounds,
            &retained_buffers,
        );

        assert_eq!(renderer.raw_text.len(), 1);
        assert_eq!(renderer.raw_text[0].position, Point::new(34.0, 68.0));
        assert_eq!(renderer.raw_text[0].color, Color::WHITE);
        assert_eq!(renderer.raw_text[0].clip_bounds, clip_bounds);
        assert_eq!(retained_buffers.len(), 1);
        assert!(renderer.raw_text[0].buffer.upgrade().is_some());
    }

    #[test]
    fn draw_render_runs_uses_glyph_colors_without_clipped_overlays() {
        let selected_color = Color::from_rgb(0.2, 0.7, 0.9);
        let runs = [RenderRun::for_test_with_color_spans(
            "abc",
            2,
            3,
            3,
            Font::MONOSPACE,
            vec![
                (0..1, 0, 1, Color::WHITE),
                (1..2, 1, 1, selected_color),
                (2..3, 2, 1, Color::WHITE),
            ],
        )];
        let config = config();
        let clip_bounds = Rectangle::new(Point::ORIGIN, Size::new(200.0, 80.0));
        let retained_buffers = TextRunBufferStore::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_render_runs(
            &mut renderer,
            &runs,
            &config,
            clip_bounds,
            &retained_buffers,
        );

        assert_eq!(renderer.raw_text.len(), 1);
        assert_eq!(renderer.raw_text[0].clip_bounds, clip_bounds);
        assert_eq!(renderer.raw_text[0].color, Color::WHITE);
        assert_eq!(retained_buffers.len(), 1);

        let buffer = renderer.raw_text[0]
            .buffer
            .upgrade()
            .expect("Retained shaped buffer");
        let glyph_colors = buffer
            .layout_runs()
            .flat_map(|layout_run| {
                layout_run
                    .glyphs
                    .iter()
                    .map(|glyph| (glyph.start..glyph.end, glyph.color_opt))
            })
            .collect::<Vec<_>>();

        assert_eq!(glyph_colors.len(), 3);
        assert_eq!(glyph_colors[0], (0..1, Some(text::to_color(Color::WHITE))));
        assert_eq!(
            glyph_colors[1],
            (1..2, Some(text::to_color(selected_color)))
        );
        assert_eq!(glyph_colors[2], (2..3, Some(text::to_color(Color::WHITE))));
    }

    #[test]
    fn draw_render_runs_accepts_empty_run_list() {
        let config = config();
        let clip_bounds = Rectangle::new(Point::ORIGIN, Size::new(200.0, 80.0));
        let retained_buffers = TextRunBufferStore::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_render_runs(
            &mut renderer,
            &[],
            &config,
            clip_bounds,
            &retained_buffers,
        );

        assert!(renderer.raw_text.is_empty());
        assert_eq!(retained_buffers.len(), 0);
    }

    #[derive(Default)]
    struct RecordingTextRenderer {
        raw_text: Vec<text::Raw>,
    }

    impl text::Renderer for RecordingTextRenderer {
        fn fill_raw(&mut self, raw: text::Raw) {
            self.raw_text.push(raw);
        }
    }

    fn glyph_geometry(
        buffer: &cosmic_text::Buffer,
    ) -> Vec<(usize, usize, u32, u32)> {
        buffer
            .layout_runs()
            .flat_map(|layout_run| {
                layout_run.glyphs.iter().map(|glyph| {
                    (
                        glyph.start,
                        glyph.end,
                        glyph.x.to_bits(),
                        glyph.w.to_bits(),
                    )
                })
            })
            .collect()
    }
}
