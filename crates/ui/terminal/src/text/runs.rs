//! Builds text runs from the cells of a terminal snapshot.
//!
//! This is the first step of drawing text. The renderer needs larger text
//! runs for correct shaping of complex scripts, but terminal cells still
//! define grid columns, colors, selection, cursor text, and hyperlink state.
//! This module groups contiguous cells that can be shaped together while
//! preserving per-cell foreground spans for drawing. The runs go to `draw`,
//! which shapes and draws them.

use std::ops::Range;

use iced::font::{Style as FontStyle, Weight as FontWeight};
use iced::{Color, Font};
use otty_libterm::surface::{
    Flags, Point as TerminalPoint, SelectionRange, SnapshotCell, SnapshotView,
};

use crate::theme::Theme;

/// Terminal text that is shaped as one piece, with its grid position and
/// foreground spans.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct TextRun {
    text: String,
    line: i32,
    start_column: usize,
    cell_columns: usize,
    font: Font,
    color_spans: Vec<TextRunColorSpan>,
}

impl TextRun {
    pub(super) fn text(&self) -> &str {
        &self.text
    }

    pub(super) fn line(&self) -> i32 {
        self.line
    }

    pub(super) fn start_column(&self) -> usize {
        self.start_column
    }

    pub(super) fn cell_columns(&self) -> usize {
        self.cell_columns
    }

    pub(super) fn font(&self) -> Font {
        self.font
    }

    pub(super) fn color_spans(&self) -> &[TextRunColorSpan] {
        &self.color_spans
    }

    pub(super) fn fallback_foreground(&self) -> Color {
        self.color_spans
            .first()
            .map_or(Color::TRANSPARENT, TextRunColorSpan::foreground)
    }

    /// Whether the run has more than one foreground color.
    ///
    /// Such a run needs its colors baked into the shaped glyphs; a one-color
    /// run can take its color when it is drawn.
    pub(super) fn has_color_overrides(&self) -> bool {
        let fallback_foreground = self.fallback_foreground();

        self.color_spans
            .iter()
            .any(|span| span.foreground() != fallback_foreground)
    }
}

impl TextRun {
    fn new(line: i32, start_column: usize, font: Font) -> Self {
        Self {
            text: String::new(),
            line,
            start_column,
            font,
            ..Default::default()
        }
    }

    fn append_cell(
        &mut self,
        indexed: &SnapshotCell,
        cell_columns: usize,
    ) -> (Range<usize>, usize) {
        let byte_start = self.text.len();
        let span_start_column = self.cell_columns;
        self.text.push(indexed.cell.c);

        if let Some(zerowidth) = indexed.cell.zerowidth() {
            self.text.extend(zerowidth.iter());
        }

        self.cell_columns += cell_columns;
        (byte_start..self.text.len(), span_start_column)
    }

    fn append_color_span(
        &mut self,
        byte_range: Range<usize>,
        start_column: usize,
        cell_columns: usize,
        foreground: Color,
    ) {
        if byte_range.is_empty() {
            return;
        };

        if let Some(span) = self.color_spans.last_mut()
            && span.foreground == foreground
            && span.byte_range.end == byte_range.start
            && span.start_column + span.cell_columns == start_column
        {
            span.byte_range.end = byte_range.end;
            span.cell_columns += cell_columns;
            return;
        }

        self.color_spans.push(TextRunColorSpan {
            byte_range,
            start_column,
            cell_columns,
            foreground,
        });
    }

    fn end_column(&self) -> usize {
        self.start_column + self.cell_columns
    }
}

#[cfg(test)]
impl TextRun {
    pub(super) fn new_empty_color_spans(
        text: &str,
        line: i32,
        start_column: usize,
        cell_columns: usize,
        style: TextRunStyle,
    ) -> Self {
        Self {
            text: text.to_string(),
            line,
            start_column,
            cell_columns,
            font: style.font,
            color_spans: (!text.is_empty())
                .then_some(TextRunColorSpan {
                    byte_range: 0..text.len(),
                    start_column: 0,
                    cell_columns,
                    foreground: style.foreground,
                })
                .into_iter()
                .collect(),
        }
    }

