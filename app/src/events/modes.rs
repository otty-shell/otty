//! Routes the modes widget: reduces its intents and runs its effects.
//! Saving the pinned modes happens here, off the reducer.

use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::modes::types::Mode;
use crate::widgets::modes::{ModesEffect, ModesEvent, storage};

/// Route a modes event: reduce its intents and run its effects.
pub(crate) fn handle(app: &mut App, event: ModesEvent) -> Task<AppEvent> {
    match event {
        ModesEvent::Intent(intent) => {
            app.widgets.modes.reduce(intent).map(AppEvent::Modes)
        },
        ModesEvent::Effect(effect) => handle_effect(effect),
    }
}

fn handle_effect(effect: ModesEffect) -> Task<AppEvent> {
    match effect {
        // The root view reads the active mode from the widget when it
        // draws; ticket 06 adds what has to happen on a switch.
        ModesEffect::ModeChanged(_) => Task::none(),
        ModesEffect::SavePinned(pinned) => save_pinned(pinned),
    }
}

/// Write the pinned modes to `modes.json` in the background. A failed
/// write is logged; the switcher keeps what the user chose.
fn save_pinned(pinned: Vec<Mode>) -> Task<AppEvent> {
    Task::perform(async move { storage::save_pinned(&pinned) }, |result| {
        if let Err(err) = result {
            log::warn!("saving the pinned modes failed: {err}");
        }
    })
    .discard()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::modes::{ModesIntent, ModesWidget};

    fn app() -> App {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        app
    }

    #[test]
    fn an_intent_is_reduced_by_the_widget_and_its_effect_comes_back() {
        let mut app = app();

        let events = collect_outputs(handle(
            &mut app,
            ModesEvent::Intent(ModesIntent::Select(Mode::Docker)),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Effect(
                ModesEffect::ModeChanged(Mode::Docker)
            ))]
        ));
        assert_eq!(app.widgets.modes.vm().active, Mode::Docker);
    }

    #[test]
    fn unpinning_asks_to_save_the_new_pinned_list() {
        let mut app = app();

        let events = collect_outputs(handle(
            &mut app,
            ModesEvent::Intent(ModesIntent::Unpin(Mode::Docker)),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Effect(ModesEffect::SavePinned(
                pinned
            )))] if pinned.is_empty()
        ));
    }

    #[test]
    fn a_mode_change_needs_nothing_else_yet() {
        let mut app = app();

        let events = collect_outputs(handle(
            &mut app,
            ModesEvent::Effect(ModesEffect::ModeChanged(Mode::Docker)),
        ));

        assert!(events.is_empty());
    }
}
