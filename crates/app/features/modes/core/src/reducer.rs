#![forbid(clippy::disallowed_methods)]

use otty_app_domain::mode::Mode;
use otty_app_i18n::{self as i18n, Key};

use super::event::{ModesIntent, ModesOutcome};
use super::state::ModesState;

/// Reduce a modes intent into state changes and outcomes. Switching mode
/// reports [`ModesOutcome::ModeChanged`]; pinning and unpinning report
/// [`ModesOutcome::SavePinned`]; the menu opening and closing report
/// [`ModesOutcome::MenuOpened`] and [`ModesOutcome::MenuClosed`], before the
/// switch the closing came with. The reducer does no I/O.
pub(super) fn reduce(
    state: &mut ModesState,
    intent: ModesIntent,
) -> Vec<ModesOutcome> {
    let was_menu_open = state.is_menu_open();

    let outcomes = reduce_intent(state, intent);

    let mut all = menu_change(state, was_menu_open);
    all.extend(outcomes);
    all
}

/// Change the state for `intent` and return its outcomes, all but the menu
/// change.
fn reduce_intent(
    state: &mut ModesState,
    intent: ModesIntent,
) -> Vec<ModesOutcome> {
    match intent {
        ModesIntent::Select(mode) => select(state, mode),
        ModesIntent::SelectAt { position } => match state.mode_at(position) {
            Some(mode) => select(state, mode),
            None => Vec::new(),
        },
        ModesIntent::Pin(mode) => {
            if !state.pin(mode) {
                return Vec::new();
            }
            save_pinned(state)
        },
        ModesIntent::Unpin(mode) => {
            if !state.unpin(mode) {
                return Vec::new();
            }
            save_pinned(state)
        },
        ModesIntent::ToggleMenu => {
            state.toggle_menu();
            Vec::new()
        },
        ModesIntent::CloseMenu => {
            state.close_menu();
            Vec::new()
        },
        ModesIntent::TabShown => show_terminal_for_a_tab(state),
        ModesIntent::PinnedSaved(result) => pinned_saved(result),
    }
}

/// Switch to `mode` and close the menu; report the switch only when the
/// active mode changed.
fn select(state: &mut ModesState, mode: Mode) -> Vec<ModesOutcome> {
    if !state.select(mode) {
        return Vec::new();
    }

    vec![ModesOutcome::ModeChanged(mode)]
}

/// A tab came to the front: tabs live in Terminal mode, so another mode
/// switches to it and closes the menu. In Terminal mode nothing changes, and
/// an open menu stays open.
fn show_terminal_for_a_tab(state: &mut ModesState) -> Vec<ModesOutcome> {
    if state.active() == Mode::Terminal {
        return Vec::new();
    }

    select(state, Mode::Terminal)
}

/// Report the menu opening or closing, when the intent changed it.
fn menu_change(state: &ModesState, was_menu_open: bool) -> Vec<ModesOutcome> {
    match (was_menu_open, state.is_menu_open()) {
        (false, true) => vec![ModesOutcome::MenuOpened],
        (true, false) => vec![ModesOutcome::MenuClosed(state.active())],
        _ => Vec::new(),
    }
}

/// Ask to save the pinned list as it is now.
fn save_pinned(state: &ModesState) -> Vec<ModesOutcome> {
    vec![ModesOutcome::SavePinned(state.pinned().to_vec())]
}

/// What a finished save of the pinned modes leads to: nothing when it
/// worked, the text that tells the user when it failed.
fn pinned_saved(result: Result<(), String>) -> Vec<ModesOutcome> {
    let Err(error) = result else {
        return Vec::new();
    };

    let text = i18n::with_error(Key::TplPinnedModesSaveFailed, &error);
    vec![ModesOutcome::PinnedSaveFailed(text)]
}

#[cfg(test)]
mod tests {
    use otty_app_i18n::{self as i18n, Key};

    use super::*;

    fn effects(
        state: &mut ModesState,
        intent: ModesIntent,
    ) -> Vec<ModesOutcome> {
        reduce(state, intent)
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
        assert_eq!(effects, vec![ModesOutcome::ModeChanged(Mode::Containers)]);
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
    fn a_shown_tab_in_another_mode_switches_to_terminal() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::Select(Mode::Containers));

        let effects = effects(&mut state, ModesIntent::TabShown);

