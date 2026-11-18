//! How the sessions table is ordered: the column and the way, what a press
//! on a column title does, and the order of the shown sessions (spec
//! Decision 6). Text columns start A to Z, Updated starts with the newest
//! session, as the tables of Containers mode do.

use otty_app_domain::text_order::compare_text;
use otty_ui_kit::components::data_table::{ColumnSort, SortDirection};

use super::sessions_table::TableSession;

/// Position of the Agent column; the Title column, the first one, is the
/// fallback arm of [`sort_sessions`].
pub(crate) const AGENT_COLUMN: usize = 1;
/// Position of the Profile column.
pub(crate) const PROFILE_COLUMN: usize = 2;
/// Position of the Updated column, the last one.
pub(crate) const UPDATED_COLUMN: usize = 3;

/// The sort of a table nobody sorted yet: the newest session first.
pub(crate) fn newest_first() -> ColumnSort {
    ColumnSort::new(UPDATED_COLUMN, SortDirection::Descending)
}

/// The sort after the title of `column` was pressed: the same column turns
/// around, a new column starts in its natural way. A column the table does
/// not have changes nothing.
pub(crate) fn pressed(sort: ColumnSort, column: usize) -> ColumnSort {
    if column > UPDATED_COLUMN {
        return sort;
    }

    sort.pressed(column, natural_order(column))
}

/// Order the shown sessions by `sort`; sessions equal in the sorted column
/// are ordered by title, then by id, so the order never jumps.
pub(crate) fn sort_sessions(rows: &mut [TableSession<'_>], sort: ColumnSort) {
    rows.sort_by(|a, b| {
        let primary = match sort.column() {
            AGENT_COLUMN => compare_text(a.agent_name(), b.agent_name()),
            PROFILE_COLUMN => compare_text(a.profile_name(), b.profile_name()),
            UPDATED_COLUMN => a.changed_at().cmp(&b.changed_at()),
            _ => compare_text(a.title(), b.title()),
        };

        sort.direction()
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

#[cfg(test)]
mod tests {
    use otty_agents::Project;

    use super::*;
    use crate::sessions_table::table_sessions;
    use crate::testing::{
        TITLE_COLUMN, claude, claude_work, codex, titled_session,
    };

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

    fn sorted_ids(sort: ColumnSort) -> Vec<String> {
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

    fn sort_of(column: usize, direction: SortDirection) -> ColumnSort {
        ColumnSort::new(column, direction)
    }

    #[test]
    fn the_newest_session_comes_first_until_a_title_is_pressed() {
        let sort = newest_first();

        assert_eq!(sort, sort_of(UPDATED_COLUMN, SortDirection::Descending));
        assert_eq!(sorted_ids(sort), ["c1", "w1", "x1", "c2"]);
    }

    #[test]
    fn a_new_column_starts_in_its_natural_way_and_the_same_one_turns_around() {
        let by_title = pressed(newest_first(), TITLE_COLUMN);
        let turned = pressed(by_title, TITLE_COLUMN);
        let by_updated = pressed(turned, UPDATED_COLUMN);

        assert_eq!(by_title, sort_of(TITLE_COLUMN, SortDirection::Ascending));
        assert_eq!(turned, sort_of(TITLE_COLUMN, SortDirection::Descending));
        assert_eq!(
            by_updated,
            sort_of(UPDATED_COLUMN, SortDirection::Descending)
        );
        assert_eq!(pressed(by_updated, 7), by_updated, "no such column");
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
        let by_title = pressed(newest_first(), TITLE_COLUMN);

        assert_eq!(UPDATED_COLUMN, 3);
        assert_eq!(pressed(by_title, 4), by_title, "no such column");
    }

    #[test]
    fn updated_sorts_by_the_time_of_the_last_change() {
        let oldest_first = sort_of(UPDATED_COLUMN, SortDirection::Ascending);

        assert_eq!(sorted_ids(oldest_first), ["c2", "x1", "w1", "c1"]);
    }
}
