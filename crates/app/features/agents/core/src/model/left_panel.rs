use std::path::PathBuf;
use std::time::SystemTime;

use otty_agents::{Profile, ProfileStatus, Project, SourceFailure};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::session_row::SessionTone;

use super::super::state::{AgentsState, ProfileState};
use super::limits::LimitsViewModel;
use super::{failure_reasons, folder_name};

/// The Agents mode left panel: the profiles, then the projects (spec
/// Decision 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeftPanelViewModel {
    /// The shown profiles, in their order.
    pub(crate) profiles: Vec<ProfileRowViewModel>,
    /// The Hidden profiles row and, while it is open, the hidden
    /// profiles; `None` while no profile is hidden.
    pub(crate) hidden: Option<HiddenProfilesViewModel>,
    /// Whether Add profile shows: once the profiles were read.
    pub(crate) can_add: bool,
    /// The projects, the latest first.
    pub(crate) projects: Vec<ProjectRowViewModel>,
    /// The text under the profiles while there is no project to list:
    /// they are being read, or there are none.
    pub(crate) note: Option<&'static str>,
}

impl LeftPanelViewModel {
    /// Build the left panel for the current state at `now`, the time the
    /// reset times count down from.
    pub(in super::super) fn from_state(
        state: &AgentsState,
        now: SystemTime,
    ) -> Self {
        let profiles = state
            .profiles()
            .iter()
            .map(|shown| ProfileRowViewModel::from_profile(state, shown, now))
            .collect();
        let projects: Vec<ProjectRowViewModel> = state
            .projects()
            .iter()
            .map(|project| ProjectRowViewModel::from_project(state, project))
            .collect();

        let is_reading = state.profiles().iter().any(ProfileState::is_reading);
        let note = if !state.is_loaded() {
            Some(i18n::t(Key::AgentsLoading))
        } else if projects.is_empty() && !is_reading {
            Some(i18n::t(Key::AgentsNoProjects))
        } else {
            None
        };

        let hidden =
            (!state.hidden().is_empty()).then(|| HiddenProfilesViewModel {
                label: i18n::with_count(
                    Key::TplAgentsHiddenProfiles,
                    state.hidden().len() as u64,
                ),
                is_open: state.is_hidden_list_open(),
                profiles: state
                    .hidden()
                    .iter()
                    .map(|profile| HiddenProfileViewModel {
                        icon: state.icon_of(profile.agent()),
                        profile: profile.clone(),
                    })
                    .collect(),
            });

        Self {
            profiles,
            hidden,
            can_add: state.is_loaded(),
            projects,
            note,
        }
    }
}

/// One profile: its agent's icon, its name and its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileRowViewModel {
    /// The profile, sent when the row is pressed.
    pub(crate) profile: Profile,
    /// Whether the profile is the one selected.
    pub(crate) is_selected: bool,
    /// The name of the agent's icon.
    pub(crate) icon: &'static str,
    /// The profile name.
    pub(crate) name: String,
    /// The state word: connected, not logged in, reading, ...
    pub(crate) state: String,
    /// The color of the state word.
    pub(crate) tone: SessionTone,
    /// Why the sessions could not be read, when the last read failed.
    pub(crate) failure: Option<String>,
    /// The limit bars under the row, or why there are none.
    pub(crate) limits: Option<LimitsViewModel>,
}

impl ProfileRowViewModel {
    fn from_profile(
        state: &AgentsState,
        shown: &ProfileState,
        now: SystemTime,
    ) -> Self {
        let profile = shown.profile();
        let (word, tone) = state_word(shown);
        let is_selected = state
            .selected_profile()
            .is_some_and(|selected| selected.profile() == profile);

        Self {
            profile: profile.clone(),
            is_selected,
            icon: state.icon_of(profile.agent()),
            name: profile.name().to_string(),
            state: i18n::t(word).to_string(),
            tone,
            failure: shown.failures().map(failure_text),
            limits: LimitsViewModel::from_limits(shown.limits(), now),
        }
    }
}

/// The Hidden profiles row and the hidden profiles under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HiddenProfilesViewModel {
    /// The row text with the count.
    pub(crate) label: String,
    /// Whether the hidden profiles are listed.
    pub(crate) is_open: bool,
    /// The hidden profiles, in their order.
    pub(crate) profiles: Vec<HiddenProfileViewModel>,
}

