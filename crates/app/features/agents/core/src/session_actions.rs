//! What the project screen does with the sessions of the selected project:
//! the search and the sort of its table, the selected session, Continue
//! with a double press, Enter or the row menu, the other items of the row
//! menu, New session, and the folder opened in a terminal tab (spec
//! Decisions 3 and 6).

use std::path::Path;

use otty_agents::{AgentCommand, AgentSession, Profile, ProjectSession};
use otty_ui_kit::components::tag_search::TagSearchAction;

use super::event::{AgentsOutcome, SessionAction};
use super::menus::AgentsMenu;
use super::sessions_table::SessionKey;
use super::state::AgentsState;

/// Do what the user did in the search over the table of the selected
/// project.
pub(super) fn search(state: &mut AgentsState, action: TagSearchAction) {
    let Some(schema) = state.session_schema() else {
        return;
    };
    let Some(folder) = state.selected_folder().map(Path::to_path_buf) else {
        return;
    };

    state.table_mut().perform_search(&folder, action, &schema);
}

/// Select the session at `index` of the shown table; `false` when there is
/// no such row.
pub(super) fn select(state: &mut AgentsState, index: usize) -> bool {
    let Some(key) = shown_key(state, index) else {
        return false;
    };

    state.table_mut().select(key);
    true
}

/// Select the session at `index` of the shown table and continue it in a
/// terminal tab.
pub(super) fn open(
    state: &mut AgentsState,
    index: usize,
) -> Vec<AgentsOutcome> {
    if !select(state, index) {
        return Vec::new();
    }

    continue_selected(state)
}

/// Continue the selected session of the project on screen in a terminal
/// tab: Enter does it. Nothing happens outside Agents mode, or while the
/// selected session is not shown in the table.
pub(super) fn continue_selected(state: &AgentsState) -> Vec<AgentsOutcome> {
    if !state.is_active() {
        return Vec::new();
    }
    let Some(key) = state.table().selected() else {
        return Vec::new();
    };

    let shown = state.shown_sessions();
    let entry = shown.iter().find(|session| key.is_of(session.entry()));
    let Some(entry) = entry.map(|session| session.entry()) else {
        return Vec::new();
    };

    continue_in_tab(state, entry)
}

/// Select the session at `index` of the shown table and open its menu at
/// the pointer.
pub(super) fn open_menu(
    state: &mut AgentsState,
    index: usize,
) -> Vec<AgentsOutcome> {
    let Some(key) = shown_key(state, index) else {
        return Vec::new();
    };

    state.table_mut().select(key.clone());
    state.open_menu(AgentsMenu::Session(key));
    vec![AgentsOutcome::MenuOpened]
}

/// Do what the item `action` of the open session menu stands for, and
/// close the menu. A session that is no longer in the selected project
/// does nothing.
pub(super) fn act(
    state: &mut AgentsState,
    action: SessionAction,
) -> Vec<AgentsOutcome> {
    let key = match state.menu().map(|open| open.menu()) {
        Some(AgentsMenu::Session(key)) => key.clone(),
        _ => return Vec::new(),
    };
    let _ = state.close_menu();

    let Some(entry) = project_session(state, &key) else {
        return Vec::new();
    };

    match action {
        SessionAction::Continue => continue_in_tab(state, entry),
        SessionAction::ContinueInSplit => continue_in_split(state, entry),
        SessionAction::CopyResumeCommand => resume_command(state, entry)
            .map(AgentsOutcome::CopyCommand)
            .into_iter()
            .collect(),
        SessionAction::CopySessionId => {
            let id = entry.session().id().as_str().to_string();
            vec![AgentsOutcome::CopyText(id)]
        },
        SessionAction::OpenFolder => open_project_folder(state),
    }
}

/// Open the folder of the selected project in a terminal tab with the
/// user's shell, unless the folder is gone.
pub(super) fn open_project_folder(state: &AgentsState) -> Vec<AgentsOutcome> {
    let Some(folder) = openable_selected_folder(state) else {
        return Vec::new();
    };

    vec![AgentsOutcome::OpenFolder(folder.to_path_buf())]
}

/// Start a new session in the selected project: right away with the only
/// profile that can start one, else open the list of the profiles at the
/// pointer.
pub(super) fn new_session(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let Some(folder) = openable_selected_folder(state) else {
        return Vec::new();
    };

    let profiles = state.startable_profiles();
    match profiles.as_slice() {
        [] => Vec::new(),
        [profile] => open_tab(state, profile, None, folder),
        _ => {
            state.open_menu(AgentsMenu::NewSession);
            vec![AgentsOutcome::MenuOpened]
        },
    }
}

