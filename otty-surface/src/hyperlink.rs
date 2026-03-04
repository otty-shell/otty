use std::sync::LazyLock;

use regex_automata::hybrid::regex::{
    Cache as HybridRegexCache, Regex as HybridRegex,
};
use regex_automata::util::syntax::Config as RegexSyntaxConfig;

use crate::cell::Hyperlink;
use crate::{Flags, Point, SnapshotCell, SnapshotSize, point_to_viewport};

/// Regex pattern for detecting URLs in terminal content.
///
/// Matches common URL schemes including http(s), ftp, file, git, ssh, and various
/// alternative protocols. Excludes whitespace and control characters.
const URL_PATTERN: &str = "(?:ipfs:|ipns:|magnet:|mailto:|gemini://|gopher://|https://|http://|news:|file://|git://|ssh:|ftp://)[^\\s<>\\x00-\\x1F\\x7F-\\x9F\"']+";
static URL_LINE_REGEX: LazyLock<HybridRegex> =
    LazyLock::new(build_url_line_regex);

/// Hyperlink span mapped to a contiguous range of cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperlinkSpan {
    /// Hyperlink payload (either OSC 8 or regex‑detected).
    pub link: crate::cell::Hyperlink,
    /// First cell of the hyperlink in grid coordinates.
    pub start: Point,
    /// Last cell of the hyperlink in grid coordinates.
    pub end: Point,
}

/// Internal map for efficient hyperlink lookup and storage.
///
/// Maintains both a list of hyperlink spans and a cell-to-span index
/// for O(1) lookup by grid coordinates.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub(crate) struct HyperlinkMap {
    /// All hyperlink spans in the viewport.
    spans: Vec<HyperlinkSpan>,
    /// Maps each visible cell to its span ID (if any).
    cell_to_span: Vec<Option<u32>>,
    /// Number of columns in the terminal.
    columns: usize,
    /// Number of visible lines on screen.
    screen_lines: usize,
}

impl HyperlinkSpan {
    /// Checks if a given point lies within this hyperlink span.
    ///
    /// Returns `true` if the point is between (inclusive) the start and end coordinates.
    #[cfg(test)]
    #[inline]
    pub(crate) fn contains(&self, point: Point) -> bool {
        match point.line.cmp(&self.start.line) {
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Greater => point.line < self.end.line,
            std::cmp::Ordering::Equal => {
                point.column >= self.start.column
                    && (point.line < self.end.line
                        || point.column <= self.end.column)
            },
        }
    }
}

impl HyperlinkMap {
    /// Builds a hyperlink map from a snapshot.
    pub(crate) fn build(
        cells: &[SnapshotCell],
        size: SnapshotSize,
        display_offset: usize,
    ) -> Self {
        let visible_cells = size.columns * size.screen_lines;
        if visible_cells == 0 || cells.is_empty() {
            return Self::default();
        }

        let mut map = Self {
            spans: Vec::new(),
            cell_to_span: vec![None; visible_cells],
            columns: size.columns,
            screen_lines: size.screen_lines,
        };

        map.ingest_osc_spans(cells, display_offset);
        map.ingest_detected_urls_from_cells(cells, display_offset);
        map
    }

    /// Returns the span ID for a given point in grid coordinates.
    ///
    /// Returns `None` if the point has no associated hyperlink or is outside the viewport.
    pub(crate) fn span_id_for_point(
        &self,
        display_offset: usize,
        point: Point,
    ) -> Option<u32> {
        let viewport_point = point_to_viewport(display_offset, point)?;
        if viewport_point.line >= self.screen_lines {
            return None;
        }
        let idx = viewport_point.line * self.columns + viewport_point.column.0;
        self.cell_to_span.get(idx).and_then(|v| *v)
    }

