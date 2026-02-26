use std::collections::BTreeMap;

use super::model::TabItem;

/// Runtime state for workspace tabs.
#[derive(Default)]
pub(crate) struct TabState {
    active_tab_id: Option<u64>,
    tab_items: BTreeMap<u64, TabItem>,
    next_tab_id: u64,
}

impl TabState {
    /// Return active tab identifier.
    pub(crate) fn active_tab_id(&self) -> Option<u64> {
        self.active_tab_id
    }

    /// Return all tab items keyed by tab identifier.
    pub(crate) fn tab_items(&self) -> &BTreeMap<u64, TabItem> {
        &self.tab_items
    }

    /// Return mutable tab item by identifier.
    pub(crate) fn tab_item_mut(&mut self, tab_id: u64) -> Option<&mut TabItem> {
        self.tab_items.get_mut(&tab_id)
    }

    /// Return number of tabs.
    pub(crate) fn len(&self) -> usize {
        self.tab_items.len()
    }

    /// Return whether there are no tabs.
    pub(crate) fn is_empty(&self) -> bool {
        self.tab_items.is_empty()
    }

    /// Return active tab item if present.
    pub(crate) fn active_tab(&self) -> Option<&TabItem> {
        let tab_id = self.active_tab_id?;
        self.tab_items.get(&tab_id)
    }

    /// Check whether tab with the provided identifier exists.
    pub(crate) fn contains(&self, tab_id: u64) -> bool {
        self.tab_items.contains_key(&tab_id)
    }

    /// Allocate next unique tab identifier.
    pub(crate) fn allocate_tab_id(&mut self) -> u64 {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;
        tab_id
    }

    /// Insert tab metadata by identifier.
    pub(crate) fn insert(&mut self, tab_id: u64, item: TabItem) {
        self.tab_items.insert(tab_id, item);
    }

    /// Remove tab metadata by identifier.
    pub(crate) fn remove(&mut self, tab_id: u64) -> Option<TabItem> {
        self.tab_items.remove(&tab_id)
    }

    /// Activate tab identifier.
    pub(crate) fn activate(&mut self, tab_id: Option<u64>) {
        self.active_tab_id = tab_id;
    }

    /// Update title for an existing tab.
    pub(crate) fn set_title(&mut self, tab_id: u64, title: String) {
        if let Some(tab) = self.tab_item_mut(tab_id) {
            tab.set_title(title);
        }
    }

    /// Return previous tab identifier before `tab_id`.
    pub(crate) fn previous_tab_id(&self, tab_id: u64) -> Option<u64> {
        self.tab_items
            .range(..tab_id)
            .next_back()
            .map(|(&id, _)| id)
    }

    /// Return last tab identifier in order.
    pub(crate) fn last_tab_id(&self) -> Option<u64> {
        self.tab_items.keys().next_back().copied()
    }
}
