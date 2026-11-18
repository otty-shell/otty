//! How the sessions table is ordered: the column and the way, what a press
//! on a column title does, and the order of the shown sessions (spec
//! Decision 6). Text columns start A to Z, Updated starts with the newest
//! session, as the tables of Containers mode do.

use std::cmp::Ordering;

use otty_ui_kit::components::data_table::SortDirection;

use super::sessions_table::TableSession;

/// Position of the Title column, the fallback arm of [`sort_sessions`].
#[cfg(test)]
pub(crate) const TITLE_COLUMN: usize = 0;
/// Position of the Agent column.
pub(crate) const AGENT_COLUMN: usize = 1;
/// Position of the Profile column.
pub(crate) const PROFILE_COLUMN: usize = 2;
/// Position of the Updated column, the last one.
pub(crate) const UPDATED_COLUMN: usize = 3;

/// The column the sessions table is ordered by and the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionSort {
    column: usize,
    order: SortDirection,
}

impl SessionSort {
    /// The position of the sorted column.
    pub(crate) fn column(&self) -> usize {
        self.column
    }

    /// The way the column is ordered.
    pub(crate) fn order(&self) -> SortDirection {
        self.order
    }

    /// The sort after the title of `column` was pressed: the same column
    /// turns around, a new column starts in its natural way. A column the
    /// table does not have changes nothing.
    pub(crate) fn pressed(self, column: usize) -> Self {
        if column > UPDATED_COLUMN {
            return self;
        }
        if column == self.column {
            return Self {
                column,
                order: self.order.reversed(),
            };
        }

        Self {
            column,
            order: natural_order(column),
        }
    }
}

impl Default for SessionSort {
    /// The sort of a table nobody sorted yet: the newest session first.
    fn default() -> Self {
        Self {
            column: UPDATED_COLUMN,
            order: SortDirection::Descending,
        }
    }
}

/// Order the shown sessions by `sort`; sessions equal in the sorted column
/// are ordered by title, then by id, so the order never jumps.
pub(crate) fn sort_sessions(rows: &mut [TableSession<'_>], sort: SessionSort) {
    rows.sort_by(|a, b| {
        let primary = match sort.column {
            AGENT_COLUMN => compare_text(a.agent_name(), b.agent_name()),
            PROFILE_COLUMN => compare_text(a.profile_name(), b.profile_name()),
            UPDATED_COLUMN => a.changed_at().cmp(&b.changed_at()),
            _ => compare_text(a.title(), b.title()),
        };

        sort.order
            .apply(primary)
            .then_with(|| compare_text(a.title(), b.title()))
            .then_with(|| a.id().as_str().cmp(b.id().as_str()))
    });
}

/// The way a column is ordered the first time it is pressed: text A to Z,
/// Updated newest first.
fn natural_order(column: usize) -> SortDirection {
    if column == UPDATED_COLUMN {
        return SortDirection::Descending;
    }

    SortDirection::Ascending
}

/// Compare two texts ignoring case; texts that differ only in case keep a
/// fixed order.
fn compare_text(a: &str, b: &str) -> Ordering {
    a.chars()
        .flat_map(char::to_lowercase)
        .cmp(b.chars().flat_map(char::to_lowercase))
        .then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use otty_agents::Project;

    use super::*;
    use crate::sessions_table::table_sessions;
    use crate::testing::{claude, claude_work, codex, titled_session};

    /// One project in `/work/otty` with sessions of three profiles: c1 and
    /// c2 of Claude Code, w1 of Claude Code · Work, x1 of Codex.
    fn project() -> Project {
        let claude_sessions = [
            titled_session("c1", "fix tabs", 400),
            titled_session("c2", "Add docs", 100),
        ];
        let work_sessions = [titled_session("w1", "Add docs", 300)];
        let codex_sessions = [titled_session("x1", "Bump deps", 200)];

        otty_agents::projects([
            (&claude(), &claude_sessions[..]),
            (&claude_work(), &work_sessions[..]),
            (&codex(), &codex_sessions[..]),
        ])
        .remove(0)
    }

    fn sorted_ids(sort: SessionSort) -> Vec<String> {
        let project = project();
        let mut rows = table_sessions(&project, |agent| match agent.as_str() {
            "codex" => "Codex",
            _ => "Claude Code",
        });

        sort_sessions(&mut rows, sort);

        rows.iter()
            .map(|row| row.id().as_str().to_string())
            .collect()
    }

    fn sort_of(column: usize, order: SortDirection) -> SessionSort {
        SessionSort { column, order }
    }

    #[test]
    fn the_newest_session_comes_first_until_a_title_is_pressed() {
        let sort = SessionSort::default();

        assert_eq!(sort, sort_of(UPDATED_COLUMN, SortDirection::Descending));
        assert_eq!(sorted_ids(sort), ["c1", "w1", "x1", "c2"]);
    }

    #[test]
    fn a_new_column_starts_in_its_natural_way_and_the_same_one_turns_around() {
        let by_title = SessionSort::default().pressed(TITLE_COLUMN);
        let turned = by_title.pressed(TITLE_COLUMN);
        let by_updated = turned.pressed(UPDATED_COLUMN);

        assert_eq!(by_title, sort_of(TITLE_COLUMN, SortDirection::Ascending));
        assert_eq!(turned, sort_of(TITLE_COLUMN, SortDirection::Descending));
        assert_eq!(
            by_updated,
            sort_of(UPDATED_COLUMN, SortDirection::Descending)
        );
        assert_eq!(by_updated.pressed(7), by_updated, "no such column");
    }

    #[test]
    fn titles_sort_ignoring_case_and_equal_titles_by_id() {
        let ascending = sort_of(TITLE_COLUMN, SortDirection::Ascending);
        let descending = sort_of(TITLE_COLUMN, SortDirection::Descending);

        assert_eq!(sorted_ids(ascending), ["c2", "w1", "x1", "c1"]);
        assert_eq!(sorted_ids(descending), ["c1", "x1", "c2", "w1"]);
    }

    #[test]
    fn agents_and_profiles_sort_by_their_names_then_by_title() {
        let by_agent = sort_of(AGENT_COLUMN, SortDirection::Ascending);
        let by_profile = sort_of(PROFILE_COLUMN, SortDirection::Descending);

        assert_eq!(sorted_ids(by_agent), ["c2", "w1", "c1", "x1"]);
        assert_eq!(sorted_ids(by_profile), ["x1", "w1", "c2", "c1"]);
    }

    /// Owner, issue 34: Updated is the fourth and last column, after the
    /// Branch column went away.
    #[test]
    fn updated_is_the_last_column_and_none_comes_after_it() {
        let by_title = SessionSort::default().pressed(TITLE_COLUMN);

        assert_eq!(UPDATED_COLUMN, 3);
        assert_eq!(by_title.pressed(4), by_title, "no such column");
    }

    #[test]
    fn updated_sorts_by_the_time_of_the_last_change() {
        let oldest_first = sort_of(UPDATED_COLUMN, SortDirection::Ascending);

        assert_eq!(sorted_ids(oldest_first), ["c2", "x1", "w1", "c1"]);
    }
}
