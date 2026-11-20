//! Which projects the left panel lists while the search field is open: the
//! text typed in the field and the choices of the funnel menu (spec
//! Decision 6).

use std::path::Path;
use std::time::{Duration, SystemTime};

use otty_agents::{AgentId, Project};

/// How long ago a project's latest session may have changed to count as
/// used lately.
const RECENT: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// The choices of the funnel menu. With none on, every project shows,
/// the latest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProjectFilter {
    agent: Option<AgentId>,
    is_recent_only: bool,
    is_missing_only: bool,
    is_sorted_by_name: bool,
}

impl ProjectFilter {
    /// The agent whose projects show; `None` for every agent.
    pub(crate) fn agent(&self) -> Option<&AgentId> {
        self.agent.as_ref()
    }

    /// Whether only the projects used in the last 30 days show.
    pub(crate) fn is_recent_only(&self) -> bool {
        self.is_recent_only
    }

    /// Whether only the projects whose folder is gone show.
    pub(crate) fn is_missing_only(&self) -> bool {
        self.is_missing_only
    }

    /// Whether the projects are sorted by name rather than by their latest
    /// session.
    pub(crate) fn is_sorted_by_name(&self) -> bool {
        self.is_sorted_by_name
    }

    /// Whether any choice is on, so the funnel is drawn in the accent color.
    pub(crate) fn is_any_on(&self) -> bool {
        *self != Self::default()
    }

    /// Take a choice of the funnel menu or the cross of a chip.
    pub(crate) fn apply(&mut self, change: ProjectFilterChange) {
        match change {
            ProjectFilterChange::Agent(agent) => self.agent = agent,
            ProjectFilterChange::RecentOnly(is_on) => {
                self.is_recent_only = is_on;
            },
            ProjectFilterChange::MissingOnly(is_on) => {
                self.is_missing_only = is_on;
            },
            ProjectFilterChange::SortByName(is_on) => {
                self.is_sorted_by_name = is_on;
            },
            ProjectFilterChange::ClearAll => *self = Self::default(),
        }
    }
}

/// A choice of the funnel menu, or the cross of a chip that turns one off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectFilterChange {
    /// Show the projects of every agent (`None`) or of this agent.
    Agent(Option<AgentId>),
    /// Show only the projects used in the last 30 days, or all of them.
    RecentOnly(bool),
    /// Show only the projects whose folder is gone, or all of them.
    MissingOnly(bool),
    /// Sort the projects by name, or by their latest session.
    SortByName(bool),
    /// Turn every choice off.
    ClearAll,
}

/// The projects that match `query` and `filter`, in the order to show
/// them. `projects` come the latest first, and keep that order unless the
/// filter sorts them by name.
///
/// The query matches any part of the folder path, its name too, whatever
/// the case of the letters; spaces around it do not count. `is_missing`
/// tells whether a project folder is gone, and `now` is the time the last
/// 30 days count back from.
pub(crate) fn filter_projects<'a>(
    projects: &'a [Project],
    query: &str,
    filter: &ProjectFilter,
    is_missing: impl Fn(&Path) -> bool,
    now: SystemTime,
) -> Vec<&'a Project> {
    let query = query.trim().to_lowercase();

    let mut shown: Vec<&Project> = projects
        .iter()
        .filter(|project| matches_query(project, &query))
        .filter(|project| {
            filter
                .agent
                .as_ref()
                .is_none_or(|agent| has_agent(project, agent))
        })
        .filter(|project| !filter.is_recent_only || is_recent(project, now))
        .filter(|project| {
            !filter.is_missing_only || is_missing(project.folder())
        })
        .collect();

    if filter.is_sorted_by_name {
        shown.sort_by_key(|project| name_key(project.folder()));
    }
    shown
}

/// The number of sessions the row of `project` counts: those of the agent
/// the filter picks, or all of them.
pub(crate) fn shown_sessions(
    project: &Project,
    filter: &ProjectFilter,
) -> usize {
    project
        .sessions()
        .iter()
        .filter(|entry| {
            filter
                .agent
                .as_ref()
                .is_none_or(|agent| entry.profile().agent() == agent)
        })
        .count()
}

/// Whether the folder path of `project` holds `query`, already in lower
/// case; an empty query matches every project.
fn matches_query(project: &Project, query: &str) -> bool {
    let path = project.folder().to_string_lossy().to_lowercase();

    path.contains(query)
}

/// Whether `project` has a session of a profile of `agent`.
fn has_agent(project: &Project, agent: &AgentId) -> bool {
    project
        .sessions()
        .iter()
        .any(|entry| entry.profile().agent() == agent)
}

/// Whether the latest session of `project` changed in the 30 days before
/// `now`. A time after `now` counts as recent.
fn is_recent(project: &Project, now: SystemTime) -> bool {
    match now.duration_since(project.changed_at()) {
        Ok(age) => age <= RECENT,
        Err(_) => true,
    }
}

