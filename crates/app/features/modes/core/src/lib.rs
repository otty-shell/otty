//! The modes feature of the OTTY app: the active mode, the pinned modes of
//! the mode switcher and the All modes menu.
//!
//! The active mode decides what fills the left panel, the main area, the
//! right panel and the status bar; the binary reads it with
//! [`Modes::active`] and, on [`ModesOutcome::ModeChanged`], tells the other
//! features. The switcher has one place, above the left panel content, so it
//! is the feature's own method, [`Modes::mode_switcher`]. The All modes menu
//! is an overlay of the kind menu, through [`Overlays`].
//!
//! The feature reads and writes no files. Pinning and unpinning report
//! [`ModesOutcome::SavePinned`]; `otty-app-modes-io` writes the pinned modes
//! file and the answer comes back as [`ModesIntent::PinnedSaved`]. The side
//! panels stay with the frame.
//!
//! # Example
//!
//! ```
//! use otty_app_domain::mode::Mode;
//! use otty_app_modes::{DEFAULT_PINNED, Modes, ModesIntent, ModesOutcome};
//! use otty_app_slots::Feature;
//!
//! let mut modes = Modes::with_pinned(&DEFAULT_PINNED);
//! assert_eq!(modes.active(), Mode::Terminal);
//!
//! let switched = modes.reduce(ModesIntent::Select(Mode::Containers), ());
//! assert_eq!(switched, vec![ModesOutcome::ModeChanged(Mode::Containers)]);
//!
//! let unpinned = modes.reduce(ModesIntent::Unpin(Mode::Containers), ());
//! assert_eq!(unpinned, vec![ModesOutcome::SavePinned(Vec::new())]);
//! // The active mode stays active, as a temporary tab.
//! assert_eq!(modes.active(), Mode::Containers);
//! ```

mod event;
mod model;
mod reducer;
mod state;
mod types;
mod view;

pub use event::{ModesIntent, ModesOutcome};
use iced::Element;
use model::ModesViewModel;
use otty_app_domain::mode::Mode;
use otty_app_slots::{Feature, Overlay, Overlays};
use otty_app_theme::theme::ThemeProps;
use state::ModesState;
pub use types::DEFAULT_PINNED;
use view::{mode_menu, mode_switcher};

/// The modes feature: the active mode, the pinned modes and the All modes
/// menu.
///
/// # Example
///
/// ```
/// use otty_app_domain::mode::Mode;
/// use otty_app_modes::Modes;
///
/// // Terminal can not be pinned, and a mode is pinned once.
/// let modes = Modes::with_pinned(&[
///     Mode::Terminal,
///     Mode::Containers,
///     Mode::Containers,
/// ]);
///
/// assert_eq!(modes.pinned(), &[Mode::Containers]);
/// assert_eq!(
///     modes.switcher_modes(),
///     vec![Mode::Terminal, Mode::Containers]
/// );
/// assert!(!modes.is_menu_open());
/// ```
pub struct Modes {
    state: ModesState,
}

impl Modes {
    /// Mode shown in the left panel, main area, right panel and status bar.
    pub fn active(&self) -> Mode {
        self.state.active()
    }

    /// Return whether the All modes menu is open.
    pub fn is_menu_open(&self) -> bool {
        self.state.is_menu_open()
    }

    /// Pinned modes in the order they were pinned; never Terminal.
    pub fn pinned(&self) -> &[Mode] {
        self.state.pinned()
    }

    /// Terminal, then the pinned modes in pin order: each mode at its
    /// switcher position, which its mode shortcut reaches. A temporary tab
    /// has no position, so it is not listed.
    pub fn switcher_modes(&self) -> Vec<Mode> {
        std::iter::once(Mode::Terminal)
            .chain(self.state.pinned().iter().copied())
            .collect()
    }

    /// Start in Terminal mode with `pinned` pinned in order. Entries the pin
    /// rules refuse (Terminal, repeats, entries past the limit) are left out
    /// and logged.
    pub fn with_pinned(pinned: &[Mode]) -> Self {
        let state = ModesState::with_pinned(pinned);

        if state.pinned().len() < pinned.len() {
            log::warn!(
                "modes.json: Terminal, repeated modes and modes past the pin \
                 limit skipped"
            );
        }

        Self { state }
    }

    /// The mode switcher row (Terminal, the pinned modes, the temporary tab
    /// and "⋯"), which the frame puts above the left panel content.
    ///
    /// It is a method of its own and not [`otty_app_slots::LeftPanel`]: that
    /// trait draws one of the stacked sections of Terminal mode, and the
    /// switcher stands above the content of every mode.
    pub fn mode_switcher<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Element<'a, ModesIntent> {
        mode_switcher::view(mode_switcher::ModeSwitcherProps {
            vm: ModesViewModel::from_state(&self.state),
            theme,
        })
    }
}

impl Feature for Modes {
    type Intent = ModesIntent;
    type Outcome = ModesOutcome;
    type Ctx<'a> = ();

    fn reduce(&mut self, intent: ModesIntent, _ctx: ()) -> Vec<ModesOutcome> {
        reducer::reduce(&mut self.state, intent)
    }
}

impl Overlays for Modes {
    /// The open All modes menu, as a menu under the switcher. A press
    /// outside it closes it.
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, ModesIntent>> {
        let vm = ModesViewModel::from_state(&self.state);
        if !vm.is_menu_open {
            return None;
        }

        Some(Overlay::menu(mode_menu::view(mode_menu::ModeMenuProps {
            vm,
            theme,
        })))
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_slots::OverlayKind;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    #[test]
    fn a_closed_menu_has_no_overlay() {
        let themes = ThemeManager::new();
        let modes = Modes::with_pinned(&DEFAULT_PINNED);

        assert!(modes.overlay(ThemeProps::new(themes.current())).is_none());
    }

    #[test]
    fn an_open_menu_is_an_overlay_of_the_kind_menu() {
        let themes = ThemeManager::new();
        let mut modes = Modes::with_pinned(&DEFAULT_PINNED);
        let _ = modes.reduce(ModesIntent::ToggleMenu, ());

        let overlay = modes
            .overlay(ThemeProps::new(themes.current()))
            .expect("the open menu should be an overlay");

        assert_eq!(overlay.kind(), OverlayKind::Menu);
        let mut ui = simulator(overlay.into_content());
        assert!(ui.find(i18n::t(Key::ModesPinnedSection)).is_ok());
    }

    #[test]
    fn switcher_modes_are_terminal_then_the_pinned_ones() {
        let mut modes = Modes::with_pinned(&DEFAULT_PINNED);
        let pinned = modes.switcher_modes();
        let _ = modes.reduce(ModesIntent::Unpin(Mode::Containers), ());
        let _ = modes.reduce(ModesIntent::Select(Mode::Containers), ());

        let unpinned = modes.switcher_modes();

        assert_eq!(pinned, vec![Mode::Terminal, Mode::Containers]);
        assert_eq!(unpinned, vec![Mode::Terminal]);
    }

    #[test]
    fn with_pinned_keeps_what_the_pin_rules_allow() {
        let modes = Modes::with_pinned(&[Mode::Terminal, Mode::Containers]);

        assert_eq!(modes.active(), Mode::Terminal);
        assert_eq!(modes.pinned(), &[Mode::Containers]);
    }
}
