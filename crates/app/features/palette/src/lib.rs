//! The command palette of the OTTY app: a search over the open sessions, the
//! launches and the app commands, opened by the title bar search or its
//! shortcut.
//!
//! The palette changes only its own state. Picking an entry closes it and
//! reports what was picked: [`PaletteOutcome::CommandPicked`] runs through
//! the same path as a shortcut, and the binary hands a picked session or
//! launch to the feature that owns it. What the palette lists, and whether a
//! command can run, it reads from [`PaletteCtx`], which the binary builds
//! from the other features before every intent.
//!
//! # Example
//!
//! ```
//! use otty_app_domain::commands::{AppCommand, CommandCtx};
//! use otty_app_domain::mode::Mode;
//! use otty_app_palette::{
//!     Palette, PaletteCtx, PaletteIntent, PaletteOutcome,
//! };
//! use otty_app_slots::Feature;
//! use otty_ui_kit::components::command_palette::PaletteAction;
//!
//! let ctx = || PaletteCtx {
//!     sessions: Vec::new(),
//!     launches: Vec::new(),
//!     pinned_modes: vec![Mode::Terminal, Mode::Containers],
//!     commands: CommandCtx {
//!         is_terminal_mode: true,
//!         has_active_tab: false,
//!         has_focused_pane: false,
//!     },
//! };
//! let mut palette = Palette::new();
//!
//! let opened = palette.reduce(PaletteIntent::Open, ctx());
//! assert_eq!(opened, vec![PaletteOutcome::FocusInput]);
//!
//! // The first entry without sessions and launches is New terminal.
//! let picked =
//!     palette.reduce(PaletteIntent::Action(PaletteAction::RunActive), ctx());
//! assert_eq!(
//!     picked,
//!     vec![PaletteOutcome::CommandPicked(AppCommand::NewTerminal)]
//! );
//! assert!(!palette.is_open());
//! ```

mod ctx;
mod event;
mod items;
mod model;
mod reducer;
mod state;
mod types;
mod view;

pub use ctx::{PaletteCtx, PaletteLaunch, PaletteSession};
pub use event::{PaletteIntent, PaletteOutcome};
use model::PaletteViewModel;
use otty_app_slots::{Feature, Overlay, Overlays};
use otty_app_theme::theme::ThemeProps;
use state::PaletteState;
use view::command_palette;

/// The command palette feature: closed, or open with its entries, the typed
/// query and the active entry.
///
/// # Example
///
/// ```
/// use otty_app_palette::Palette;
///
/// let palette = Palette::new();
///
/// assert!(!palette.is_open());
/// assert_eq!(palette.query(), None);
/// ```
pub struct Palette {
    state: PaletteState,
}

impl Palette {
    /// Return whether the palette is open.
    pub fn is_open(&self) -> bool {
        self.state.is_open()
    }

    /// Return what is typed in the palette input while it is open, or
    /// `None` while it is closed.
    pub fn query(&self) -> Option<&str> {
        self.state.list().map(|list| list.query())
    }

    /// Create a closed palette.
    pub fn new() -> Self {
        Self {
            state: PaletteState::default(),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}

impl Feature for Palette {
    type Intent = PaletteIntent;
    type Outcome = PaletteOutcome;
    type Ctx<'a> = PaletteCtx<'a>;

    fn reduce(
        &mut self,
        intent: PaletteIntent,
        ctx: PaletteCtx<'_>,
    ) -> Vec<PaletteOutcome> {
        reducer::reduce(&mut self.state, intent, &ctx)
    }
}

impl Overlays for Palette {
    /// The open palette, as a menu centered under the title bar over a
    /// transparent layer that covers the area the frame gives: a press on
    /// that layer closes it.
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, PaletteIntent>> {
        let vm = PaletteViewModel::from_state(&self.state)?;

        Some(Overlay::menu(command_palette::view(
            command_palette::CommandPaletteProps { vm, theme },
        )))
    }
}

#[cfg(test)]
mod tests {
    use otty_app_domain::commands::CommandCtx;
    use otty_app_slots::OverlayKind;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::command_palette::PaletteAction;

    use super::*;

    fn ctx() -> PaletteCtx<'static> {
        PaletteCtx {
            sessions: Vec::new(),
            launches: Vec::new(),
            pinned_modes: Vec::new(),
            commands: CommandCtx {
                is_terminal_mode: true,
                has_active_tab: false,
                has_focused_pane: false,
            },
        }
    }

    /// The open palette is a menu: the frame closes it on an outside click
    /// and a shortcut, and Esc closes it.
    #[test]
    fn the_open_palette_is_a_menu_overlay() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut palette = Palette::new();
        let closed = palette.overlay(theme).is_none();

        let _ = palette.reduce(PaletteIntent::Open, ctx());

        let kind = palette.overlay(theme).map(|open| open.kind());
        assert!(closed);
        assert_eq!(kind, Some(OverlayKind::Menu));
    }

    #[test]
    fn the_query_is_what_was_typed_while_the_palette_is_open() {
        let mut palette = Palette::new();
        let _ = palette.reduce(PaletteIntent::Open, ctx());

        let _ = palette.reduce(
            PaletteIntent::Action(PaletteAction::QueryChanged(String::from(
                "spl",
            ))),
            ctx(),
        );
        let typed = palette.query().map(String::from);
        let _ = palette.reduce(PaletteIntent::Dismiss, ctx());

        assert_eq!(typed.as_deref(), Some("spl"));
        assert_eq!(palette.query(), None);
    }
}
