use super::types::{DEFAULT_PINNED, MAX_PINNED};
use crate::domain::mode::Mode;

/// Modes state: the active mode, the pinned modes in pin order and whether
/// the All modes menu is open (spec Decision 5).
///
/// Rules kept here: Terminal is never pinned, a mode is pinned at most once
/// and at most `pin_limit` modes are pinned. The active mode does not have
/// to be pinned; the switcher shows it as a temporary tab.
#[derive(Debug)]
pub(crate) struct ModesState {
    active: Mode,
    pinned: Vec<Mode>,
    pin_limit: usize,
    is_menu_open: bool,
}

impl ModesState {
    /// Mode shown in the left panel, main area, right panel and status bar.
    pub(crate) fn active(&self) -> Mode {
        self.active
    }

    /// Pinned modes in the order they were pinned; never Terminal.
    pub(crate) fn pinned(&self) -> &[Mode] {
        &self.pinned
    }

    /// Most modes that can be pinned at once.
    pub(crate) fn pin_limit(&self) -> usize {
        self.pin_limit
    }

    /// Return whether the All modes menu is open.
    pub(crate) fn is_menu_open(&self) -> bool {
        self.is_menu_open
    }

    /// Return whether `mode` is pinned.
    pub(crate) fn is_pinned(&self, mode: Mode) -> bool {
        self.pinned.contains(&mode)
    }

    /// Return whether every pin slot is taken.
    pub(crate) fn is_full(&self) -> bool {
        self.pinned.len() >= self.pin_limit
    }

    /// Mode at a switcher position: Terminal at 0, then the pinned modes in
    /// pin order. `None` past the pinned modes; a temporary tab never has a
    /// position.
    pub(crate) fn mode_at(&self, position: usize) -> Option<Mode> {
        match position {
            0 => Some(Mode::Terminal),
            _ => self.pinned.get(position - 1).copied(),
        }
    }

    /// Start in Terminal mode with the menu closed and `pinned` pinned in
    /// order. Entries the pin rules refuse (Terminal, repeats, entries past
    /// the limit) are left out.
    pub(crate) fn with_pinned(pinned: &[Mode]) -> Self {
        Self::build(pinned, MAX_PINNED)
    }

    /// Make `mode` active and close the menu; return whether the active
    /// mode changed.
    pub(crate) fn select(&mut self, mode: Mode) -> bool {
        self.is_menu_open = false;

        if self.active == mode {
            return false;
        }

        self.active = mode;
        true
    }

    /// Pin `mode` after the pinned ones; return whether it was pinned.
    /// Terminal, an already pinned mode and a mode past the limit are
    /// refused.
    pub(crate) fn pin(&mut self, mode: Mode) -> bool {
        if mode == Mode::Terminal || self.is_pinned(mode) || self.is_full() {
            return false;
        }

        self.pinned.push(mode);
        true
    }

    /// Unpin `mode`; return whether it was pinned. The active mode stays
    /// active even when it is the one unpinned.
    pub(crate) fn unpin(&mut self, mode: Mode) -> bool {
        let Some(index) = self.pinned.iter().position(|pinned| *pinned == mode)
        else {
            return false;
        };

        self.pinned.remove(index);
        true
    }

    /// Open the menu while closed, close it while open.
    pub(crate) fn toggle_menu(&mut self) {
        self.is_menu_open = !self.is_menu_open;
    }

    /// Close the menu.
    pub(crate) fn close_menu(&mut self) {
        self.is_menu_open = false;
    }

    /// Terminal active, menu closed, `pinned` pinned through the pin rules.
    fn build(pinned: &[Mode], pin_limit: usize) -> Self {
        let mut state = Self {
            active: Mode::Terminal,
            pinned: Vec::new(),
            pin_limit,
            is_menu_open: false,
        };

        for mode in pinned {
            state.pin(*mode);
        }

        state
    }
}

#[cfg(test)]
impl ModesState {
    /// Like [`ModesState::with_pinned`], with room for only `pin_limit`
    /// pinned modes. Today there is one mode to pin, so tests use a small
    /// limit to reach the full-slots rules.
    pub(crate) fn with_pin_limit(pinned: &[Mode], pin_limit: usize) -> Self {
        Self::build(pinned, pin_limit)
    }
}

impl Default for ModesState {
    /// First start: Terminal active, the default modes pinned.
    fn default() -> Self {
        Self::with_pinned(&DEFAULT_PINNED)
    }
}
