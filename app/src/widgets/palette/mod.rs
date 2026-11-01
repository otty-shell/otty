pub(crate) mod event;
mod items;
pub(crate) mod model;
mod reducer;
mod state;
pub(crate) mod types;
pub(crate) mod view;

pub(crate) use event::{PaletteEffect, PaletteEvent, PaletteIntent};
use iced::Task;
pub(crate) use model::PaletteViewModel;
pub(crate) use reducer::{PaletteCtx, PaletteLaunch, PaletteSession};
use state::PaletteState;

/// Command palette widget: a search over open sessions, launches and app
/// actions. Running an entry is reported as an effect; the palette never
/// changes other widgets' state.
pub(crate) struct PaletteWidget {
    state: PaletteState,
}

impl PaletteWidget {
    /// Create a closed palette.
    pub(crate) fn new() -> Self {
        Self {
            state: PaletteState::default(),
        }
    }

    /// Reduce an intent into state changes and effect events.
    pub(crate) fn reduce(
        &mut self,
        intent: PaletteIntent,
        ctx: &PaletteCtx<'_>,
    ) -> Task<PaletteEvent> {
        reducer::reduce(&mut self.state, intent, ctx)
    }

    /// View model while the palette is open.
    pub(crate) fn vm(&self) -> Option<PaletteViewModel<'_>> {
        self.state.list().map(|list| PaletteViewModel { list })
    }

    /// Return whether the palette is open.
    pub(crate) fn is_open(&self) -> bool {
        self.state.is_open()
    }
}