/// One hidden profile with Show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HiddenProfileViewModel {
    /// The name of the agent's icon.
    pub(crate) icon: &'static str,
    /// The profile, sent with Show.
    pub(crate) profile: Profile,
}

/// One project: its folder name, the path as a hint and its sessions count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectRowViewModel {
    /// The project folder, sent when the row is pressed.
    pub(crate) folder: PathBuf,
    /// The last part of the folder path.
    pub(crate) name: String,
    /// The whole folder path.
    pub(crate) path: String,
    /// The number of sessions of every profile in the folder.
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
            path: folder.to_string_lossy().into_owned(),
            sessions: project.sessions().len(),
            is_missing: state.is_missing(folder),
            is_selected: state.selected_folder() == Some(folder),
        }
    }
}

/// The state word of a shown profile and its color: while its sessions
/// are read, when that failed, else its status.
pub(super) fn state_word(shown: &ProfileState) -> (Key, SessionTone) {
    if shown.is_reading() {
        (Key::AgentsStateReading, SessionTone::Muted)
    } else if shown.failures().is_some() {
        (Key::AgentsStateReadFailed, SessionTone::Failed)
    } else {
        status_word(shown.status())
    }
}

/// The state word of a profile and its color.
fn status_word(status: &ProfileStatus) -> (Key, SessionTone) {
    match status {
        ProfileStatus::Connected { .. } => {
            (Key::AgentsStateConnected, SessionTone::Muted)
        },
        ProfileStatus::NotLoggedIn => {
            (Key::AgentsStateNotLoggedIn, SessionTone::Waiting)
        },
        ProfileStatus::ProgramNotFound => {
            (Key::AgentsStateProgramNotFound, SessionTone::Failed)
        },
        ProfileStatus::ConfigDirNotFound => {
            (Key::AgentsStateConfigDirNotFound, SessionTone::Failed)
        },
    }
}

