use std::error::Error;
use std::mem;
use std::ops::RangeInclusive;

use log::{debug, warn};
pub use regex_automata::hybrid::BuildError;
use regex_automata::hybrid::dfa::{Builder, Cache, Config, DFA};
use regex_automata::nfa::thompson::Config as ThompsonConfig;
use regex_automata::util::syntax::Config as SyntaxConfig;
use regex_automata::{Anchored, Input, MatchKind};

use crate::cell::{Cell, Flags};
use crate::grid::{BidirectionalIterator, Dimensions, GridIterator, Indexed};
use crate::index::{Boundary, Column, Direction, Point, Side};
use crate::surface::Surface;

/// Inclusive grid range describing a regex match.
pub type Match = RangeInclusive<Point>;

/// Terminal regex search state shared across invocations.
#[derive(Clone, Debug)]
pub struct RegexSearch {
    left_fdfa: LazyDfa,
    left_rdfa: LazyDfa,
    right_rdfa: LazyDfa,
    right_fdfa: LazyDfa,
}

impl RegexSearch {
    /// Build the forward and backward search DFAs.
    pub fn new(search: &str) -> Result<RegexSearch, Box<BuildError>> {
        let has_uppercase = search.chars().any(|c| c.is_uppercase());
        let syntax_config =
            SyntaxConfig::new().case_insensitive(!has_uppercase);
        let config = Config::new()
            .minimum_cache_clear_count(Some(3))
            .minimum_bytes_per_state(Some(10));
        let max_size = config.get_cache_capacity();
        let thompson_config =
            ThompsonConfig::new().nfa_size_limit(Some(max_size));

        let left_rdfa = LazyDfa::new(
            search,
            config.clone(),
            syntax_config,
            thompson_config.clone(),
            Direction::Right,
            true,
        )?;
        let has_empty = left_rdfa.dfa.get_nfa().has_empty();
        let left_fdfa = LazyDfa::new(
            search,
            config.clone(),
            syntax_config,
            thompson_config.clone(),
            Direction::Left,
            has_empty,
        )?;

        let right_fdfa = LazyDfa::new(
            search,
            config.clone(),
            syntax_config,
            thompson_config.clone(),
            Direction::Right,
            has_empty,
        )?;
        let right_rdfa = LazyDfa::new(
            search,
            config,
            syntax_config,
            thompson_config,
            Direction::Left,
            true,
        )?;

        Ok(RegexSearch {
            left_fdfa,
            left_rdfa,
            right_fdfa,
            right_rdfa,
        })
    }
}

/// Runtime-evaluated DFA used for a single direction.
#[derive(Clone, Debug)]
struct LazyDfa {
    dfa: DFA,
    cache: Cache,
    direction: Direction,
    match_all: bool,
}

impl LazyDfa {
    fn new(
        search: &str,
        mut config: Config,
        syntax: SyntaxConfig,
        mut thompson: ThompsonConfig,
        direction: Direction,
        match_all: bool,
    ) -> Result<Self, Box<BuildError>> {
        thompson = match direction {
            Direction::Left => thompson.reverse(true),
            Direction::Right => thompson.reverse(false),
        };
        config = if match_all {
            config.match_kind(MatchKind::All)
        } else {
            config.match_kind(MatchKind::LeftmostFirst)
        };

        let dfa = Builder::new()
            .configure(config)
            .syntax(syntax)
            .thompson(thompson)
            .build(search)?;
        let cache = dfa.create_cache();

        Ok(Self {
            direction,
            cache,
            dfa,
            match_all,
        })
    }
}

impl Surface {
    /// Find next search match relative to `origin` in `direction`.
    pub fn search_next(
        &self,
        regex: &mut RegexSearch,
        mut origin: Point,
        direction: Direction,
        side: Side,
        mut max_lines: Option<usize>,
    ) -> Option<Match> {
        origin = self.expand_wide(origin, direction);
        max_lines =
            max_lines.filter(|max_lines| max_lines + 1 < self.total_lines());

        match direction {
            Direction::Right => {
                self.next_match_right(regex, origin, side, max_lines)
            },
            Direction::Left => {
                self.next_match_left(regex, origin, side, max_lines)
            },
        }
    }

