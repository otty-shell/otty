use otty_app_domain::exit_behavior::ExitBehavior;
use otty_app_domain::quick_launch::WizardTabInit;
use otty_app_domain::tab::TabMachine;
use otty_ui_term::settings::Settings;

use super::ctx::TabsCtx;
use super::event::{TabsIntent, TabsOutcome};
use super::state::TabsState;
use super::types::{TabContent, TabItem};

/// Reduce a tabs intent into state changes and return what happened. The
/// tabs change only their own state: what runs in a tab, and the mode that
/// shows it, are told through the outcomes.
pub(crate) fn reduce(
    state: &mut TabsState,
    intent: TabsIntent,
    ctx: &TabsCtx<'_>,
) -> Vec<TabsOutcome> {
    match intent {
        TabsIntent::ActivateTab { tab_id } => activate(state, tab_id),
        TabsIntent::CloseTab { tab_id } => close(state, tab_id),
        TabsIntent::CloseActive => match state.active_tab_id() {
            Some(tab_id) => close(state, tab_id),
            None => Vec::new(),
        },
        TabsIntent::SetTitle {
            tab_id,
            title,
            folder,
        } => {
            state.set_title(tab_id, title, folder);
            Vec::new()
        },
        TabsIntent::OpenTerminalTab => {
            open_terminal_tab(state, ctx.shell_name.to_string())
        },
        TabsIntent::OpenCommandTab {
            title,
            settings,
            exit_behavior,
            machine,
        } => open_command_tab(state, title, settings, exit_behavior, machine),
        TabsIntent::OpenWizardTab { title, init } => {
            open_wizard_tab(state, title, init)
        },
        TabsIntent::HostNameRead(host_name) => {
            state.set_host_name(host_name);
            Vec::new()
        },
    }
}

/// Make an open tab the active one. It needs no terminal to start, so it
/// comes to the front at once.
fn activate(state: &mut TabsState, tab_id: u64) -> Vec<TabsOutcome> {
    if !state.contains(tab_id) {
        return Vec::new();
    }

    state.activate(Some(tab_id));

    vec![TabsOutcome::Activated { tab_id }, TabsOutcome::Shown]
}

/// Close a tab. When it was the active one, the tab before it becomes
/// active, or the last tab when it was the first.
fn close(state: &mut TabsState, tab_id: u64) -> Vec<TabsOutcome> {
    if !state.contains(tab_id) {
        return Vec::new();
    }

    let next_active = if state.active_tab_id() == Some(tab_id) {
        state
            .previous_tab_id(tab_id)
            .or_else(|| state.last_tab_id())
    } else {
        state.active_tab_id()
    };

    state.remove(tab_id);

    if state.is_empty() {
        state.activate(None);
    } else if state.active_tab_id() == Some(tab_id) {
        state.activate(next_active);
    }

    vec![TabsOutcome::Closed {
        tab_id,
        new_active_id: state.active_tab_id(),
    }]
}

/// Add an active tab of `content` on `machine` at the end of the list and
/// return its id.
fn open_tab(
    state: &mut TabsState,
    title: String,
    content: TabContent,
    machine: TabMachine,
) -> u64 {
    let tab_id = state.allocate_tab_id();

    state.insert(tab_id, TabItem::new(title, content, machine));
    state.activate(Some(tab_id));

    tab_id
}

fn open_terminal_tab(state: &mut TabsState, title: String) -> Vec<TabsOutcome> {
    let tab_id = open_tab(
        state,
        title.clone(),
        TabContent::Terminal,
        TabMachine::ThisComputer,
    );

    vec![
        TabsOutcome::TerminalTabOpened { tab_id, title },
        TabsOutcome::ScrollSessionsToEnd,
    ]
}

fn open_command_tab(
    state: &mut TabsState,
    title: String,
    settings: Box<Settings>,
    exit_behavior: ExitBehavior,
    machine: TabMachine,
) -> Vec<TabsOutcome> {
    let tab_id = open_tab(state, title.clone(), TabContent::Terminal, machine);

    vec![
        TabsOutcome::CommandTabOpened {
            tab_id,
            title,
            settings,
            exit_behavior,
        },
        TabsOutcome::ScrollSessionsToEnd,
    ]
}

