//! What the New session dialog does: it opens on a folder, looks the
//! typed folder up, proposes a profile and starts the agent tab (spec
//! Decisions 3 and 15).

use std::path::{Path, PathBuf};

use otty_agents::Profile;

use super::event::NewSessionDialogIntent;
use super::state::{FolderState, NewSessionDialog};
use crate::event::AgentsOutcome;
use crate::folder_suggestions::shown_path;
use crate::session_actions;
use crate::state::{AgentsState, ProfileState};
use crate::types::LoadedFolder;

/// Open the New session dialog on `folder`; with none, on the folder of
/// the focused terminal tab on this computer, else the home folder. The
/// folder is looked up and the field takes the keyboard.
pub(crate) fn open(
    state: &mut AgentsState,
    folder: Option<PathBuf>,
) -> Vec<AgentsOutcome> {
    let home = state.env().home().to_path_buf();
    let folder = folder
        .or_else(|| state.local_folder().map(Path::to_path_buf))
        .unwrap_or_else(|| home.clone());

    let dialog = NewSessionDialog::new(shown_path(&folder, &home));
    let mut outcomes = dialog.folder().look_up(&home);
    let _ = state.close_menu();
    state.open_new_session(dialog);

    outcomes.push(AgentsOutcome::FocusNewSessionFolder);
    outcomes
}

/// Do what the user did in the open New session dialog.
pub(crate) fn reduce(
    state: &mut AgentsState,
    intent: NewSessionDialogIntent,
) -> Vec<AgentsOutcome> {
    let home = state.env().home().to_path_buf();

    match intent {
        NewSessionDialogIntent::FolderChanged(text) => {
            let Some(dialog) = state.new_session_mut() else {
                return Vec::new();
            };

            dialog.set_text(text, &home);
            dialog.folder().look_up(&home)
        },
        NewSessionDialogIntent::SuggestionPicked(index) => {
            let Some(dialog) = state.new_session_mut() else {
                return Vec::new();
            };
            if !dialog.pick(index, &home) {
                return Vec::new();
            }

            dialog.folder().look_up(&home)
        },
        NewSessionDialogIntent::SuggestionHighlighted(row) => {
            if let Some(dialog) = state.new_session_mut() {
                dialog.folder_mut().highlight(row);
            }

            Vec::new()
        },
        NewSessionDialogIntent::SuggestionsClosed => {
            if let Some(dialog) = state.new_session_mut() {
                dialog.folder_mut().close();
            }

            Vec::new()
        },
        NewSessionDialogIntent::ProfilePicked(profile) => {
            state.keep_new_session_profile(profile);
            Vec::new()
        },
        NewSessionDialogIntent::Submitted => start(state, &home),
        NewSessionDialogIntent::Cancelled => {
            let _ = state.close_new_session();
            Vec::new()
        },
    }
}

/// Take the answer about the typed folder, while the dialog is open and
/// the text is still the one asked about.
pub(crate) fn apply_lookup(state: &mut AgentsState, loaded: LoadedFolder) {
    let home = state.env().home().to_path_buf();
    let Some(dialog) = state.new_session_mut() else {
        return;
    };

    if !dialog.folder_mut().apply(loaded, &home) {
        log::debug!("the answer about a folder typed before was dropped");
    }
}

/// The profile a new session starts with: the one the user picked last,
/// else the selected profile, else the first one; each only while it can
/// start. `None` while no profile can start.
pub(crate) fn new_session_profile(state: &AgentsState) -> Option<&Profile> {
    let kept = state
        .kept_new_session_profile()
        .filter(|profile| state.can_start(profile));
    let selected = state
        .selected_profile()
        .map(ProfileState::profile)
        .filter(|profile| state.can_start(profile));

    kept.or(selected)
        .or_else(|| state.startable_profiles().first().copied())
}

