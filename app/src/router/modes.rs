//! Routes the modes widget: reduces its intents and runs its effects.
//! Saving the pinned modes, with a toast when that fails, and moving
//! keyboard focus when the All modes menu opens or closes happen here, off
//! the reducer. A mode change also tells the
//! Containers widget whether it may talk to the engines (spec Decision 14).

use iced::Task;

use super::{AppEvent, notifications};
use crate::app::App;
use crate::domain::mode::Mode;
use crate::focus;
use crate::i18n::{self, Key};
use crate::widgets::containers::event::{ContainersEvent, ContainersIntent};
use crate::widgets::modes::event::{ModesEffect, ModesEvent};
use crate::widgets::modes::storage;
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Route a modes event: reduce its intents and run its effects.
pub(crate) fn route(app: &mut App, event: ModesEvent) -> Task<AppEvent> {
    match event {
        ModesEvent::Intent(intent) => {
            app.widgets.modes.reduce(intent).map(AppEvent::Modes)
        },
        ModesEvent::Effect(effect) => handle_effect(effect),
    }
}

fn handle_effect(effect: ModesEffect) -> Task<AppEvent> {
    match effect {
        // The root view reads the active mode when it draws. Back in
        // Terminal mode the panes get the current grid size, since the
        // window may have changed meanwhile, and the active pane its keys;
        // the Containers widget stops asking for engine work.
        ModesEffect::ModeChanged(Mode::Terminal) => Task::batch([
            Task::done(AppEvent::SyncTerminalGridSizes),
            focus_active_terminal(),
            containers(ContainersIntent::Leave),
        ]),
        // Entering Containers mode searches for engines and loads their lists.
        ModesEffect::ModeChanged(Mode::Containers) => {
            containers(ContainersIntent::Enter)
        },
        ModesEffect::SavePinned(pinned) => save_pinned(pinned),
        // An open menu takes focus from the terminal, so typed keys do not
        // reach the shell and Esc reaches the app, which closes the menu.
        ModesEffect::MenuOpened => focus::nothing(),
        // Once the menu closes in Terminal mode, the active pane gets its
        // keys back. In another mode nothing takes the keyboard.
        ModesEffect::MenuClosed(Mode::Terminal) => focus_active_terminal(),
        ModesEffect::MenuClosed(_) => Task::none(),
    }
}

fn containers(intent: ContainersIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Containers(ContainersEvent::Intent(intent)))
}

fn focus_active_terminal() -> Task<AppEvent> {
    Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
        TerminalWorkspaceIntent::FocusActive,
    )))
}

/// Write the pinned modes to `modes.json` in the background.
fn save_pinned(pinned: Vec<Mode>) -> Task<AppEvent> {
    Task::future(async move {
        storage::save_pinned(&pinned).map_err(|err| format!("{err}"))
    })
    .then(pinned_saved)
}

