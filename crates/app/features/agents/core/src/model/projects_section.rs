use std::path::{Path, PathBuf};
use std::time::SystemTime;

use otty_agents::Project;
use otty_app_i18n::{self as i18n, Key};

use super::super::menus::AgentsMenu;
use super::super::profile_state::ProfileState;
use super::super::project_filter::{ProjectFilterChange, shown_sessions};
use super::super::state::AgentsState;
use super::folder_name;

/// The Projects section: the search field while it is open, then a row for
/// each project it lets through (spec Decision 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectsSectionViewModel<'a> {
    /// Whether the section shows its rows.
    pub(crate) is_open: bool,
    /// The number of every project, shown after the title while the
    /// section is closed.
    pub(crate) count: usize,
    /// The search field, its chips and its note; `None` while it is
    /// closed.
    pub(crate) search: Option<ProjectSearchViewModel<'a>>,
    /// The projects the search lets through, in their order.
    pub(crate) projects: Vec<ProjectRowViewModel>,
    /// The text in place of the rows when there is no project at all.
    pub(crate) note: Option<&'static str>,
}

impl<'a> ProjectsSectionViewModel<'a> {
    /// Build the section for the current state at `now`, the time the last
    /// 30 days of the filter count back from.
    pub(in super::super) fn from_state(
        state: &'a AgentsState,
        now: SystemTime,
    ) -> Self {
        let search = state.search();
        let all = state.profiles().projects();
        let is_missing = |folder: &Path| state.profiles().is_missing(folder);
        let shown = search.shown(all, is_missing, now);

        let projects: Vec<ProjectRowViewModel> = shown
            .iter()
            .map(|project| ProjectRowViewModel::from_project(state, project))
            .collect();
        let is_reading = state
            .profiles()
            .shown()
            .iter()
            .any(ProfileState::is_reading_sessions);
        let note =
            (state.profiles().is_loaded() && all.is_empty() && !is_reading)
                .then(|| i18n::t(Key::AgentsNoProjects));
        let search_vm = search.is_open().then(|| {
            ProjectSearchViewModel::from_state(state, shown.len(), all.len())
        });

        Self {
            is_open: state.is_projects_open(),
            count: all.len(),
            search: search_vm,
            projects,
            note,
        }
    }
}

/// The search field under the Projects header: its text, the chips of the
/// filters that are on, and the note under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectSearchViewModel<'a> {
    /// The text typed in the field.
    pub(crate) query: &'a str,
    /// One chip for each filter that is on.
    pub(crate) chips: Vec<FilterChipViewModel>,
    /// Whether the funnel is drawn in the accent color: a filter is on or
    /// its menu is open.
    pub(crate) is_funnel_lit: bool,
    /// How many projects match, or what to type.
    pub(crate) note: String,
}

impl<'a> ProjectSearchViewModel<'a> {
    /// The open field of `state`, with `shown` of the `total` projects let
    /// through.
    fn from_state(state: &'a AgentsState, shown: usize, total: usize) -> Self {
        let search = state.search();
        let filter = search.filter();

        let note = if !search.query().is_empty() {
            i18n::agents_projects_match(shown, total)
        } else if filter.is_any_on() {
            i18n::agents_projects_shown(shown, total)
        } else {
            i18n::t(Key::AgentsSearchHint).to_string()
        };
        let is_menu_open = state
            .menu()
            .is_some_and(|open| *open.menu() == AgentsMenu::ProjectFilter);

        Self {
            query: search.query(),
            chips: chips(state),
            is_funnel_lit: filter.is_any_on() || is_menu_open,
            note,
        }
    }
}

/// A filter that is on, with the change its cross sends to turn it off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FilterChipViewModel {
    /// The text of the chip, such as "Last 30 days".
    pub(crate) label: String,
    /// What the cross sends.
    pub(crate) clear: ProjectFilterChange,
}

/// One project: its folder name and the number of its sessions, or that
/// its folder is gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectRowViewModel {
    /// The project folder, sent when the row is pressed.
    pub(crate) folder: PathBuf,
    /// The last part of the folder path.
    pub(crate) name: String,
    /// The number of sessions the row counts: of the agent the filter
    /// picks, or of every profile.
    pub(crate) sessions: usize,
    /// Whether the folder no longer exists.
    pub(crate) is_missing: bool,
    /// Whether this is the selected project.
    pub(crate) is_selected: bool,
}

