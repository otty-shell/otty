use iced::Task;
use otty_ui_term::settings::Settings;

use super::event::{TabsEffect, TabsEvent, TabsIntent};
use super::state::TabsState;
use super::types::{TabContent, TabItem, TabMachine};
use crate::domain::exit_behavior::ExitBehavior;
use crate::domain::quick_launch::WizardTabInit;
use crate::i18n::{self, Key};

/// Reduce a tabs intent event into state mutation and effect tasks.
pub(crate) fn reduce(
    state: &mut TabsState,
    event: TabsIntent,
) -> Task<TabsEvent> {
    match event {
        TabsIntent::ActivateTab { tab_id } => activate(state, tab_id),
        TabsIntent::CloseTab { tab_id } => close(state, tab_id),
        TabsIntent::SetTitle {
            tab_id,
            title,
            folder,
        } => {
            state.set_title(tab_id, title, folder);
            Task::none()
        },
        TabsIntent::RefreshSettingsTitle { title } => {
            state.set_settings_title(title);
            Task::none()
        },
        TabsIntent::OpenTerminalTab { title } => {
            open_terminal_tab(state, title)
        },
        TabsIntent::OpenCommandTab {
            title,
            settings,
            exit_behavior,
            machine,
        } => open_command_tab(state, title, *settings, exit_behavior, machine),
        TabsIntent::OpenSettingsTab => open_settings_tab(state),
        TabsIntent::OpenWizardTab { title, init } => {
            open_wizard_tab(state, title, init)
        },
        TabsIntent::OpenErrorTab { title, message } => {
            open_error_tab(state, title, message)
        },
    }
}

fn activate(state: &mut TabsState, tab_id: u64) -> Task<TabsEvent> {
    if !state.contains(tab_id) {
        return Task::none();
    }

    state.activate(Some(tab_id));
    Task::done(TabsEvent::Effect(TabsEffect::Activated { tab_id }))
}

fn close(state: &mut TabsState, tab_id: u64) -> Task<TabsEvent> {
    if !state.contains(tab_id) {
        return Task::none();
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

    Task::done(TabsEvent::Effect(TabsEffect::Closed {
        tab_id,
        new_active_id: state.active_tab_id(),
        remaining: state.len(),
    }))
}

fn open_terminal_tab(state: &mut TabsState, title: String) -> Task<TabsEvent> {
    let tab_id = state.allocate_tab_id();

    state.insert(
        tab_id,
        TabItem::new(
            tab_id,
            title.clone(),
            TabContent::Terminal,
            TabMachine::ThisComputer,
        ),
    );
    state.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(TabsEvent::Effect(TabsEffect::TerminalTabOpened {
            tab_id,
            title,
        })),
        Task::done(TabsEvent::Effect(TabsEffect::ScrollBarToEnd)),
    ])
}

fn open_command_tab(
    state: &mut TabsState,
    title: String,
    settings: Settings,
    exit_behavior: ExitBehavior,
    machine: TabMachine,
) -> Task<TabsEvent> {
    let tab_id = state.allocate_tab_id();

    state.insert(
        tab_id,
        TabItem::new(tab_id, title.clone(), TabContent::Terminal, machine),
    );
    state.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(TabsEvent::Effect(TabsEffect::CommandTabOpened {
            tab_id,
            title,
            settings: Box::new(settings),
            exit_behavior,
        })),
        Task::done(TabsEvent::Effect(TabsEffect::ScrollBarToEnd)),
    ])
}

fn open_wizard_tab(
    state: &mut TabsState,
    title: String,
    init: WizardTabInit,
) -> Task<TabsEvent> {
    let tab_id = state.allocate_tab_id();
    state.insert(
        tab_id,
        TabItem::new(
            tab_id,
            title,
            TabContent::QuickLaunchWizard,
            TabMachine::ThisComputer,
        ),
    );
    state.activate(Some(tab_id));
    Task::batch(vec![
        Task::done(TabsEvent::Effect(TabsEffect::WizardTabOpened {
            tab_id,
            init,
        })),
        Task::done(TabsEvent::Effect(TabsEffect::ScrollBarToEnd)),
    ])
}

