//! The sessions table of the project screen: the search of each project
//! and the sort, kept while OTTY runs, the selected session, and which
//! sessions the table shows in which order (spec Decision 6). The rules of
//! the tag search live in the kit; this module gives it the keys and the
//! text of each session.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use otty_agents::{AgentId, Project, ProjectSession, SessionId};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::tag_search::{
    SearchRow, TagKey, TagSchema, TagSearch, TagSearchAction,
};

use super::session_sort::{SessionSort, sort_sessions};

/// The internal key of the Title column.
const TITLE_KEY: &str = "title";
/// The internal key of the Agent column.
const AGENT_KEY: &str = "agent";
/// The internal key of the Profile column.
const PROFILE_KEY: &str = "profile";

/// Which session a row stands for: the agent and the name of its profile,
/// and its id. Profile names are unique, and the name finds the session
/// again after the sessions are read again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionKey {
    agent: AgentId,
    profile: String,
    id: SessionId,
}

impl SessionKey {
    /// The key of the session of `entry`.
    pub(crate) fn of(entry: &ProjectSession) -> Self {
        Self {
            agent: entry.profile().agent(),
            profile: entry.profile().name().to_string(),
            id: entry.session().id().clone(),
        }
    }

    /// Whether `entry` is the session of this key.
    pub(crate) fn is_of(&self, entry: &ProjectSession) -> bool {
        entry.profile().agent() == self.agent
            && entry.profile().name() == self.profile
            && entry.session().id() == &self.id
    }
}

/// One session of a project as the table shows it: the session with its
/// profile, and the name of its agent.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TableSession<'a> {
    entry: &'a ProjectSession,
    agent_name: &'static str,
}

impl<'a> TableSession<'a> {
    /// The session with its profile.
    pub(crate) fn entry(&self) -> &'a ProjectSession {
        self.entry
    }

    /// The name of the session's agent, such as "Codex".
    pub(crate) fn agent_name(&self) -> &'static str {
        self.agent_name
    }

    /// The name of the session's profile.
    pub(crate) fn profile_name(&self) -> &'a str {
        self.entry.profile().name()
    }

    /// The title of the session.
    pub(crate) fn title(&self) -> &'a str {
        self.entry.session().title()
    }

    /// When the session last changed.
    pub(crate) fn changed_at(&self) -> SystemTime {
        self.entry.session().changed_at()
    }

    /// The id of the session.
    pub(crate) fn id(&self) -> &'a SessionId {
        self.entry.session().id()
    }

    /// The session as the search sees it: its title is its name, and each
    /// key has the text of its cell.
    fn search_row(&self) -> SearchRow {
        SearchRow::new(self.title())
            .value(TITLE_KEY, self.title())
            .value(AGENT_KEY, self.agent_name)
            .value(PROFILE_KEY, self.profile_name())
    }
}

/// What the sessions table keeps while OTTY runs: the search of each
/// project, the sort shared by every project, and the selected session.
#[derive(Debug, Default)]
pub(crate) struct SessionsTable {
    searches: HashMap<PathBuf, TagSearch>,
    /// What a project shows until someone searches in it.
    untouched: TagSearch,
    sort: SessionSort,
    selected: Option<SessionKey>,
}

impl SessionsTable {
    /// The search of the project in `folder`.
    pub(crate) fn search(&self, folder: &Path) -> &TagSearch {
        self.searches.get(folder).unwrap_or(&self.untouched)
    }

    /// The column the table is ordered by and the way.
    pub(crate) fn sort(&self) -> SessionSort {
        self.sort
    }

    /// The selected session, in whichever project it is.
    pub(crate) fn selected(&self) -> Option<&SessionKey> {
        self.selected.as_ref()
    }

    /// Do what the user did in the search of the project in `folder`;
    /// `schema` is what that table lists.
    pub(crate) fn perform_search(
        &mut self,
        folder: &Path,
        action: TagSearchAction,
        schema: &TagSchema,
    ) {
        let search = self.searches.entry(folder.to_path_buf()).or_default();

        let _ = search.perform(action, schema);
    }

    /// Take a press on the title of `column`.
    pub(crate) fn press_sort(&mut self, column: usize) {
        self.sort = self.sort.pressed(column);
    }

    /// Select the session of `key`.
    pub(crate) fn select(&mut self, key: SessionKey) {
        self.selected = Some(key);
    }
}

/// The keys of the search in column order, shown with the titles of their
/// columns in the interface language.
pub(crate) fn keys() -> Vec<TagKey> {
    vec![
        TagKey::new(TITLE_KEY, i18n::t(Key::AgentsColumnTitle)),
        TagKey::new(AGENT_KEY, i18n::t(Key::AgentsAgent)),
        TagKey::new(PROFILE_KEY, i18n::t(Key::AgentsColumnProfile)),
    ]
}

/// Every session of `project`, newest first, each with the name of its
/// agent from `agent_name`.
pub(crate) fn table_sessions(
    project: &Project,
    agent_name: impl Fn(AgentId) -> &'static str,
) -> Vec<TableSession<'_>> {
    project
        .sessions()
        .iter()
        .map(|entry| TableSession {
            entry,
            agent_name: agent_name(entry.profile().agent()),
        })
        .collect()
}

