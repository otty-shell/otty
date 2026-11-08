#![forbid(clippy::disallowed_methods)]

use iced::Task;

use super::event::{ModesEffect, ModesEvent, ModesIntent};
use super::state::ModesState;
use crate::domain::mode::Mode;

/// Reduce a modes intent into state changes and effects. Switching mode
/// reports [`ModesEffect::ModeChanged`]; pinning and unpinning report
/// [`ModesEffect::SavePinned`]. The reducer does no I/O.
pub(super) fn reduce(
    state: &mut ModesState,
    intent: ModesIntent,
) -> Task<ModesEvent> {
    match intent {
        ModesIntent::Select(mode) => select(state, mode),
        ModesIntent::SelectAt { position } => match state.mode_at(position) {
            Some(mode) => select(state, mode),
            None => Task::none(),
        },
        ModesIntent::Pin(mode) => {
            if !state.pin(mode) {
                return Task::none();
            }
            save_pinned(state)
        },
        ModesIntent::Unpin(mode) => {
            if !state.unpin(mode) {
                return Task::none();
            }
            save_pinned(state)
        },
        ModesIntent::ToggleMenu => {
            state.toggle_menu();
            Task::none()
        },
        ModesIntent::CloseMenu => {
            state.close_menu();
            Task::none()
        },
    }
}

/// Switch to `mode` and close the menu; report the switch only when the
/// active mode changed.
fn select(state: &mut ModesState, mode: Mode) -> Task<ModesEvent> {
    if !state.select(mode) {
        return Task::none();
    }

    effect(ModesEffect::ModeChanged(mode))
}

/// Ask the app to save the pinned list as it is now.
fn save_pinned(state: &ModesState) -> Task<ModesEvent> {
    effect(ModesEffect::SavePinned(state.pinned().to_vec()))
}

