//! Runs the effects of the modes widget. Saving the pinned modes, with a
//! toast when that fails, and moving keyboard focus when the All modes menu
//! opens or closes happen here, off the reducer. A mode change also tells
//! the Containers widget whether it may talk to the engines (spec Decision
//! 14).

use iced::Task;

use super::notifications;
use crate::dispatch::AppEvent;
use crate::domain::mode::Mode;
use crate::focus;
use crate::i18n::{self, Key};
use crate::widgets::containers::event::{ContainersEvent, ContainersIntent};
use crate::widgets::modes::event::ModesEffect;
use crate::widgets::modes::storage;
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Run a modes effect.
pub(crate) fn run(effect: ModesEffect) -> Task<AppEvent> {
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
    use crate::app::App;
    use crate::testing::task_outputs::{collect_outputs, focuses_nothing};
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };

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
        let events =
            collect_outputs(run(ModesEffect::ModeChanged(Mode::Terminal)));

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
        let events =
            collect_outputs(run(ModesEffect::ModeChanged(Mode::Containers)));

        assert_eq!(events.len(), 1);
        assert!(is_containers(&events[0], &ContainersIntent::Enter));
    }

    #[test]
    fn nothing_talks_to_the_engines_while_terminal_mode_is_active() {
        let (mut app, _) = App::new();

        let to_containers =
            collect_outputs(run(ModesEffect::ModeChanged(Mode::Containers)));
        let to_terminal =
            collect_outputs(run(ModesEffect::ModeChanged(Mode::Terminal)));
        for event in to_containers.into_iter().chain(to_terminal) {
            if let AppEvent::Containers(ContainersEvent::Intent(intent)) = event
            {
                let _ = app.widgets.containers.reduce(intent);
            }
        }

        assert!(!app.widgets.containers.is_active());
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
        let task = run(ModesEffect::MenuOpened);

        assert!(focuses_nothing(task));
    }

    #[test]
    fn a_menu_closed_in_terminal_mode_gives_the_active_pane_its_keys_back() {
        let events =
            collect_outputs(run(ModesEffect::MenuClosed(Mode::Terminal)));

        assert_eq!(events.len(), 1);
        assert!(focuses_the_active_pane(&events[0]));
    }

    /// Variant A: after an overlay closes in Containers mode, nothing has the
    /// keyboard.
    #[test]
    fn a_menu_closed_in_containers_mode_focuses_nothing() {
        let task = run(ModesEffect::MenuClosed(Mode::Containers));

        assert_eq!(task.units(), 0);
    }
}