        assert_eq!(state.active(), Mode::Terminal);
        assert_eq!(effects, vec![ModesOutcome::ModeChanged(Mode::Terminal)]);
    }

    #[test]
    fn a_shown_tab_in_terminal_mode_changes_nothing() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::ToggleMenu);

        let effects = effects(&mut state, ModesIntent::TabShown);

        assert_eq!(state.active(), Mode::Terminal);
        assert!(state.is_menu_open());
        assert!(effects.is_empty());
    }

    #[test]
    fn a_shown_tab_that_switches_modes_closes_the_menu() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::Select(Mode::Containers));
        let _ = effects(&mut state, ModesIntent::ToggleMenu);

        let _ = effects(&mut state, ModesIntent::TabShown);

        assert_eq!(state.active(), Mode::Terminal);
        assert!(!state.is_menu_open());
    }

    #[test]
    fn opening_the_menu_reports_it() {
        let mut state = ModesState::default();

        let effects = effects(&mut state, ModesIntent::ToggleMenu);

        assert!(state.is_menu_open());
        assert_eq!(effects, vec![ModesOutcome::MenuOpened]);
    }

    #[test]
    fn closing_the_menu_reports_it_with_the_active_mode() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::ToggleMenu);
        let toggled = effects(&mut state, ModesIntent::ToggleMenu);
        let _ = effects(&mut state, ModesIntent::Select(Mode::Containers));
        let _ = effects(&mut state, ModesIntent::ToggleMenu);

        let closed = effects(&mut state, ModesIntent::CloseMenu);

        assert!(!state.is_menu_open());
        assert_eq!(toggled, vec![ModesOutcome::MenuClosed(Mode::Terminal)]);
        assert_eq!(closed, vec![ModesOutcome::MenuClosed(Mode::Containers)]);
    }

    #[test]
    fn closing_a_closed_menu_reports_nothing() {
        let mut state = ModesState::default();

        let effects = effects(&mut state, ModesIntent::CloseMenu);

        assert!(!state.is_menu_open());
        assert!(effects.is_empty());
    }

    #[test]
    fn choosing_a_mode_in_the_open_menu_reports_its_closing_and_the_switch() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::ToggleMenu);
        let switched =
            effects(&mut state, ModesIntent::Select(Mode::Containers));
        let _ = effects(&mut state, ModesIntent::ToggleMenu);

        let kept = effects(&mut state, ModesIntent::Select(Mode::Containers));

        assert_eq!(
            switched,
            vec![
                ModesOutcome::MenuClosed(Mode::Containers),
                ModesOutcome::ModeChanged(Mode::Containers),
            ]
        );
        assert_eq!(kept, vec![ModesOutcome::MenuClosed(Mode::Containers)]);
    }

    #[test]
    fn pin_and_unpin_report_no_menu_change() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::ToggleMenu);

        let unpinned =
            effects(&mut state, ModesIntent::Unpin(Mode::Containers));
        let pinned = effects(&mut state, ModesIntent::Pin(Mode::Containers));

        assert_eq!(unpinned, vec![ModesOutcome::SavePinned(Vec::new())]);
        assert_eq!(
            pinned,
            vec![ModesOutcome::SavePinned(vec![Mode::Containers])]
        );
    }

    #[test]
    fn pinning_an_unpinned_mode_adds_it_last_and_saves() {
        let mut state = ModesState::with_pinned(&[]);

        let effects = effects(&mut state, ModesIntent::Pin(Mode::Containers));

        assert_eq!(state.pinned(), &[Mode::Containers]);
        assert_eq!(
            effects,
            vec![ModesOutcome::SavePinned(vec![Mode::Containers])]
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
        assert_eq!(effects, vec![ModesOutcome::SavePinned(Vec::new())]);
    }

    #[test]
    fn unpinning_the_active_mode_keeps_it_active() {
        let mut state = ModesState::default();
        let _ = effects(&mut state, ModesIntent::Select(Mode::Containers));

        let effects = effects(&mut state, ModesIntent::Unpin(Mode::Containers));

        assert_eq!(state.active(), Mode::Containers);
        assert_eq!(effects, vec![ModesOutcome::SavePinned(Vec::new())]);
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

        let saves: Vec<ModesOutcome> = intents
            .into_iter()
            .flat_map(|intent| effects(&mut state, intent))
            .filter(|effect| matches!(effect, ModesOutcome::SavePinned(_)))
            .collect();

        assert_eq!(
            saves,
            vec![
                ModesOutcome::SavePinned(Vec::new()),
                ModesOutcome::SavePinned(vec![Mode::Containers]),
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
        assert_eq!(effects, vec![ModesOutcome::ModeChanged(Mode::Terminal)]);
    }

    #[test]
    fn select_at_position_one_switches_to_the_first_pinned_mode() {
        let mut state = ModesState::default();

        let effects =
            effects(&mut state, ModesIntent::SelectAt { position: 1 });

        assert_eq!(state.active(), Mode::Containers);
        assert_eq!(effects, vec![ModesOutcome::ModeChanged(Mode::Containers)]);
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

    /// The switcher keeps what the user chose, and the user is told that
    /// the choice will not be there after a restart.
    #[test]
    fn a_failed_save_of_the_pinned_modes_tells_the_user() {
        let mut state = ModesState::default();

        let outcomes = effects(
            &mut state,
            ModesIntent::PinnedSaved(Err(String::from(
                "modes file IO failed: disk full",
            ))),
        );

        let expected = i18n::with_error(
            Key::TplPinnedModesSaveFailed,
            "modes file IO failed: disk full",
        );
        assert_eq!(outcomes, vec![ModesOutcome::PinnedSaveFailed(expected)]);
        assert_eq!(state.pinned(), &[Mode::Containers]);
    }

    #[test]
    fn a_saved_pin_tells_nothing() {
        let mut state = ModesState::default();

        let outcomes = effects(&mut state, ModesIntent::PinnedSaved(Ok(())));

        assert!(outcomes.is_empty());
    }
}