fn effect(effect: ModesEffect) -> Task<ModesEvent> {
    Task::done(ModesEvent::Effect(effect))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;

    fn effects(
        state: &mut ModesState,
        intent: ModesIntent,
    ) -> Vec<ModesEffect> {
        collect_outputs(reduce(state, intent))
            .into_iter()
            .map(|event| match event {
                ModesEvent::Effect(effect) => effect,
                ModesEvent::Intent(intent) => {
                    panic!("reducer should emit only effects, got {intent:?}")
                },
            })
            .collect()
    }

    #[test]
    fn first_start_is_terminal_with_containers_pinned_and_the_menu_closed() {
        let state = ModesState::default();

        assert_eq!(state.active(), Mode::Terminal);
        assert_eq!(state.pinned(), &[Mode::Containers]);
        assert_eq!(state.pin_limit(), 4);
        assert!(!state.is_menu_open());
    }

    #[test]
    fn selecting_another_mode_makes_it_active_and_reports_the_change() {
        let mut state = ModesState::default();

        let effects =
            effects(&mut state, ModesIntent::Select(Mode::Containers));

        assert_eq!(state.active(), Mode::Containers);
        assert_eq!(effects, vec![ModesEffect::ModeChanged(Mode::Containers)]);
    }

    #[test]
    fn selecting_the_active_mode_reports_nothing() {
        let mut state = ModesState::default();

        let effects = effects(&mut state, ModesIntent::Select(Mode::Terminal));

        assert_eq!(state.active(), Mode::Terminal);
        assert!(effects.is_empty());
    }

    #[test]
    fn selecting_a_mode_closes_the_menu() {
        for mode in Mode::ALL {
            let mut state = ModesState::default();
            let _ = effects(&mut state, ModesIntent::ToggleMenu);

            let _ = effects(&mut state, ModesIntent::Select(mode));

            assert!(!state.is_menu_open(), "{mode:?}");
        }
    }

    #[test]
    fn toggle_opens_and_closes_the_menu_without_effects() {
        let mut state = ModesState::default();

        let opened = effects(&mut state, ModesIntent::ToggleMenu);
        let was_open = state.is_menu_open();
        let closed = effects(&mut state, ModesIntent::ToggleMenu);

        assert!(was_open);
        assert!(!state.is_menu_open());
        assert!(opened.is_empty());
        assert!(closed.is_empty());
    }

    #[test]
    fn close_menu_closes_an_open_menu_and_keeps_a_closed_one_closed() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::ToggleMenu);

        let first = effects(&mut state, ModesIntent::CloseMenu);
        let was_open = state.is_menu_open();
        let second = effects(&mut state, ModesIntent::CloseMenu);

        assert!(!was_open);
        assert!(!state.is_menu_open());
        assert!(first.is_empty());
        assert!(second.is_empty());
    }

    #[test]
    fn pinning_an_unpinned_mode_adds_it_last_and_saves() {
        let mut state = ModesState::with_pinned(&[]);

        let effects = effects(&mut state, ModesIntent::Pin(Mode::Containers));

        assert_eq!(state.pinned(), &[Mode::Containers]);
        assert_eq!(
            effects,
            vec![ModesEffect::SavePinned(vec![Mode::Containers])]
        );
    }

    #[test]
    fn pinning_a_pinned_mode_changes_nothing() {
        let mut state = ModesState::default();

        let effects = effects(&mut state, ModesIntent::Pin(Mode::Containers));

        assert_eq!(state.pinned(), &[Mode::Containers]);
        assert!(effects.is_empty());
    }

    #[test]
    fn terminal_cannot_be_pinned() {
        let mut state = ModesState::with_pinned(&[]);

        let effects = effects(&mut state, ModesIntent::Pin(Mode::Terminal));

        assert!(state.pinned().is_empty());
        assert!(effects.is_empty());
    }

    #[test]
    fn pinning_when_every_slot_is_taken_is_refused() {
        let mut state = ModesState::with_pin_limit(&[], 0);

        let effects = effects(&mut state, ModesIntent::Pin(Mode::Containers));

        assert!(state.is_full());
        assert!(state.pinned().is_empty());
        assert!(effects.is_empty());
    }

    #[test]
    fn the_limit_is_four_pinned_modes() {
        let state = ModesState::with_pinned(&[]);

        assert_eq!(state.pin_limit(), 4);
        assert!(!state.is_full());
    }

    #[test]
    fn unpinning_a_pinned_mode_removes_it_and_saves() {
        let mut state = ModesState::default();

        let effects = effects(&mut state, ModesIntent::Unpin(Mode::Containers));

        assert!(state.pinned().is_empty());
        assert!(!state.is_pinned(Mode::Containers));
        assert_eq!(effects, vec![ModesEffect::SavePinned(Vec::new())]);
    }

    #[test]
    fn unpinning_the_active_mode_keeps_it_active() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::Select(Mode::Containers));

        let effects = effects(&mut state, ModesIntent::Unpin(Mode::Containers));

        assert_eq!(state.active(), Mode::Containers);
        assert_eq!(effects, vec![ModesEffect::SavePinned(Vec::new())]);
    }

    #[test]
    fn unpinning_terminal_or_an_unpinned_mode_changes_nothing() {
        let mut state = ModesState::with_pinned(&[]);

        let terminal = effects(&mut state, ModesIntent::Unpin(Mode::Terminal));
        let containers =
            effects(&mut state, ModesIntent::Unpin(Mode::Containers));

        assert!(state.pinned().is_empty());
        assert!(terminal.is_empty());
        assert!(containers.is_empty());
    }

    #[test]
    fn pin_and_unpin_keep_the_menu_open() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::ToggleMenu);

        let _ = effects(&mut state, ModesIntent::Unpin(Mode::Containers));
        let open_after_unpin = state.is_menu_open();
        let _ = effects(&mut state, ModesIntent::Pin(Mode::Containers));

        assert!(open_after_unpin);
        assert!(state.is_menu_open());
    }

    #[test]
    fn only_pin_and_unpin_save() {
        let mut state = ModesState::default();
        let intents = [
            ModesIntent::ToggleMenu,
            ModesIntent::Select(Mode::Containers),
            ModesIntent::CloseMenu,
            ModesIntent::Select(Mode::Terminal),
            ModesIntent::Unpin(Mode::Containers),
            ModesIntent::Pin(Mode::Containers),
        ];

        let saves: Vec<ModesEffect> = intents
            .into_iter()
            .flat_map(|intent| effects(&mut state, intent))
            .filter(|effect| matches!(effect, ModesEffect::SavePinned(_)))
            .collect();

        assert_eq!(
            saves,
            vec![
                ModesEffect::SavePinned(Vec::new()),
                ModesEffect::SavePinned(vec![Mode::Containers]),
            ]
        );
    }

    #[test]
    fn with_pinned_leaves_out_what_the_pin_rules_refuse() {
        let state = ModesState::with_pinned(&[
            Mode::Terminal,
            Mode::Containers,
            Mode::Containers,
        ]);

        assert_eq!(state.active(), Mode::Terminal);
        assert_eq!(state.pinned(), &[Mode::Containers]);
    }

    #[test]
    fn with_pinned_stops_at_the_limit() {
        let state = ModesState::with_pin_limit(&[Mode::Containers], 0);

        assert!(state.pinned().is_empty());
    }

    #[test]
    fn select_at_position_zero_switches_to_terminal() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::Select(Mode::Containers));

        let effects =
            effects(&mut state, ModesIntent::SelectAt { position: 0 });

        assert_eq!(state.active(), Mode::Terminal);
        assert_eq!(effects, vec![ModesEffect::ModeChanged(Mode::Terminal)]);
    }

    #[test]
    fn select_at_position_one_switches_to_the_first_pinned_mode() {
        let mut state = ModesState::default();

        let effects =
            effects(&mut state, ModesIntent::SelectAt { position: 1 });

        assert_eq!(state.active(), Mode::Containers);
        assert_eq!(effects, vec![ModesEffect::ModeChanged(Mode::Containers)]);
    }

    #[test]
    fn select_at_a_position_past_the_pinned_modes_does_nothing() {
        for position in 2..=4 {
            let mut state = ModesState::default();

            let effects =
                effects(&mut state, ModesIntent::SelectAt { position });

            assert_eq!(state.active(), Mode::Terminal, "{position}");
            assert!(effects.is_empty(), "{position}");
        }
    }

    #[test]
    fn select_at_without_pinned_modes_does_nothing() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::Unpin(Mode::Containers));

        let effects =
            effects(&mut state, ModesIntent::SelectAt { position: 1 });

        assert_eq!(state.active(), Mode::Terminal);
        assert!(effects.is_empty());
    }

    #[test]
    fn select_at_skips_the_temporary_tab() {
        // Containers unpinned but active: its tab is temporary, not pinned
        // mode 1.
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::Unpin(Mode::Containers));
        let _ = effects(&mut state, ModesIntent::Select(Mode::Containers));

        let effects =
            effects(&mut state, ModesIntent::SelectAt { position: 1 });

        assert_eq!(state.active(), Mode::Containers);
        assert!(effects.is_empty());
    }
}