    fn next_match_right(
        &self,
        regex: &mut RegexSearch,
        origin: Point,
        side: Side,
        max_lines: Option<usize>,
    ) -> Option<Match> {
        let start = self.line_search_left(origin);
        let mut end = start;

        end = match max_lines {
            Some(max_lines) => {
                let line =
                    (start.line + max_lines).grid_clamp(self, Boundary::None);
                Point::new(line, self.last_column())
            },
            _ => end.sub(self, Boundary::None, 1),
        };

        let mut regex_iter =
            RegexIter::new(start, end, Direction::Right, self, regex)
                .peekable();
        let first_match = regex_iter.peek()?.clone();

        let regex_match = regex_iter
            .find(|regex_match| {
                let match_point = Self::match_side(regex_match, side);

                match_point.line < start.line
                    || match_point.line > origin.line
                    || (match_point.line == origin.line
                        && match_point.column >= origin.column)
            })
            .unwrap_or(first_match);

        Some(regex_match)
    }

    fn next_match_left(
        &self,
        regex: &mut RegexSearch,
        origin: Point,
        side: Side,
        max_lines: Option<usize>,
    ) -> Option<Match> {
        let start = self.line_search_right(origin);
        let mut end = start;

        end = match max_lines {
            Some(max_lines) => {
                let line =
                    (start.line - max_lines).grid_clamp(self, Boundary::None);
                Point::new(line, Column(0))
            },
            _ => end.add(self, Boundary::None, 1),
        };

        let mut regex_iter =
            RegexIter::new(start, end, Direction::Left, self, regex).peekable();
        let first_match = regex_iter.peek()?.clone();

        let regex_match = regex_iter
            .find(|regex_match| {
                let match_point = Self::match_side(regex_match, side);

                match_point.line > start.line
                    || match_point.line < origin.line
                    || (match_point.line == origin.line
                        && match_point.column <= origin.column)
            })
            .unwrap_or(first_match);

        Some(regex_match)
    }

    fn match_side(regex_match: &Match, side: Side) -> Point {
        match side {
            Side::Right => *regex_match.end(),
            Side::Left => *regex_match.start(),
        }
    }

    /// Find regex match searching to the left. `start` and `end` bounds are inclusive.
    pub fn regex_search_left(
        &self,
        regex: &mut RegexSearch,
        start: Point,
        end: Point,
    ) -> Option<Match> {
        let match_start =
            self.regex_search(start, end, &mut regex.left_fdfa)?;
        let match_end =
            self.regex_search(match_start, start, &mut regex.left_rdfa)?;

        Some(match_start..=match_end)
    }

    /// Find regex match searching to the right. `start` and `end` bounds are inclusive.
    pub fn regex_search_right(
        &self,
        regex: &mut RegexSearch,
        start: Point,
        end: Point,
    ) -> Option<Match> {
        let match_end = self.regex_search(start, end, &mut regex.right_fdfa)?;
        let match_start =
            self.regex_search(match_end, start, &mut regex.right_rdfa)?;

        Some(match_start..=match_end)
    }

    fn regex_search(
        &self,
        start: Point,
        end: Point,
        regex: &mut LazyDfa,
    ) -> Option<Point> {
        match self.regex_search_internal(start, end, regex) {
            Ok(regex_match) => regex_match,
            Err(err) => {
                warn!("Regex exceeded complexity limit");
                debug!("    {err}");
                None
            },
        }
    }