    /// Returns the hyperlink span for a given point in grid coordinates.
    ///
    /// Returns `None` if the point has no associated hyperlink.
    pub(crate) fn span_for_point(
        &self,
        display_offset: usize,
        point: Point,
    ) -> Option<&HyperlinkSpan> {
        let id = self.span_id_for_point(display_offset, point)?;
        self.spans.get(id as usize)
    }

    /// Associates a cell index with a hyperlink span ID.
    fn set_span_for_index(&mut self, span_id: u32, index: usize) {
        if let Some(slot) = self.cell_to_span.get_mut(index) {
            *slot = Some(span_id);
        }
    }

    /// Creates a new hyperlink span and returns its ID.
    fn push_span(
        &mut self,
        link: crate::cell::Hyperlink,
        start: Point,
        end: Point,
    ) -> u32 {
        self.spans.push(HyperlinkSpan { link, start, end });
        (self.spans.len() - 1) as u32
    }

    /// Processes OSC 8 hyperlinks from snapshot cells.
    ///
    /// OSC 8 hyperlinks are explicit hyperlinks set via terminal escape sequences.
    /// Adjacent cells with the same hyperlink are merged into a single span,
    /// respecting line wrapping (WRAPLINE flag).
    fn ingest_osc_spans(
        &mut self,
        cells: &[SnapshotCell],
        display_offset: usize,
    ) {
        let mut current_span: Option<u32> = None;
        let mut last_viewport: Option<Point<usize>> = None;
        let mut last_flags = Flags::empty();

        for indexed in cells {
            let Some(viewport_point) =
                point_to_viewport(display_offset, indexed.point)
            else {
                continue;
            };

            let cell_index =
                viewport_point.line * self.columns + viewport_point.column.0;

            match indexed.cell.hyperlink() {
                Some(link) => {
                    let span_id = if self.should_extend_span(
                        current_span,
                        &link,
                        viewport_point,
                        last_viewport,
                        last_flags,
                    ) {
                        self.extend_current_span(
                            current_span.unwrap(),
                            indexed.point,
                        )
                    } else {
                        self.push_span(link, indexed.point, indexed.point)
                    };

                    self.set_span_for_index(span_id, cell_index);
                    current_span = Some(span_id);
                },
                None => {
                    current_span = None;
                },
            }

            last_viewport = Some(viewport_point);
            last_flags = indexed.cell.flags;
        }
    }

    /// Checks whether the current hyperlink span should be extended with a new cell.
    ///
    /// Returns `true` if:
    /// - A span is currently active
    /// - The new link matches the current span's link
    /// - The new position is adjacent to the previous position
    fn should_extend_span(
        &self,
        current_span: Option<u32>,
        new_link: &Hyperlink,
        current_pos: Point<usize>,
        last_pos: Option<Point<usize>>,
        last_flags: Flags,
    ) -> bool {
        let Some(span_id) = current_span else {
            return false;
        };

        let Some(span) = self.spans.get(span_id as usize) else {
            return false;
        };

        if span.link != *new_link {
            return false;
        }

        self.is_adjacent_position(current_pos, last_pos, last_flags)
    }

    /// Checks if the current position is adjacent to the previous position.
    ///
    /// Positions are considered adjacent if:
    /// - Same line, next column
    /// - Next line at column 0, with WRAPLINE flag set on previous line
    fn is_adjacent_position(
        &self,
        current: Point<usize>,
        previous: Option<Point<usize>>,
        previous_flags: Flags,
    ) -> bool {
        let Some(prev) = previous else {
            return false;
        };

        if current.line == prev.line && current.column.0 == prev.column.0 + 1 {
            return true;
        }

        current.line == prev.line + 1
            && current.column.0 == 0
            && previous_flags.contains(Flags::WRAPLINE)
    }

    /// Extends the current span by updating its end point.
    fn extend_current_span(&mut self, span_id: u32, new_end: Point) -> u32 {
        if let Some(span) = self.spans.get_mut(span_id as usize) {
            span.end = new_end;
        }
        span_id
    }