impl ProjectRowViewModel {
    fn from_project(state: &AgentsState, project: &Project) -> Self {
        let folder = project.folder();

        Self {
            folder: folder.to_path_buf(),
            name: folder_name(folder),
            sessions: shown_sessions(project, state.search().filter()),
            is_missing: state.profiles().is_missing(folder),
            is_selected: state.selected_folder() == Some(folder),
        }
    }
}

/// One chip for each filter that is on, in the order of the funnel menu,
/// each with the change its cross sends.
fn chips(state: &AgentsState) -> Vec<FilterChipViewModel> {
    let filter = state.search().filter();
    let mut chips = Vec::new();

    if let Some(agent) = filter.agent() {
        let name = state
            .agent(agent)
            .map_or(agent.as_str(), |agent| agent.descriptor().name());
        chips.push(FilterChipViewModel {
            label: i18n::agents_named(Key::TplAgentsChipAgentOnly, name),
            clear: ProjectFilterChange::Agent(None),
        });
    }
    let toggles = [
        (
            filter.is_recent_only(),
            Key::AgentsChipRecent,
            ProjectFilterChange::RecentOnly(false),
        ),
        (
            filter.is_missing_only(),
            Key::AgentsChipMissing,
            ProjectFilterChange::MissingOnly(false),
        ),
        (
            filter.is_sorted_by_name(),
            Key::AgentsChipSortedByName,
            ProjectFilterChange::SortByName(false),
        ),
    ];
    for (is_on, label, clear) in toggles {
        if is_on {
            chips.push(FilterChipViewModel {
                label: i18n::t(label).to_string(),
                clear,
            });
        }
    }

    chips
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::menus::MenuPlace;
    use crate::reducer::reduce;
    use crate::testing::{
        CODEX, agents, claude, connected, env, now, read, session,
        two_agents_with_sessions, with_profiles,
    };
    use crate::types::LoadedSessions;

    fn section(state: &AgentsState) -> ProjectsSectionViewModel<'_> {
        ProjectsSectionViewModel::from_state(state, now())
    }

    fn search(state: &mut AgentsState, query: &str) {
        let _ = reduce(state, AgentsIntent::ToggleProjectSearch);
        let _ = reduce(
            state,
            AgentsIntent::ProjectSearchChanged(String::from(query)),
        );
    }

    fn filter(state: &mut AgentsState, change: ProjectFilterChange) {
        let _ = reduce(state, AgentsIntent::ProjectFilter(change));
    }

    fn rows<'a>(vm: &'a ProjectsSectionViewModel<'_>) -> Vec<(&'a str, usize)> {
        vm.projects
            .iter()
            .map(|row| (row.name.as_str(), row.sessions))
            .collect()
    }

    #[test]
    fn projects_show_their_name_count_and_selection() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        );

        let vm = section(&state);

        assert_eq!(
            vm.projects,
            vec![
                ProjectRowViewModel {
                    folder: PathBuf::from("/work/site"),
                    name: String::from("site"),
                    sessions: 1,
                    is_missing: false,
                    is_selected: false,
                },
                ProjectRowViewModel {
                    folder: PathBuf::from("/work/otty"),
                    name: String::from("otty"),
                    sessions: 2,
                    is_missing: false,
                    is_selected: true,
                },
            ]
        );
        assert!(vm.is_open);
        assert_eq!(vm.count, 2);
        assert_eq!(vm.search, None);
        assert_eq!(vm.note, None);
    }

    #[test]
    fn a_project_whose_folder_is_gone_is_marked() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: claude(),
                sessions: crate::types::SessionsRead::Read(vec![
                    session("c1", "/work/gone", 2),
                    session("c2", "/work/otty", 1),
                ]),
                missing_folders: vec![PathBuf::from("/work/gone")],
            }),
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/gone")),
        );

        let vm = section(&state);

        let missing: Vec<_> = vm
            .projects
            .iter()
            .map(|row| (row.folder.as_path(), row.is_missing))
            .collect();
        assert_eq!(
            missing,
            [
                (Path::new("/work/gone"), true),
                (Path::new("/work/otty"), false)
            ]
        );
    }

    #[test]
    fn the_note_says_what_is_read_or_that_there_are_no_projects() {
        let mut loading = AgentsState::new(agents(), env());
        let _ = reduce(&mut loading, AgentsIntent::Enter);
        let mut empty = with_profiles(&[(claude(), connected())]);
        let reading = with_profiles(&[(claude(), connected())]);
        read(&mut empty, &claude(), Vec::new());

        assert_eq!(section(&loading).note, None);
        assert_eq!(section(&empty).note, Some(i18n::t(Key::AgentsNoProjects)));
        assert_eq!(section(&reading).note, None);
    }

    #[test]
    fn the_open_field_says_what_to_type_then_how_many_projects_match() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::ToggleProjectSearch);

        let empty = section(&state).search.expect("the field is open");
        assert_eq!(empty.note, i18n::t(Key::AgentsSearchHint));
        assert!(empty.chips.is_empty());
        assert!(!empty.is_funnel_lit);

        search_text(&mut state, "OTT");
        let typed = section(&state);

        assert_eq!(rows(&typed), [("otty", 2)]);
        assert_eq!(
            typed.search.map(|search| (search.query, search.note)),
            Some(("OTT", i18n::agents_projects_match(1, 2)))
        );
        assert_eq!(typed.count, 2);
    }

    fn search_text(state: &mut AgentsState, query: &str) {
        let _ = reduce(
            state,
            AgentsIntent::ProjectSearchChanged(String::from(query)),
        );
    }

    #[test]
    fn each_filter_that_is_on_is_a_chip_and_lights_the_funnel() {
        let mut state = two_agents_with_sessions();
        search(&mut state, "");
        for change in [
            ProjectFilterChange::Agent(Some(CODEX)),
            ProjectFilterChange::RecentOnly(true),
            ProjectFilterChange::MissingOnly(true),
            ProjectFilterChange::SortByName(true),
        ] {
            filter(&mut state, change);
        }

        let vm = section(&state).search.expect("the field is open");

        assert_eq!(
            vm.chips,
            vec![
                FilterChipViewModel {
                    label: i18n::agents_named(
                        Key::TplAgentsChipAgentOnly,
                        "Codex"
                    ),
                    clear: ProjectFilterChange::Agent(None),
                },
                FilterChipViewModel {
                    label: i18n::t(Key::AgentsChipRecent).to_string(),
                    clear: ProjectFilterChange::RecentOnly(false),
                },
                FilterChipViewModel {
                    label: i18n::t(Key::AgentsChipMissing).to_string(),
                    clear: ProjectFilterChange::MissingOnly(false),
                },
                FilterChipViewModel {
                    label: i18n::t(Key::AgentsChipSortedByName).to_string(),
                    clear: ProjectFilterChange::SortByName(false),
                },
            ]
        );
        assert!(vm.is_funnel_lit);
        assert_eq!(vm.note, i18n::agents_projects_shown(0, 2));
    }

    #[test]
    fn an_agent_filter_counts_only_its_sessions() {
        let mut state = two_agents_with_sessions();
        search(&mut state, "");

        filter(&mut state, ProjectFilterChange::Agent(Some(CODEX)));

        assert_eq!(rows(&section(&state)), [("otty", 1)]);
    }

    #[test]
    fn the_open_funnel_menu_lights_the_funnel() {
        let mut state = two_agents_with_sessions();
        search(&mut state, "");
        state.set_pointer(MenuPlace::new(
            Point::ORIGIN,
            Size::new(800.0, 600.0),
        ));

        let _ = reduce(&mut state, AgentsIntent::OpenFilterMenu);

        let vm = section(&state).search.expect("the field is open");
        assert!(vm.is_funnel_lit);
    }

    #[test]
    fn a_closed_section_still_counts_its_projects() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::ToggleProjectsSection);

        let vm = section(&state);

        assert!(!vm.is_open);
        assert_eq!(vm.count, 2);
    }
}
