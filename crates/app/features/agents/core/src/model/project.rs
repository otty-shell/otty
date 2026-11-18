use std::time::SystemTime;

use otty_agents::Project;
use otty_app_domain::folder_path::{folder_name, short_folder_path};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::data_table::{TableCell, TableColumn};
use otty_ui_kit::components::tag_search::{Suggestion, TagKey, TagSearch};

use super::super::menus::AgentsMenu;
use super::super::session_sort::SessionSort;
use super::super::sessions_table::{self, TableSession};
use super::super::state::AgentsState;
use super::super::stats_dialog::can_show_project_stats;
use super::age::age_text;

/// The project screen: the title row with the folder and New session, the
/// search over the sessions table, and the table (spec Decision 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectViewModel<'a> {
    /// The last part of the folder path.
    pub(crate) name: String,
    /// The folder path, written short as a terminal prompt writes it,
    /// such as `~/…/ottyrun/otty`.
    pub(crate) path: String,
    /// The whole folder path, shown in the tooltip of `path`.
    pub(crate) full_path: String,
    /// Whether the folder no longer exists: nothing can open in it.
    pub(crate) is_missing: bool,
    /// Whether New session can start: the folder exists and a profile can
    /// start a session.
    pub(crate) can_start: bool,
    /// Whether the list of the profiles of New session is open.
    pub(crate) is_new_session_menu_open: bool,
    /// Whether Project statistics can open: a profile with a session in
    /// the project reads statistics, even while the folder is gone.
    pub(crate) can_show_stats: bool,
    /// The search of this project: the field's text and the tags.
    pub(crate) search: &'a TagSearch,
    /// The keys of the search in the interface language; the tags read
    /// their shown names here.
    pub(crate) keys: Vec<TagKey>,
    /// What the open suggestion list shows; empty while it is closed.
    pub(crate) suggestions: Vec<Suggestion>,
    /// The columns of the table.
    pub(crate) columns: Vec<TableColumn>,
    /// The sessions the search lets through, in the order of `sort`.
    pub(crate) rows: Vec<Vec<TableCell>>,
    /// Position of the selected session among `rows`.
    pub(crate) selected: Option<usize>,
    /// The column the table is sorted by and the way.
    pub(crate) sort: SessionSort,
    /// Shown instead of rows when the search lets no session through.
    pub(crate) empty_message: Option<String>,
}

impl<'a> ProjectViewModel<'a> {
    /// The screen of the selected `project`, with the ages of its sessions
    /// counted up to `now`.
    pub(in super::super) fn from_project(
        state: &'a AgentsState,
        project: &'a Project,
        now: SystemTime,
    ) -> Self {
        let folder = project.folder();
        let is_missing = state.profiles().is_missing(folder);
        let table = state.table();
        let search = table.search(folder);

        let shown = state.shown_sessions();
        let selected = table.selected().and_then(|key| {
            shown.iter().position(|session| key.is_of(session.entry()))
        });
        let rows = shown.iter().map(|session| cells(session, now)).collect();
        let empty_message = shown
            .is_empty()
            .then(|| i18n::t(Key::AgentsSessionsNothingMatches).to_string());
        // The schema lists every session, so it is built only while the
        // suggestions are open.
        let schema = search.is_open().then(|| state.session_schema());
        let suggestions = schema
            .flatten()
            .map(|schema| search.suggestions(&schema))
            .unwrap_or_default();
        let is_new_session_menu_open = state
            .menu()
            .is_some_and(|open| *open.menu() == AgentsMenu::NewSession);

        Self {
            name: folder_name(folder, Some(state.env().home())),
            path: short_folder_path(folder, Some(state.env().home())),
            full_path: folder.to_string_lossy().into_owned(),
            is_missing,
            can_start: state.can_start_in_selected_project(),
            is_new_session_menu_open,
            can_show_stats: can_show_project_stats(state),
            search,
            keys: sessions_table::keys(),
            suggestions,
            columns: session_columns(),
            rows,
            selected,
            sort: table.sort(),
            empty_message,
        }
    }
}

/// The columns of the sessions table with their width shares and minimum
/// widths, as the canvas draws them; every column sorts. A minimum keeps
/// the title and a typical value readable; on a narrow window the table
/// scrolls sideways instead of going below the minimums.
pub(crate) fn session_columns() -> Vec<TableColumn> {
    let column = |key: Key, share: u16, min: u16| {
        TableColumn::new(i18n::t(key), share)
            .min_width(min)
            .sortable(true)
    };

    vec![
        column(Key::AgentsColumnTitle, 5, 220),
        column(Key::AgentsAgent, 2, 110),
        column(Key::AgentsColumnProfile, 2, 120),
        // "12 min ago".
        column(Key::AgentsColumnUpdated, 1, 96),
    ]
}