/// Open the list of the profiles a new session can start with, while the
/// selected project can start one.
pub(super) fn open_new_session_menu(
    state: &mut AgentsState,
) -> Vec<AgentsOutcome> {
    let can_start = openable_selected_folder(state).is_some()
        && !state.startable_profiles().is_empty();
    if !can_start {
        return Vec::new();
    }

    state.open_menu(AgentsMenu::NewSession);
    vec![AgentsOutcome::MenuOpened]
}

/// Start a new session of `profile` in the selected project.
pub(super) fn new_session_with(
    state: &AgentsState,
    profile: &Profile,
) -> Vec<AgentsOutcome> {
    let Some(folder) = openable_selected_folder(state) else {
        return Vec::new();
    };

    open_tab(state, profile, None, folder)
}

/// The key of the session at `index` of the shown table.
fn shown_key(state: &AgentsState, index: usize) -> Option<SessionKey> {
    let shown = state.shown_sessions();

    shown
        .get(index)
        .map(|session| SessionKey::of(session.entry()))
}

/// The session of `key` among all the sessions of the selected project,
/// whatever its search lets through.
fn project_session<'a>(
    state: &'a AgentsState,
    key: &SessionKey,
) -> Option<&'a ProjectSession> {
    let project = state.selected_project()?;

    project.sessions().iter().find(|entry| key.is_of(entry))
}

/// Continue the session of `entry` in a terminal tab in the selected
/// project, unless its folder is gone.
fn continue_in_tab(
    state: &AgentsState,
    entry: &ProjectSession,
) -> Vec<AgentsOutcome> {
    let Some(folder) = openable_selected_folder(state) else {
        return Vec::new();
    };

    open_tab(state, entry.profile(), Some(entry.session()), folder)
}

/// Continue the session of `entry` in a new pane next to the focused pane
/// of the terminal tab in front; in a new tab when Terminal mode has no
/// such pane.
fn continue_in_split(
    state: &AgentsState,
    entry: &ProjectSession,
) -> Vec<AgentsOutcome> {
    if !state.has_local_pane() {
        return continue_in_tab(state, entry);
    }
    let Some(folder) = openable_selected_folder(state) else {
        return Vec::new();
    };

    let command = launch_command(state, entry.profile(), Some(entry.session()));
    command
        .map(|command| AgentsOutcome::OpenSplit {
            folder: folder.to_path_buf(),
            command,
        })
        .into_iter()
        .collect()
}

/// The command that continues the session of `entry`, as its tab would
/// run it, even while its program is not found or its folder is gone: the
/// user may copy it to run elsewhere. `None` for an agent OTTY does not
/// know.
fn resume_command(
    state: &AgentsState,
    entry: &ProjectSession,
) -> Option<AgentCommand> {
    let agent = state.agent(entry.profile().agent())?;

    Some(agent.launch(entry.profile(), Some(entry.session().id())))
}

/// The folder of the selected project, while it is listed and exists.
fn openable_selected_folder(state: &AgentsState) -> Option<&Path> {
    let folder = state.selected_project()?.folder();
    if state.is_missing(folder) {
        return None;
    }

    Some(folder)
}

/// The terminal tab that starts `session` of `profile`, or a new session
/// when there is none, in `folder`. It is named after the session, or the
/// profile for a new one. A profile that can not start gives no tab.
fn open_tab(
    state: &AgentsState,
    profile: &Profile,
    session: Option<&AgentSession>,
    folder: &Path,
) -> Vec<AgentsOutcome> {
    let Some(command) = launch_command(state, profile, session) else {
        return Vec::new();
    };

    let title = match session {
        Some(session) => session.title().to_string(),
        None => profile.name().to_string(),
    };

    vec![AgentsOutcome::OpenTab {
        title,
        folder: folder.to_path_buf(),
        command,
    }]
}

