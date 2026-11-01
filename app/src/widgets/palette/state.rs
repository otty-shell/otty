use otty_ui_kit::components::palette_list::{PaletteItem, PaletteList};

use super::types::PaletteTarget;

/// Command palette state: closed, or open with its entries, the typed query
/// and the active entry (all three kept by the kit [`PaletteList`]).
#[derive(Debug, Default)]
pub(crate) struct PaletteState {
    list: Option<PaletteList<PaletteTarget>>,
}

impl PaletteState {
    /// Entries, query and active entry while the palette is open.
    pub(crate) fn list(&self) -> Option<&PaletteList<PaletteTarget>> {
        self.list.as_ref()
    }

    /// Return whether the palette is open.
    pub(crate) fn is_open(&self) -> bool {
        self.list.is_some()
    }

    /// Target of the active entry.
    pub(crate) fn active_target(&self) -> Option<&PaletteTarget> {
        self.list.as_ref().and_then(|list| list.active_key())
    }

    /// Target of the entry at `index` among the matches.
    pub(crate) fn target_at(&self, index: usize) -> Option<&PaletteTarget> {
        self.list.as_ref().and_then(|list| list.key_at(index))
    }

    /// Open with `items`, an empty query and the first entry active.
    pub(crate) fn open(&mut self, items: Vec<PaletteItem<PaletteTarget>>) {
        self.list = Some(PaletteList::new(items));
    }

    /// Close the palette; return whether it was open.
    pub(crate) fn close(&mut self) -> bool {
        self.list.take().is_some()
    }

    /// Replace the entries with `items` (fresh from the other widgets) and
    /// match them against `query`. Does nothing while closed.
    pub(crate) fn change_query(
        &mut self,
        items: Vec<PaletteItem<PaletteTarget>>,
        query: String,
    ) {
        if !self.is_open() {
            return;
        }

        let mut list = PaletteList::new(items);
        list.set_query(query);
        self.list = Some(list);
    }

    /// Make the previous match active.
    pub(crate) fn move_up(&mut self) {
        if let Some(list) = self.list.as_mut() {
            list.move_up();
        }
    }

    /// Make the next match active.
    pub(crate) fn move_down(&mut self) {
        if let Some(list) = self.list.as_mut() {
            list.move_down();
        }
    }
}
