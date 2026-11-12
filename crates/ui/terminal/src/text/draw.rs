//! Shapes text runs with `cosmic_text` and draws them on the terminal grid.
//!
//! This is the last step of drawing text: it takes the runs built in
//! `runs`, turns each into a shaped run and hands it to the renderer at its
//! place in the grid. Shaping is the expensive part, so shaped runs are
//! remembered in `shaped_run_cache` and a run is shaped again only when its
//! own text or style changed: one changed cell does not reshape the whole
//! screen and scrolling reshapes nothing. Resizing the window reshapes
//! nothing while the number of columns stays the same; a new column count
//! changes the text of the runs, because the blanks at the end of a line are
//! part of its run. `font_coverage` tells when the search through the
//! installed fonts can be skipped.

use std::cell::RefCell;
use std::sync::Arc;

use iced::{Color, Point, Rectangle};
use iced_core::Pixels;
use iced_core::text::LineHeight;
use iced_graphics::text::{self, Renderer as TextRenderer, cosmic_text};

use crate::text::font_coverage::{
    FontCoverage, any_font_has_char, is_drawn_only_from_own_glyph,
};
use crate::text::runs::TextRun;
use crate::text::shaped_run_cache::{ShapedRun, ShapedRunCache};

/// Where and how large the text of a terminal is drawn: the place of its
/// grid and the font metrics of this draw.
pub(crate) struct TextDrawConfig {
    layout_position: Point,
    display_offset: f32,
    cell_width: f32,
    cell_height: f32,
    font_size: f32,
    font_scale_factor: f32,
}

impl TextDrawConfig {
    /// Create draw configuration from the current terminal layout metrics.
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

/// What a terminal widget keeps between draws of its text: the shaped runs
/// the renderer still points at and what is remembered to shape less.
pub(crate) struct TextDrawState {
    shaped: RefCell<ShapedText>,
}

impl TextDrawState {
    fn buffer_count(&self) -> usize {
        self.shaped.borrow().cache.len()
    }
}

impl TextDrawState {
    pub(crate) fn new() -> Self {
        Self {
            shaped: RefCell::new(ShapedText::default()),
        }
    }

    /// Shape the runs of a new frame, reusing every run shaped before.
    fn refresh(
        &self,
        runs: &[TextRun],
        config: &TextDrawConfig,
        font_version: text::Version,
        font_system: &mut cosmic_text::FontSystem,
    ) {
        let mut shaped = self.shaped.borrow_mut();
        let ShapedText {
            layout,
            cache,
            coverage,
            frame_runs,
        } = &mut *shaped;

        let new_layout = ShapeLayout::new(config, font_version);
        if *layout != Some(new_layout) {
            let fonts_changed =
                layout.is_none_or(|old| old.font_version != font_version);
            if fonts_changed {
                coverage.clear();
            }
            cache.clear();
            *layout = Some(new_layout);
        }

        cache.start_frame();
        frame_runs.clear();
        for run in runs {
            let shaped = cache.get_or_shape(run, || {
                shape_run_checking_coverage(run, config, coverage, font_system)
            });
            frame_runs.push(shaped);
        }
        cache.finish_frame();
    }
}

impl Clone for TextDrawState {
    fn clone(&self) -> Self {
        Self {
            shaped: RefCell::new(self.shaped.borrow().clone()),
        }
    }
}

impl std::fmt::Debug for TextDrawState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextDrawState")
            .field("buffer_count", &self.buffer_count())
            .finish()
    }
}

/// The shaped text of a terminal: the runs of the latest frame and what
/// is remembered between draws to shape less.
#[derive(Clone, Default)]
struct ShapedText {
    /// Layout the cached runs were shaped for.
    layout: Option<ShapeLayout>,
    cache: ShapedRunCache,
    coverage: FontCoverage,
    /// Shaped runs of the latest frame, in the order of its text runs.
    frame_runs: Vec<ShapedRun>,
}