    fn regex_search_internal(
        &self,
        start: Point,
        end: Point,
        regex: &mut LazyDfa,
    ) -> Result<Option<Point>, Box<dyn Error>> {
        let topmost_line = self.topmost_line();
        let screen_lines = self.screen_lines() as i32;
        let last_column = self.last_column();

        let next = match regex.direction {
            Direction::Right => GridIterator::next,
            Direction::Left => GridIterator::prev,
        };

        let regex_anchored = if regex.match_all {
            Anchored::Yes
        } else {
            Anchored::No
        };
        let input = Input::new(&[]).anchored(regex_anchored);
        let mut state = regex
            .dfa
            .start_state_forward(&mut regex.cache, &input)
            .unwrap();

        let mut iter = self.grid().iter_from(start);
        let mut regex_match = None;
        let mut done = false;

        let mut cell = iter.cell();
        self.skip_fullwidth(&mut iter, &mut cell, regex.direction);
        let mut c = cell.c;
        let mut last_wrapped = iter.cell().flags.contains(Flags::WRAPLINE);

        let mut point = iter.point();
        let mut last_point = point;
        let mut consumed_bytes = 0;

        macro_rules! reset_state {
            () => {{
                state =
                    regex.dfa.start_state_forward(&mut regex.cache, &input)?;
                consumed_bytes = 0;
                regex_match = None;
            }};
        }

        'outer: loop {
            let mut buf = [0; 4];
            let utf8_len = c.encode_utf8(&mut buf).len();

            for i in 0..utf8_len {
                let byte = match regex.direction {
                    Direction::Right => buf[i],
                    Direction::Left => buf[utf8_len - i - 1],
                };

                state = regex.dfa.next_state(&mut regex.cache, state, byte)?;
                consumed_bytes += 1;

                if i == 0 && state.is_match() {
                    regex_match = Some(last_point);
                } else if state.is_dead() {
                    if consumed_bytes == 2 {
                        reset_state!();

                        if i == 0 {
                            continue 'outer;
                        }
                    } else {
                        break 'outer;
                    }
                }
            }

            if point == end || done {
                state = regex.dfa.next_eoi_state(&mut regex.cache, state)?;
                if state.is_match() {
                    regex_match = Some(point);
                } else if state.is_dead() && consumed_bytes == 1 {
                    regex_match = None;
                }

                break;
            }

            let mut cell = match next(&mut iter) {
                Some(Indexed { cell, .. }) => cell,
                None => {
                    let line = topmost_line - point.line + screen_lines - 1;
                    let start = Point::new(line, last_column - point.column);
                    iter = self.grid().iter_from(start);
                    iter.cell()
                },
            };

            done = iter.point() == end;

            self.skip_fullwidth(&mut iter, &mut cell, regex.direction);

            c = cell.c;
            let wrapped = iter.cell().flags.contains(Flags::WRAPLINE);

            last_point = mem::replace(&mut point, iter.point());

            if (last_point.column == last_column
                && point.column == Column(0)
                && !last_wrapped)
                || (last_point.column == Column(0)
                    && point.column == last_column
                    && !wrapped)
            {
                state = regex.dfa.next_eoi_state(&mut regex.cache, state)?;
                if state.is_match() {
                    regex_match = Some(last_point);
                }

                match regex_match {
                    Some(_)
                        if (!state.is_dead() || consumed_bytes > 1)
                            && consumed_bytes != 0 =>
                    {
                        break;
                    },
                    _ => reset_state!(),
                }
            }

            last_wrapped = wrapped;
        }

        Ok(regex_match)
    }

    fn skip_fullwidth<'a>(
        &self,
        iter: &'a mut GridIterator<'_, Cell>,
        cell: &mut &'a Cell,
        direction: Direction,
    ) {
        match direction {
            Direction::Right
                if cell.flags.contains(Flags::WIDE_CHAR)
                    && iter.point().column < self.last_column() =>
            {
                iter.next();
            },
            Direction::Right
                if cell.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER) =>
            {
                if let Some(Indexed { cell: new_cell, .. }) = iter.next() {
                    *cell = new_cell;
                }
                iter.next();
            },
            Direction::Left if cell.flags.contains(Flags::WIDE_CHAR_SPACER) => {
                if let Some(Indexed { cell: new_cell, .. }) = iter.prev() {
                    *cell = new_cell;
                }

                let prev = iter.point().sub(self, Boundary::Grid, 1);
                if self.grid()[prev]
                    .flags
                    .contains(Flags::LEADING_WIDE_CHAR_SPACER)
                {
                    iter.prev();
                }
            },
            _ => (),
        }
    }
}

/// Iterator over regex matches.
pub struct RegexIter<'a> {
    point: Point,
    end: Point,
    direction: Direction,
    regex: &'a mut RegexSearch,
    surface: &'a Surface,
    done: bool,
}

impl<'a> RegexIter<'a> {
    pub fn new(
        start: Point,
        end: Point,
        direction: Direction,
        surface: &'a Surface,
        regex: &'a mut RegexSearch,
    ) -> Self {
        Self {
            point: start,
            done: false,
            end,
            direction,
            surface,
            regex,
        }
    }

    fn skip(&mut self) {
        self.point = self.surface.expand_wide(self.point, self.direction);

        self.point = match self.direction {
            Direction::Right => self.point.add(self.surface, Boundary::None, 1),
            Direction::Left => self.point.sub(self.surface, Boundary::None, 1),
        };
    }

    fn next_match(&mut self) -> Option<Match> {
        match self.direction {
            Direction::Right => self
                .surface
                .regex_search_right(self.regex, self.point, self.end),
            Direction::Left => self
                .surface
                .regex_search_left(self.regex, self.point, self.end),
        }
    }
}