    pub(super) fn new_with_color_spans(
        text: &str,
        line: i32,
        start_column: usize,
        cell_columns: usize,
        font: Font,
        color_spans: Vec<(Range<usize>, usize, usize, Color)>,
    ) -> Self {
        Self {
            text: text.to_string(),
            line,
            start_column,
            cell_columns,
            font,
            color_spans: color_spans
                .into_iter()
                .map(|(byte_range, start_column, cell_columns, foreground)| {
                    TextRunColorSpan {
                        byte_range,
                        start_column,
                        cell_columns,
                        foreground,
                    }
                })
                .collect(),
        }
    }
}

/// Foreground color range within a [`TextRun`].
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TextRunColorSpan {
    byte_range: Range<usize>,
    start_column: usize,
    cell_columns: usize,
    foreground: Color,
}

impl TextRunColorSpan {
    pub(super) fn byte_range(&self) -> &Range<usize> {
        &self.byte_range
    }

    pub(super) fn foreground(&self) -> Color {
        self.foreground
    }
}

/// Resolved text style for one terminal cell during run construction.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct TextRunStyle {
    foreground: Color,
    font: Font,
    selected: bool,
    inverse: bool,
    cursor_override: bool,
    hovered_hyperlink: bool,
}

#[cfg(test)]
impl TextRunStyle {
    pub(super) fn from_fg_and_font(foreground: Color, font: Font) -> Self {
        Self {
            foreground,
            font,
            ..Default::default()
        }
    }
}

struct TextRunBuildContext<'a> {
    selection: Option<&'a SelectionRange>,
    cursor_point: TerminalPoint,
    theme: &'a Theme,
    base_font: Font,
    hovered_span_id: Option<u32>,
    cursor_text_override: bool,
}

/// Build the text runs of the current terminal snapshot view.
pub(crate) fn build_text_runs(
    view: &SnapshotView<'_>,
    theme: &Theme,
    base_font: Font,
    hovered_span_id: Option<u32>,
    cursor_text_override: bool,
) -> Vec<TextRun> {
    let context = TextRunBuildContext {
        selection: view.selection,
        cursor_point: view.cursor.point,
        theme,
        base_font,
        hovered_span_id,
        cursor_text_override,
    };

    build_text_runs_from_cells(view.cells, &context, |point| {
        view.hyperlink_span_id_at(point)
    })
}

fn build_text_runs_from_cells<F>(
    cells: &[SnapshotCell],
    context: &TextRunBuildContext<'_>,
    span_id_at: F,
) -> Vec<TextRun>
where
    F: Fn(TerminalPoint) -> Option<u32>,
{
    let mut runs = Vec::new();
    let mut current: Option<TextRun> = None;

    for indexed in cells {
        let flags = indexed.cell.flags;
        if flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }
        if indexed.cell.c == '\t' {
            flush_current_run(&mut runs, &mut current);
            continue;
        }

        let point = indexed.point;
        let line = point.line.0;
        let column = point.column.0;
        let cell_columns = if flags.contains(Flags::WIDE_CHAR) {
            2
        } else {
            1
        };
        let style = resolve_text_style(indexed, context, &span_id_at);
        let requires_isolated_run = requires_grid_isolated_run(indexed);
        let can_extend = current.as_ref().is_some_and(|run| {
            !requires_isolated_run
                && run.line == line
                && run.end_column() == column
                && run.font == style.font
        });

        if !can_extend {
            flush_current_run(&mut runs, &mut current);
        }

        let is_renderable = is_renderable_text_cell(indexed);
        if !is_renderable && current.is_none() {
            continue;
        }

        let run = current
            .get_or_insert_with(|| TextRun::new(line, column, style.font));
        let (byte_range, span_start_column) =
            run.append_cell(indexed, cell_columns);
        run.append_color_span(
            byte_range,
            span_start_column,
            cell_columns,
            style.foreground,
        );

        if requires_isolated_run {
            flush_current_run(&mut runs, &mut current);
        }
    }

    flush_current_run(&mut runs, &mut current);
    runs
}

fn flush_current_run(runs: &mut Vec<TextRun>, current: &mut Option<TextRun>) {
    let Some(run) = current.take() else {
        return;
    };

    if !run.text.is_empty() {
        runs.push(run);
    }
}

fn is_renderable_text_cell(indexed: &SnapshotCell) -> bool {
    indexed.cell.c != ' '
        || indexed
            .cell
            .zerowidth()
            .is_some_and(|zerowidth| !zerowidth.is_empty())
}