    fn is_cell_assigned(&self, cell_index: usize) -> bool {
        self.cell_to_span
            .get(cell_index)
            .and_then(|id| *id)
            .is_some()
    }

    fn ingest_detected_symbol(
        &mut self,
        segment: &mut DetectedSegment,
        symbol: &DetectedSymbol,
    ) {
        if self.is_cell_assigned(symbol.cell_index) || symbol.is_spacer {
            segment.flush(self);
        } else {
            segment.push(symbol.point, symbol.ch, symbol.cell_index);
        }
    }

    /// Detects URLs from snapshot cells when no backing surface is available.
    ///
    /// This fallback path is used by composed snapshots (e.g. block surface)
    /// where we only have visible cells in viewport order.
    fn ingest_detected_urls_from_cells(
        &mut self,
        cells: &[SnapshotCell],
        display_offset: usize,
    ) {
        if cells.is_empty() || self.columns == 0 {
            return;
        }

        let regex = &*URL_LINE_REGEX;
        let mut cache = regex.create_cache();
        let mut line = LogicalLine::default();
        for row in 0..self.screen_lines {
            let row_start = row * self.columns;
            let row_end = ((row + 1) * self.columns).min(cells.len());
            if row_start >= row_end {
                break;
            }

            let row_cells = &cells[row_start..row_end];
            line.push_row(row_cells, row_start, self.columns, display_offset);

            if !row_wraps(row_cells) {
                self.ingest_detected_logical_line(&line, regex, &mut cache);
                line.clear();
            }
        }

        if !line.is_empty() {
            self.ingest_detected_logical_line(&line, regex, &mut cache);
        }
    }

    fn ingest_detected_logical_line(
        &mut self,
        line: &LogicalLine,
        regex: &HybridRegex,
        cache: &mut HybridRegexCache,
    ) {
        if line.text.is_empty() {
            return;
        }

        for matched in regex.find_iter(cache, line.text.as_str()) {
            let mut segment = DetectedSegment::default();
            let mut last_symbol_index = None;

            for byte_index in matched.start()..matched.end() {
                let Some(&symbol_index) = line.byte_to_symbol.get(byte_index)
                else {
                    continue;
                };
                if last_symbol_index == Some(symbol_index) {
                    continue;
                }
                last_symbol_index = Some(symbol_index);

                if let Some(symbol) = line.symbols.get(symbol_index) {
                    self.ingest_detected_symbol(&mut segment, symbol);
                }
            }

            segment.flush(self);
        }
    }
}

#[derive(Default)]
struct LogicalLine {
    text: String,
    symbols: Vec<DetectedSymbol>,
    byte_to_symbol: Vec<usize>,
}