/// The command that continues `session` of `profile`, or starts a new
/// session when there is none. `None` when the profile can not start or
/// its agent is not known.
fn launch_command(
    state: &AgentsState,
    profile: &Profile,
    session: Option<&AgentSession>,
) -> Option<AgentCommand> {
    if !state.can_start(profile) {
        return None;
    }
    let Some(agent) = state.agent(profile.agent()) else {
        log::debug!("no agent {} to start", profile.agent().as_str());
        return None;
    };

    Some(agent.launch(profile, session.map(AgentSession::id)))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::{Point, Size};
    use otty_agents::{AgentCommand, ProfileStatus};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::menus::MenuPlace;
    use crate::reducer::reduce;
    use crate::session_sort::TITLE_COLUMN;
    use crate::testing::{
        claude, codex, connected, two_agents_with_sessions, with_profiles,
    };
    use crate::types::LoadedSessions;

    /// Agents mode with the sessions of [`two_agents_with_sessions`] and
    /// `/work/otty` selected: its table shows x1, then c1.
    fn otty_selected() -> AgentsState {
        let mut state = two_agents_with_sessions();
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        );

        state
    }

    fn shown_ids(state: &AgentsState) -> Vec<String> {
        state
            .shown_sessions()
            .iter()
            .map(|session| session.id().as_str().to_string())
            .collect()
    }

    fn place() -> MenuPlace {
        MenuPlace::new(Point::new(300.0, 200.0), Size::new(1200.0, 800.0))
    }

    fn resume_c1() -> AgentCommand {
        AgentCommand::new("claude")
            .with_arg("--resume")
            .with_arg("c1")
    }

    fn c1_tab() -> AgentsOutcome {
        AgentsOutcome::OpenTab {
            title: String::from("Title of c1"),
            folder: PathBuf::from("/work/otty"),
            command: resume_c1(),
        }
    }

    /// Open the menu of the row at `index` and press `action` in it.
    fn act_on_row(
        state: &mut AgentsState,
        index: usize,
        action: SessionAction,
    ) -> Vec<AgentsOutcome> {
        let _ = reduce(state, AgentsIntent::OpenSessionMenu(index));

        reduce(state, AgentsIntent::SessionMenu(action))
    }

    fn search(state: &mut AgentsState, action: TagSearchAction) {
        let _ = reduce(state, AgentsIntent::SessionSearch(action));
    }

    /// Add the tag `text` to the search of the selected project, as Enter
    /// does with no suggestion.
    fn add_tag(state: &mut AgentsState, text: &str) {
        search(state, TagSearchAction::InputChanged(text.to_string()));
        search(state, TagSearchAction::Close);
        search(state, TagSearchAction::Apply);
    }

    #[test]
    fn the_table_shows_the_newest_session_first_and_a_title_press_sorts_it() {
        let mut state = otty_selected();
        let newest_first = shown_ids(&state);

        let _ = reduce(&mut state, AgentsIntent::SortSessions(TITLE_COLUMN));

        assert_eq!(newest_first, ["x1", "c1"]);
        assert_eq!(shown_ids(&state), ["c1", "x1"]);
    }

    #[test]
    fn a_tag_filters_the_table_of_its_project_and_each_project_keeps_its_own() {
        let mut state = otty_selected();

        add_tag(&mut state, "Agent:Codex");
        let otty = shown_ids(&state);
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/site")),
        );
        let site = shown_ids(&state);
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        );

        assert_eq!(otty, ["x1"]);
        assert_eq!(site, ["c2"], "the other project has no tag");
        assert_eq!(shown_ids(&state), ["x1"], "the tag stays with its project");
    }

    #[test]
    fn enter_continues_the_selected_session_of_the_table() {
        let mut state = otty_selected();
        let nothing_selected =
            reduce(&mut state, AgentsIntent::ContinueSelected);

        let _ = reduce(&mut state, AgentsIntent::SelectSession(1));
        let continued = reduce(&mut state, AgentsIntent::ContinueSelected);

        assert!(nothing_selected.is_empty());
        assert_eq!(continued, vec![c1_tab()]);
    }

    #[test]
    fn enter_does_nothing_outside_the_mode_or_for_a_session_the_search_hides() {
        let mut state = otty_selected();
        let _ = reduce(&mut state, AgentsIntent::SelectSession(1));

        add_tag(&mut state, "Agent:Codex");
        let hidden = reduce(&mut state, AgentsIntent::ContinueSelected);
        search(&mut state, TagSearchAction::ClearTags);
        let _ = reduce(&mut state, AgentsIntent::Leave);
        let outside = reduce(&mut state, AgentsIntent::ContinueSelected);

        assert!(hidden.is_empty());
        assert!(outside.is_empty());
    }

    #[test]
    fn a_double_press_selects_the_session_and_continues_it() {
        let mut state = otty_selected();

        let continued = reduce(&mut state, AgentsIntent::OpenSession(1));
        let past_the_end = reduce(&mut state, AgentsIntent::OpenSession(5));

        assert_eq!(continued, vec![c1_tab()]);
        assert_eq!(
            state.table().selected(),
            Some(&SessionKey::of(&state.projects()[1].sessions()[1]))
        );
        assert!(past_the_end.is_empty());
    }

    #[test]
    fn a_right_press_selects_the_session_and_opens_its_menu_at_the_pointer() {
        let mut state = otty_selected();
        state.set_pointer(place());

        let opened = reduce(&mut state, AgentsIntent::OpenSessionMenu(0));
        let nothing = reduce(&mut state, AgentsIntent::OpenSessionMenu(9));

        assert_eq!(opened, vec![AgentsOutcome::MenuOpened]);
        assert!(nothing.is_empty());
        let open = state.menu().expect("the menu is open");
        let x1 = SessionKey::of(&state.projects()[1].sessions()[0]);
        assert_eq!(open.menu(), &AgentsMenu::Session(x1.clone()));
        assert_eq!(open.place(), place());
        assert_eq!(state.table().selected(), Some(&x1));
    }

    #[test]
    fn the_row_menu_continues_copies_and_opens_the_folder_then_closes() {
        let mut state = otty_selected();

        let continued = act_on_row(&mut state, 1, SessionAction::Continue);
        let command =
            act_on_row(&mut state, 1, SessionAction::CopyResumeCommand);
        let id = act_on_row(&mut state, 1, SessionAction::CopySessionId);
        let folder = act_on_row(&mut state, 1, SessionAction::OpenFolder);

        assert_eq!(continued, vec![c1_tab()]);
        assert_eq!(command, vec![AgentsOutcome::CopyCommand(resume_c1())]);
        assert_eq!(id, vec![AgentsOutcome::CopyText(String::from("c1"))]);
        assert_eq!(
            folder,
            vec![AgentsOutcome::OpenFolder(PathBuf::from("/work/otty"))]
        );
        assert!(state.menu().is_none());
    }

    #[test]
    fn an_item_with_no_session_menu_open_does_nothing() {
        let mut state = otty_selected();
        let _ = reduce(&mut state, AgentsIntent::OpenFilterMenu);

        let outcomes = reduce(
            &mut state,
            AgentsIntent::SessionMenu(SessionAction::Continue),
        );

        assert!(outcomes.is_empty());
        assert!(state.menu().is_some(), "the funnel menu stays");
    }

    #[test]
    fn continue_in_a_split_splits_a_local_pane_and_else_opens_a_tab() {
        let mut state = otty_selected();

        let without_pane =
            act_on_row(&mut state, 1, SessionAction::ContinueInSplit);
        state.set_has_local_pane(true);
        let with_pane =
            act_on_row(&mut state, 1, SessionAction::ContinueInSplit);

        assert_eq!(without_pane, vec![c1_tab()]);
        assert_eq!(
            with_pane,
            vec![AgentsOutcome::OpenSplit {
                folder: PathBuf::from("/work/otty"),
                command: resume_c1(),
            }]
        );
    }

    #[test]
    fn a_profile_whose_program_is_not_found_continues_nowhere_but_copies() {
        let mut state = with_profiles(&[
            (claude(), ProfileStatus::ProgramNotFound),
            (codex(), connected()),
        ]);
        crate::testing::read(
            &mut state,
            &claude(),
            vec![crate::testing::session("c1", "/work/otty", 1)],
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        );
        state.set_has_local_pane(true);

        let continued = act_on_row(&mut state, 0, SessionAction::Continue);
        let split = act_on_row(&mut state, 0, SessionAction::ContinueInSplit);
        let copied =
            act_on_row(&mut state, 0, SessionAction::CopyResumeCommand);

        assert!(continued.is_empty());
        assert!(split.is_empty());
        assert_eq!(copied, vec![AgentsOutcome::CopyCommand(resume_c1())]);
    }

    #[test]
    fn a_missing_folder_opens_nothing_but_its_session_can_still_be_copied() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: claude(),
                sessions: Ok(vec![crate::testing::session(
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
        state.set_has_local_pane(true);

        let mut opened = reduce(&mut state, AgentsIntent::OpenSession(0));
        opened.extend(reduce(&mut state, AgentsIntent::ContinueSelected));
        opened.extend(reduce(&mut state, AgentsIntent::OpenProjectFolder));
        opened.extend(reduce(&mut state, AgentsIntent::NewSession));
        for action in [
            SessionAction::Continue,
            SessionAction::ContinueInSplit,
            SessionAction::OpenFolder,
        ] {
            opened.extend(act_on_row(&mut state, 0, action));
        }
        let copied = act_on_row(&mut state, 0, SessionAction::CopySessionId);

        assert!(opened.is_empty(), "got {opened:?}");
        assert_eq!(copied, vec![AgentsOutcome::CopyText(String::from("c1"))]);
    }

    #[test]
    fn the_title_row_button_opens_the_project_folder_in_a_shell_tab() {
        let mut state = otty_selected();

        let opened = reduce(&mut state, AgentsIntent::OpenProjectFolder);

        assert_eq!(
            opened,
            vec![AgentsOutcome::OpenFolder(PathBuf::from("/work/otty"))]
        );
    }
}