/// Open the launch editor. It needs no terminal, so it comes to the front
/// as it opens.
fn open_wizard_tab(
    state: &mut TabsState,
    title: String,
    init: WizardTabInit,
) -> Vec<TabsOutcome> {
    let tab_id = open_tab(
        state,
        title,
        TabContent::QuickLaunchWizard,
        TabMachine::ThisComputer,
    );

    vec![
        TabsOutcome::WizardTabOpened { tab_id, init },
        TabsOutcome::ScrollSessionsToEnd,
        TabsOutcome::Shown,
    ]
}

#[cfg(test)]
mod tests {
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::event::TabsOutcome;
    use crate::model::TabsViewModel;

    /// Shell the tests open terminal tabs with.
    const SHELL: &str = "bash";

    /// Reduce `intent` with [`SHELL`] as the shell.
    fn reduce(state: &mut TabsState, intent: TabsIntent) -> Vec<TabsOutcome> {
        super::reduce(state, intent, &TabsCtx { shell_name: SHELL })
    }

    /// Open a terminal tab with the shell `shell_name`.
    fn open_terminal(state: &mut TabsState, shell_name: &str) -> u64 {
        let _ = super::reduce(
            state,
            TabsIntent::OpenTerminalTab,
            &TabsCtx { shell_name },
        );
        state.active_tab_id().expect("terminal tab should open")
    }

    /// The closed tab and the tab active after it, if the outcomes report a
    /// closed tab.
    fn closed(outcomes: &[TabsOutcome]) -> Option<(u64, Option<u64>)> {
        outcomes.iter().find_map(|outcome| match outcome {
            TabsOutcome::Closed {
                tab_id,
                new_active_id,
            } => Some((*tab_id, *new_active_id)),
            _ => None,
        })
    }

    fn command_tab(machine: TabMachine) -> TabsIntent {
        TabsIntent::OpenCommandTab {
            title: String::from("htop"),
            settings: Box::new(Settings::default()),
            exit_behavior: ExitBehavior::Close,
            machine,
        }
    }

    fn group_names(state: &TabsState) -> Vec<String> {
        TabsViewModel::from_state(state)
            .groups
            .iter()
            .map(|group| String::from(group.name))
            .collect()
    }

    #[test]
    fn activate_nonexistent_tab_is_noop() {
        let mut state = TabsState::default();

        let outcomes =
            reduce(&mut state, TabsIntent::ActivateTab { tab_id: 999 });

        assert!(outcomes.is_empty());
        assert!(state.active_tab_id().is_none());
    }