/// Start the agent tab in the typed folder with the proposed profile, as
/// New session of a project screen does, and close the dialog. Nothing
/// happens while the folder is not found or no profile can start. The
/// sessions of the profile are read again the next time the mode opens, so
/// the folder shows as a project once the agent wrote its first session.
fn start(state: &mut AgentsState, home: &Path) -> Vec<AgentsOutcome> {
    let folder_state =
        state.new_session().map(|dialog| dialog.folder_state(home));
    let Some(FolderState::Found(folder)) = folder_state else {
        return Vec::new();
    };
    let Some(profile) = new_session_profile(state).cloned() else {
        return Vec::new();
    };

    let outcomes = session_actions::open_tab(state, &profile, None, &folder);
    if outcomes.is_empty() {
        return outcomes;
    }

    let _ = state.close_new_session();
    state.keep_new_session_profile(profile.clone());
    state.read_sessions_later(profile);
    outcomes
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{AgentCommand, ProfileStatus};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        claude, claude_work, codex, connected, fresh_profiles_loaded, read,
        session, with_profiles,
    };
    use crate::types::{FolderAsker, FolderQuery, LoadedFolder};

    fn dialog(intent: NewSessionDialogIntent) -> AgentsIntent {
        AgentsIntent::NewSessionDialog(intent)
    }

    fn query(folder: &str, list_in: &str, prefix: &str) -> FolderQuery {
        FolderQuery {
            asker: FolderAsker::NewSessionDialog,
            folder: PathBuf::from(folder),
            list_in: PathBuf::from(list_in),
            prefix: String::from(prefix),
        }
    }

    /// The answer that `query` is a folder with `suggestions`, or none.
    fn looked_up(
        query: FolderQuery,
        is_folder: bool,
        suggestions: &[&str],
    ) -> AgentsIntent {
        AgentsIntent::FolderLookedUp(LoadedFolder {
            query,
            is_folder,
            suggestions: suggestions.iter().map(PathBuf::from).collect(),
        })
    }

    /// Claude Code · Work and Codex can start; nothing is selected but the
    /// first profile.
    fn two_profiles() -> AgentsState {
        with_profiles(&[(claude_work(), connected()), (codex(), connected())])
    }

    /// The dialog open on `/work/new`, which the answer says is a folder.
    fn open_on_a_new_folder(state: &mut AgentsState) {
        let _ = reduce(
            state,
            AgentsIntent::OpenNewSessionDialog(Some(PathBuf::from(
                "/work/new",
            ))),
        );
        let _ = reduce(
            state,
            looked_up(query("/work/new", "/work", "new"), true, &[]),
        );
    }

    fn open_tab(
        title: &str,
        folder: &str,
        command: AgentCommand,
    ) -> AgentsOutcome {
        AgentsOutcome::OpenTab {
            title: String::from(title),
            folder: PathBuf::from(folder),
            command,
        }
    }

    #[test]
    fn the_dialog_opens_on_the_given_folder_and_looks_it_up() {
        let mut state = two_profiles();

        let outcomes = reduce(
            &mut state,
            AgentsIntent::OpenNewSessionDialog(Some(PathBuf::from(
                "/home/user/projects/otty",
            ))),
        );

        let dialog = state.new_session().expect("the dialog is open");
        assert_eq!(dialog.text(), "~/projects/otty");
        assert_eq!(
            outcomes,
            vec![
                AgentsOutcome::LookUpFolder(query(
                    "/home/user/projects/otty",
                    "/home/user/projects",
                    "otty",
                )),
                AgentsOutcome::FocusNewSessionFolder,
            ]
        );
    }

    /// The "+" of the Projects section gives no folder: the folder of the
    /// focused terminal tab on this computer, else the home folder.
    #[test]
    fn with_no_folder_the_dialog_opens_on_the_terminal_folder_else_home() {
        let mut state = two_profiles();
        let _ = reduce(&mut state, AgentsIntent::OpenNewSessionDialog(None));
        let home = state.new_session().map(|dialog| dialog.text().to_string());
        let _ = reduce(&mut state, dialog(NewSessionDialogIntent::Cancelled));
        state.set_local_folder(Some(PathBuf::from("/srv/app")));

        let _ = reduce(&mut state, AgentsIntent::OpenNewSessionDialog(None));

        assert_eq!(home.as_deref(), Some("~"));
        assert_eq!(
            state.new_session().map(|dialog| dialog.text()),
            Some("/srv/app")
        );
    }

    #[test]
    fn typing_looks_the_new_text_up() {
        let mut state = two_profiles();
        open_on_a_new_folder(&mut state);

        let outcomes = reduce(
            &mut state,
            dialog(NewSessionDialogIntent::FolderChanged(String::from(
                "~/pro",
            ))),
        );
        let relative = reduce(
            &mut state,
            dialog(NewSessionDialogIntent::FolderChanged(String::from("pro"))),
        );

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::LookUpFolder(query(
                "/home/user/pro",
                "/home/user",
                "pro",
            ))]
        );
        assert!(relative.is_empty(), "a relative path is not looked up");
    }

    /// Owner, issue 36: Start opens the agent tab in the folder exactly as
    /// New session of a project screen does: the same command and the
    /// config folder variable of an added profile.
    #[test]
    fn start_opens_the_agent_tab_in_the_folder_with_the_chosen_profile() {
        let mut state = two_profiles();
        open_on_a_new_folder(&mut state);

        let started =
            reduce(&mut state, dialog(NewSessionDialogIntent::Submitted));

        assert_eq!(
            started,
            vec![open_tab(
                "Claude Code · Work",
                "/work/new",
                AgentCommand::new("claude")
                    .with_env("CLAUDE_CONFIG_DIR", "/home/user/.claude-work"),
            )]
        );
        assert!(state.new_session().is_none(), "the dialog closes");
    }

    #[test]
    fn a_picked_profile_starts_and_is_kept_for_the_next_dialog() {
        let mut state = two_profiles();
        open_on_a_new_folder(&mut state);
        let _ = reduce(
            &mut state,
            dialog(NewSessionDialogIntent::ProfilePicked(codex())),
        );

        let started =
            reduce(&mut state, dialog(NewSessionDialogIntent::Submitted));
        open_on_a_new_folder(&mut state);

        assert_eq!(
            started,
            vec![open_tab("Codex", "/work/new", AgentCommand::new("codex"))]
        );
        assert_eq!(new_session_profile(&state), Some(&codex()));
    }

    /// Before a pick the dialog proposes the selected profile, else the
    /// first one that can start.
    #[test]
    fn the_proposed_profile_is_the_selected_one_that_can_start() {
        let mut state = with_profiles(&[
            (claude(), ProfileStatus::ProgramNotFound),
            (claude_work(), connected()),
            (codex(), connected()),
        ]);
        let first = new_session_profile(&state).cloned();

        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        assert_eq!(first, Some(claude_work()), "Claude Code can not start");
        assert_eq!(new_session_profile(&state), Some(&codex()));
    }

    #[test]
    fn start_is_off_for_a_missing_folder_or_one_not_checked_yet() {
        let mut state = two_profiles();
        let _ = reduce(
            &mut state,
            AgentsIntent::OpenNewSessionDialog(Some(PathBuf::from(
                "/work/gone",
            ))),
        );
        let unchecked =
            reduce(&mut state, dialog(NewSessionDialogIntent::Submitted));
        let _ = reduce(
            &mut state,
            looked_up(query("/work/gone", "/work", "gone"), false, &[]),
        );

        let missing =
            reduce(&mut state, dialog(NewSessionDialogIntent::Submitted));

        assert!(unchecked.is_empty());
        assert!(missing.is_empty());
        assert!(state.new_session().is_some(), "the dialog stays open");
    }

    #[test]
    fn start_is_off_while_no_profile_can_start() {
        let mut state =
            with_profiles(&[(claude(), ProfileStatus::ProgramNotFound)]);
        open_on_a_new_folder(&mut state);

        let started =
            reduce(&mut state, dialog(NewSessionDialogIntent::Submitted));

        assert!(started.is_empty());
        assert_eq!(new_session_profile(&state), None);
    }

    #[test]
    fn cancel_closes_the_dialog() {
        let mut state = two_profiles();
        open_on_a_new_folder(&mut state);

        let outcomes =
            reduce(&mut state, dialog(NewSessionDialogIntent::Cancelled));

        assert!(outcomes.is_empty());
        assert!(state.new_session().is_none());
    }

    /// A picked suggestion goes in the field and its folders are looked up,
    /// so the next level can be typed.
    #[test]
    fn a_picked_suggestion_looks_its_folder_up() {
        let mut state = two_profiles();
        open_on_a_new_folder(&mut state);
        let _ = reduce(
            &mut state,
            dialog(NewSessionDialogIntent::FolderChanged(String::from(
                "/work/o",
            ))),
        );
        let _ = reduce(
            &mut state,
            looked_up(query("/work/o", "/work", "o"), false, &["/work/otty"]),
        );

        let outcomes = reduce(
            &mut state,
            dialog(NewSessionDialogIntent::SuggestionPicked(0)),
        );

        assert_eq!(
            state.new_session().map(|dialog| dialog.text()),
            Some("/work/otty/")
        );
        assert_eq!(
            outcomes,
            vec![AgentsOutcome::LookUpFolder(query(
                "/work/otty",
                "/work/otty",
                "",
            ))]
        );
    }

    /// Issue 39: ↓ in the field highlights a row; Enter then picks it, as a
    /// press does; typing clears the highlight.
    #[test]
    fn a_highlighted_row_is_picked_and_typing_clears_the_highlight() {
        let mut state = two_profiles();
        open_on_a_new_folder(&mut state);
        let typed = |state: &mut AgentsState, text: &str| {
            let _ = reduce(
                state,
                dialog(NewSessionDialogIntent::FolderChanged(String::from(
                    text,
                ))),
            );
        };
        let highlighted = |state: &AgentsState| {
            state
                .new_session()
                .and_then(|dialog| dialog.folder().highlighted())
        };
        typed(&mut state, "/work/o");
        let _ = reduce(
            &mut state,
            looked_up(
                query("/work/o", "/work", "o"),
                false,
                &["/work/otty", "/work/other"],
            ),
        );

        let _ = reduce(
            &mut state,
            dialog(NewSessionDialogIntent::SuggestionHighlighted(Some(1))),
        );
        let after_down = highlighted(&state);
        typed(&mut state, "/work/ot");
        let after_typing = highlighted(&state);
        let _ = reduce(
            &mut state,
            dialog(NewSessionDialogIntent::SuggestionHighlighted(Some(0))),
        );
        let _ = reduce(
            &mut state,
            dialog(NewSessionDialogIntent::SuggestionPicked(0)),
        );

        assert_eq!(after_down, Some(1));
        assert_eq!(after_typing, None);
        assert_eq!(
            state.new_session().map(|dialog| dialog.text()),
            Some("/work/otty/")
        );
    }

    /// The folder is not stored: the project shows once the agent wrote
    /// its first session and the sessions of the profile are read again,
    /// the next time Agents mode opens.
    #[test]
    fn after_a_start_the_next_opening_reads_the_sessions_of_the_profile() {
        let mut state = with_profiles(&[]);
        let _ = reduce(
            &mut state,
            fresh_profiles_loaded(&[
                (claude_work(), connected()),
                (codex(), connected()),
            ]),
        );
        read(
            &mut state,
            &claude_work(),
            vec![session("c1", "/work/otty", 1)],
        );
        read(&mut state, &codex(), Vec::new());
        open_on_a_new_folder(&mut state);
        let _ = reduce(&mut state, dialog(NewSessionDialogIntent::Submitted));
        let _ = reduce(&mut state, AgentsIntent::Leave);

        let opened = reduce(&mut state, AgentsIntent::Enter);
        let again = reduce(&mut state, AgentsIntent::Enter);

        assert!(
            opened.contains(&AgentsOutcome::LoadSessions(claude_work())),
            "{opened:?}"
        );
        assert!(!opened.contains(&AgentsOutcome::LoadSessions(codex())));
        assert!(
            !again.iter().any(|outcome| matches!(
                outcome,
                AgentsOutcome::LoadSessions(_)
            ))
        );
    }
}