/// Layout metrics and fonts that every shaped run depends on.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ShapeLayout {
    cell_width: u32,
    cell_height: u32,
    font_size: u32,
    font_scale_factor: u32,
    font_version: text::Version,
}

impl ShapeLayout {
    fn new(config: &TextDrawConfig, font_version: text::Version) -> Self {
        Self {
            cell_width: config.cell_width.to_bits(),
            cell_height: config.cell_height.to_bits(),
            font_size: config.font_size.to_bits(),
            font_scale_factor: config.font_scale_factor.to_bits(),
            font_version,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LineBaseline {
    line: i32,
    baseline: f32,
}

/// Shape, cache, and draw terminal text runs through the Iced text renderer.
pub(crate) fn draw_text_runs<Renderer>(
    renderer: &mut Renderer,
    runs: &[TextRun],
    config: &TextDrawConfig,
    clip_bounds: Rectangle,
    draw_state: &TextDrawState,
) where
    Renderer: TextRenderer,
{
    let mut font_system =
        text::font_system().write().expect("Write font system");
    let font_version = font_system.version();
    let font_system = font_system.raw();
    draw_state.refresh(runs, config, font_version, font_system);

    let shaped = draw_state.shaped.borrow();
    let shaped_runs = &shaped.frame_runs;
    let line_baselines = collect_line_baselines(runs, shaped_runs);

    for (run, shaped) in runs.iter().zip(shaped_runs) {
        let origin = text_run_origin(run, config);
        let line_baseline =
            baseline_for_line(&line_baselines, run.line(), shaped.baseline_y());
        let position = Point::new(
            origin.x,
            origin.y + line_baseline - shaped.baseline_y(),
        );

        renderer.fill_raw(text::Raw {
            buffer: Arc::downgrade(shaped.buffer()),
            position,
            color: run_draw_color(run),
            clip_bounds,
        });
    }
}

/// Shape one run, drawing the placeholder glyph directly when its character
/// is known to be in no font.
///
/// The full font search is kept until it has failed once for this font. By
/// then it has loaded every font anyway, so looking a character up is cheap
/// and costs no extra font loading.
fn shape_run_checking_coverage(
    run: &TextRun,
    config: &TextDrawConfig,
    coverage: &mut FontCoverage,
    font_system: &mut cosmic_text::FontSystem,
) -> ShapedRun {
    let font = run.font();
    let looked_up_char = lone_plain_glyph_char(run.text());
    let shaping = shaping_mode(looked_up_char, |ch| {
        coverage
            .is_missing(font, ch, || any_font_has_char(font_system, font, ch))
    });

    let shaped = shape_text_run(run, config, shaping, font_system);
    let search_ran_out = looked_up_char.is_some()
        && shaping == cosmic_text::Shaping::Advanced
        && draws_placeholder(&shaped);
    if search_ran_out {
        coverage.fallback_ran_out(font);
    }

    shaped
}

/// The character of a run whose font coverage can be looked up: the run is
/// that one character and the shaper has no other way to draw it.
fn lone_plain_glyph_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    let first = chars.next()?;
    let is_lone = chars.next().is_none();

    (is_lone && is_drawn_only_from_own_glyph(first)).then_some(first)
}

/// Pick how to shape a run.
///
/// Advanced shaping searches the installed fonts for missing glyphs. Basic
/// shaping uses the terminal font only, which is enough to draw the
/// placeholder glyph for a character that no font has.
fn shaping_mode(
    looked_up_char: Option<char>,
    is_missing_from_every_font: impl FnOnce(char) -> bool,
) -> cosmic_text::Shaping {
    match looked_up_char {
        Some(ch) if is_missing_from_every_font(ch) => {
            cosmic_text::Shaping::Basic
        },
        _ => cosmic_text::Shaping::Advanced,
    }
}

fn draws_placeholder(shaped: &ShapedRun) -> bool {
    shaped
        .buffer()
        .layout_runs()
        .flat_map(|layout_run| layout_run.glyphs.iter())
        .any(|glyph| glyph.glyph_id == 0)
}

fn shape_text_run(
    run: &TextRun,
    config: &TextDrawConfig,
    shaping: cosmic_text::Shaping,
    font_system: &mut cosmic_text::FontSystem,
) -> ShapedRun {
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
    set_buffer_text(font_system, &mut buffer, run, shaping, None);
    if let Some(letter_spacing) = grid_fit_letter_spacing(&buffer, run, config)
    {
        set_buffer_text(
            font_system,
            &mut buffer,
            run,
            shaping,
            Some(letter_spacing),
        );
    }

    ShapedRun::new(buffer)
}

fn text_run_origin(run: &TextRun, config: &TextDrawConfig) -> Point {
    Point::new(
        config.layout_position.x
            + (run.start_column() as f32 * config.cell_width),
        config.layout_position.y
            + ((run.line() as f32 + config.display_offset)
                * config.cell_height),
    )
}

/// Color to draw a shaped run with.
///
/// A run with several colors has them baked into its glyphs, so it is drawn
/// in white. A one-color run is shaped without a color and takes it here,
/// which lets a color change reuse the shaped buffer.
fn run_draw_color(run: &TextRun) -> Color {
    if run.has_color_overrides() {
        Color::WHITE
    } else {
        run.fallback_foreground()
    }
}

fn set_buffer_text(
    font_system: &mut cosmic_text::FontSystem,
    buffer: &mut cosmic_text::Buffer,
    run: &TextRun,
    shaping: cosmic_text::Shaping,
    letter_spacing: Option<f32>,
) {
    let default_attrs = text_attributes(run.font(), letter_spacing);

    if !run.has_color_overrides() {
        buffer.set_text(
            font_system,
            run.text(),
            &default_attrs,
            shaping,
            Some(cosmic_text::Align::Left),
        );
        return;
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
        shaping,
        Some(cosmic_text::Align::Left),
    );
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
    run: &TextRun,
    config: &TextDrawConfig,
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

fn collect_line_baselines(
    runs: &[TextRun],
    shaped_runs: &[ShapedRun],
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
    use crate::text::runs::{TextRun, TextRunStyle};

    fn run(text: &str) -> TextRun {
        TextRun::new_empty_color_spans(
            text,
            2,
            3,
            5,
            TextRunStyle::from_fg_and_font(Color::WHITE, Font::MONOSPACE),
        )
    }

    const ADVANCED: cosmic_text::Shaping = cosmic_text::Shaping::Advanced;
    const BASIC: cosmic_text::Shaping = cosmic_text::Shaping::Basic;

    /// Chars to try the placeholder paths with. Which of them are in no font
    /// depends on the machine, so the tests look that up first. The list
    /// has chars with no script (private use, unassigned), chars with a
    /// script (Han, Tangut), and one that most machines do have.
    const PROBE_CHARS: [char; 8] = [
        '\u{10FFFD}',
        '\u{DFFFD}',
        '\u{3FFFD}',
        '\u{E0B0}',
        '\u{31350}',
        '\u{2EBF0}',
        '\u{18000}',
        '中',
    ];

    fn run_on_line(text: &str, line: i32) -> TextRun {
        TextRun::new_empty_color_spans(
            text,
            line,
            0,
            text.chars().count(),
            TextRunStyle::from_fg_and_font(Color::WHITE, Font::MONOSPACE),
        )
    }

    fn lone_char_run(ch: char, cell_columns: usize) -> TextRun {
        TextRun::new_empty_color_spans(
            &ch.to_string(),
            0,
            0,
            cell_columns,
            TextRunStyle::from_fg_and_font(Color::WHITE, Font::MONOSPACE),
        )
    }

    fn config() -> TextDrawConfig {
        TextDrawConfig::new(Point::new(10.0, 20.0), 1.0, 8.0, 16.0, 14.0, 1.2)
    }

    fn config_at(position: Point) -> TextDrawConfig {
        TextDrawConfig::new(position, 1.0, 8.0, 16.0, 14.0, 1.2)
    }

    fn config_with_cell_width(cell_width: f32) -> TextDrawConfig {
        TextDrawConfig::new(
            Point::new(10.0, 20.0),
            1.0,
            cell_width,
            16.0,
            14.0,
            1.2,
        )
    }

    #[test]
    fn shape_text_run_uses_terminal_grid_constraints() {
        let run = run("ab cd");
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");

        let shaped = shape_text_run(&run, &config, ADVANCED, font_system.raw());
        let origin = text_run_origin(&run, &config);
        let first_glyph_x = shaped
            .buffer()
            .layout_runs()
            .next()
            .and_then(|layout_run| layout_run.glyphs.first())
            .map(|glyph| glyph.x);

        assert_eq!(shaped.buffer().wrap(), cosmic_text::Wrap::None);
        assert_eq!(shaped.buffer().monospace_width(), Some(8.0));
        assert_eq!(shaped.buffer().size(), (Some(40.0), Some(16.0)));
        assert_eq!(origin.x, 34.0);
        assert_eq!(first_glyph_x, Some(0.0));
        assert!(origin.y.is_finite());
    }

    #[test]
    fn same_line_runs_keep_stable_vertical_origin_when_split() {
        let plain_run = run("abc");
        let complex_run = run("ที่นี่");
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");

        let plain =
            shape_text_run(&plain_run, &config, ADVANCED, font_system.raw());
        let complex =
            shape_text_run(&complex_run, &config, ADVANCED, font_system.raw());
        let plain_origin = text_run_origin(&plain_run, &config);
        let complex_origin = text_run_origin(&complex_run, &config);

        assert_eq!(plain_origin.y, complex_origin.y);
        assert!(plain.baseline_y().is_finite());
        assert!(complex.baseline_y().is_finite());
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
            .map(|run| {
                shape_text_run(run, &config, ADVANCED, font_system.raw())
            })
            .collect::<Vec<_>>();

        let baselines = collect_line_baselines(&runs, &shaped_runs);
        let plain_baseline = baseline_for_line(&baselines, runs[0].line(), 0.0);
        let complex_baseline =
            baseline_for_line(&baselines, runs[1].line(), 0.0);

        assert_eq!(plain_baseline, complex_baseline);
    }

    #[test]
    fn shape_text_run_uses_monospace_width_for_combining_marks() {
        let text = "e\u{0301}x";
        let run = TextRun::new_empty_color_spans(
            text,
            2,
            3,
            2,
            TextRunStyle::from_fg_and_font(Color::WHITE, Font::MONOSPACE),
        );
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");
        let shaped = shape_text_run(&run, &config, ADVANCED, font_system.raw());

        assert_eq!(shaped.buffer().monospace_width(), Some(8.0));
    }

    #[test]
    fn shape_text_run_keeps_hebrew_niqqud_in_layout_text() {
        let text = "ב\u{05b0}\u{05bc}";
        let run = TextRun::new_empty_color_spans(
            text,
            2,
            3,
            1,
            TextRunStyle::from_fg_and_font(Color::WHITE, Font::MONOSPACE),
        );
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");
        let shaped = shape_text_run(&run, &config, ADVANCED, font_system.raw());
        let layout_text = shaped
            .buffer()
            .layout_runs()
            .map(|layout_run| layout_run.text)
            .collect::<Vec<_>>()
            .join("");
        let glyph_ranges = shaped
            .buffer()
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
        let plain = TextRun::new_empty_color_spans(
            "abc",
            2,
            3,
            3,
            TextRunStyle::from_fg_and_font(Color::WHITE, Font::MONOSPACE),
        );
        let colored = TextRun::new_with_color_spans(
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

        let plain =
            shape_text_run(&plain, &config, ADVANCED, font_system.raw());
        let colored =
            shape_text_run(&colored, &config, ADVANCED, font_system.raw());
        let plain_geometry = glyph_geometry(plain.buffer());
        let colored_geometry = glyph_geometry(colored.buffer());

        assert_eq!(plain_geometry, colored_geometry);
    }

    #[test]
    fn shape_text_run_fits_monospace_advances_to_terminal_grid() {
        let run = TextRun::new_empty_color_spans(
            "otty git:(chars)x",
            0,
            0,
            17,
            TextRunStyle::from_fg_and_font(Color::WHITE, Font::MONOSPACE),
        );
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");
        let shaped = shape_text_run(&run, &config, ADVANCED, font_system.raw());
        let glyphs = shaped
            .buffer()
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
    fn draw_text_runs_submits_raw_text_and_retains_buffers() {
        let runs = [run("e\u{0301} ที่นี่ ن\u{064f} ש\u{05c1}")];
        let config = config();
        let clip_bounds = Rectangle::new(Point::ORIGIN, Size::new(200.0, 80.0));
        let draw_state = TextDrawState::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_text_runs(&mut renderer, &runs, &config, clip_bounds, &draw_state);

        assert_eq!(renderer.raw_text.len(), 1);
        assert_eq!(renderer.raw_text[0].position, Point::new(34.0, 68.0));
        assert_eq!(renderer.raw_text[0].color, Color::WHITE);
        assert_eq!(renderer.raw_text[0].clip_bounds, clip_bounds);
        assert_eq!(draw_state.buffer_count(), 1);
        assert!(renderer.raw_text[0].buffer.upgrade().is_some());
    }

    #[test]
    fn draw_text_runs_uses_glyph_colors_without_clipped_overlays() {
        let selected_color = Color::from_rgb(0.2, 0.7, 0.9);
        let runs = [TextRun::new_with_color_spans(
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
        let draw_state = TextDrawState::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_text_runs(&mut renderer, &runs, &config, clip_bounds, &draw_state);

        assert_eq!(renderer.raw_text.len(), 1);
        assert_eq!(renderer.raw_text[0].clip_bounds, clip_bounds);
        assert_eq!(renderer.raw_text[0].color, Color::WHITE);
        assert_eq!(draw_state.buffer_count(), 1);

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
    fn draw_text_runs_accepts_empty_run_list() {
        let config = config();
        let clip_bounds = Rectangle::new(Point::ORIGIN, Size::new(200.0, 80.0));
        let draw_state = TextDrawState::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_text_runs(&mut renderer, &[], &config, clip_bounds, &draw_state);

        assert!(renderer.raw_text.is_empty());
        assert_eq!(draw_state.buffer_count(), 0);
    }

    #[test]
    fn draw_text_runs_reuses_shaped_buffers_when_only_position_changes() {
        let runs = [run("cached text")];
        let clip_bounds =
            Rectangle::new(Point::ORIGIN, Size::new(300.0, 120.0));
        let draw_state = TextDrawState::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_text_runs(
            &mut renderer,
            &runs,
            &config_at(Point::new(10.0, 20.0)),
            clip_bounds,
            &draw_state,
        );
        let first_buffer = renderer.raw_text[0]
            .buffer
            .upgrade()
            .expect("retained shaped buffer");

        renderer.raw_text.clear();
        draw_text_runs(
            &mut renderer,
            &runs,
            &config_at(Point::new(40.0, 60.0)),
            clip_bounds,
            &draw_state,
        );
        let second_buffer = renderer.raw_text[0]
            .buffer
            .upgrade()
            .expect("retained shaped buffer");

        assert!(Arc::ptr_eq(&first_buffer, &second_buffer));
        assert_eq!(renderer.raw_text[0].position, Point::new(64.0, 108.0));
    }

    #[test]
    fn draw_text_runs_reshapes_when_cell_width_changes() {
        let runs = [run("cached text")];
        let clip_bounds =
            Rectangle::new(Point::ORIGIN, Size::new(300.0, 120.0));
        let draw_state = TextDrawState::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_text_runs(
            &mut renderer,
            &runs,
            &config_with_cell_width(8.0),
            clip_bounds,
            &draw_state,
        );
        let first_buffer = renderer.raw_text[0]
            .buffer
            .upgrade()
            .expect("retained shaped buffer");

        draw_state
            .shaped
            .borrow_mut()
            .coverage
            .fallback_ran_out(Font::MONOSPACE);

        renderer.raw_text.clear();
        draw_text_runs(
            &mut renderer,
            &runs,
            &config_with_cell_width(9.0),
            clip_bounds,
            &draw_state,
        );
        let second_buffer = renderer.raw_text[0]
            .buffer
            .upgrade()
            .expect("retained shaped buffer");

        assert!(!Arc::ptr_eq(&first_buffer, &second_buffer));
        assert_eq!(second_buffer.monospace_width(), Some(9.0));
        // The fonts are the same, so what is known about them stays.
        assert!(draw_state.shaped.borrow_mut().coverage.is_missing(
            Font::MONOSPACE,
            '中',
            || false
        ));
    }

    /// Loading a font changes how text is shaped for the whole program, so
    /// this test runs in a copy of the test program, away from the tests that
    /// expect a buffer to stay.
    #[test]
    fn draw_text_runs_reshapes_when_the_font_system_changes() {
        const IN_THE_COPY: &str = "OTTY_TERMINAL_TEST_ALONE";

        if std::env::var_os(IN_THE_COPY).is_none() {
            let output = std::process::Command::new(
                std::env::current_exe().expect("the test program"),
            )
            .arg("draw_text_runs_reshapes_when_the_font_system_changes")
            .env(IN_THE_COPY, "1")
            .output()
            .expect("the copy should start");
            let said = String::from_utf8_lossy(&output.stdout);

            assert!(
                output.status.success() && said.contains("1 passed"),
                "{said}"
            );

            return;
        }

        let runs = [run("cached text")];
        let clip_bounds =
            Rectangle::new(Point::ORIGIN, Size::new(300.0, 120.0));
        let draw_state = TextDrawState::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_text_runs(
            &mut renderer,
            &runs,
            &config(),
            clip_bounds,
            &draw_state,
        );
        let first_buffer = renderer.raw_text[0]
            .buffer
            .upgrade()
            .expect("retained shaped buffer");
        draw_state
            .shaped
            .borrow_mut()
            .coverage
            .fallback_ran_out(Font::MONOSPACE);

        text::font_system()
            .write()
            .expect("Write font system")
            .load_font(std::borrow::Cow::Owned(Vec::new()));
        renderer.raw_text.clear();
        draw_text_runs(
            &mut renderer,
            &runs,
            &config(),
            clip_bounds,
            &draw_state,
        );
        let second_buffer = renderer.raw_text[0]
            .buffer
            .upgrade()
            .expect("retained shaped buffer");

        assert!(!Arc::ptr_eq(&first_buffer, &second_buffer));
        // New fonts may have chars the old ones lacked.
        assert!(!draw_state.shaped.borrow_mut().coverage.is_missing(
            Font::MONOSPACE,
            '中',
            || false
        ));
    }

    #[test]
    fn draw_text_runs_reshapes_only_the_changed_run() {
        let clip_bounds =
            Rectangle::new(Point::ORIGIN, Size::new(300.0, 120.0));
        let draw_state = TextDrawState::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_text_runs(
            &mut renderer,
            &[run_on_line("first", 0), run_on_line("second", 1)],
            &config(),
            clip_bounds,
            &draw_state,
        );
        let first_buffers = drawn_buffers(&renderer);

        renderer.raw_text.clear();
        draw_text_runs(
            &mut renderer,
            &[run_on_line("first", 0), run_on_line("changed", 1)],
            &config(),
            clip_bounds,
            &draw_state,
        );
        let second_buffers = drawn_buffers(&renderer);

        assert!(Arc::ptr_eq(&first_buffers[0], &second_buffers[0]));
        assert!(!Arc::ptr_eq(&first_buffers[1], &second_buffers[1]));
    }

    #[test]
    fn draw_text_runs_reuses_the_buffer_of_a_run_that_moved() {
        let clip_bounds =
            Rectangle::new(Point::ORIGIN, Size::new(300.0, 120.0));
        let draw_state = TextDrawState::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_text_runs(
            &mut renderer,
            &[run_on_line("scrolled", 4)],
            &config(),
            clip_bounds,
            &draw_state,
        );
        let first_buffers = drawn_buffers(&renderer);
        let first_position = renderer.raw_text[0].position;

        renderer.raw_text.clear();
        draw_text_runs(
            &mut renderer,
            &[run_on_line("scrolled", 1)],
            &config(),
            clip_bounds,
            &draw_state,
        );
        let second_buffers = drawn_buffers(&renderer);

        assert!(Arc::ptr_eq(&first_buffers[0], &second_buffers[0]));
        assert_eq!(
            first_position.y - renderer.raw_text[0].position.y,
            3.0 * 16.0
        );
    }

    #[test]
    fn draw_text_runs_shares_one_buffer_between_equal_runs() {
        let clip_bounds =
            Rectangle::new(Point::ORIGIN, Size::new(300.0, 120.0));
        let draw_state = TextDrawState::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_text_runs(
            &mut renderer,
            &[run_on_line("same", 0), run_on_line("same", 1)],
            &config(),
            clip_bounds,
            &draw_state,
        );
        let buffers = drawn_buffers(&renderer);

        assert_eq!(renderer.raw_text.len(), 2);
        assert!(Arc::ptr_eq(&buffers[0], &buffers[1]));
        assert_eq!(draw_state.buffer_count(), 1);
        assert_ne!(
            renderer.raw_text[0].position,
            renderer.raw_text[1].position
        );
    }

    #[test]
    fn draw_text_runs_recolors_a_one_color_run_without_reshaping() {
        let selected_color = Color::from_rgb(0.2, 0.7, 0.9);
        let selected_run = TextRun::new_empty_color_spans(
            "cached text",
            2,
            3,
            5,
            TextRunStyle::from_fg_and_font(selected_color, Font::MONOSPACE),
        );
        let clip_bounds =
            Rectangle::new(Point::ORIGIN, Size::new(300.0, 120.0));
        let draw_state = TextDrawState::new();
        let mut renderer = RecordingTextRenderer::default();

        draw_text_runs(
            &mut renderer,
            &[run("cached text")],
            &config(),
            clip_bounds,
            &draw_state,
        );
        let first_buffers = drawn_buffers(&renderer);
        assert_eq!(renderer.raw_text[0].color, Color::WHITE);

        renderer.raw_text.clear();
        draw_text_runs(
            &mut renderer,
            &[selected_run],
            &config(),
            clip_bounds,
            &draw_state,
        );
        let second_buffers = drawn_buffers(&renderer);

        assert!(Arc::ptr_eq(&first_buffers[0], &second_buffers[0]));
        assert_eq!(renderer.raw_text[0].color, selected_color);
    }

    #[test]
    fn lone_plain_glyph_char_is_a_single_char_with_no_other_way_to_draw() {
        assert_eq!(lone_plain_glyph_char("中"), Some('中'));
        assert_eq!(lone_plain_glyph_char("\u{10FFFD}"), Some('\u{10FFFD}'));

        assert_eq!(lone_plain_glyph_char(""), None);
        assert_eq!(lone_plain_glyph_char("a"), None);
        assert_eq!(lone_plain_glyph_char("é"), None);
        assert_eq!(lone_plain_glyph_char("가"), None);
        assert_eq!(lone_plain_glyph_char("中文"), None);
        assert_eq!(lone_plain_glyph_char("😀\u{FE0F}"), None);
    }

    #[test]
    fn shaping_mode_skips_the_font_search_for_a_char_no_font_has() {
        let shaping = shaping_mode(Some('\u{10FFFD}'), |ch| {
            assert_eq!(ch, '\u{10FFFD}');
            true
        });

        assert_eq!(shaping, BASIC);
    }

    #[test]
    fn shaping_mode_keeps_the_font_search_for_a_char_some_font_has() {
        assert_eq!(shaping_mode(Some('中'), |_| false), ADVANCED);
    }

    #[test]
    fn shaping_mode_keeps_the_font_search_for_any_other_text() {
        let shaping = shaping_mode(None, |_| {
            panic!("only a lone plain glyph char is looked up")
        });

        assert_eq!(shaping, ADVANCED);
    }

    #[test]
    fn basic_shaping_draws_the_same_placeholder_as_the_full_font_search() {
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");
        let font_system = font_system.raw();

        // One column is the usual case (an icon with no icon font) and is
        // the one that also goes through the grid fitting; two columns is a
        // wide char.
        for (ch, cell_columns) in
            PROBE_CHARS.into_iter().flat_map(|ch| [(ch, 1), (ch, 2)])
        {
            let run = lone_char_run(ch, cell_columns);
            let some_font_has_it =
                any_font_has_char(font_system, Font::MONOSPACE, ch);
            let searched = shape_text_run(&run, &config, ADVANCED, font_system);

            // Whatever fonts this machine has: a char the full search gives
            // up on must also be in no font for the cheap lookup.
            let search_gave_up = draws_placeholder(&searched);
            if search_gave_up {
                assert!(!some_font_has_it, "{ch:?}");
            }

            if search_gave_up && !some_font_has_it {
                let direct = shape_text_run(&run, &config, BASIC, font_system);

                assert!(draws_placeholder(&direct), "{ch:?}");
                assert_eq!(
                    glyph_identity(direct.buffer()),
                    glyph_identity(searched.buffer()),
                    "{ch:?} in {cell_columns} columns"
                );
                assert_eq!(
                    glyph_geometry(direct.buffer()),
                    glyph_geometry(searched.buffer()),
                    "{ch:?} in {cell_columns} columns"
                );
                assert_eq!(
                    direct.baseline_y(),
                    searched.baseline_y(),
                    "{ch:?} in {cell_columns} columns"
                );
            }
        }
    }

    #[test]
    fn font_search_is_skipped_only_after_the_shaper_ran_out_of_fonts() {
        let config = config();
        let mut font_system =
            text::font_system().write().expect("Write font system");
        let font_system = font_system.raw();
        let mut coverage = FontCoverage::default();
        let missing_chars = PROBE_CHARS
            .into_iter()
            .filter(|ch| {
                let searched = shape_text_run(
                    &lone_char_run(*ch, 2),
                    &config,
                    ADVANCED,
                    font_system,
                );

                draws_placeholder(&searched)
            })
            .collect::<Vec<_>>();

        // Text some font has never turns the lookups on.
        shape_run_checking_coverage(
            &run("plain text"),
            &config,
            &mut coverage,
            font_system,
        );
        assert!(!coverage.is_missing(Font::MONOSPACE, '中', || false));

        for ch in &missing_chars {
            let shaped = shape_run_checking_coverage(
                &lone_char_run(*ch, 2),
                &config,
                &mut coverage,
                font_system,
            );

            assert!(draws_placeholder(&shaped));
        }

        // Once a char ended as the placeholder, later chars are looked up.
        let lookups_are_on =
            coverage.is_missing(Font::MONOSPACE, '中', || false);
        assert_eq!(lookups_are_on, !missing_chars.is_empty());
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

    fn drawn_buffers(
        renderer: &RecordingTextRenderer,
    ) -> Vec<Arc<cosmic_text::Buffer>> {
        renderer
            .raw_text
            .iter()
            .map(|raw| raw.buffer.upgrade().expect("retained shaped buffer"))
            .collect()
    }

    fn glyph_identity(
        buffer: &cosmic_text::Buffer,
    ) -> Vec<(cosmic_text::fontdb::ID, u16)> {
        buffer
            .layout_runs()
            .flat_map(|layout_run| {
                layout_run
                    .glyphs
                    .iter()
                    .map(|glyph| (glyph.font_id, glyph.glyph_id))
            })
            .collect()
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
