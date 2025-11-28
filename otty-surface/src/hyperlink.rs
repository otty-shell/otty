use crate::cell::Hyperlink;
use crate::index::{Boundary, Direction};
use crate::search::{Match, RegexIter, RegexSearch};
use crate::{
    Flags, Point, SnapshotCell, SnapshotSize, Surface, point_to_viewport,
};

/// Regex pattern for detecting URLs in terminal content.
///
/// Matches common URL schemes including http(s), ftp, file, git, ssh, and various
/// alternative protocols. Excludes whitespace and control characters.
const URL_PATTERN: &str = "(?:ipfs:|ipns:|magnet:|mailto:|gemini://|gopher://|https://|http://|news:|file://|git://|ssh:|ftp://)[^\\s<>\\x00-\\x1F\\x7F-\\x9F\"']+";

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
            }
        }
    }
}

impl HyperlinkMap {
    /// Builds a hyperlink map from the given surface and snapshot cells.
    ///
    /// This method:
    /// 1. Creates a cell-to-span index for O(1) lookups
    /// 2. Processes OSC 8 hyperlinks (explicit terminal escape sequences)
    /// 3. Detects URLs using regex pattern matching
    ///
    /// OSC 8 hyperlinks take precedence over detected URLs.
    pub(crate) fn build(
        surface: &Surface,
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
        map.ingest_detected_urls(surface, cells, display_offset);
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
            let Some(viewport_point) = point_to_viewport(display_offset, indexed.point) else {
                continue;
            };

            let cell_index = viewport_point.line * self.columns + viewport_point.column.0;

            match indexed.cell.hyperlink() {
                Some(link) => {
                    let span_id = if self.should_extend_span(
                        current_span,
                        &link,
                        viewport_point,
                        last_viewport,
                        last_flags,
                    ) {
                        self.extend_current_span(current_span.unwrap(), indexed.point)
                    } else {
                        self.push_span(link, indexed.point, indexed.point)
                    };

                    self.set_span_for_index(span_id, cell_index);
                    current_span = Some(span_id);
                }
                None => {
                    current_span = None;
                }
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

    /// Detects and processes URLs from cell content using regex pattern matching.
    ///
    /// URLs are detected using the URL_PATTERN regex. Only cells that are not
    /// already assigned to an OSC 8 hyperlink are considered (OSC 8 takes precedence).
    fn ingest_detected_urls(
        &mut self,
        surface: &Surface,
        cells: &[SnapshotCell],
        display_offset: usize,
    ) {
        if cells.is_empty() {
            return;
        }

        let start = cells.first().unwrap().point;
        let end = cells.last().unwrap().point;
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };

        let mut regex = RegexSearch::new(URL_PATTERN)
            .expect("hyperlink regex must compile");

        let iter =
            RegexIter::new(start, end, Direction::Right, surface, &mut regex);
        for regex_match in iter {
            self.ingest_detected_match(surface, display_offset, &regex_match);
        }
    }

    /// Processes a single detected URL match from regex search.
    ///
    /// Iterates through each character of the URL, skipping cells that:
    /// - Are already assigned to OSC 8 hyperlinks
    /// - Are outside the viewport
    /// - Are wide character spacers
    ///
    /// Contiguous segments are accumulated and flushed as separate spans when interrupted.
    fn ingest_detected_match(
        &mut self,
        surface: &Surface,
        display_offset: usize,
        regex_match: &Match,
    ) {
        let mut point = *regex_match.start();
        let end = *regex_match.end();
        let mut segment = DetectedSegment::default();

        loop {
            let viewport_point = point_to_viewport(display_offset, point);
            let mut assigned = true;
            let mut cell_index = 0;

            if let Some(view) = viewport_point {
                if view.line < self.screen_lines {
                    cell_index = view.line * self.columns + view.column.0;
                    assigned = self
                        .cell_to_span
                        .get(cell_index)
                        .and_then(|v| *v)
                        .is_some();
                }
            }

            if assigned || viewport_point.is_none() {
                segment.flush(self);
            } else {
                let cell = &surface.grid()[point.line][point.column];
                if !cell.flags.intersects(
                    Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER,
                ) {
                    segment.push(point, cell.c, cell_index);
                } else {
                    segment.flush(self);
                }
            }

            if point == end {
                break;
            }

            point = surface.expand_wide(point, Direction::Right);
            point = point.add(surface, Boundary::None, 1);
        }

        segment.flush(self);
    }
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

        let link = Hyperlink::new(None::<String>, self.text.clone());
        let span_id = map.push_span(link, self.start.unwrap(), self.end);
        for idx in &self.indices {
            map.set_span_for_index(span_id, *idx);
        }

        self.text.clear();
        self.indices.clear();
        self.start = None;
    }
}