fn requires_grid_isolated_run(indexed: &SnapshotCell) -> bool {
    indexed.cell.c.len_utf8() > 1
        || indexed
            .cell
            .zerowidth()
            .is_some_and(|zerowidth| !zerowidth.is_empty())
}

fn resolve_text_style<F>(
    indexed: &SnapshotCell,
    context: &TextRunBuildContext<'_>,
    span_id_at: &F,
) -> TextRunStyle
where
    F: Fn(TerminalPoint) -> Option<u32>,
{
    let flags = indexed.cell.flags;
    let is_inverse = flags.contains(Flags::INVERSE);
    let is_dim = flags.intersects(Flags::DIM | Flags::DIM_BOLD);
    let selected = context
        .selection
        .is_some_and(|range| range.contains(indexed.point));
    let hovered_hyperlink = context
        .hovered_span_id
        .is_some_and(|target| span_id_at(indexed.point) == Some(target));
    let cursor_override =
        context.cursor_text_override && indexed.point == context.cursor_point;

    let mut foreground = context.theme.get_color(indexed.cell.fg);
    let mut background = context.theme.get_color(indexed.cell.bg);
    if is_dim {
        foreground.a *= 0.7;
    }
    if is_inverse || selected {
        std::mem::swap(&mut foreground, &mut background);
    }
    if cursor_override {
        foreground = background;
    }

    let mut font = context.base_font;
    if flags.intersects(Flags::BOLD | Flags::DIM_BOLD) {
        font.weight = FontWeight::Bold;
    }
    if flags.contains(Flags::ITALIC) {
        font.style = FontStyle::Italic;
    }

    TextRunStyle {
        foreground,
        font,
        selected,
        inverse: is_inverse,
        cursor_override,
        hovered_hyperlink,
    }
}

#[cfg(test)]
mod tests {
    use iced::font::{Style as FontStyle, Weight as FontWeight};
    use otty_libterm::escape::{Color as AnsiColor, StdColor};
    use otty_libterm::surface::{Cell, Column, Line};

    use super::*;

    fn cell(line: i32, column: usize, c: char) -> SnapshotCell {
        SnapshotCell {
            point: TerminalPoint::new(Line(line), Column(column)),
            cell: Cell {
                c,
                ..Cell::default()
            },
        }
    }

    fn cells_from_text(line: i32, text: &str) -> Vec<SnapshotCell> {
        text.chars()
            .enumerate()
            .map(|(column, c)| cell(line, column, c))
            .collect()
    }

    fn cells_from_clusters(
        line: i32,
        clusters: &[(char, &[char])],
    ) -> Vec<SnapshotCell> {
        clusters
            .iter()
            .enumerate()
            .map(|(column, (base, marks))| {
                let mut snapshot_cell = cell(line, column, *base);
                for mark in *marks {
                    snapshot_cell.cell.push_zerowidth(*mark);
                }
                snapshot_cell
            })
            .collect()
    }

    fn build(cells: &[SnapshotCell]) -> Vec<TextRun> {
        let theme = Theme::default();
        let context = TextRunBuildContext {
            selection: None,
            cursor_point: TerminalPoint::default(),
            theme: &theme,
            base_font: Font::MONOSPACE,
            hovered_span_id: None,
            cursor_text_override: false,
        };

        build_text_runs_from_cells(cells, &context, |_| None)
    }