    #[test]
    fn open_terminal_tab_activates_new_tab_named_after_the_shell() {
        let mut state = TabsState::default();

        let _ = reduce(&mut state, TabsIntent::OpenTerminalTab);

        assert_eq!(state.len(), 1);
        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.title(), "bash");
        assert_eq!(active.content(), TabContent::Terminal);
    }

    /// The terminal feature starts the shell of the new tab; the tab comes
    /// to the front once its terminal starts, so the tabs do not report it
    /// shown.
    #[test]
    fn open_terminal_tab_reports_the_tab_and_scrolls_the_list() {
        let mut state = TabsState::default();

        let outcomes = reduce(&mut state, TabsIntent::OpenTerminalTab);

        let tab_id = state.active_tab_id().expect("terminal tab should open");
        assert!(matches!(
            outcomes.as_slice(),
            [
                TabsOutcome::TerminalTabOpened { tab_id: id, title },
                TabsOutcome::ScrollSessionsToEnd,
            ] if *id == tab_id && title == "bash"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn open_command_tab_activates_new_tab_and_hands_on_its_command() {
        use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

        let mut settings = Settings::default();
        settings.engine = settings.engine.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program("/bin/sh"),
            ),
        );
        let mut state = TabsState::default();

        let outcomes = reduce(
            &mut state,
            TabsIntent::OpenCommandTab {
                title: String::from("nvim main.rs"),
                settings: Box::new(settings),
                exit_behavior: ExitBehavior::KeepOpen,
                machine: TabMachine::ThisComputer,
            },
        );

        assert_eq!(state.len(), 1);
        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.title(), "nvim main.rs");
        assert_eq!(active.content(), TabContent::Terminal);
        let tab_id = state.active_tab_id().expect("should have active tab");
        assert!(matches!(
            outcomes.as_slice(),
            [
                TabsOutcome::CommandTabOpened {
                    tab_id: id,
                    title,
                    exit_behavior: ExitBehavior::KeepOpen,
                    ..
                },
                TabsOutcome::ScrollSessionsToEnd,
            ] if *id == tab_id && title == "nvim main.rs"
        ));
    }

    #[test]
    fn open_command_tab_keeps_the_machine_it_was_opened_for() {
        let mut state = TabsState::default();

        let _ = reduce(
            &mut state,
            command_tab(TabMachine::Ssh(String::from("laptop"))),
        );

        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.machine(), &TabMachine::Ssh(String::from("laptop")));
    }

    #[test]
    fn tabs_opened_without_a_launch_run_on_this_computer() {
        let mut state = TabsState::default();
        let _ = open_terminal(&mut state, "zsh");

        let _ = reduce(
            &mut state,
            TabsIntent::OpenWizardTab {
                title: String::from("New launch"),
                init: WizardTabInit::Create {
                    parent_path: Vec::new(),
                },
            },
        );

        assert_eq!(state.len(), 2);
        assert!(
            state
                .tab_items()
                .values()
                .all(|tab| tab.machine() == &TabMachine::ThisComputer)
        );
    }

    /// The launch editor needs no terminal, so it comes to the front as it
    /// opens.
    #[test]
    fn open_wizard_tab_activates_launch_editor_tab_and_shows_it() {
        let mut state = TabsState::default();
        let _ = open_terminal(&mut state, "shell");

        let outcomes = reduce(
            &mut state,
            TabsIntent::OpenWizardTab {
                title: String::from("New launch"),
                init: WizardTabInit::Create {
                    parent_path: vec![String::from("Servers")],
                },
            },
        );

        assert_eq!(state.len(), 2);
        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.title(), "New launch");
        assert_eq!(active.content(), TabContent::QuickLaunchWizard);
        let tab_id = state.active_tab_id().expect("should have active tab");
        assert!(matches!(
            outcomes.as_slice(),
            [
                TabsOutcome::WizardTabOpened {
                    tab_id: id,
                    init: WizardTabInit::Create { parent_path },
                },
                TabsOutcome::ScrollSessionsToEnd,
                TabsOutcome::Shown,
            ] if *id == tab_id && parent_path == &vec![String::from("Servers")]
        ));
    }

    #[test]
    fn close_active_tab_activates_previous() {
        let mut state = TabsState::default();
        let first_id = open_terminal(&mut state, "First");
        let second_id = open_terminal(&mut state, "Second");

        let _ = reduce(&mut state, TabsIntent::CloseTab { tab_id: second_id });

        assert_eq!(state.active_tab_id(), Some(first_id));
    }

    #[test]
    fn close_last_tab_clears_active() {
        let mut state = TabsState::default();
        let tab_id = open_terminal(&mut state, "Only");

        let _ = reduce(&mut state, TabsIntent::CloseTab { tab_id });

        assert!(state.active_tab_id().is_none());
        assert!(state.is_empty());
    }

    #[test]
    fn set_title_updates_tab_and_reports_nothing() {
        let mut state = TabsState::default();
        let tab_id = open_terminal(&mut state, "zsh");

        let outcomes = reduce(
            &mut state,
            TabsIntent::SetTitle {
                tab_id,
                title: String::from("new"),
                folder: Some(String::from("otty")),
            },
        );

        let tab = state.active_tab().expect("should have active tab");
        assert!(outcomes.is_empty());
        assert_eq!(tab.title(), "new");
        assert_eq!(tab.folder(), Some("otty"));
    }

    #[test]
    fn activate_background_tab_makes_it_active_and_shows_it() {
        let mut state = TabsState::default();
        let first_id = open_terminal(&mut state, "First");
        let _second_id = open_terminal(&mut state, "Second");

        let outcomes =
            reduce(&mut state, TabsIntent::ActivateTab { tab_id: first_id });

        assert_eq!(state.active_tab_id(), Some(first_id));
        assert!(matches!(
            outcomes.as_slice(),
            [TabsOutcome::Activated { tab_id }, TabsOutcome::Shown]
                if *tab_id == first_id
        ));
    }

    #[test]
    fn close_background_tab_keeps_active_tab() {
        let mut state = TabsState::default();
        let first_id = open_terminal(&mut state, "First");
        let second_id = open_terminal(&mut state, "Second");

        let outcomes =
            reduce(&mut state, TabsIntent::CloseTab { tab_id: first_id });

        assert_eq!(state.active_tab_id(), Some(second_id));
        assert_eq!(state.len(), 1);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(closed(&outcomes), Some((first_id, Some(second_id))));
    }

    #[test]
    fn close_active_middle_tab_activates_the_tab_before_it() {
        let mut state = TabsState::default();
        let first_id = open_terminal(&mut state, "First");
        let second_id = open_terminal(&mut state, "Second");
        let _third_id = open_terminal(&mut state, "Third");
        let _ =
            reduce(&mut state, TabsIntent::ActivateTab { tab_id: second_id });

        let outcomes =
            reduce(&mut state, TabsIntent::CloseTab { tab_id: second_id });

        assert_eq!(state.active_tab_id(), Some(first_id));
        assert_eq!(closed(&outcomes), Some((second_id, Some(first_id))));
    }

    #[test]
    fn close_active_first_tab_activates_the_last_tab() {
        // The first tab has no tab before it, so the last one becomes
        // active, not its right-hand neighbour.
        let mut state = TabsState::default();
        let first_id = open_terminal(&mut state, "First");
        let _second_id = open_terminal(&mut state, "Second");
        let third_id = open_terminal(&mut state, "Third");
        let _ =
            reduce(&mut state, TabsIntent::ActivateTab { tab_id: first_id });

        let outcomes =
            reduce(&mut state, TabsIntent::CloseTab { tab_id: first_id });

        assert_eq!(state.active_tab_id(), Some(third_id));
        assert_eq!(closed(&outcomes), Some((first_id, Some(third_id))));
    }

    #[test]
    fn close_only_tab_reports_no_active_tab() {
        let mut state = TabsState::default();
        let tab_id = open_terminal(&mut state, "Only");

        let outcomes = reduce(&mut state, TabsIntent::CloseTab { tab_id });

        assert_eq!(closed(&outcomes), Some((tab_id, None)));
    }

    #[test]
    fn close_unknown_tab_is_noop() {
        let mut state = TabsState::default();
        let tab_id = open_terminal(&mut state, "Only");

        let outcomes =
            reduce(&mut state, TabsIntent::CloseTab { tab_id: tab_id + 1 });

        assert!(outcomes.is_empty());
        assert_eq!(state.active_tab_id(), Some(tab_id));
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn new_tab_ids_are_not_reused_after_a_close() {
        let mut state = TabsState::default();
        let first_id = open_terminal(&mut state, "First");
        let _ = reduce(&mut state, TabsIntent::CloseTab { tab_id: first_id });

        let second_id = open_terminal(&mut state, "Second");

        assert_ne!(first_id, second_id);
    }

    #[test]
    fn close_active_closes_the_active_tab() {
        let mut state = TabsState::default();
        let first_id = open_terminal(&mut state, "First");
        let second_id = open_terminal(&mut state, "Second");

        let outcomes = reduce(&mut state, TabsIntent::CloseActive);

        assert_eq!(state.active_tab_id(), Some(first_id));
        assert_eq!(closed(&outcomes), Some((second_id, Some(first_id))));
    }

    #[test]
    fn close_active_without_tabs_does_nothing() {
        let mut state = TabsState::default();

        let outcomes = reduce(&mut state, TabsIntent::CloseActive);

        assert!(outcomes.is_empty());
    }

    /// The host name reaches the tabs as an intent and names the group of
    /// this computer's tabs; until it is read, or when it can not be read,
    /// the group is called "This computer".
    #[test]
    fn a_read_host_name_names_the_group_of_this_computer() {
        let mut state = TabsState::default();
        let _ = open_terminal(&mut state, "zsh");
        let before = group_names(&state);

        let read = reduce(
            &mut state,
            TabsIntent::HostNameRead(Some(String::from("fedora"))),
        );
        let named = group_names(&state);
        let _ = reduce(&mut state, TabsIntent::HostNameRead(None));

        assert!(read.is_empty());
        assert_eq!(before, vec![i18n::t(Key::ThisComputer)]);
        assert_eq!(named, vec!["fedora"]);
        assert_eq!(group_names(&state), vec![i18n::t(Key::ThisComputer)]);
    }
}
