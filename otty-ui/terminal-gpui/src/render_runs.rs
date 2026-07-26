use std::ops::Range;

use gpui::{Font, FontStyle, FontWeight, Hsla};
use otty_libterm::surface::{
    Colors, Flags, Point, SelectionRange, SnapshotCell, SnapshotView,
};

use crate::TerminalTheme;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderRun {
    text: String,
    line: i32,
    start_column: usize,
    cell_columns: usize,
    font: Font,
    spans: Vec<RenderTextSpan>,
}

impl RenderRun {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn line(&self) -> i32 {
        self.line
    }

    pub(crate) fn start_column(&self) -> usize {
        self.start_column
    }

    pub(crate) fn cell_columns(&self) -> usize {
        self.cell_columns
    }

    pub(crate) fn font(&self) -> &Font {
        &self.font
    }

    pub(crate) fn spans(&self) -> &[RenderTextSpan] {
        &self.spans
    }

    fn new(line: i32, start_column: usize, font: Font) -> Self {
        Self {
            text: String::new(),
            line,
            start_column,
            cell_columns: 0,
            font,
            spans: Vec::new(),
        }
    }

    fn append(
        &mut self,
        indexed: &SnapshotCell,
        width: usize,
        color: Hsla,
        underline: bool,
        strikethrough: bool,
    ) {
        let byte_start = self.text.len();
        let start_column = self.cell_columns;
        self.text.push(indexed.cell.c);
        if let Some(zerowidth) = indexed.cell.zerowidth() {
            self.text.extend(zerowidth);
        }
        let byte_range = byte_start..self.text.len();
        self.cell_columns += width;

        if let Some(span) = self.spans.last_mut()
            && span.foreground == color
            && span.underline == underline
            && span.strikethrough == strikethrough
            && span.byte_range.end == byte_range.start
        {
            span.byte_range.end = byte_range.end;
            span.cell_columns += width;
            return;
        }

        self.spans.push(RenderTextSpan {
            byte_range,
            start_column,
            cell_columns: width,
            foreground: color,
            underline,
            strikethrough,
        });
    }

    fn end_column(&self) -> usize {
        self.start_column + self.cell_columns
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderTextSpan {
    byte_range: Range<usize>,
    start_column: usize,
    cell_columns: usize,
    foreground: Hsla,
    underline: bool,
    strikethrough: bool,
}

impl RenderTextSpan {
    pub(crate) fn byte_range(&self) -> &Range<usize> {
        &self.byte_range
    }

    pub(crate) fn foreground(&self) -> Hsla {
        self.foreground
    }

    pub(crate) fn is_underlined(&self) -> bool {
        self.underline
    }

    pub(crate) fn is_struck_through(&self) -> bool {
        self.strikethrough
    }
}

#[derive(Clone, Debug, PartialEq)]
struct RenderStyle {
    font: Font,
    foreground: Hsla,
    underline: bool,
    strikethrough: bool,
}

impl RenderStyle {
    fn from_cell(
        indexed: &SnapshotCell,
        selection: Option<&SelectionRange>,
        cursor: Point,
        cursor_is_focused: bool,
        theme: &TerminalTheme,
        base_font: &Font,
        dynamic: &Colors,
    ) -> Self {
        let palette = theme.palette();
        let flags = indexed.cell.flags;
        let mut foreground = palette.resolve(indexed.cell.fg, dynamic).hsla();
        let mut background = palette.resolve(indexed.cell.bg, dynamic).hsla();

        if flags.intersects(Flags::DIM | Flags::DIM_BOLD) {
            foreground.a *= 0.7;
        }
        if flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut foreground, &mut background);
        }
        if selection.is_some_and(|range| range.contains(indexed.point)) {
            foreground = theme.selection_foreground().hsla();
        }
        if cursor_is_focused && indexed.point == cursor {
            foreground = palette.background().hsla();
        }

        let mut font = base_font.clone();
        if flags.intersects(Flags::BOLD | Flags::DIM_BOLD) {
            font.weight = FontWeight::BOLD;
        }
        if flags.contains(Flags::ITALIC) {
            font.style = FontStyle::Italic;
        }

        Self {
            font,
            foreground,
            underline: flags.intersects(Flags::ALL_UNDERLINES),
            strikethrough: flags.contains(Flags::STRIKEOUT),
        }
    }
}

