mod constants;
mod errors;
pub(crate) mod event;
mod model;
mod reducer;
#[expect(
    clippy::disallowed_methods,
    reason = "this module checks programs and files before a launch"
)]
pub(crate) mod services;
mod state;
#[expect(clippy::disallowed_methods, reason = "this module saves the launches")]
pub(crate) mod storage;
pub(crate) mod types;
pub(crate) mod view;
mod wizard_draft;

pub(crate) use constants::QUICK_LAUNCHES_TICK_MS;
use event::{QuickLaunchEvent, QuickLaunchIntent};
use iced::Task;
pub(crate) use reducer::QuickLaunchCtx;
use state::QuickLaunchState;
use types::LaunchSummary;

/// Quick launch widget: manages the tree of saved commands,
/// inline editing, context menus, drag-and-drop, wizard forms,
/// and async launch lifecycle.
pub(crate) struct QuickLaunchWidget {
    state: QuickLaunchState,
}

impl QuickLaunchWidget {
    /// Create a widget by loading persisted state from disk.
    pub(crate) fn load() -> Self {
        Self {
            state: storage::load_initial_quick_launch_state(),
        }
    }

    /// Process an event into state updates and follow-up actions.
    pub(crate) fn reduce(
        &mut self,
        intent: QuickLaunchIntent,
        ctx: &QuickLaunchCtx<'_>,
    ) -> Task<QuickLaunchEvent> {
        reducer::reduce(&mut self.state, intent, ctx)
    }

    /// Return a tree view model for the sidebar panel.
    pub(crate) fn vm(&self) -> model::SidebarTreeViewModel<'_> {
        model::SidebarTreeViewModel::from_state(&self.state)
    }

    /// Every launch in the tree, folders left out, in tree order, each with
    /// the path that runs it.
    pub(crate) fn launches(&self) -> Vec<LaunchSummary<'_>> {
        self.state.data().launches()
    }

    /// The launches at the top level of the tree, in tree order, each with
    /// the path that runs it.
    pub(crate) fn top_level_launches(&self) -> Vec<LaunchSummary<'_>> {
        self.state.data().top_level_launches()
    }

    /// Return what the error tab `tab_id` shows, if it is an error tab.
    pub(crate) fn error_tab_vm(
        &self,
        tab_id: u64,
    ) -> Option<model::ErrorTabViewModel<'_>> {
        model::ErrorTabViewModel::from_state(&self.state, tab_id)
    }

    /// Return what the launch editor in tab `tab_id` shows, if the tab has
    /// one.
    pub(crate) fn wizard_vm(
        &self,
        tab_id: u64,
    ) -> Option<model::WizardFormViewModel<'_>> {
        model::WizardFormViewModel::from_state(&self.state, tab_id)
    }

    /// Return whether any launch is currently in progress.
    pub(crate) fn has_active_launches(&self) -> bool {
        self.state.has_active_launches()
    }

    /// Return whether the context menu of the tree is open.
    pub(crate) fn has_context_menu(&self) -> bool {
        self.state.context_menu().is_some()
    }

    /// Return the open context menu of the tree, if any.
    pub(crate) fn context_menu_vm(
        &self,
    ) -> Option<model::ContextMenuViewModel> {
        model::ContextMenuViewModel::from_state(&self.state)
    }

    /// Return whether the SSH secret dialog is open.
    pub(crate) fn has_secret_prompt(&self) -> bool {
        self.state.secret_prompt().is_some()
    }

    /// Return the open SSH secret dialog, if any.
    pub(crate) fn secret_prompt_vm(
        &self,
    ) -> Option<model::SecretPromptViewModel<'_>> {
        model::SecretPromptViewModel::from_state(&self.state)
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
}

#[cfg(test)]
impl QuickLaunchWidget {
    /// Return what is typed in the open SSH prompt.
    pub(crate) fn secret_prompt_value(&self) -> Option<&str> {
        self.state
            .secret_prompt()
            .map(state::SecretPromptState::value)
    }
}
