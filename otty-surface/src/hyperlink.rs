use once_cell::sync::Lazy;
use regex::Regex;

use crate::{Flags, Point, SnapshotCell, SnapshotSize, point_to_viewport};

static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        "(?:ipfs:|ipns:|magnet:|mailto:|gemini://|gopher://|https://|http://|news:|file://|git://|ssh:|ftp://)[^\\s<>\\x00-\\x1F\\x7F-\\x9F\"']+",
    )
    .expect("hyperlink regex must compile")
});

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


#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub(crate) struct HyperlinkMap {
    spans: Vec<HyperlinkSpan>,
    cell_to_span: Vec<Option<u32>>,
    columns: usize,
    screen_lines: usize,
}

impl HyperlinkSpan {
    #[cfg(test)]
    #[inline]
    pub(crate) fn contains(&self, point: Point) -> bool {
        let starts_before = point.line > self.start.line
            || (point.line == self.start.line
                && point.column >= self.start.column);
        let ends_after = point.line < self.end.line
            || (point.line == self.end.line && point.column <= self.end.column);
        starts_before && ends_after
    }
}

impl HyperlinkMap {
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
        map.ingest_detected_urls(cells, display_offset);
        map
    }

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

    pub(crate) fn span_for_point(
        &self,
        display_offset: usize,
        point: Point,
    ) -> Option<&HyperlinkSpan> {
        let id = self.span_id_for_point(display_offset, point)?;
        self.spans.get(id as usize)
    }

    fn set_span_for_index(&mut self, span_id: u32, index: usize) {
        if let Some(slot) = self.cell_to_span.get_mut(index) {
            *slot = Some(span_id);
        }
    }

    fn push_span(
        &mut self,
        link: crate::cell::Hyperlink,
        start: Point,
        end: Point,
    ) -> u32 {
        self.spans.push(HyperlinkSpan { link, start, end });
        (self.spans.len() - 1) as u32
    }

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

            let idx =
                viewport_point.line * self.columns + viewport_point.column.0;
            let hyperlink = indexed.cell.hyperlink();
            if let Some(link) = hyperlink {
                let is_adjacent = last_viewport.is_some_and(|prev| {
                    (viewport_point.line == prev.line
                        && viewport_point.column.0 == prev.column.0 + 1)
                        || (viewport_point.line == prev.line + 1
                            && viewport_point.column.0 == 0
                            && last_flags.contains(Flags::WRAPLINE))
                });

                let span_id = if is_adjacent
                    && current_span
                        .and_then(|id| self.spans.get(id as usize))
                        .is_some_and(|span| span.link == link)
                {
                    let id = current_span.unwrap();
                    if let Some(span) = self.spans.get_mut(id as usize) {
                        span.end = indexed.point;
                    }
                    id
                } else {
                    self.push_span(link, indexed.point, indexed.point)
                };

                self.set_span_for_index(span_id, idx);
                current_span = Some(span_id);
            } else {
                current_span = None;
            }

            last_viewport = Some(viewport_point);
            last_flags = indexed.cell.flags;
        }
    }

    fn ingest_detected_urls(
        &mut self,
        cells: &[SnapshotCell],
        display_offset: usize,
    ) {
        let mut run_chars = String::new();
        let mut run_points: Vec<Point> = Vec::new();
        let mut run_indices: Vec<usize> = Vec::new();
        let mut last_viewport: Option<Point<usize>> = None;
        let mut last_flags = Flags::empty();

        for indexed in cells {
            let Some(viewport_point) =
                point_to_viewport(display_offset, indexed.point)
            else {
                continue;
            };

            if viewport_point.line >= self.screen_lines {
                continue;
            }

            let idx =
                viewport_point.line * self.columns + viewport_point.column.0;

            let contiguous = last_viewport.is_some_and(|prev| {
                (viewport_point.line == prev.line
                    && viewport_point.column.0 == prev.column.0 + 1)
                    || (viewport_point.line == prev.line + 1
                        && viewport_point.column.0 == 0
                        && last_flags.contains(Flags::WRAPLINE))
            });

            let assigned =
                self.cell_to_span.get(idx).and_then(|v| *v).is_some();
            if assigned || !contiguous {
                Self::flush_detected_run(
                    &mut run_chars,
                    &mut run_points,
                    &mut run_indices,
                    self,
                );
            }

            if assigned {
                last_viewport = None;
                last_flags = Flags::empty();
                continue;
            }

            run_chars.push(indexed.cell.c);
            run_points.push(indexed.point);
            run_indices.push(idx);

            last_viewport = Some(viewport_point);
            last_flags = indexed.cell.flags;
        }

        Self::flush_detected_run(
            &mut run_chars,
            &mut run_points,
            &mut run_indices,
            self,
        );
    }

    fn flush_detected_run(
        run_chars: &mut String,
        run_points: &mut Vec<Point>,
        run_indices: &mut Vec<usize>,
        map: &mut HyperlinkMap,
    ) {
        if run_chars.is_empty() {
            return;
        }

        for mat in URL_REGEX.find_iter(run_chars) {
            let start = mat.start();
            let end = mat.end();
            if end == 0 || start >= run_points.len() {
                continue;
            }

            let link = crate::cell::Hyperlink::new(
                None::<String>,
                mat.as_str().to_owned(),
            );
            let span_id =
                map.push_span(link, run_points[start], run_points[end - 1]);
            for idx in start..end {
                if let Some(cell_idx) = run_indices.get(idx) {
                    map.set_span_for_index(span_id, *cell_idx);
                }
            }
        }

        run_chars.clear();
        run_points.clear();
        run_indices.clear();
    }
}
