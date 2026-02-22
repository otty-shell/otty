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

    /// Return mutable tab collection.
    pub(crate) fn tab_items_mut(&mut self) -> &mut BTreeMap<u64, TabItem> {
        &mut self.tab_items
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

#[cfg(test)]
mod tests {
    use super::TabState;
    use crate::features::tab::{TabContent, TabItem};

    #[test]
    fn given_allocate_tab_id_when_called_then_ids_are_monotonic() {
        let mut state = TabState::default();

        assert_eq!(state.allocate_tab_id(), 0);
        assert_eq!(state.allocate_tab_id(), 1);
    }

    #[test]
    fn given_tabs_when_previous_tab_requested_then_returns_ordered_neighbor() {
        let mut state = TabState::default();
        state.insert(
            1,
            TabItem {
                id: 1,
                title: String::from("One"),
                content: TabContent::Settings,
            },
        );
        state.insert(
            5,
            TabItem {
                id: 5,
                title: String::from("Five"),
                content: TabContent::Settings,
            },
        );
        state.insert(
            8,
            TabItem {
                id: 8,
                title: String::from("Eight"),
                content: TabContent::Settings,
            },
        );

        assert_eq!(state.previous_tab_id(8), Some(5));
        assert_eq!(state.previous_tab_id(1), None);
        assert_eq!(state.last_tab_id(), Some(8));
    }
}