impl LogicalLine {
    fn push_row(
        &mut self,
        row_cells: &[SnapshotCell],
        row_start: usize,
        columns: usize,
        display_offset: usize,
    ) {
        for (column, indexed) in row_cells.iter().enumerate() {
            let ch = indexed.cell.c;
            self.text.push(ch);
            let fallback_index = row_start + column;
            let cell_index = point_to_viewport(display_offset, indexed.point)
                .map(|point| point.line * columns + point.column.0)
                .unwrap_or(fallback_index);
            let is_spacer = indexed.cell.flags.intersects(
                Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER,
            );

            let symbol_index = self.symbols.len();
            self.symbols.push(DetectedSymbol {
                point: indexed.point,
                ch,
                cell_index,
                is_spacer,
            });

            for _ in 0..ch.len_utf8() {
                self.byte_to_symbol.push(symbol_index);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn clear(&mut self) {
        self.text.clear();
        self.symbols.clear();
        self.byte_to_symbol.clear();
    }
}

struct DetectedSymbol {
    point: Point,
    ch: char,
    cell_index: usize,
    is_spacer: bool,
}

fn row_wraps(row_cells: &[SnapshotCell]) -> bool {
    row_cells
        .iter()
        .rev()
        .find(|indexed| {
            !indexed.cell.flags.intersects(
                Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER,
            )
        })
        .map(|indexed| indexed.cell.flags.contains(Flags::WRAPLINE))
        .unwrap_or(false)
}

fn build_url_line_regex() -> HybridRegex {
    let mut builder = HybridRegex::builder();
    builder.syntax(RegexSyntaxConfig::new().case_insensitive(true));
    builder
        .build(URL_PATTERN)
        .expect("hyperlink regex must compile")
}

/// Accumulates contiguous characters of a detected URL before flushing as a span.
///
/// Used during URL detection to build up segments that may be interrupted by
/// already-assigned cells, wide character spacers, or viewport boundaries.
#[derive(Default)]
struct DetectedSegment {
    /// Accumulated URL text.
    text: String,
    /// Grid coordinates of the first character (set on first push).
    start: Option<Point>,
    /// Grid coordinates of the last character.
    end: Point,
    /// Cell indices for each character in the viewport.
    indices: Vec<usize>,
}

impl DetectedSegment {
    /// Adds a character to the current segment being built.
    ///
    /// Tracks the text content, grid coordinates, and cell indices.
    fn push(&mut self, point: Point, ch: char, cell_index: usize) {
        if self.start.is_none() {
            self.start = Some(point);
        }

        self.end = point;
        self.text.push(ch);
        self.indices.push(cell_index);
    }

    /// Commits the current segment as a hyperlink span if it contains any text.
    ///
    /// After flushing, the segment is cleared and ready to accumulate new characters.
    fn flush(&mut self, map: &mut HyperlinkMap) {
        if self.text.is_empty() {
            return;
        }

        let Some(start) = self.start else {
            return;
        };
        let link = Hyperlink::new(None::<String>, self.text.clone());
        let span_id = map.push_span(link, start, self.end);
        for idx in &self.indices {
            map.set_span_for_index(span_id, *idx);
        }

        self.text.clear();
        self.indices.clear();
        self.start = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Cell;
    use crate::index::{Column, Line};

    fn cells_from_rows(rows: &[&str], columns: usize) -> Vec<SnapshotCell> {
        let mut result = Vec::new();
        for (line_index, row) in rows.iter().enumerate() {
            let mut chars = row.chars();
            for column in 0..columns {
                let cell = Cell {
                    c: chars.next().unwrap_or(' '),
                    ..Cell::default()
                };
                result.push(SnapshotCell {
                    point: Point::new(Line(line_index as i32), Column(column)),
                    cell,
                });
            }
        }
        result
    }

    #[test]
    fn build_detects_plain_url() {
        let columns = 40;
        let cells = cells_from_rows(&["visit https://otty.sh now"], columns);
        let size = SnapshotSize {
            columns,
            screen_lines: 1,
            total_lines: 1,
        };
        let map = HyperlinkMap::build(&cells, size, 0);

        let span = map
            .span_for_point(0, Point::new(Line(0), Column(8)))
            .expect("url span");
        assert_eq!(span.link.uri(), "https://otty.sh");
        assert!(
            map.span_for_point(0, Point::new(Line(0), Column(22)))
                .is_none()
        );
    }

    #[test]
    fn build_detects_wrapped_url() {
        let columns = 12;
        let mut cells =
            cells_from_rows(&["https://otty", ".sh and more"], columns);
        cells[columns - 1].cell.flags.insert(Flags::WRAPLINE);
        let size = SnapshotSize {
            columns,
            screen_lines: 2,
            total_lines: 2,
        };
        let map = HyperlinkMap::build(&cells, size, 0);

        let first = map
            .span_for_point(0, Point::new(Line(0), Column(0)))
            .expect("first row span");
        let second = map
            .span_for_point(0, Point::new(Line(1), Column(0)))
            .expect("second row span");

        assert_eq!(first.link.uri(), "https://otty.sh");
        assert_eq!(second.link.uri(), "https://otty.sh");
    }
}
