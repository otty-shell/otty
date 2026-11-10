use otty_app_domain::mode::Mode;

use super::state::ModesState;

/// Read-only view model of the mode switcher and the All modes menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModesViewModel {
    /// The active mode, which decides what the panels show.
    pub(crate) active: Mode,
    /// Switcher tabs: Terminal, the pinned modes in pin order, then a
    /// temporary tab for the active mode when it is not pinned.
    pub(crate) tabs: Vec<ModeSwitcherTabViewModel>,
    /// The All modes menu is open.
    pub(crate) is_menu_open: bool,
    /// Rows of the menu's Pinned section: Terminal, then the pinned modes.
    pub(crate) pinned_rows: Vec<ModeMenuRowViewModel>,
    /// Rows of the menu's Other modes section, in [`Mode::ALL`] order.
    pub(crate) other_rows: Vec<ModeMenuRowViewModel>,
    /// Pinned modes besides Terminal, the "N" of the "N of 4" note.
    pub(crate) pinned_count: usize,
    /// Most modes that can be pinned, the "4" of the "N of 4" note.
    pub(crate) pin_limit: usize,
}

impl ModesViewModel {
    /// Build the view model from the widget state.
    pub(super) fn from_state(state: &ModesState) -> Self {
        let active = state.active();
        let shown = std::iter::once(Mode::Terminal)
            .chain(state.pinned().iter().copied());

        let mut tabs: Vec<ModeSwitcherTabViewModel> = shown
            .clone()
            .map(|mode| ModeSwitcherTabViewModel {
                mode,
                is_active: mode == active,
                is_temporary: false,
            })
            .collect();
        if !tabs.iter().any(|tab| tab.is_active) {
            tabs.push(ModeSwitcherTabViewModel {
                mode: active,
                is_active: true,
                is_temporary: true,
            });
        }

        let pinned_rows = shown
            .map(|mode| ModeMenuRowViewModel {
                mode,
                is_active: mode == active,
                pin: pinned_button(mode),
            })
            .collect();
        let other_rows = Mode::ALL
            .into_iter()
            .filter(|mode| *mode != Mode::Terminal && !state.is_pinned(*mode))
            .map(|mode| ModeMenuRowViewModel {
                mode,
                is_active: mode == active,
                pin: other_button(state),
            })
            .collect();

        Self {
            active,
            tabs,
            is_menu_open: state.is_menu_open(),
            pinned_rows,
            other_rows,
            pinned_count: state.pinned().len(),
            pin_limit: state.pin_limit(),
        }
    }
}

/// One icon tab of the mode switcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeSwitcherTabViewModel {
    pub(crate) mode: Mode,
    /// The tab of the active mode.
    pub(crate) is_active: bool,
    /// The active mode is not pinned: the tab has an outline and goes away
    /// once another mode is chosen.
    pub(crate) is_temporary: bool,
}

/// One row of the All modes menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeMenuRowViewModel {
    pub(crate) mode: Mode,
    /// The row of the active mode.
    pub(crate) is_active: bool,
    /// The pin toggle at the end of the row.
    pub(crate) pin: PinButton,
}

/// State of the pin toggle on a menu row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PinButton {
    /// No toggle: Terminal is always pinned.
    Hidden,
    /// The mode is pinned; the toggle unpins it.
    Unpin,
    /// The mode is not pinned and a slot is free; the toggle pins it.
    Pin,
    /// Every slot is taken: the toggle is disabled with the hint "Unpin
    /// another mode first".
    PinDisabled,
}

/// Pin toggle of a row in the Pinned section.
fn pinned_button(mode: Mode) -> PinButton {
    if mode == Mode::Terminal {
        return PinButton::Hidden;
    }

    PinButton::Unpin
}

/// Pin toggle of a row in the Other modes section.
fn other_button(state: &ModesState) -> PinButton {
    if state.is_full() {
        return PinButton::PinDisabled;
    }

    PinButton::Pin
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(
        mode: Mode,
        is_active: bool,
        is_temporary: bool,
    ) -> ModeSwitcherTabViewModel {
        ModeSwitcherTabViewModel {
            mode,
            is_active,
            is_temporary,
        }
    }

    fn row(
        mode: Mode,
        is_active: bool,
        pin: PinButton,
    ) -> ModeMenuRowViewModel {
        ModeMenuRowViewModel {
            mode,
            is_active,
            pin,
        }
    }

    #[test]
    fn first_start_shows_terminal_then_containers_and_a_closed_menu() {
        let vm = ModesViewModel::from_state(&ModesState::default());

        assert_eq!(
            vm,
            ModesViewModel {
                active: Mode::Terminal,
                tabs: vec![
                    tab(Mode::Terminal, true, false),
                    tab(Mode::Containers, false, false),
                ],
                is_menu_open: false,
                pinned_rows: vec![
                    row(Mode::Terminal, true, PinButton::Hidden),
                    row(Mode::Containers, false, PinButton::Unpin),
                ],
                other_rows: Vec::new(),
                pinned_count: 1,
                pin_limit: 4,
            }
        );
    }

    #[test]
    fn the_active_pinned_mode_is_its_own_tab_not_a_temporary_one() {
        let mut state = ModesState::default();
        state.select(Mode::Containers);

        let vm = ModesViewModel::from_state(&state);

        assert_eq!(
            vm.tabs,
            vec![
                tab(Mode::Terminal, false, false),
                tab(Mode::Containers, true, false),
            ]
        );
    }

    #[test]
    fn an_unpinned_mode_lists_under_other_modes_with_a_pin_toggle() {
        let state = ModesState::with_pinned(&[]);

        let vm = ModesViewModel::from_state(&state);

        assert_eq!(vm.tabs, vec![tab(Mode::Terminal, true, false)]);
        assert_eq!(
            vm.pinned_rows,
            vec![row(Mode::Terminal, true, PinButton::Hidden)]
        );
        assert_eq!(
            vm.other_rows,
            vec![row(Mode::Containers, false, PinButton::Pin)]
        );
        assert_eq!(vm.pinned_count, 0);
    }

    #[test]
    fn an_unpinned_active_mode_gets_a_temporary_tab_after_the_pinned_ones() {
        let mut state = ModesState::default();
        state.select(Mode::Containers);
        state.unpin(Mode::Containers);

        let vm = ModesViewModel::from_state(&state);

        assert_eq!(
            vm.tabs,
            vec![
                tab(Mode::Terminal, false, false),
                tab(Mode::Containers, true, true),
            ]
        );
        assert_eq!(
            vm.other_rows,
            vec![row(Mode::Containers, true, PinButton::Pin)]
        );
    }

    #[test]
    fn the_temporary_tab_goes_away_when_another_mode_is_chosen() {
        let mut state = ModesState::with_pinned(&[]);
        state.select(Mode::Containers);

        state.select(Mode::Terminal);
        let vm = ModesViewModel::from_state(&state);

        assert_eq!(vm.tabs, vec![tab(Mode::Terminal, true, false)]);
    }

    #[test]
    fn with_every_slot_taken_the_pin_toggle_is_disabled() {
        let state = ModesState::with_pin_limit(&[], 0);

        let vm = ModesViewModel::from_state(&state);

        assert_eq!(
            vm.other_rows,
            vec![row(Mode::Containers, false, PinButton::PinDisabled)]
        );
        assert_eq!(vm.pinned_count, 0);
        assert_eq!(vm.pin_limit, 0);
    }

    #[test]
    fn an_open_menu_is_reported() {
        let mut state = ModesState::default();
        state.toggle_menu();

        let vm = ModesViewModel::from_state(&state);

        assert!(vm.is_menu_open);
    }
}