    #[test]
    fn preserves_combining_marks_in_cell_clusters() {
        type Cluster<'a> = (char, &'a [char]);
        type Sample<'a> = (&'a str, &'a [Cluster<'a>]);

        let samples: [Sample<'_>; 7] = [
            ("e\u{0301}", &[('e', &['\u{0301}'])]),
            (
                "ที่นี่",
                &[
                    ('ท', &['\u{0e35}', '\u{0e48}']),
                    ('น', &['\u{0e35}', '\u{0e48}']),
                ],
            ),
            ("น้ำ", &[('น', &['\u{0e49}']), ('\u{0e33}', &[])]),
            (
                "กำลัง",
                &[
                    ('ก', &[]),
                    ('\u{0e33}', &[]),
                    ('ล', &['\u{0e31}']),
                    ('ง', &[]),
                ],
            ),
            ("ນ້ຳ", &[('ນ', &['\u{0ec9}']), ('\u{0eb3}', &[])]),
            ("مُ", &[('م', &['\u{064f}'])]),
            ("שׁ", &[('ש', &['\u{05c1}'])]),
        ];

        for (expected, clusters) in samples {
            let cells = cells_from_clusters(0, clusters);
            let runs = build(&cells);
            let rendered_text =
                runs.iter().map(TextRun::text).collect::<Vec<_>>().join("");
            let cell_columns =
                runs.iter().map(TextRun::cell_columns).sum::<usize>();

            assert_eq!(rendered_text, expected);
            assert_eq!(cell_columns, clusters.len());
        }
    }

    #[test]
    fn preserves_hebrew_niqqud_in_text_runs() {
        let cells = cells_from_clusters(
            0,
            &[
                ('ב', &['\u{05b0}', '\u{05bc}']),
                ('ר', &['\u{05b5}']),
                ('א', &[]),
                ('ש', &['\u{05b4}', '\u{05c1}']),
                ('י', &[]),
                ('ת', &[]),
            ],
        );

        let runs = build(&cells);
        let rendered_text =
            runs.iter().map(TextRun::text).collect::<Vec<_>>().join("");
        let cell_columns =
            runs.iter().map(TextRun::cell_columns).sum::<usize>();

        assert_eq!(
            rendered_text,
            "ב\u{05b0}\u{05bc}ר\u{05b5}אש\u{05b4}\u{05c1}ית"
        );
        assert_eq!(cell_columns, 6);
    }

    #[test]
    fn builds_contiguous_runs_with_grid_column_accounting() {
        let cells = cells_from_text(0, "ab cd");

        let runs = build(&cells);

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text(), "ab cd");
        assert_eq!(runs[0].line(), 0);
        assert_eq!(runs[0].start_column(), 0);
        assert_eq!(runs[0].cell_columns(), 5);
    }

    #[test]
    fn skips_leading_spaces_and_keeps_start_column_aligned() {
        let cells = cells_from_text(0, "  ab");

        let runs = build(&cells);

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text(), "ab");
        assert_eq!(runs[0].start_column(), 2);
        assert_eq!(runs[0].cell_columns(), 2);
    }

    #[test]
    fn wide_char_spacer_does_not_emit_a_glyph() {
        let mut wide = cell(0, 0, 'あ');
        wide.cell.flags.insert(Flags::WIDE_CHAR);
        let mut spacer = cell(0, 1, ' ');
        spacer.cell.flags.insert(Flags::WIDE_CHAR_SPACER);
        let next = cell(0, 2, 'x');

        let runs = build(&[wide, spacer, next]);

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text(), "あ");
        assert_eq!(runs[0].start_column(), 0);
        assert_eq!(runs[0].cell_columns(), 2);
        assert_eq!(runs[1].text(), "x");
        assert_eq!(runs[1].start_column(), 2);
        assert_eq!(runs[1].cell_columns(), 1);
    }

    #[test]
    fn complex_cell_clusters_start_at_their_grid_columns() {
        let cells = cells_from_clusters(
            0,
            &[
                ('ท', &['\u{0e35}', '\u{0e48}']),
                ('น', &['\u{0e35}', '\u{0e48}']),
            ],
        );

        let runs = build(&cells);

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text(), "ท\u{0e35}\u{0e48}");
        assert_eq!(runs[0].start_column(), 0);
        assert_eq!(runs[0].cell_columns(), 1);
        assert_eq!(runs[1].text(), "น\u{0e35}\u{0e48}");
        assert_eq!(runs[1].start_column(), 1);
        assert_eq!(runs[1].cell_columns(), 1);
    }

    #[test]
    fn splits_runs_on_font_style_boundaries() {
        let mut cells = cells_from_text(0, "abc");
        cells[1].cell.flags.insert(Flags::BOLD);
        cells[2].cell.flags.insert(Flags::ITALIC);

        let runs = build(&cells);

        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].text(), "a");
        assert_eq!(runs[1].text(), "b");
        assert_eq!(runs[2].text(), "c");
        assert_eq!(runs[1].font().weight, FontWeight::Bold);
        assert_eq!(runs[2].font().style, FontStyle::Italic);
    }

    #[test]
    fn foreground_changes_create_color_spans_without_splitting_shape_run() {
        let mut cells = cells_from_text(0, "abc");
        cells[1].cell.fg = AnsiColor::Std(StdColor::Red);

        let runs = build(&cells);

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text(), "abc");
        assert_eq!(runs[0].color_spans().len(), 3);
        assert_ne!(
            runs[0].color_spans()[0].foreground(),
            runs[0].color_spans()[1].foreground()
        );
        assert_ne!(
            runs[0].color_spans()[1].foreground(),
            runs[0].color_spans()[2].foreground()
        );
    }

    #[test]
    fn selection_keeps_shape_run_boundaries_stable() {
        let cells = cells_from_text(0, "abc");
        let selection = SelectionRange::new(
            TerminalPoint::new(Line(0), Column(1)),
            TerminalPoint::new(Line(0), Column(1)),
            false,
        );
        let theme = Theme::default();
        let context = TextRunBuildContext {
            selection: Some(&selection),
            cursor_point: TerminalPoint::default(),
            theme: &theme,
            base_font: Font::MONOSPACE,
            hovered_span_id: None,
            cursor_text_override: false,
        };

        let runs = build_text_runs_from_cells(&cells, &context, |_| None);

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text(), "abc");
        assert_eq!(runs[0].start_column(), 0);
        assert_eq!(runs[0].cell_columns(), 3);
        assert_eq!(runs[0].color_spans().len(), 3);
        assert_eq!(
            runs[0].color_spans()[1].foreground(),
            theme.get_color(cells[1].cell.bg)
        );
    }

    #[test]
    fn selected_text_foreground_stays_distinct_from_selected_background() {
        let cells = cells_from_text(0, "a");
        let selection = SelectionRange::new(
            TerminalPoint::new(Line(0), Column(0)),
            TerminalPoint::new(Line(0), Column(0)),
            false,
        );
        let theme = Theme::default();
        let context = TextRunBuildContext {
            selection: Some(&selection),
            cursor_point: TerminalPoint::default(),
            theme: &theme,
            base_font: Font::MONOSPACE,
            hovered_span_id: None,
            cursor_text_override: false,
        };

        let runs = build_text_runs_from_cells(&cells, &context, |_| None);
        let selected_background = theme.get_color(cells[0].cell.fg);

        assert_eq!(runs.len(), 1);
        assert_ne!(runs[0].color_spans()[0].foreground(), selected_background);
    }

    #[test]
    fn hovered_hyperlink_does_not_split_shape_run() {
        let cells = cells_from_text(0, "abc");
        let theme = Theme::default();
        let context = TextRunBuildContext {
            selection: None,
            cursor_point: TerminalPoint::default(),
            theme: &theme,
            base_font: Font::MONOSPACE,
            hovered_span_id: Some(7),
            cursor_text_override: false,
        };

        let runs = build_text_runs_from_cells(&cells, &context, |point| {
            (point.column.0 == 1).then_some(7)
        });

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text(), "abc");
    }

    #[test]
    fn cursor_text_override_changes_color_span_without_splitting_shape_run() {
        let cells = cells_from_text(0, "abc");
        let theme = Theme::default();
        let context = TextRunBuildContext {
            selection: None,
            cursor_point: TerminalPoint::new(Line(0), Column(1)),
            theme: &theme,
            base_font: Font::MONOSPACE,
            hovered_span_id: None,
            cursor_text_override: true,
        };

        let runs = build_text_runs_from_cells(&cells, &context, |_| None);

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text(), "abc");
        assert_eq!(runs[0].color_spans().len(), 3);
        assert_eq!(
            runs[0].color_spans()[1].foreground(),
            theme.get_color(cells[1].cell.bg)
        );
    }

    #[test]
    fn run_has_color_overrides_only_with_more_than_one_color() {
        let selected = Color::from_rgb(0.2, 0.7, 0.9);
        let one_color = TextRun::new_with_color_spans(
            "ab",
            0,
            0,
            2,
            Font::MONOSPACE,
            vec![(0..1, 0, 1, selected), (1..2, 1, 1, selected)],
        );
        let two_colors = TextRun::new_with_color_spans(
            "ab",
            0,
            0,
            2,
            Font::MONOSPACE,
            vec![(0..1, 0, 1, Color::WHITE), (1..2, 1, 1, selected)],
        );

        assert!(!one_color.has_color_overrides());
        assert!(two_colors.has_color_overrides());
        assert!(!TextRun::default().has_color_overrides());
    }
}