/// The cells of a session: title, agent, profile and how long ago it
/// changed.
fn cells(session: &TableSession<'_>, now: SystemTime) -> Vec<TableCell> {
    vec![
        TableCell::plain(session.title()),
        TableCell::plain(session.agent_name()),
        TableCell::muted(session.profile_name()),
        TableCell::muted(age_text(now, session.changed_at())),
    ]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::ProfileStatus;
    use otty_ui_kit::components::tag_search::TagSearchAction;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::model::MainAreaViewModel;
    use crate::reducer::reduce;
    use crate::testing::{
        claude, codex, connected, now, read, session, with_profiles,
    };
    use crate::types::LoadedSessions;

    /// Claude Code and Codex, with c1 two minutes ago and x1 half a minute
    /// ago in `/work/otty`, which is selected.
    fn otty() -> AgentsState {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);
        read(
            &mut state,
            &claude(),
            vec![session("c1", "/work/otty", 1_000_000 - 120)],
        );
        read(
            &mut state,
            &codex(),
            vec![session("x1", "/work/otty", 1_000_000 - 30)],
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        );

        state
    }

    fn project(state: &AgentsState) -> ProjectViewModel<'_> {
        match MainAreaViewModel::from_state(state, now()) {
            MainAreaViewModel::Project(project) => *project,
            other => panic!("expected a project, got {other:?}"),
        }
    }

    #[test]
    fn the_table_lists_each_session_with_its_agent_profile_and_age() {
        let state = otty();

        let project = project(&state);

        assert_eq!(project.name, "otty");
        assert_eq!(project.path, "/work/otty");
        assert_eq!(project.full_path, "/work/otty");
        assert!(!project.is_missing);
        assert!(project.can_start);
        assert_eq!(project.columns, session_columns());
        assert_eq!(project.sort, SessionSort::default());
        assert_eq!(
            project.rows,
            vec![
                vec![
                    TableCell::plain("Title of x1"),
                    TableCell::plain("Codex"),
                    TableCell::muted("Codex"),
                    TableCell::muted(i18n::t(Key::JustNow)),
                ],
                vec![
                    TableCell::plain("Title of c1"),
                    TableCell::plain("Claude Code"),
                    TableCell::muted("Claude Code"),
                    TableCell::muted(i18n::with_count(Key::TplMinutesAgo, 2)),
                ],
            ]
        );
        assert_eq!(project.selected, None);
        assert_eq!(project.empty_message, None);
    }

    /// Owner, issue 34: no Branch column; a session's branch is not shown.
    #[test]
    fn the_columns_are_title_agent_profile_and_updated() {
        let titles: Vec<String> = session_columns()
            .iter()
            .map(|column| column.title().to_string())
            .collect();

        assert_eq!(
            titles,
            [
                i18n::t(Key::AgentsColumnTitle),
                i18n::t(Key::AgentsAgent),
                i18n::t(Key::AgentsColumnProfile),
                i18n::t(Key::AgentsColumnUpdated),
            ]
        );
    }

    /// Owner, issue 34: the path is short, as a terminal prompt writes it,
    /// and the whole path goes to its tooltip.
    #[test]
    fn a_deep_project_in_the_home_folder_has_a_short_path_and_its_full_one() {
        let folder = "/home/user/projects/main/ottyrun/otty";
        let mut state = with_profiles(&[(claude(), connected())]);
        read(&mut state, &claude(), vec![session("c1", folder, 1)]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from(folder)),
        );

        let project = project(&state);

        assert_eq!(project.path, "~/…/ottyrun/otty");
        assert_eq!(project.full_path, folder);
    }

    #[test]
    fn the_selected_session_is_marked_where_the_sort_puts_it() {
        let mut state = otty();
        let _ = reduce(&mut state, AgentsIntent::SelectSession(0));
        let _ = reduce(
            &mut state,
            AgentsIntent::SortSessions(crate::testing::TITLE_COLUMN),
        );

        let project = project(&state);

        assert_eq!(project.selected, Some(1), "x1 comes after c1 by title");
    }

    #[test]
    fn a_search_that_lets_nothing_through_says_so_and_suggests_while_open() {
        let mut state = otty();
        for action in [
            TagSearchAction::Focused,
            TagSearchAction::InputChanged(String::from("profile")),
        ] {
            let _ = reduce(&mut state, AgentsIntent::SessionSearch(action));
        }

        let project = project(&state);

        assert!(project.rows.is_empty());
        assert_eq!(
            project.empty_message.as_deref(),
            Some(i18n::t(Key::AgentsSessionsNothingMatches))
        );
        assert_eq!(project.search.input(), "profile");
        assert!(!project.suggestions.is_empty(), "the Profile key is one");
        assert_eq!(project.keys, sessions_table::keys());
    }

    #[test]
    fn a_missing_project_can_not_start_a_session() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: claude(),
                sessions: crate::types::SessionsRead::Read(vec![session(
                    "c1",
                    "/work/gone",
                    1,
                )]),
                missing_folders: vec![PathBuf::from("/work/gone")],
            }),
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/gone")),
        );

        let project = project(&state);

        assert!(project.is_missing);
        assert!(!project.can_start);
        assert!(project.can_show_stats, "the history is still there");
        assert_eq!(project.rows.len(), 1, "its sessions still show");
    }

    #[test]
    fn a_project_of_agents_that_read_statistics_can_show_them() {
        let state = otty();

        assert!(project(&state).can_show_stats);
    }

    #[test]
    fn a_project_of_an_agent_that_reads_no_statistics_can_not_show_them() {
        let stranger = otty_agents::Profile::default_for(
            otty_agents::AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );
        let mut state = with_profiles(&[(stranger.clone(), connected())]);
        read(&mut state, &stranger, vec![session("s1", "/work/otty", 1)]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        );

        assert!(!project(&state).can_show_stats);
    }

    #[test]
    fn a_project_whose_profiles_can_not_start_offers_no_new_session() {
        let mut state =
            with_profiles(&[(claude(), ProfileStatus::ProgramNotFound)]);
        read(&mut state, &claude(), vec![session("c1", "/work/otty", 1)]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        );

        assert!(!project(&state).can_start);
    }

    #[test]
    fn the_new_session_button_is_lit_while_its_list_is_open() {
        let mut state = otty();
        let closed = project(&state).is_new_session_menu_open;

        let _ = reduce(&mut state, AgentsIntent::NewSession);

        assert!(!closed);
        assert!(project(&state).is_new_session_menu_open);
    }
}