/// The note of a failed read: each way of reading the sessions and why it
/// failed, such as "files: not available: no projects folder".
pub(super) fn failure_text(failures: &[SourceFailure]) -> String {
    i18n::with_error(Key::TplAgentsSessionsFailed, &failure_reasons(failures))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use otty_agents::ProfileStatus;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, claude, claude_work, codex, codex_work, connected, env,
        five_hour_limits, limits_answer, limits_loaded, no_files, now, read,
        session, two_agents_with_sessions, with_profiles,
    };
    use crate::types::LoadedSessions;

    fn states(vm: &LeftPanelViewModel) -> Vec<(&str, &str, SessionTone)> {
        vm.profiles
            .iter()
            .map(|row| (row.name.as_str(), row.state.as_str(), row.tone))
            .collect()
    }

    #[test]
    fn each_profile_shows_its_icon_name_and_state() {
        let mut state = with_profiles(&[
            (claude(), connected()),
            (codex(), ProfileStatus::NotLoggedIn),
            (claude_work(), ProfileStatus::ProgramNotFound),
            (codex_work(), ProfileStatus::ConfigDirNotFound),
        ]);
        read(&mut state, &claude(), Vec::new());
        read(&mut state, &codex(), Vec::new());
        read(&mut state, &claude_work(), Vec::new());

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(
            states(&vm),
            [
                (
                    "Claude Code",
                    i18n::t(Key::AgentsStateConnected),
                    SessionTone::Muted
                ),
                (
                    "Codex",
                    i18n::t(Key::AgentsStateNotLoggedIn),
                    SessionTone::Waiting
                ),
                (
                    "Claude Code · Work",
                    i18n::t(Key::AgentsStateProgramNotFound),
                    SessionTone::Failed
                ),
                (
                    "Codex · Work",
                    i18n::t(Key::AgentsStateConfigDirNotFound),
                    SessionTone::Failed
                ),
            ]
        );
        assert_eq!(vm.profiles[0].icon, "claude-code");
        assert_eq!(vm.profiles[1].icon, "codex");
        assert!(vm.profiles.iter().all(|row| row.failure.is_none()));
    }

    #[test]
    fn a_profile_says_while_its_sessions_are_read_and_when_that_failed() {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: codex(),
                sessions: Err(vec![no_files()]),
                missing_folders: Vec::new(),
            }),
        );

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(vm.profiles[0].state, i18n::t(Key::AgentsStateReading));
        assert_eq!(vm.profiles[1].state, i18n::t(Key::AgentsStateReadFailed));
        assert_eq!(vm.profiles[1].tone, SessionTone::Failed);
        assert_eq!(
            vm.profiles[1].failure.as_deref(),
            Some(
                i18n::with_error(
                    Key::TplAgentsSessionsFailed,
                    "files: not available: no projects folder"
                )
                .as_str()
            )
        );
    }

    #[test]
    fn a_profile_with_limits_has_their_bars_under_its_row() {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);
        let _ = reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![("server", Ok(five_hour_limits(42.0)))]),
                now(),
            ),
        );

        let vm = LeftPanelViewModel::from_state(&state, now());

        let bars = vm.profiles[0]
            .limits
            .as_ref()
            .map(|limits| limits.bars.len());
        assert_eq!(bars, Some(1));
        assert_eq!(vm.profiles[1].limits, None, "nothing was read yet");
    }

    #[test]
    fn a_selected_profile_is_marked_and_hidden_ones_are_counted() {
        let mut state = with_profiles(&[
            (claude(), connected()),
            (codex(), connected()),
            (codex_work().with_hidden(true), connected()),
        ]);
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));
        let _ = reduce(&mut state, AgentsIntent::ToggleHiddenProfiles);

        let vm = LeftPanelViewModel::from_state(&state, now());

        let selected: Vec<_> =
            vm.profiles.iter().map(|row| row.is_selected).collect();
        assert_eq!(selected, [false, true]);
        assert_eq!(vm.profiles[1].profile, codex());
        assert!(vm.can_add);
        assert_eq!(
            vm.hidden,
            Some(HiddenProfilesViewModel {
                label: i18n::with_count(Key::TplAgentsHiddenProfiles, 1),
                is_open: true,
                profiles: vec![HiddenProfileViewModel {
                    icon: "codex",
                    profile: codex_work().with_hidden(true),
                }],
            })
        );
    }

    #[test]
    fn with_no_hidden_profile_there_is_no_hidden_row_and_before_a_read_no_add()
    {
        let mut state = crate::state::AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(vm.hidden, None);
        assert!(!vm.can_add);
    }

    #[test]
    fn projects_show_their_name_path_count_and_selection() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        );

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(
            vm.projects,
            vec![
                ProjectRowViewModel {
                    folder: PathBuf::from("/work/site"),
                    name: String::from("site"),
                    path: String::from("/work/site"),
                    sessions: 1,
                    is_missing: false,
                    is_selected: false,
                },
                ProjectRowViewModel {
                    folder: PathBuf::from("/work/otty"),
                    name: String::from("otty"),
                    path: String::from("/work/otty"),
                    sessions: 2,
                    is_missing: false,
                    is_selected: true,
                },
            ]
        );
        assert_eq!(vm.note, None);
    }

    #[test]
    fn a_project_whose_folder_is_gone_is_marked_missing() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: claude(),
                sessions: Ok(vec![
                    session("c1", "/work/gone", 2),
                    session("c2", "/work/otty", 1),
                ]),
                missing_folders: vec![PathBuf::from("/work/gone")],
            }),
        );

        let vm = LeftPanelViewModel::from_state(&state, now());

        let missing: Vec<_> = vm
            .projects
            .iter()
            .map(|row| (Path::new(&row.path), row.is_missing))
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
        let mut loading = crate::state::AgentsState::new(agents(), env());
        let _ = reduce(&mut loading, AgentsIntent::Enter);
        let mut empty = with_profiles(&[(claude(), connected())]);
        let reading = with_profiles(&[(claude(), connected())]);
        read(&mut empty, &claude(), Vec::new());

        assert_eq!(
            LeftPanelViewModel::from_state(&loading, now()).note,
            Some(i18n::t(Key::AgentsLoading))
        );
        assert_eq!(
            LeftPanelViewModel::from_state(&empty, now()).note,
            Some(i18n::t(Key::AgentsNoProjects))
        );
        assert_eq!(LeftPanelViewModel::from_state(&reading, now()).note, None);
    }
}