/// What a finished save of the pinned modes leads to: nothing when it
/// worked. A failed write is logged and told in a toast; the switcher
/// keeps what the user chose.
fn pinned_saved(result: Result<(), String>) -> Task<AppEvent> {
    let Err(error) = result else {
        return Task::none();
    };

    log::warn!("saving the pinned modes failed: {error}");
    notifications::push(i18n::with_error(Key::TplPinnedModesSaveFailed, &error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::{collect_outputs, focuses_nothing};
    use crate::testing::window::open_launch_editor_intent;
    use crate::widgets::modes::ModesWidget;
    use crate::widgets::modes::event::ModesIntent;
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };
    use crate::widgets::tabs::TabsCtx;
    use crate::widgets::tabs::event::TabsIntent;

    fn app() -> App {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        app
    }

    #[test]
    fn an_intent_is_reduced_by_the_widget_and_its_effect_comes_back() {
        let mut app = app();

        let events = collect_outputs(route(
            &mut app,
            ModesEvent::Intent(ModesIntent::Select(Mode::Containers)),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Effect(
                ModesEffect::ModeChanged(Mode::Containers)
            ))]
        ));
        assert_eq!(app.widgets.modes.vm().active, Mode::Containers);
    }

    #[test]
    fn unpinning_asks_to_save_the_new_pinned_list() {
        let mut app = app();

        let events = collect_outputs(route(
            &mut app,
            ModesEvent::Intent(ModesIntent::Unpin(Mode::Containers)),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Effect(ModesEffect::SavePinned(
                pinned
            )))] if pinned.is_empty()
        ));
    }

    fn focuses_the_active_pane(event: &AppEvent) -> bool {
        matches!(
            event,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))
        )
    }

    #[test]
    fn coming_back_to_terminal_syncs_the_grid_size_and_focuses_the_active_pane()
    {
        let mut app = app();

        let events = collect_outputs(route(
            &mut app,
            ModesEvent::Effect(ModesEffect::ModeChanged(Mode::Terminal)),
        ));

        assert!(matches!(
            events.first(),
            Some(AppEvent::SyncTerminalGridSizes)
        ));
        assert!(events.iter().any(focuses_the_active_pane));
        assert!(
            events
                .iter()
                .any(|event| is_containers(event, &ContainersIntent::Leave))
        );
        assert_eq!(events.len(), 3);
    }

    fn is_containers(event: &AppEvent, expected: &ContainersIntent) -> bool {
        matches!(
            event,
            AppEvent::Containers(ContainersEvent::Intent(intent)) if intent == expected
        )
    }

    #[test]
    fn switching_to_containers_only_starts_the_engine_search() {
        let mut app = app();

        let events = collect_outputs(route(
            &mut app,
            ModesEvent::Effect(ModesEffect::ModeChanged(Mode::Containers)),
        ));

        assert_eq!(events.len(), 1);
        assert!(is_containers(&events[0], &ContainersIntent::Enter));
    }

    #[test]
    fn nothing_talks_to_the_engines_while_terminal_mode_is_active() {
        let mut app = app();

        let to_containers = collect_outputs(route(
            &mut app,
            ModesEvent::Effect(ModesEffect::ModeChanged(Mode::Containers)),
        ));
        let to_terminal = collect_outputs(route(
            &mut app,
            ModesEvent::Effect(ModesEffect::ModeChanged(Mode::Terminal)),
        ));
        for event in to_containers.into_iter().chain(to_terminal) {
            if let AppEvent::Containers(ContainersEvent::Intent(intent)) = event
            {
                let _ = app.widgets.containers.reduce(intent);
            }
        }

        assert!(!app.widgets.containers.is_active());
    }

    #[test]
    fn switching_to_containers_and_back_keeps_the_tabs_and_the_active_tab() {
        let mut app = app();
        let ctx = TabsCtx { shell_name: "sh" };
        let _ = app.widgets.tabs.reduce(open_launch_editor_intent(), &ctx);
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenTerminalTab, &ctx);
        let tab_count = app.widgets.tabs.len();
        let active_tab = app.widgets.tabs.active_tab_id();

        let to_containers = collect_outputs(route(
            &mut app,
            ModesEvent::Intent(ModesIntent::Select(Mode::Containers)),
        ));
        let in_containers =
            (app.widgets.tabs.len(), app.widgets.tabs.active_tab_id());
        let back = collect_outputs(route(
            &mut app,
            ModesEvent::Intent(ModesIntent::Select(Mode::Terminal)),
        ));

        assert_eq!(to_containers.len(), 1);
        assert_eq!(in_containers, (tab_count, active_tab));
        assert!(matches!(
            back.as_slice(),
            [AppEvent::Modes(ModesEvent::Effect(
                ModesEffect::ModeChanged(Mode::Terminal)
            ))]
        ));
        assert_eq!(app.widgets.modes.active(), Mode::Terminal);
        assert_eq!(app.widgets.tabs.len(), tab_count);
        assert_eq!(app.widgets.tabs.active_tab_id(), active_tab);
    }

    /// The switcher keeps what the user chose, and a toast says the
    /// choice will not be there after a restart.
    #[test]
    fn a_failed_save_of_the_pinned_modes_tells_the_user() {
        let events = collect_outputs(pinned_saved(Err(String::from(
            "modes file IO failed: disk full",
        ))));

        let expected = crate::i18n::with_error(
            crate::i18n::Key::TplPinnedModesSaveFailed,
            "modes file IO failed: disk full",
        );
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::Push(text)
            ))] if *text == expected
        ));
        assert!(expected.contains("disk full"));
    }

    #[test]
    fn a_saved_pin_tells_nothing() {
        assert!(collect_outputs(pinned_saved(Ok(()))).is_empty());
    }

    #[test]
    fn a_menu_opened_effect_takes_the_keyboard_focus() {
        let mut app = app();

        let task = route(&mut app, ModesEvent::Effect(ModesEffect::MenuOpened));

        assert!(focuses_nothing(task));
    }

    #[test]
    fn a_menu_closed_in_terminal_mode_gives_the_active_pane_its_keys_back() {
        let mut app = app();

        let events = collect_outputs(route(
            &mut app,
            ModesEvent::Effect(ModesEffect::MenuClosed(Mode::Terminal)),
        ));

        assert_eq!(events.len(), 1);
        assert!(focuses_the_active_pane(&events[0]));
    }

    /// Variant A: after an overlay closes in Containers mode, nothing has the
    /// keyboard.
    #[test]
    fn a_menu_closed_in_containers_mode_focuses_nothing() {
        let mut app = app();

        let task = route(
            &mut app,
            ModesEvent::Effect(ModesEffect::MenuClosed(Mode::Containers)),
        );

        assert_eq!(task.units(), 0);
    }
}
