use iced::Task;

use super::command::TabsCommand;
use super::event::TabsEffect;
use super::model::{TabContent, TabItem};
use super::state::TabsState;

/// Reduce a tabs command into state mutation and effect tasks.
pub(crate) fn reduce(
    state: &mut TabsState,
    command: TabsCommand,
) -> Task<TabsEffect> {
    match command {
        TabsCommand::Activate { tab_id } => activate(state, tab_id),
        TabsCommand::Close { tab_id } => close(state, tab_id),
        TabsCommand::SetTitle { tab_id, title } => {
            state.set_title(tab_id, title);
            Task::none()
        },
        TabsCommand::OpenTerminalTab { terminal_id, title } => {
            open_terminal_tab(state, terminal_id, title)
        },
        TabsCommand::OpenSettingsTab => open_settings_tab(state),
        TabsCommand::OpenWizardTab { title } => open_wizard_tab(state, title),
        TabsCommand::OpenErrorTab { title } => open_error_tab(state, title),
    }
}

fn activate(state: &mut TabsState, tab_id: u64) -> Task<TabsEffect> {
    if !state.contains(tab_id) {
        return Task::none();
    }

    state.activate(Some(tab_id));
    Task::done(TabsEffect::Activated { tab_id })
}

fn close(state: &mut TabsState, tab_id: u64) -> Task<TabsEffect> {
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

    Task::done(TabsEffect::Closed {
        tab_id,
        new_active_id: state.active_tab_id(),
        remaining: state.len(),
    })
}

fn open_terminal_tab(
    state: &mut TabsState,
    terminal_id: u64,
    title: String,
) -> Task<TabsEffect> {
    let tab_id = state.allocate_tab_id();

    state.insert(
        tab_id,
        TabItem::new(tab_id, title.clone(), TabContent::Terminal),
    );
    state.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(TabsEffect::TerminalTabOpened {
            tab_id,
            terminal_id,
            title,
        }),
        Task::done(TabsEffect::ScrollBarToEnd),
    ])
}

fn open_wizard_tab(state: &mut TabsState, title: String) -> Task<TabsEffect> {
    let tab_id = state.allocate_tab_id();
    state.insert(
        tab_id,
        TabItem::new(tab_id, title, TabContent::QuickLaunchWizard),
    );
    state.activate(Some(tab_id));
    Task::batch(vec![
        Task::done(TabsEffect::WizardTabOpened { tab_id }),
        Task::done(TabsEffect::ScrollBarToEnd),
    ])
}

fn open_error_tab(state: &mut TabsState, title: String) -> Task<TabsEffect> {
    let tab_id = state.allocate_tab_id();
    state.insert(
        tab_id,
        TabItem::new(tab_id, title, TabContent::QuickLaunchError),
    );
    state.activate(Some(tab_id));
    Task::batch(vec![
        Task::done(TabsEffect::ErrorTabOpened { tab_id }),
        Task::done(TabsEffect::ScrollBarToEnd),
    ])
}

fn open_settings_tab(state: &mut TabsState) -> Task<TabsEffect> {
    let tab_id = state.allocate_tab_id();

    state.insert(
        tab_id,
        TabItem::new(tab_id, String::from("Settings"), TabContent::Settings),
    );
    state.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(TabsEffect::SettingsTabOpened),
        Task::done(TabsEffect::ScrollBarToEnd),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activate_nonexistent_tab_is_noop() {
        let mut state = TabsState::default();
        let _ = reduce(&mut state, TabsCommand::Activate { tab_id: 999 });
        assert!(state.active_tab_id().is_none());
    }

    #[test]
    fn open_terminal_tab_activates_new_tab() {
        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsCommand::OpenTerminalTab {
                terminal_id: 1,
                title: String::from("bash"),
            },
        );

        assert_eq!(state.len(), 1);
        let active = state.active_tab().expect("should have active tab");
        assert_eq!(active.title(), "bash");
        assert_eq!(active.content(), TabContent::Terminal);
    }

    #[test]
    fn open_settings_tab_activates_settings() {
        let mut state = TabsState::default();
        let _ = reduce(&mut state, TabsCommand::OpenSettingsTab);

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
            TabsCommand::OpenTerminalTab {
                terminal_id: 1,
                title: String::from("First"),
            },
        );
        let first_id = state.active_tab_id().unwrap();

        let _ = reduce(
            &mut state,
            TabsCommand::OpenTerminalTab {
                terminal_id: 2,
                title: String::from("Second"),
            },
        );
        let second_id = state.active_tab_id().unwrap();

        let _ = reduce(&mut state, TabsCommand::Close { tab_id: second_id });
        assert_eq!(state.active_tab_id(), Some(first_id));
    }

    #[test]
    fn close_last_tab_clears_active() {
        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsCommand::OpenTerminalTab {
                terminal_id: 1,
                title: String::from("Only"),
            },
        );
        let tab_id = state.active_tab_id().unwrap();

        let _ = reduce(&mut state, TabsCommand::Close { tab_id });
        assert!(state.active_tab_id().is_none());
        assert!(state.is_empty());
    }

    #[test]
    fn set_title_updates_tab() {
        let mut state = TabsState::default();
        let _ = reduce(
            &mut state,
            TabsCommand::OpenTerminalTab {
                terminal_id: 1,
                title: String::from("old"),
            },
        );
        let tab_id = state.active_tab_id().unwrap();

        let _ = reduce(
            &mut state,
            TabsCommand::SetTitle {
                tab_id,
                title: String::from("new"),
            },
        );

        let tab = state.active_tab().unwrap();
        assert_eq!(tab.title(), "new");
    }
}
