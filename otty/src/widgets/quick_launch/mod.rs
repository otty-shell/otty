mod errors;
pub(crate) mod event;
pub(crate) mod model;
pub(crate) mod reducer;
mod services;
pub(crate) mod state;
mod storage;
pub(crate) mod view;
mod wizard_model;

pub(crate) use event::{
    QuickLaunchEffect, QuickLaunchEvent, QuickLaunchIntent,
};
use iced::Task;
pub(crate) use reducer::QuickLaunchCtx;
use state::QuickLaunchState;

/// Quick launch widget: manages the tree of saved commands,
/// inline editing, context menus, drag-and-drop, wizard forms,
/// and async launch lifecycle.
pub(crate) struct QuickLaunchWidget {
    state: QuickLaunchState,
}

impl QuickLaunchWidget {
    /// Create a new widget with default state.
    pub(crate) fn new() -> Self {
        Self {
            state: QuickLaunchState::default(),
        }
    }

    /// Create a widget by loading persisted state from disk.
    pub(crate) fn load() -> Self {
        Self {
            state: storage::load_initial_quick_launch_state(),
        }
    }

    /// Process an event into state updates and follow-up actions.
    pub(crate) fn reduce(
        &mut self,
        event: QuickLaunchIntent,
        ctx: &QuickLaunchCtx<'_>,
    ) -> Task<QuickLaunchEvent> {
        reducer::reduce(&mut self.state, event, ctx)
    }

    /// Return a tree view model for the sidebar panel.
    pub(crate) fn tree_vm(&self) -> model::QuickLaunchTreeViewModel<'_> {
        model::QuickLaunchTreeViewModel {
            data: self.state.data(),
            selected_path: self.state.selected_path(),
            hovered_path: self.state.hovered_path(),
            inline_edit: self.state.inline_edit(),
            launching: self.state.launching(),
            drop_target: self.state.drop_target(),
        }
    }

    /// Return error tab state for a given tab id.
    pub(crate) fn error_tab(
        &self,
        tab_id: u64,
    ) -> Option<&state::QuickLaunchErrorState> {
        self.state.error_tab(tab_id)
    }

    /// Return wizard editor state for a given tab id.
    pub(crate) fn wizard_editor(
        &self,
        tab_id: u64,
    ) -> Option<&state::WizardEditorState> {
        self.state.wizard().editor(tab_id)
    }

    /// Return whether any launch is currently in progress.
    pub(crate) fn has_active_launches(&self) -> bool {
        self.state.has_active_launches()
    }

    /// Return the context menu state, if any.
    pub(crate) fn context_menu(&self) -> Option<&state::ContextMenuState> {
        self.state.context_menu()
    }

    /// Return the launching map.
    pub(crate) fn launching(
        &self,
    ) -> &std::collections::HashMap<model::NodePath, model::LaunchInfo> {
        self.state.launching()
    }

    /// Return whether there are unsaved changes that need persisting.
    pub(crate) fn state_is_dirty(&self) -> bool {
        self.state.is_dirty()
    }

    /// Return whether auto-persist is currently running.
    pub(crate) fn persist_in_flight(&self) -> bool {
        self.state.is_persist_in_flight()
    }

    /// Return whether inline edit is active.
    pub(crate) fn has_inline_edit(&self) -> bool {
        self.state.inline_edit().is_some()
    }

    /// Return read-only access to state for tests.
    #[cfg(test)]
    pub(crate) fn state(&self) -> &QuickLaunchState {
        &self.state
    }
}