impl Iterator for RegexIter<'_> {
    type Item = Match;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.point == self.end {
            self.done = true;
        }

        let regex_match = self.next_match()?;

        self.point = *regex_match.end();
        if self.point == self.end {
            self.done = true;
        } else {
            self.skip();
        }

        Some(regex_match)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Dimensions;
    use crate::index::{Column, Line};
    use crate::{Surface, SurfaceConfig};

    struct TestDimensions {
        columns: usize,
        screen_lines: usize,
    }

    impl TestDimensions {
        fn new(columns: usize, screen_lines: usize) -> Self {
            Self {
                columns,
                screen_lines,
            }
        }
    }

    impl Dimensions for TestDimensions {
        fn total_lines(&self) -> usize {
            self.screen_lines
        }

        fn screen_lines(&self) -> usize {
            self.screen_lines
        }

        fn columns(&self) -> usize {
            self.columns
        }
    }

    fn setup_surface(columns: usize, lines: usize) -> Surface {
        let size = TestDimensions::new(columns, lines);
        Surface::new(SurfaceConfig::default(), &size)
    }

    fn fill_surface_text(surface: &mut Surface, text: &str) {
        let grid = surface.grid_mut();
        let mut line_idx = 0i32;
        let mut col = Column(0);
        let max_lines = grid.screen_lines() as i32;

        for ch in text.chars() {
            if ch == '\n' {
                line_idx += 1;
                col = Column(0);
                if line_idx >= max_lines {
                    break;
                }
            } else if col.0 < grid.columns() && line_idx < max_lines {
                grid[Line(line_idx)][col].c = ch;
                col += 1;
            }
        }
    }

    #[test]
    fn test_regex_search_new_creates_valid_regex() {
        let result = RegexSearch::new("test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_regex_search_new_with_uppercase() {
        let result = RegexSearch::new("Test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_regex_search_new_with_invalid_pattern() {
        let result = RegexSearch::new("[");
        assert!(result.is_err());
    }

    #[test]
    fn test_simple_forward_search() {
        let mut surface = setup_surface(20, 5);
        fill_surface_text(&mut surface, "hello world\ntest line\nfoo bar");

        let mut regex = RegexSearch::new("world").unwrap();
        let start = Point::new(Line(0), Column(0));
        let end = Point::new(Line(2), Column(6));

        let result = surface.regex_search_right(&mut regex, start, end);
        assert!(result.is_some());

        let match_range = result.unwrap();
        assert_eq!(*match_range.start(), Point::new(Line(0), Column(6)));
        assert_eq!(*match_range.end(), Point::new(Line(0), Column(10)));
    }

    #[test]
    fn test_simple_backward_search() {
        let mut surface = setup_surface(20, 5);
        fill_surface_text(&mut surface, "hello world\ntest line\nfoo bar");

        let mut regex = RegexSearch::new("hello").unwrap();
        let start = Point::new(Line(2), Column(6));
        let end = Point::new(Line(0), Column(0));

        let result = surface.regex_search_left(&mut regex, start, end);
        assert!(result.is_some());

        let match_range = result.unwrap();
        assert_eq!(*match_range.start(), Point::new(Line(0), Column(0)));
        assert_eq!(*match_range.end(), Point::new(Line(0), Column(4)));
    }

    #[test]
    fn test_search_next_right() {
        let mut surface = setup_surface(20, 5);
        fill_surface_text(&mut surface, "test test test\nmore test");

        let mut regex = RegexSearch::new("test").unwrap();
        let origin = Point::new(Line(0), Column(0));

        let result = surface.search_next(
            &mut regex,
            origin,
            Direction::Right,
            Side::Left,
            None,
        );
        assert!(result.is_some());

        let match_range = result.unwrap();
        assert_eq!(*match_range.start(), Point::new(Line(0), Column(0)));
        assert_eq!(*match_range.end(), Point::new(Line(0), Column(3)));
    }

    #[test]
    fn test_search_next_left() {
        let mut surface = setup_surface(20, 5);
        fill_surface_text(&mut surface, "test test test\nmore test");

        let mut regex = RegexSearch::new("test").unwrap();
        let origin = Point::new(Line(1), Column(10));

        let result = surface.search_next(
            &mut regex,
            origin,
            Direction::Left,
            Side::Left,
            None,
        );
        assert!(result.is_some());

        let match_range = result.unwrap();
        assert!(match_range.start().line <= Line(1));
    }

    #[test]
    fn test_search_no_match() {
        let mut surface = setup_surface(20, 5);
        fill_surface_text(&mut surface, "hello world");

        let mut regex = RegexSearch::new("notfound").unwrap();
        let start = Point::new(Line(0), Column(0));
        let end = Point::new(Line(4), Column(19));

        let result = surface.regex_search_right(&mut regex, start, end);
        assert!(result.is_none());
    }

    #[test]
    fn test_case_insensitive_search() {
        let mut surface = setup_surface(20, 5);
        fill_surface_text(&mut surface, "Hello WORLD hello");

        // Lowercase pattern should match case-insensitively
        let mut regex = RegexSearch::new("hello").unwrap();
        let start = Point::new(Line(0), Column(0));
        let end = Point::new(Line(0), Column(16));

        let result = surface.regex_search_right(&mut regex, start, end);
        assert!(result.is_some());
    }

    #[test]
    fn test_case_sensitive_search() {
        let mut surface = setup_surface(20, 5);
        fill_surface_text(&mut surface, "hello WORLD");

        // Uppercase pattern should be case-sensitive
        let mut regex = RegexSearch::new("WORLD").unwrap();
        let start = Point::new(Line(0), Column(0));
        let end = Point::new(Line(0), Column(10));

        let result = surface.regex_search_right(&mut regex, start, end);
        assert!(result.is_some());

        let match_range = result.unwrap();
        assert_eq!(*match_range.start(), Point::new(Line(0), Column(6)));
    }

    #[test]
    fn test_regex_iter_forward() {
        let mut surface = setup_surface(30, 5);
        fill_surface_text(&mut surface, "test one test two test three");

        let mut regex = RegexSearch::new("test").unwrap();
        let start = Point::new(Line(0), Column(0));
        let end = Point::new(Line(0), Column(28));

        let matches: Vec<Match> =
            RegexIter::new(start, end, Direction::Right, &surface, &mut regex)
                .collect();

        assert_eq!(matches.len(), 3);
        assert_eq!(*matches[0].start(), Point::new(Line(0), Column(0)));
        assert_eq!(*matches[1].start(), Point::new(Line(0), Column(9)));
        assert_eq!(*matches[2].start(), Point::new(Line(0), Column(18)));
    }

    #[test]
    fn test_regex_iter_backward() {
        let mut surface = setup_surface(30, 5);
        fill_surface_text(&mut surface, "test one test two test three");

        let mut regex = RegexSearch::new("test").unwrap();
        let start = Point::new(Line(0), Column(28));
        let end = Point::new(Line(0), Column(0));

        let matches: Vec<Match> =
            RegexIter::new(start, end, Direction::Left, &surface, &mut regex)
                .collect();

        assert!(!matches.is_empty());
    }

    #[test]
    fn test_search_with_max_lines() {
        let mut surface = setup_surface(20, 10);
        fill_surface_text(
            &mut surface,
            "line1\nline2\nline3\nline4\nline5\ntarget here",
        );

        let mut regex = RegexSearch::new("target").unwrap();
        let origin = Point::new(Line(0), Column(0));

        // Search with max_lines limit
        let result = surface.search_next(
            &mut regex,
            origin,
            Direction::Right,
            Side::Left,
            Some(3),
        );

        // Should not find "target" since it's beyond max_lines
        assert!(result.is_none() || result.unwrap().start().line < Line(4));
    }

    #[test]
    fn test_regex_pattern_with_special_chars() {
        let mut surface = setup_surface(30, 5);
        fill_surface_text(&mut surface, "hello@example.com is email");

        let mut regex = RegexSearch::new(r"\w+@\w+\.\w+").unwrap();
        let start = Point::new(Line(0), Column(0));
        let end = Point::new(Line(0), Column(26));

        let result = surface.regex_search_right(&mut regex, start, end);
        assert!(result.is_some());

        let match_range = result.unwrap();
        // Should match the email address
        assert!(match_range.start().column.0 == 0);
    }

    #[test]
    fn test_empty_surface_search() {
        let surface = setup_surface(20, 5);

        let mut regex = RegexSearch::new("test").unwrap();
        let start = Point::new(Line(0), Column(0));
        let end = Point::new(Line(4), Column(19));

        let result = surface.regex_search_right(&mut regex, start, end);
        // Empty surface should not match
        assert!(result.is_none());
    }

    #[test]
    fn test_search_at_boundaries() {
        let mut surface = setup_surface(10, 3);
        fill_surface_text(&mut surface, "start\nmiddle\nend");

        let mut regex = RegexSearch::new("end").unwrap();
        let start = Point::new(Line(0), Column(0));
        let end = Point::new(Line(2), Column(2));

        let result = surface.regex_search_right(&mut regex, start, end);
        assert!(result.is_some());

        let match_range = result.unwrap();
        assert_eq!(*match_range.start(), Point::new(Line(2), Column(0)));
    }
}