fn open_error_tab(
    state: &mut TabsState,
    title: String,
    message: String,
) -> Task<TabsEvent> {
    let tab_id = state.allocate_tab_id();
    state.insert(
        tab_id,
        TabItem::new(
            tab_id,
            title.clone(),
            TabContent::QuickLaunchError,
            TabMachine::ThisComputer,
        ),
    );
    state.activate(Some(tab_id));
    Task::batch(vec![
        Task::done(TabsEvent::Effect(TabsEffect::ErrorTabOpened {
            tab_id,
            title,
            message,
        })),
        Task::done(TabsEvent::Effect(TabsEffect::ScrollBarToEnd)),
    ])
}

fn open_settings_tab(state: &mut TabsState) -> Task<TabsEvent> {
    if let Some(tab_id) = state.tab_items().iter().find_map(|(tab_id, tab)| {
        (tab.content() == TabContent::Settings).then_some(*tab_id)
    }) {
        state.activate(Some(tab_id));
        return Task::done(TabsEvent::Effect(TabsEffect::Activated { tab_id }));
    }

    let tab_id = state.allocate_tab_id();

    state.insert(
        tab_id,
        TabItem::new(
            tab_id,
            i18n::t(Key::SettingsTabTitle).to_string(),
            TabContent::Settings,
            TabMachine::ThisComputer,
        ),
    );
    state.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(TabsEvent::Effect(TabsEffect::SettingsTabOpened)),
        Task::done(TabsEvent::Effect(TabsEffect::ScrollBarToEnd)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;

    fn open_terminal(state: &mut TabsState, title: &str) -> u64 {
        let _ = reduce(
            state,
            TabsIntent::OpenTerminalTab {
                title: String::from(title),
            },
        );
        state.active_tab_id().expect("terminal tab should open")
    }

    fn closed_effect(
        events: &[TabsEvent],
    ) -> Option<(u64, Option<u64>, usize)> {
        events.iter().find_map(|event| match event {
            TabsEvent::Effect(TabsEffect::Closed {
                tab_id,
                new_active_id,
                remaining,
            }) => Some((*tab_id, *new_active_id, *remaining)),
            _ => None,
        })
    }

    #[test]
    fn activate_nonexistent_tab_is_noop() {
        let mut state = TabsState::default();
        let _ = reduce(&mut state, TabsIntent::ActivateTab { tab_id: 999 });
        assert!(state.active_tab_id().is_none());
    }

    #[test]
    fn open_terminal_tab_activates_new_tab() {
        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsIntent::OpenTerminalTab {
                title: String::from("bash"),
            },
        );

        assert_eq!(state.len(), 1);
        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.title(), "bash");
        assert_eq!(active.content(), TabContent::Terminal);
    }

    #[cfg(unix)]
    #[test]
    fn open_command_tab_activates_new_tab() {
        use otty_ui_term::settings::{
            LocalSessionOptions, SessionKind, Settings,
        };

        let mut settings = Settings::default();
        settings.backend = settings.backend.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program("/bin/sh"),
            ),
        );

        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsIntent::OpenCommandTab {
                title: String::from("nvim main.rs"),
                settings: Box::new(settings),
                exit_behavior: ExitBehavior::Close,
                machine: TabMachine::ThisComputer,
            },
        );

        assert_eq!(state.len(), 1);
        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.title(), "nvim main.rs");
        assert_eq!(active.content(), TabContent::Terminal);
    }

    #[test]
    fn open_command_tab_keeps_the_machine_it_was_opened_for() {
        let mut state = TabsState::default();

        let _ = reduce(
            &mut state,
            TabsIntent::OpenCommandTab {
                title: String::from("htop"),
                settings: Box::new(Settings::default()),
                exit_behavior: ExitBehavior::Close,
                machine: TabMachine::Ssh(String::from("laptop")),
            },
        );

        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.machine(), &TabMachine::Ssh(String::from("laptop")));
    }

    #[test]
    fn tabs_opened_without_a_launch_run_on_this_computer() {
        let mut state = TabsState::default();
        let _ = open_terminal(&mut state, "zsh");
        let _ = reduce(&mut state, TabsIntent::OpenSettingsTab);
        let _ = reduce(
            &mut state,
            TabsIntent::OpenWizardTab {
                title: String::from("New launch"),
                init: WizardTabInit::Create {
                    parent_path: Vec::new(),
                },
            },
        );
        let _ = reduce(
            &mut state,
            TabsIntent::OpenErrorTab {
                title: String::from("Launch failed"),
                message: String::from("boom"),
            },
        );

        assert_eq!(state.len(), 4);
        assert!(
            state
                .tab_items()
                .values()
                .all(|tab| tab.machine() == &TabMachine::ThisComputer)
        );
    }

    #[test]
    fn open_settings_tab_activates_settings() {
        let mut state = TabsState::default();
        let _ = reduce(&mut state, TabsIntent::OpenSettingsTab);

        assert_eq!(state.len(), 1);
        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.title(), "Settings");
        assert_eq!(active.content(), TabContent::Settings);
    }

    #[test]
    fn open_settings_tab_when_already_open_activates_existing_tab() {
        let mut state = TabsState::default();
        let _ = reduce(&mut state, TabsIntent::OpenSettingsTab);
        let settings_id =
            state.active_tab_id().expect("settings tab should open");

        let _ = reduce(
            &mut state,
            TabsIntent::OpenTerminalTab {
                title: String::from("shell"),
            },
        );
        assert_eq!(state.len(), 2);

        let _ = reduce(&mut state, TabsIntent::OpenSettingsTab);

        assert_eq!(state.len(), 2);
        assert_eq!(state.active_tab_id(), Some(settings_id));
    }

    #[test]
    fn refresh_settings_title_updates_only_the_settings_tab() {
        let mut state = TabsState::default();
        let _ = reduce(&mut state, TabsIntent::OpenSettingsTab);
        let settings_id =
            state.active_tab_id().expect("settings tab should open");
        let _ = reduce(
            &mut state,
            TabsIntent::OpenTerminalTab {
                title: String::from("shell"),
            },
        );
        let terminal_id =
            state.active_tab_id().expect("terminal tab should open");

        let _ = reduce(
            &mut state,
            TabsIntent::RefreshSettingsTitle {
                title: String::from("Einstellungen"),
            },
        );

        assert_eq!(
            state
                .tab_items()
                .get(&settings_id)
                .expect("settings tab should remain open")
                .title(),
            "Einstellungen"
        );
        assert_eq!(
            state
                .tab_items()
                .get(&terminal_id)
                .expect("terminal tab should remain open")
                .title(),
            "shell"
        );
    }

    #[test]
    fn close_active_tab_activates_previous() {
        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsIntent::OpenTerminalTab {
                title: String::from("First"),
            },
        );
        let first_id = state.active_tab_id().unwrap();

        let _ = reduce(
            &mut state,
            TabsIntent::OpenTerminalTab {
                title: String::from("Second"),
            },
        );
        let second_id = state.active_tab_id().unwrap();

        let _ = reduce(&mut state, TabsIntent::CloseTab { tab_id: second_id });
        assert_eq!(state.active_tab_id(), Some(first_id));
    }

    #[test]
    fn close_last_tab_clears_active() {
        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsIntent::OpenTerminalTab {
                title: String::from("Only"),
            },
        );
        let tab_id = state.active_tab_id().unwrap();

        let _ = reduce(&mut state, TabsIntent::CloseTab { tab_id });
        assert!(state.active_tab_id().is_none());
        assert!(state.is_empty());
    }

    #[test]
    fn set_title_updates_tab() {
        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsIntent::OpenTerminalTab {
                title: String::from("old"),
            },
        );
        let tab_id = state.active_tab_id().unwrap();

        let _ = reduce(
            &mut state,
            TabsIntent::SetTitle {
                tab_id,
                title: String::from("new"),
                folder: Some(String::from("otty")),
            },
        );

        let tab = state.active_tab().unwrap();
        assert_eq!(tab.title(), "new");
        assert_eq!(tab.folder(), Some("otty"));
    }

    #[test]
    fn open_terminal_tab_emits_opened_and_scroll_effects() {
        let mut state = TabsState::default();

        let events = collect_outputs(reduce(
            &mut state,
            TabsIntent::OpenTerminalTab {
                title: String::from("bash"),
            },
        ));

        let tab_id = state.active_tab_id().expect("terminal tab should open");
        assert!(events.iter().any(|event| matches!(
            event,
            TabsEvent::Effect(TabsEffect::TerminalTabOpened { tab_id: id, title })
                if *id == tab_id && title == "bash"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            TabsEvent::Effect(TabsEffect::ScrollBarToEnd)
        )));
    }

    #[test]
    fn open_wizard_tab_activates_launch_editor_tab() {
        let mut state = TabsState::default();
        let _ = open_terminal(&mut state, "shell");

        let events = collect_outputs(reduce(
            &mut state,
            TabsIntent::OpenWizardTab {
                title: String::from("New launch"),
                init: WizardTabInit::Create {
                    parent_path: vec![String::from("Servers")],
                },
            },
        ));

        assert_eq!(state.len(), 2);
        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.title(), "New launch");
        assert_eq!(active.content(), TabContent::QuickLaunchWizard);
        let tab_id = active.id();
        assert!(events.iter().any(|event| matches!(
            event,
            TabsEvent::Effect(TabsEffect::WizardTabOpened {
                tab_id: id,
                init: WizardTabInit::Create { parent_path },
            }) if *id == tab_id && parent_path == &vec![String::from("Servers")]
        )));
    }

    #[test]
    fn open_error_tab_activates_error_tab() {
        let mut state = TabsState::default();
        let _ = open_terminal(&mut state, "shell");

        let events = collect_outputs(reduce(
            &mut state,
            TabsIntent::OpenErrorTab {
                title: String::from("Launch failed"),
                message: String::from("boom"),
            },
        ));

        assert_eq!(state.len(), 2);
        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.title(), "Launch failed");
        assert_eq!(active.content(), TabContent::QuickLaunchError);
        let tab_id = active.id();
        assert!(events.iter().any(|event| matches!(
            event,
            TabsEvent::Effect(TabsEffect::ErrorTabOpened {
                tab_id: id,
                title,
                message,
            }) if *id == tab_id && title == "Launch failed" && message == "boom"
        )));
    }

    #[test]
    fn open_settings_tab_emits_settings_opened_effect() {
        let mut state = TabsState::default();

        let events =
            collect_outputs(reduce(&mut state, TabsIntent::OpenSettingsTab));

        assert!(events.iter().any(|event| matches!(
            event,
            TabsEvent::Effect(TabsEffect::SettingsTabOpened)
        )));
    }

    #[test]
    fn activate_background_tab_makes_it_active_and_emits_activated() {
        let mut state = TabsState::default();
        let first_id = open_terminal(&mut state, "First");
        let _second_id = open_terminal(&mut state, "Second");

        let events = collect_outputs(reduce(
            &mut state,
            TabsIntent::ActivateTab { tab_id: first_id },
        ));

        assert_eq!(state.active_tab_id(), Some(first_id));
        assert!(matches!(
            events.as_slice(),
            [TabsEvent::Effect(TabsEffect::Activated { tab_id })]
                if *tab_id == first_id
        ));
    }

    #[test]
    fn close_background_tab_keeps_active_tab() {
        let mut state = TabsState::default();
        let first_id = open_terminal(&mut state, "First");
        let second_id = open_terminal(&mut state, "Second");

        let events = collect_outputs(reduce(
            &mut state,
            TabsIntent::CloseTab { tab_id: first_id },
        ));

        assert_eq!(state.active_tab_id(), Some(second_id));
        assert_eq!(state.len(), 1);
        assert_eq!(
            closed_effect(&events),
            Some((first_id, Some(second_id), 1))
        );
    }

    #[test]
    fn close_active_middle_tab_activates_the_tab_before_it() {
        let mut state = TabsState::default();
        let first_id = open_terminal(&mut state, "First");
        let second_id = open_terminal(&mut state, "Second");
        let _third_id = open_terminal(&mut state, "Third");
        let _ =
            reduce(&mut state, TabsIntent::ActivateTab { tab_id: second_id });

        let events = collect_outputs(reduce(
            &mut state,
            TabsIntent::CloseTab { tab_id: second_id },
        ));

        assert_eq!(state.active_tab_id(), Some(first_id));
        assert_eq!(
            closed_effect(&events),
            Some((second_id, Some(first_id), 2))
        );
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

        let events = collect_outputs(reduce(
            &mut state,
            TabsIntent::CloseTab { tab_id: first_id },
        ));

        assert_eq!(state.active_tab_id(), Some(third_id));
        assert_eq!(closed_effect(&events), Some((first_id, Some(third_id), 2)));
    }

    #[test]
    fn close_only_tab_reports_no_active_tab_and_nothing_left() {
        let mut state = TabsState::default();
        let tab_id = open_terminal(&mut state, "Only");

        let events = collect_outputs(reduce(
            &mut state,
            TabsIntent::CloseTab { tab_id },
        ));

        assert_eq!(closed_effect(&events), Some((tab_id, None, 0)));
    }

    #[test]
    fn close_unknown_tab_is_noop() {
        let mut state = TabsState::default();
        let tab_id = open_terminal(&mut state, "Only");

        let events = collect_outputs(reduce(
            &mut state,
            TabsIntent::CloseTab { tab_id: tab_id + 1 },
        ));

        assert!(events.is_empty());
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
}
