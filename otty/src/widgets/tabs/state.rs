use std::collections::BTreeMap;

use super::types::TabItem;

/// Runtime state for workspace tabs.
#[derive(Default)]
pub(crate) struct TabsState {
    active_tab_id: Option<u64>,
    hovered_tab_id: Option<u64>,
    tab_items: BTreeMap<u64, TabItem>,
    next_tab_id: u64,
}

impl TabsState {
    /// Return active tab identifier.
    pub(crate) fn active_tab_id(&self) -> Option<u64> {
        self.active_tab_id
    }

    /// Return hovered tab identifier.
    pub(crate) fn hovered_tab_id(&self) -> Option<u64> {
        self.hovered_tab_id
    }

    /// Return whether a tab should show its close action.
    ///
    /// The close action appears only while the tab is hovered, matching the
    /// modern VS Code behavior where active tabs hide it until hovered.
    pub(crate) fn close_visible(&self, tab_id: u64) -> bool {
        self.hovered_tab_id == Some(tab_id)
    }

    /// Return all tab items keyed by tab identifier.
    pub(crate) fn tab_items(&self) -> &BTreeMap<u64, TabItem> {
        &self.tab_items
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

    /// Check whether a tab with the given identifier exists.
    pub(crate) fn contains(&self, tab_id: u64) -> bool {
        self.tab_items.contains_key(&tab_id)
    }

    /// Allocate next unique tab identifier.
    pub(super) fn allocate_tab_id(&mut self) -> u64 {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;
        tab_id
    }

    /// Insert tab metadata by identifier.
    pub(super) fn insert(&mut self, tab_id: u64, item: TabItem) {
        self.tab_items.insert(tab_id, item);
    }

    /// Remove tab metadata by identifier.
    pub(super) fn remove(&mut self, tab_id: u64) -> Option<TabItem> {
        if self.hovered_tab_id == Some(tab_id) {
            self.hovered_tab_id = None;
        }
        self.tab_items.remove(&tab_id)
    }

    /// Set the hovered tab identifier.
    pub(super) fn set_hovered(&mut self, tab_id: Option<u64>) {
        if tab_id.is_some_and(|tab_id| !self.tab_items.contains_key(&tab_id)) {
            return;
        }
        self.hovered_tab_id = tab_id;
    }

    /// Activate tab identifier.
    pub(super) fn activate(&mut self, tab_id: Option<u64>) {
        self.active_tab_id = tab_id;
    }

    /// Update title for an existing tab.
    pub(super) fn set_title(&mut self, tab_id: u64, title: String) {
        if let Some(tab) = self.tab_items.get_mut(&tab_id) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::tabs::types::TabContent;

    fn open_tab(state: &mut TabsState) -> u64 {
        let tab_id = state.allocate_tab_id();
        state.insert(
            tab_id,
            TabItem::new(tab_id, String::from("shell"), TabContent::Terminal),
        );
        tab_id
    }

    #[test]
    fn close_visible_requires_hover_even_when_active() {
        let mut state = TabsState::default();
        let tab_id = open_tab(&mut state);
        state.activate(Some(tab_id));

        assert!(!state.close_visible(tab_id));
    }

    #[test]
    fn close_visible_for_hovered_active_tab() {
        let mut state = TabsState::default();
        let tab_id = open_tab(&mut state);
        state.activate(Some(tab_id));
        state.set_hovered(Some(tab_id));

        assert!(state.close_visible(tab_id));
    }

    #[test]
    fn close_visible_for_hovered_inactive_tab() {
        let mut state = TabsState::default();
        let tab_id = open_tab(&mut state);
        let other_id = open_tab(&mut state);
        state.activate(Some(tab_id));
        state.set_hovered(Some(other_id));

        assert!(!state.close_visible(tab_id));
        assert!(state.close_visible(other_id));
    }

    #[test]
    fn close_visible_clears_when_hover_leaves() {
        let mut state = TabsState::default();
        let tab_id = open_tab(&mut state);
        state.set_hovered(Some(tab_id));
        state.set_hovered(None);

        assert!(!state.close_visible(tab_id));
    }
}
