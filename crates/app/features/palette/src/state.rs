use otty_ui_kit::components::palette_list::{PaletteItem, PaletteList};

use super::types::PaletteTarget;

/// Command palette state: closed, or open with its entries, the typed query
/// and the lit entry (all three kept by the kit [`PaletteList`]).
#[derive(Debug, Default)]
pub(super) struct PaletteState {
    list: Option<PaletteList<PaletteTarget>>,
}

impl PaletteState {
    /// Entries, query and lit entry while the palette is open.
    pub(crate) fn list(&self) -> Option<&PaletteList<PaletteTarget>> {
        self.list.as_ref()
    }

    /// Return whether the palette is open.
    pub(crate) fn is_open(&self) -> bool {
        self.list.is_some()
    }

    /// Target of the entry at `index` among the matches.
    pub(crate) fn target_at(&self, index: usize) -> Option<&PaletteTarget> {
        self.list.as_ref().and_then(|list| list.key_at(index))
    }

    /// Open with `items`, an empty query and no entry lit.
    pub(crate) fn open(&mut self, items: Vec<PaletteItem<PaletteTarget>>) {
        self.list = Some(PaletteList::new(items));
    }

    /// Close the palette; return whether it was open.
    pub(crate) fn close(&mut self) -> bool {
        self.list.take().is_some()
    }

    /// Replace the entries with `items` (fresh from the other features) and
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

    /// Light the match at `row`, the row the keys of the palette chose.
    pub(crate) fn highlight(&mut self, row: Option<usize>) {
        if let Some(list) = self.list.as_mut() {
            list.highlight(row);
        }
    }
}
