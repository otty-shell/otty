pub(crate) mod event;
pub(crate) mod model;
mod reducer;
mod state;
pub(crate) mod view;

use iced::Task;

pub(crate) use self::event::{TabsEffect, TabsEvent, TabsUiEvent};
use self::model::TabsViewModel;
use self::state::TabsState;

/// Tabs widget managing tab metadata and lifecycle.
pub(crate) struct TabsWidget {
    state: TabsState,
}

impl TabsWidget {
    /// Create the tabs widget with empty state.
    pub(crate) fn new() -> Self {
        Self {
            state: TabsState::default(),
        }
    }

    /// Reduce a tabs UI event into state updates and effect events.
    pub(crate) fn reduce(&mut self, event: TabsUiEvent) -> Task<TabsEvent> {
        reducer::reduce(&mut self.state, event)
    }

    /// Produce the tabs view model for rendering.
    pub(crate) fn vm(&self) -> TabsViewModel {
        TabsViewModel {
            tabs: self
                .state
                .tab_items()
                .iter()
                .map(|(id, item)| (*id, item.title().to_owned()))
                .collect(),
            active_tab_id: self.state.active_tab_id(),
            has_tabs: !self.state.is_empty(),
        }
    }

    /// Return the active tab identifier.
    pub(crate) fn active_tab_id(&self) -> Option<u64> {
        self.state.active_tab_id()
    }

    /// Return the content kind of the active tab.
    pub(crate) fn active_tab_content(&self) -> Option<model::TabContent> {
        self.state.active_tab().map(|tab| tab.content())
    }

    /// Return number of open tabs.
    pub(crate) fn len(&self) -> usize {
        self.state.len()
    }

    /// Return active tab title if present.
    pub(crate) fn active_tab_title(&self) -> Option<&str> {
        self.state.active_tab().map(|tab| tab.title())
    }
}
