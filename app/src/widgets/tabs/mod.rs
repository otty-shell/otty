pub(crate) mod event;
pub(crate) mod model;
mod reducer;
mod state;
pub(crate) mod types;
pub(crate) mod view;

use iced::Task;

pub(crate) use self::event::{TabsEffect, TabsEvent, TabsIntent};
use self::model::TabsViewModel;
pub(crate) use self::reducer::TabsCtx;
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

    /// Reduce a tabs intent event into state updates and effect events.
    pub(crate) fn reduce(
        &mut self,
        event: TabsIntent,
        ctx: &TabsCtx<'_>,
    ) -> Task<TabsEvent> {
        reducer::reduce(&mut self.state, event, ctx)
    }

    /// Produce the Sessions view model: open tabs grouped by machine.
    /// `host_name` names this computer (`None` when it could not be read).
    pub(crate) fn vm<'a>(
        &'a self,
        host_name: Option<&'a str>,
    ) -> TabsViewModel<'a> {
        TabsViewModel::new(
            self.state.tab_items(),
            self.state.active_tab_id(),
            host_name,
        )
    }

    /// Return the active tab identifier.
    pub(crate) fn active_tab_id(&self) -> Option<u64> {
        self.state.active_tab_id()
    }

    /// Return the content kind of the active tab.
    pub(crate) fn active_tab_content(&self) -> Option<types::TabContent> {
        self.state.active_tab().map(|tab| tab.content())
    }

    /// Return number of open tabs.
    pub(crate) fn len(&self) -> usize {
        self.state.len()
    }

    /// Return the machine the active tab runs on, if a tab is open.
    pub(crate) fn active_tab_machine(&self) -> Option<&types::TabMachine> {
        self.state.active_tab().map(|tab| tab.machine())
    }

    /// Return active tab title if present.
    pub(crate) fn active_tab_title(&self) -> Option<&str> {
        self.state.active_tab().map(|tab| tab.title())
    }
}
