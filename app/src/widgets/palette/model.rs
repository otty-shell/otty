use otty_ui_kit::components::palette_list::PaletteList;

use super::types::PaletteTarget;

/// Read-only view model of the open command palette.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PaletteViewModel<'a> {
    /// Entries, query and active entry.
    pub(crate) list: &'a PaletteList<PaletteTarget>,
}
