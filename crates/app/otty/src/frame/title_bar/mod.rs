//! The title bar at the top of the window: the window buttons, the panel
//! toggles, the search button, the bell and Settings.

mod event;
mod reducer;
mod state;
pub(crate) mod view;

pub(crate) use self::event::{TitleBarIntent, TitleBarOutcome};
use self::state::TitleBarState;

/// The title bar at the top of the window: it remembers whether the left
/// button is held on it, so a move of the pointer can start a window drag.
pub(crate) struct TitleBar {
    state: TitleBarState,
}

impl TitleBar {
    /// Return whether the left button is held on the empty title bar.
    pub(crate) fn is_title_bar_pressed(&self) -> bool {
        self.state.is_title_bar_pressed()
    }

    /// Create the title bar with no button held.
    pub(crate) fn new() -> Self {
        Self {
            state: TitleBarState::default(),
        }
    }
}

impl otty_app_slots::Component for TitleBar {
    type Intent = TitleBarIntent;
    type Outcome = TitleBarOutcome;
    type Ctx<'a> = ();

    fn reduce(
        &mut self,
        intent: TitleBarIntent,
        _ctx: (),
    ) -> Vec<TitleBarOutcome> {
        reducer::reduce(&mut self.state, intent)
    }
}