/// What a project sorts by: its folder name in lower case, then its whole
/// path, so two folders of the same name keep one order.
fn name_key(folder: &Path) -> (String, String) {
    let name = folder.file_name().unwrap_or(folder.as_os_str());

    (
        name.to_string_lossy().to_lowercase(),
        folder.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{AgentSession, Profile};

    use super::*;
    use crate::testing::{at, claude, claude_id, codex, codex_id, session};

    const DAY: u64 = 24 * 60 * 60;

    /// The time the tests filter at: day 100.
    fn today() -> SystemTime {
        at(100 * DAY)
    }

    /// Projects of Claude Code and Codex:
    ///
    /// - `/work/otty`: Claude Code and Codex, changed today;
    /// - `/home/user/GovLake`: Claude Code, 10 days ago;
    /// - `/work/data-lakehouse-docs`: Codex, 40 days ago;
    /// - `/work/alpha`: Claude Code, 50 days ago, its folder gone.
    fn sample() -> Vec<Project> {
        let claude_sessions = [
            session("c1", "/work/otty", 100 * DAY - 60),
            session("c2", "/home/user/GovLake", 90 * DAY),
            session("c3", "/work/alpha", 50 * DAY),
        ];
        let codex_sessions = [
            session("x1", "/work/otty", 100 * DAY - 30),
            session("x2", "/work/data-lakehouse-docs", 60 * DAY),
        ];
        let read: [(Profile, &[AgentSession]); 2] =
            [(claude(), &claude_sessions), (codex(), &codex_sessions)];

        otty_agents::projects(
            read.iter().map(|(profile, sessions)| (profile, *sessions)),
        )
    }

    fn is_alpha(folder: &Path) -> bool {
        folder == Path::new("/work/alpha")
    }

    fn names(projects: &[&Project]) -> Vec<String> {
        projects
            .iter()
            .map(|project| {
                project
                    .folder()
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .collect()
    }

    fn filtered(query: &str, filter: &ProjectFilter) -> Vec<String> {
        let projects = sample();

        names(&filter_projects(
            &projects,
            query,
            filter,
            is_alpha,
            today(),
        ))
    }

    fn with(changes: &[ProjectFilterChange]) -> ProjectFilter {
        let mut filter = ProjectFilter::default();
        for change in changes {
            filter.apply(change.clone());
        }

        filter
    }

    #[test]
    fn with_no_text_and_no_choice_every_project_shows_the_latest_first() {
        assert_eq!(
            filtered("", &ProjectFilter::default()),
            ["otty", "GovLake", "data-lakehouse-docs", "alpha"]
        );
    }

    #[test]
    fn the_text_matches_the_folder_name_whatever_the_case() {
        assert_eq!(
            filtered("LAKE", &ProjectFilter::default()),
            ["GovLake", "data-lakehouse-docs"]
        );
        assert_eq!(filtered("  otty ", &ProjectFilter::default()), ["otty"]);
    }

    #[test]
    fn the_text_matches_any_part_of_the_path() {
        assert_eq!(
            filtered("home/user", &ProjectFilter::default()),
            ["GovLake"]
        );
        assert_eq!(
            filtered("/work/", &ProjectFilter::default()),
            ["otty", "data-lakehouse-docs", "alpha"]
        );
        assert!(filtered("nothing", &ProjectFilter::default()).is_empty());
    }

    #[test]
    fn an_agent_keeps_the_projects_it_has_sessions_in() {
        let codex_only = with(&[ProjectFilterChange::Agent(Some(codex_id()))]);
        let claude_only =
            with(&[ProjectFilterChange::Agent(Some(claude_id()))]);

        assert_eq!(filtered("", &codex_only), ["otty", "data-lakehouse-docs"]);
        assert_eq!(filtered("", &claude_only), ["otty", "GovLake", "alpha"]);
    }

    #[test]
    fn the_last_30_days_keep_the_projects_used_lately() {
        let recent = with(&[ProjectFilterChange::RecentOnly(true)]);

        assert_eq!(filtered("", &recent), ["otty", "GovLake"]);
    }

    #[test]
    fn folder_gone_keeps_the_projects_whose_folder_is_missing() {
        let missing = with(&[ProjectFilterChange::MissingOnly(true)]);

        assert_eq!(filtered("", &missing), ["alpha"]);
    }

    #[test]
    fn sort_by_name_ignores_the_case_of_the_letters() {
        let by_name = with(&[ProjectFilterChange::SortByName(true)]);

        assert_eq!(
            filtered("", &by_name),
            ["alpha", "data-lakehouse-docs", "GovLake", "otty"]
        );
    }

    #[test]
    fn the_text_and_the_choices_work_together() {
        let filter = with(&[
            ProjectFilterChange::Agent(Some(claude_id())),
            ProjectFilterChange::RecentOnly(true),
            ProjectFilterChange::SortByName(true),
        ]);

        assert_eq!(filtered("", &filter), ["GovLake", "otty"]);
        assert_eq!(filtered("ott", &filter), ["otty"]);
    }

    #[test]
    fn a_choice_turns_off_and_clear_all_turns_off_every_choice() {
        let mut filter = with(&[
            ProjectFilterChange::Agent(Some(codex_id())),
            ProjectFilterChange::RecentOnly(true),
            ProjectFilterChange::MissingOnly(true),
            ProjectFilterChange::SortByName(true),
        ]);
        assert!(filter.is_any_on());
        assert_eq!(filter.agent(), Some(&codex_id()));
        assert!(filter.is_recent_only());
        assert!(filter.is_missing_only());
        assert!(filter.is_sorted_by_name());

        filter.apply(ProjectFilterChange::RecentOnly(false));
        assert!(!filter.is_recent_only());
        assert!(filter.is_any_on());

        filter.apply(ProjectFilterChange::ClearAll);
        assert_eq!(filter, ProjectFilter::default());
        assert!(!filter.is_any_on());
        assert_eq!(
            filtered("", &filter),
            ["otty", "GovLake", "data-lakehouse-docs", "alpha"]
        );
    }

    #[test]
    fn a_row_counts_the_sessions_of_the_chosen_agent() {
        let projects = sample();
        let otty = &projects[0];
        assert_eq!(otty.folder(), PathBuf::from("/work/otty"));

        let all = shown_sessions(otty, &ProjectFilter::default());
        let codex_only = shown_sessions(
            otty,
            &with(&[ProjectFilterChange::Agent(Some(codex_id()))]),
        );

        assert_eq!(all, 2);
        assert_eq!(codex_only, 1);
    }
}