/// The schema of the table: the keys and every session in the order of
/// `sort`, whatever the search lets through.
pub(crate) fn schema(
    mut sessions: Vec<TableSession<'_>>,
    sort: SessionSort,
) -> TagSchema {
    sort_sessions(&mut sessions, sort);

    let rows = sessions.iter().map(TableSession::search_row).collect();
    TagSchema::new(keys(), rows)
}

/// The sessions `search` lets through, in the order of `sort`.
pub(crate) fn shown<'a>(
    sessions: Vec<TableSession<'a>>,
    search: &TagSearch,
    sort: SessionSort,
) -> Vec<TableSession<'a>> {
    let keys = keys();
    let mut shown: Vec<TableSession<'a>> = sessions
        .into_iter()
        .filter(|session| search.passes(&keys, &session.search_row()))
        .collect();

    sort_sessions(&mut shown, sort);
    shown
}

#[cfg(test)]
mod tests {
    use otty_ui_kit::components::tag_search::TagSearchAction;

    use super::*;
    use crate::testing::{claude, claude_work, codex, titled_session};

    /// One project in `/work/otty`: c1 "Fix tabs" and c2 "Add docs" of
    /// Claude Code, w1 "Fix docs" of Claude Code · Work, x1 "Bump deps" of
    /// Codex.
    fn project() -> Project {
        let claude_sessions = [
            titled_session("c1", "Fix tabs", 400),
            titled_session("c2", "Add docs", 100),
        ];
        let work_sessions = [titled_session("w1", "Fix docs", 300)];
        let codex_sessions = [titled_session("x1", "Bump deps", 200)];

        otty_agents::projects([
            (&claude(), &claude_sessions[..]),
            (&claude_work(), &work_sessions[..]),
            (&codex(), &codex_sessions[..]),
        ])
        .remove(0)
    }

    fn agent_name(agent: AgentId) -> &'static str {
        match agent.as_str() {
            "codex" => "Codex",
            _ => "Claude Code",
        }
    }

    /// The ids the table shows after `actions` in the search.
    fn shown_after(actions: &[TagSearchAction]) -> Vec<String> {
        let project = project();
        let sort = SessionSort::default();
        let schema = schema(table_sessions(&project, agent_name), sort);
        let mut table = SessionsTable::default();
        for action in actions {
            table.perform_search(project.folder(), action.clone(), &schema);
        }

        let search = table.search(project.folder());
        shown(table_sessions(&project, agent_name), search, sort)
            .iter()
            .map(|session| session.id().as_str().to_string())
            .collect()
    }

    /// The actions that add the tag `text`, as Enter does with no
    /// suggestion.
    fn tag(text: &str) -> Vec<TagSearchAction> {
        vec![
            TagSearchAction::InputChanged(text.to_string()),
            TagSearchAction::Close,
            TagSearchAction::Apply,
        ]
    }

    #[test]
    fn with_no_search_every_session_shows_newest_first() {
        assert_eq!(shown_after(&[]), ["c1", "w1", "x1", "c2"]);
    }

    #[test]
    fn typed_text_finds_the_titles_ignoring_case() {
        let typed = [TagSearchAction::InputChanged(String::from("DOCS"))];

        assert_eq!(shown_after(&typed), ["w1", "c2"]);
    }

    #[test]
    fn each_key_filters_by_the_text_of_its_column() {
        assert_eq!(shown_after(&tag("Agent:codex")), ["x1"]);
        assert_eq!(shown_after(&tag("Profile:work")), ["w1"]);
        assert_eq!(shown_after(&tag("Title:fix")), ["c1", "w1"]);
    }

    #[test]
    fn tags_of_two_keys_must_both_match() {
        let mut actions = tag("Agent:Claude");
        actions.extend(tag("Title:docs"));

        assert_eq!(shown_after(&actions), ["w1", "c2"]);
    }

    /// Owner, issue 34: with the Branch column gone, the search has no
    /// branch key either.
    #[test]
    fn the_search_has_a_key_for_each_text_column() {
        assert_eq!(
            keys(),
            vec![
                TagKey::new("title", i18n::t(Key::AgentsColumnTitle)),
                TagKey::new("agent", i18n::t(Key::AgentsAgent)),
                TagKey::new("profile", i18n::t(Key::AgentsColumnProfile)),
            ]
        );
    }

    #[test]
    fn each_project_keeps_its_own_search() {
        let project = project();
        let schema = schema(
            table_sessions(&project, agent_name),
            SessionSort::default(),
        );
        let mut table = SessionsTable::default();

        table.perform_search(
            project.folder(),
            TagSearchAction::InputChanged(String::from("fix")),
            &schema,
        );

        assert_eq!(table.search(project.folder()).input(), "fix");
        assert_eq!(table.search(Path::new("/work/site")).input(), "");
    }

    #[test]
    fn a_key_finds_its_session_again_and_no_other() {
        let project = project();
        let sessions = project.sessions();
        let key = SessionKey::of(&sessions[1]);

        let found: Vec<bool> =
            sessions.iter().map(|entry| key.is_of(entry)).collect();

        assert_eq!(found, [false, true, false, false]);
    }

    #[test]
    fn the_sort_is_shared_and_turns_on_a_second_press() {
        let mut table = SessionsTable::default();

        table.press_sort(crate::session_sort::TITLE_COLUMN);
        table.press_sort(crate::session_sort::TITLE_COLUMN);

        assert_eq!(
            table.sort(),
            SessionSort::default()
                .pressed(crate::session_sort::TITLE_COLUMN)
                .pressed(crate::session_sort::TITLE_COLUMN)
        );
    }
}