pub(crate) fn build_render_runs(
    view: &SnapshotView<'_>,
    cursor_is_focused: bool,
    theme: &TerminalTheme,
    base_font: Font,
) -> Vec<RenderRun> {
    build_render_runs_with_colors(
        view.cells,
        view.selection,
        view.cursor.point,
        cursor_is_focused,
        theme,
        base_font,
        view.colors,
    )
}

#[cfg(test)]
pub(crate) fn build_render_runs_from_cells(
    cells: &[SnapshotCell],
    selection: Option<&SelectionRange>,
    cursor: Point,
    cursor_is_focused: bool,
    theme: &TerminalTheme,
    base_font: Font,
) -> Vec<RenderRun> {
    build_render_runs_with_colors(
        cells,
        selection,
        cursor,
        cursor_is_focused,
        theme,
        base_font,
        &Colors::default(),
    )
}

fn build_render_runs_with_colors(
    cells: &[SnapshotCell],
    selection: Option<&SelectionRange>,
    cursor: Point,
    cursor_is_focused: bool,
    theme: &TerminalTheme,
    base_font: Font,
    dynamic: &Colors,
) -> Vec<RenderRun> {
    let mut runs = Vec::new();
    let mut current: Option<RenderRun> = None;

    for indexed in cells {
        if indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }
        if indexed.cell.c == '\t' {
            flush(&mut runs, &mut current);
            continue;
        }

        let width =
            usize::from(indexed.cell.flags.contains(Flags::WIDE_CHAR)) + 1;
        let style = RenderStyle::from_cell(
            indexed,
            selection,
            cursor,
            cursor_is_focused,
            theme,
            &base_font,
            dynamic,
        );
        let requires_isolated_shaping = indexed
            .cell
            .zerowidth()
            .is_some_and(|value| !value.is_empty())
            || indexed.cell.flags.contains(Flags::WIDE_CHAR);
        let can_extend = current.as_ref().is_some_and(|run| {
            !requires_isolated_shaping
                && run.line == indexed.point.line.0
                && run.end_column() == indexed.point.column.0
                && run.font == style.font
        });

        if !can_extend {
            flush(&mut runs, &mut current);
        }
        if indexed.cell.c == ' '
            && indexed.cell.zerowidth().is_none_or(<[char]>::is_empty)
            && current.is_none()
        {
            continue;
        }

        current
            .get_or_insert_with(|| {
                RenderRun::new(
                    indexed.point.line.0,
                    indexed.point.column.0,
                    style.font,
                )
            })
            .append(
                indexed,
                width,
                style.foreground,
                style.underline,
                style.strikethrough,
            );

        if requires_isolated_shaping {
            flush(&mut runs, &mut current);
        }
    }

    flush(&mut runs, &mut current);
    runs
}

fn flush(runs: &mut Vec<RenderRun>, current: &mut Option<RenderRun>) {
    if let Some(run) = current.take()
        && !run.text.is_empty()
    {
        runs.push(run);
    }
}

#[cfg(test)]
mod tests {
    use gpui::{FontStyle, FontWeight};
    use otty_libterm::escape::{Color, StdColor};
    use otty_libterm::surface::{
        Cell, Column, Flags, Line, Point, SnapshotCell,
    };

    use super::*;
    use crate::TerminalTheme;

    fn cell(line: i32, column: usize, character: char) -> SnapshotCell {
        SnapshotCell {
            point: Point::new(Line(line), Column(column)),
            cell: Cell {
                c: character,
                ..Cell::default()
            },
        }
    }

