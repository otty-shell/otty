use otty_ui_kit::components::palette_list::PaletteList;

use super::state::PaletteState;
use super::types::PaletteTarget;

/// Read-only view model of the open command palette.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PaletteViewModel<'a> {
    /// Entries, query and active entry.
    pub(crate) list: &'a PaletteList<PaletteTarget>,
}

impl<'a> PaletteViewModel<'a> {
    /// Build the view model from the palette state; `None` while the
    /// palette is closed.
    pub(crate) fn from_state(state: &'a PaletteState) -> Option<Self> {
        state.list().map(|list| Self { list })
    }
}
