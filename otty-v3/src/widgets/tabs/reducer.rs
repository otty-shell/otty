use iced::Task;
use otty_ui_term::settings::Settings;

use super::event::{TabsEffect, TabsEvent, TabsUiEvent};
use super::model::{TabContent, TabItem};
use super::state::TabsState;

/// Reduce a tabs UI event into state mutation and effect tasks.
pub(crate) fn reduce(
    state: &mut TabsState,
    event: TabsUiEvent,
) -> Task<TabsEvent> {
    match event {
        TabsUiEvent::ActivateTab { tab_id } => activate(state, tab_id),
        TabsUiEvent::CloseTab { tab_id } => close(state, tab_id),
        TabsUiEvent::SetTitle { tab_id, title } => {
            state.set_title(tab_id, title);
            Task::none()
        },
        TabsUiEvent::OpenTerminalTab { terminal_id, title } => {
            open_terminal_tab(state, terminal_id, title)
        },
        TabsUiEvent::OpenCommandTab {
            terminal_id,
            title,
            settings,
        } => open_command_tab(state, terminal_id, title, *settings),
        TabsUiEvent::OpenSettingsTab => open_settings_tab(state),
        TabsUiEvent::OpenWizardTab { title } => open_wizard_tab(state, title),
        TabsUiEvent::OpenErrorTab { title } => open_error_tab(state, title),
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

fn open_terminal_tab(
    state: &mut TabsState,
    terminal_id: u64,
    title: String,
) -> Task<TabsEvent> {
    let tab_id = state.allocate_tab_id();

    state.insert(
        tab_id,
        TabItem::new(tab_id, title.clone(), TabContent::Terminal),
    );
    state.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(TabsEvent::Effect(TabsEffect::TerminalTabOpened {
            tab_id,
            terminal_id,
            title,
        })),
        Task::done(TabsEvent::Effect(TabsEffect::ScrollBarToEnd)),
    ])
}

fn open_command_tab(
    state: &mut TabsState,
    terminal_id: u64,
    title: String,
    settings: Settings,
) -> Task<TabsEvent> {
    let tab_id = state.allocate_tab_id();

    state.insert(
        tab_id,
        TabItem::new(tab_id, title.clone(), TabContent::Terminal),
    );
    state.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(TabsEvent::Effect(TabsEffect::CommandTabOpened {
            tab_id,
            terminal_id,
            title,
            settings: Box::new(settings),
        })),
        Task::done(TabsEvent::Effect(TabsEffect::ScrollBarToEnd)),
    ])
}

fn open_wizard_tab(state: &mut TabsState, title: String) -> Task<TabsEvent> {
    let tab_id = state.allocate_tab_id();
    state.insert(
        tab_id,
        TabItem::new(tab_id, title, TabContent::QuickLaunchWizard),
    );
    state.activate(Some(tab_id));
    Task::batch(vec![
        Task::done(TabsEvent::Effect(TabsEffect::WizardTabOpened { tab_id })),
        Task::done(TabsEvent::Effect(TabsEffect::ScrollBarToEnd)),
    ])
}

fn open_error_tab(state: &mut TabsState, title: String) -> Task<TabsEvent> {
    let tab_id = state.allocate_tab_id();
    state.insert(
        tab_id,
        TabItem::new(tab_id, title, TabContent::QuickLaunchError),
    );
    state.activate(Some(tab_id));
    Task::batch(vec![
        Task::done(TabsEvent::Effect(TabsEffect::ErrorTabOpened { tab_id })),
        Task::done(TabsEvent::Effect(TabsEffect::ScrollBarToEnd)),
    ])
}

fn open_settings_tab(state: &mut TabsState) -> Task<TabsEvent> {
    let tab_id = state.allocate_tab_id();

    state.insert(
        tab_id,
        TabItem::new(tab_id, String::from("Settings"), TabContent::Settings),
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

    #[test]
    fn activate_nonexistent_tab_is_noop() {
        let mut state = TabsState::default();
        let _ = reduce(&mut state, TabsUiEvent::ActivateTab { tab_id: 999 });
        assert!(state.active_tab_id().is_none());
    }

    #[test]
    fn open_terminal_tab_activates_new_tab() {
        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsUiEvent::OpenTerminalTab {
                terminal_id: 1,
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
            TabsUiEvent::OpenCommandTab {
                terminal_id: 1,
                title: String::from("nvim main.rs"),
                settings: Box::new(settings),
            },
        );

        assert_eq!(state.len(), 1);
        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.title(), "nvim main.rs");
        assert_eq!(active.content(), TabContent::Terminal);
    }

    #[test]
    fn open_settings_tab_activates_settings() {
        let mut state = TabsState::default();
        let _ = reduce(&mut state, TabsUiEvent::OpenSettingsTab);

        assert_eq!(state.len(), 1);
        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.title(), "Settings");
        assert_eq!(active.content(), TabContent::Settings);
    }

    #[test]
    fn close_active_tab_activates_previous() {
        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsUiEvent::OpenTerminalTab {
                terminal_id: 1,
                title: String::from("First"),
            },
        );
        let first_id = state.active_tab_id().unwrap();

        let _ = reduce(
            &mut state,
            TabsUiEvent::OpenTerminalTab {
                terminal_id: 2,
                title: String::from("Second"),
            },
        );
        let second_id = state.active_tab_id().unwrap();

        let _ = reduce(&mut state, TabsUiEvent::CloseTab { tab_id: second_id });
        assert_eq!(state.active_tab_id(), Some(first_id));
    }

    #[test]
    fn close_last_tab_clears_active() {
        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsUiEvent::OpenTerminalTab {
                terminal_id: 1,
                title: String::from("Only"),
            },
        );
        let tab_id = state.active_tab_id().unwrap();

        let _ = reduce(&mut state, TabsUiEvent::CloseTab { tab_id });
        assert!(state.active_tab_id().is_none());
        assert!(state.is_empty());
    }

    #[test]
    fn set_title_updates_tab() {
        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsUiEvent::OpenTerminalTab {
                terminal_id: 1,
                title: String::from("old"),
            },
        );
        let tab_id = state.active_tab_id().unwrap();

        let _ = reduce(
            &mut state,
            TabsUiEvent::SetTitle {
                tab_id,
                title: String::from("new"),
            },
        );

        let tab = state.active_tab().unwrap();
        assert_eq!(tab.title(), "new");
    }
}