    fn cells_from_text(text: &str) -> Vec<SnapshotCell> {
        text.chars()
            .enumerate()
            .map(|(column, character)| cell(0, column, character))
            .collect()
    }

    fn build(cells: &[SnapshotCell]) -> Vec<RenderRun> {
        build_render_runs_from_cells(
            cells,
            None,
            Point::default(),
            false,
            &TerminalTheme::default(),
            crate::TerminalFont::default().gpui_font(),
        )
    }

    #[test]
    fn keeps_grid_columns_for_spaces_wide_cells_and_combining_marks() {
        let mut cells = cells_from_text(" a");
        let mut wide = cell(0, 2, '界');
        wide.cell.flags.insert(Flags::WIDE_CHAR);
        wide.cell.push_zerowidth('\u{0301}');
        let mut spacer = cell(0, 3, ' ');
        spacer.cell.flags.insert(Flags::WIDE_CHAR_SPACER);
        cells.extend([wide, spacer, cell(0, 4, 'x')]);

        let runs = build(&cells);

        assert_eq!(
            runs.iter().map(RenderRun::text).collect::<String>(),
            "a界\u{0301}x"
        );
        assert_eq!(runs[0].start_column(), 1);
        assert_eq!(runs.iter().map(RenderRun::cell_columns).sum::<usize>(), 4);
    }

    #[test]
    fn splits_font_styles_but_preserves_color_spans() {
        let mut cells = cells_from_text("abc");
        cells[0].cell.fg = Color::Std(StdColor::Red);
        cells[1].cell.fg = Color::Std(StdColor::Green);
        cells[2].cell.flags.insert(Flags::BOLD | Flags::ITALIC);

        let runs = build(&cells);

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].spans().len(), 2);
        assert_eq!(runs[1].font().weight, FontWeight::BOLD);
        assert_eq!(runs[1].font().style, FontStyle::Italic);
    }

    #[test]
    fn inverse_and_selection_use_theme_colors() {
        let mut cells = cells_from_text("ab");
        cells[0].cell.flags.insert(Flags::INVERSE);
        let selection = otty_libterm::surface::SelectionRange {
            start: Point::new(Line(0), Column(1)),
            end: Point::new(Line(0), Column(1)),
            is_block: false,
        };

        let runs = build_render_runs_from_cells(
            &cells,
            Some(&selection),
            Point::new(Line(1), Column(0)),
            false,
            &TerminalTheme::default(),
            crate::TerminalFont::default().gpui_font(),
        );

        assert_ne!(
            runs[0].spans()[0].foreground(),
            runs[0].spans()[1].foreground()
        );
    }

    #[test]
    fn cursor_text_color_is_only_applied_while_focused() {
        let mut cells = cells_from_text("a");
        cells[0].cell.fg = Color::Std(StdColor::Red);
        let theme = TerminalTheme::default();
        let font = crate::TerminalFont::default().gpui_font();

        let unfocused = build_render_runs_from_cells(
            &cells,
            None,
            Point::default(),
            false,
            &theme,
            font.clone(),
        );
        let focused = build_render_runs_from_cells(
            &cells,
            None,
            Point::default(),
            true,
            &theme,
            font,
        );

        assert_eq!(
            unfocused[0].spans()[0].foreground(),
            theme
                .palette()
                .resolve(Color::Std(StdColor::Red), &Colors::default())
                .hsla(),
        );
        assert_eq!(
            focused[0].spans()[0].foreground(),
            theme.palette().background().hsla(),
        );
    }

    #[test]
    fn keeps_contiguous_complex_script_cells_in_one_shaping_run() {
        let cells = cells_from_text("سلام");

        let runs = build(&cells);

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text(), "سلام");
        assert_eq!(runs[0].cell_columns(), 4);
    }
}
