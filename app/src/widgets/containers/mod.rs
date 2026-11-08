mod action_flow;
mod action_model;
pub(crate) mod actions;
pub(crate) mod errors;
pub(crate) mod event;
mod form_flow;
mod form_model;
pub(crate) mod format;
mod forms;
mod inspect_links;
mod inspect_tree;
pub(crate) mod inspector;
pub(crate) mod inspector_tabs;
mod links;
mod logs;
mod model;
pub(crate) mod no_engine;
mod overview;
mod reducer;
mod resource_rows;
mod run_checks;
mod run_flow;
pub(crate) mod run_form;
mod run_model;
mod sort;
mod state;
mod status;
mod subscription;
mod table_search;
pub(crate) mod types;
pub(crate) mod view;

use action_model::{ActionsViewModel, ConfirmViewModel, RowMenuViewModel};
use event::{ContainersEvent, ContainersIntent};
use form_model::FormViewModel;
use iced::{Subscription, Task, task};
use inspector_tabs::InspectorTabsViewModel;
pub(crate) use model::MainViewModel;
use model::{InspectorViewModel, TreeRowViewModel};
use run_model::{ImagePanelViewModel, RunFormViewModel};
use state::ContainersState;
use status::StatusViewModel;

/// Containers widget: the engines found on this computer, their containers,
/// images, volumes and networks, the open list, the selected rows and the
/// tag search of each page (spec Decisions 5, 8, 10 and 21). It asks for
/// engine work only while Containers mode is active.
pub(crate) struct ContainersWidget {
    state: ContainersState,
    /// Aborts the running image pull when dropped or cancelled. It lives
    /// beside the state because the reducer never holds running tasks.
    pull_task: Option<task::Handle>,
}

impl ContainersWidget {
    /// Return whether Containers mode is the active mode.
    pub(crate) fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Return whether the widget counts the right panel as off the screen.
    pub(crate) fn is_right_panel_hidden(&self) -> bool {
        self.state.is_right_panel_hidden()
    }

    /// Create the widget before any engine search.
    pub(crate) fn new() -> Self {
        Self {
            state: ContainersState::default(),
            pull_task: None,
        }
    }

    /// Keep the handle of the pull that just started; an earlier one is
    /// aborted.
    pub(crate) fn set_pull_task(&mut self, handle: task::Handle) {
        self.pull_task = Some(handle.abort_on_drop());
    }

    /// Stop the running pull, if any.
    pub(crate) fn cancel_pull(&mut self) {
        if let Some(handle) = self.pull_task.take() {
            handle.abort();
        }
    }

    /// Reduce an intent into state changes and effect events. The clock of
    /// the tag search moves after the intent, so the rows the next view shows
    /// and the rows the next press is looked up in are counted at one time.
    pub(crate) fn reduce(
        &mut self,
        intent: ContainersIntent,
    ) -> Task<ContainersEvent> {
        let task = reducer::reduce(&mut self.state, intent);
        self.state.set_age_clock(format::unix_now());

        task
    }

    /// Build the rows of the engine tree in the left panel.
    pub(crate) fn tree_vm(&self) -> Vec<TreeRowViewModel> {
        model::tree_rows(&self.state)
    }

    /// Build the main area; ages are counted up to `now` (Unix seconds).
    pub(crate) fn main_vm(&self, now: i64) -> MainViewModel<'_> {
        model::main_area(&self.state, now)
    }

    /// The short text under the engine tree while no engine is usable.
    pub(crate) fn tree_note(&self) -> Option<&'static str> {
        model::panel_note(&self.state)
    }

    /// Build the texts of the status bar.
    pub(crate) fn status_vm(&self) -> StatusViewModel {
        status::status_bar(&self.state)
    }

    /// Build the inspector header of the selected row.
    pub(crate) fn inspector_vm(&self) -> Option<InspectorViewModel> {
        model::inspector(&self.state)
    }

    /// Build the inspector tabs of the selected row and the open tab's body.
    pub(crate) fn inspector_tabs_vm(
        &self,
    ) -> Option<InspectorTabsViewModel<'_>> {
        inspector_tabs::inspector_tabs(&self.state)
    }

    /// The Containers mode subscriptions: the engine events watch, the followed
    /// logs, the timer of the Run form's folders and the timer of the no-engine
    /// card. Nothing runs in another mode (spec Decisions 9, 12 and 14).
    pub(crate) fn subscription(&self) -> Subscription<ContainersEvent> {
        Subscription::batch([
            self.engine_watch(),
            self.followed_logs(),
            subscription::folder_recheck(&self.state),
            subscription::engine_search_timer(&self.state),
        ])
    }

    /// Watch the events of every reached engine while Containers mode is
    /// active; nothing is watched in another mode.
    fn engine_watch(&self) -> Subscription<ContainersEvent> {
        if !self.is_active() {
            return Subscription::none();
        }

        subscription::subscription(&self.state)
    }

    /// Follow the logs of the selected container while the Logs tab is on
    /// screen in Containers mode; nothing otherwise.
    fn followed_logs(&self) -> Subscription<ContainersEvent> {
        let target = self.state.inspector().logs().map(|logs| logs.target());

        logs::subscription(target)
    }

    /// Build the action buttons of the selected row.
    pub(crate) fn actions_vm(&self) -> Option<ActionsViewModel> {
        action_model::action_buttons(&self.state)
    }

    /// Build the open row menu.
    pub(crate) fn row_menu_vm(&self) -> Option<RowMenuViewModel> {
        action_model::row_menu(&self.state)
    }

    /// Build the open confirmation dialog.
    pub(crate) fn confirm_vm(&self) -> Option<ConfirmViewModel> {
        action_model::confirm_dialog(&self.state)
    }

    /// Build the open create dialog.
    pub(crate) fn form_vm(&self) -> Option<FormViewModel<'_>> {
        form_model::form_dialog(&self.state)
    }

    /// Build the Run container form, shown in place of the table.
    pub(crate) fn run_form_vm(&self) -> Option<RunFormViewModel<'_>> {
        run_model::run_form(&self.state)
    }

    /// Build the image panel beside the Run container form, shown in place
    /// of the inspector.
    pub(crate) fn image_panel_vm(&self) -> Option<ImagePanelViewModel> {
        run_model::image_panel(&self.state)
    }

    /// Return whether a row menu or a dialog covers the window.
    pub(crate) fn has_overlay(&self) -> bool {
        self.state.row_menu().is_some()
            || self.state.confirm().is_some()
            || self.state.form().is_some()
    }
}

#[cfg(test)]
impl ContainersWidget {
    /// Return whether a pull task is kept.
    pub(crate) fn has_pull_task(&self) -> bool {
        self.pull_task.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::hash::Hasher as _;

    use iced::advanced::subscription::{Hasher, into_recipes};
    use otty_containers::{EngineKind, fixtures};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::containers::inspector::InspectorTab;
    use crate::widgets::containers::run_form::RunEdit;
    use crate::widgets::containers::types::LoadedLists;

    /// Containers mode active with the fixture Docker engine and its lists.
    fn loaded() -> ContainersWidget {
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let mut widget = ContainersWidget::new();
        let _ = widget.reduce(ContainersIntent::Enter);
        let _ = widget.reduce(ContainersIntent::DiscoveryFinished(
            fixtures::discovery(&[EngineKind::Docker]),
        ));
        let _ = widget.reduce(ContainersIntent::ListsLoaded {
            engine: docker.address().clone(),
            lists: LoadedLists {
                containers: Ok(resources.containers().to_vec()),
                images: Ok(resources.images().to_vec()),
                volumes: Ok(resources.volumes().to_vec()),
                networks: Ok(resources.networks().to_vec()),
            },
        });
        widget
    }

    /// The identity of every running logs subscription.
    fn subscriptions(widget: &ContainersWidget) -> Vec<u64> {
        into_recipes(widget.followed_logs())
            .iter()
            .map(|recipe| {
                let mut hasher = Hasher::default();
                recipe.hash(&mut hasher);
                hasher.finish()
            })
            .collect()
    }

    #[test]
    fn each_intent_moves_the_clock_of_the_tag_search() {
        let mut widget = ContainersWidget::new();
        assert_eq!(widget.state.age_clock(), 0);

        let _ = widget.reduce(ContainersIntent::Leave);

        assert!(widget.state.age_clock() > 0);
    }

    #[test]
    fn the_logs_subscription_runs_only_on_the_logs_tab_of_a_container() {
        let mut widget = loaded();
        let on_overview = subscriptions(&widget);

        let _ = widget
            .reduce(ContainersIntent::SelectInspectorTab(InspectorTab::Logs));

        assert!(on_overview.is_empty());
        assert_eq!(subscriptions(&widget).len(), 1);
    }

    #[test]
    fn the_logs_subscription_stops_when_another_tab_opens() {
        let mut widget = loaded();
        let _ = widget
            .reduce(ContainersIntent::SelectInspectorTab(InspectorTab::Logs));

        let _ = widget.reduce(ContainersIntent::SelectInspectorTab(
            InspectorTab::Inspect,
        ));

        assert!(subscriptions(&widget).is_empty());
    }

    #[test]
    fn the_logs_subscription_is_replaced_when_the_selection_changes() {
        let mut widget = loaded();
        let _ = widget
            .reduce(ContainersIntent::SelectInspectorTab(InspectorTab::Logs));
        let first = subscriptions(&widget);

        let _ = widget.reduce(ContainersIntent::SelectRow(0));
        let second = subscriptions(&widget);
        let _ = widget.reduce(ContainersIntent::SelectRow(0));

        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert_ne!(first, second);
        assert_eq!(subscriptions(&widget), second);
    }

    /// A finished stream keeps its place in iced as long as its key stays,
    /// so only a new key can start the logs again after a restart.
    #[test]
    fn the_logs_subscription_is_replaced_when_the_container_starts_again() {
        let mut widget = loaded();
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let _ = widget
            .reduce(ContainersIntent::SelectInspectorTab(InspectorTab::Logs));
        let selected = widget
            .state
            .inspector()
            .logs()
            .expect("followed logs")
            .target()
            .container_id()
            .to_string();
        let first = subscriptions(&widget);
        let other = resources
            .containers()
            .iter()
            .map(|container| container.id())
            .find(|id| *id != selected)
            .expect("another container");

        let _ = widget.reduce(ContainersIntent::ContainerStarted {
            engine: docker.address().clone(),
            id: other.to_string(),
        });
        let after_other = subscriptions(&widget);
        let _ = widget.reduce(ContainersIntent::ContainerStarted {
            engine: docker.address().clone(),
            id: selected,
        });
        let after_own = subscriptions(&widget);

        assert_eq!(after_other, first);
        assert_eq!(after_own.len(), 1);
        assert_ne!(after_own, first);
    }

    #[test]
    fn the_logs_subscription_stops_when_containers_mode_is_left() {
        let mut widget = loaded();
        let _ = widget
            .reduce(ContainersIntent::SelectInspectorTab(InspectorTab::Logs));

        let _ = widget.reduce(ContainersIntent::Leave);

        assert!(subscriptions(&widget).is_empty());
    }

    #[test]
    fn the_logs_subscription_stops_while_the_run_form_is_open() {
        let mut widget = loaded();
        let _ = widget
            .reduce(ContainersIntent::SelectInspectorTab(InspectorTab::Logs));

        let _ = widget.reduce(ContainersIntent::OpenCreate);
        let while_open = subscriptions(&widget);
        let _ = widget.reduce(ContainersIntent::RunCancelled);

        assert!(while_open.is_empty());
        assert_eq!(subscriptions(&widget).len(), 1);
    }

    #[test]
    fn the_logs_subscription_stops_while_the_right_panel_is_hidden() {
        let mut widget = loaded();
        let _ = widget
            .reduce(ContainersIntent::SelectInspectorTab(InspectorTab::Logs));

        let _ = widget.reduce(ContainersIntent::RightPanelHidden(true));
        let while_hidden = subscriptions(&widget);
        let _ = widget.reduce(ContainersIntent::RightPanelHidden(false));

        assert!(while_hidden.is_empty());
        assert_eq!(subscriptions(&widget).len(), 1);
    }

    fn found(widget: &mut ContainersWidget) {
        let _ = collect_outputs(widget.reduce(ContainersIntent::Enter));
        let _ = collect_outputs(widget.reduce(
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        ));
    }

    #[test]
    fn engines_are_watched_only_in_containers_mode() {
        let mut widget = ContainersWidget::new();
        assert_eq!(widget.engine_watch().units(), 0);

        found(&mut widget);
        assert_eq!(widget.engine_watch().units(), 2);

        let _ = collect_outputs(widget.reduce(ContainersIntent::Leave));
        assert_eq!(widget.engine_watch().units(), 0);
    }

    #[test]
    fn containers_mode_runs_the_engine_watch_and_the_followed_logs_together() {
        let mut widget = loaded();
        assert_eq!(widget.subscription().units(), 1);

        let _ = widget
            .reduce(ContainersIntent::SelectInspectorTab(InspectorTab::Logs));
        assert_eq!(widget.subscription().units(), 2);

        let _ = widget.reduce(ContainersIntent::Leave);
        assert_eq!(widget.subscription().units(), 0);
    }

    #[test]
    fn the_no_engine_card_adds_its_timer_to_containers_mode() {
        let mut widget = ContainersWidget::new();
        let _ = widget.reduce(ContainersIntent::Enter);
        let _ = widget.reduce(ContainersIntent::DiscoveryFinished(
            fixtures::discovery(&[]),
        ));

        assert_eq!(widget.subscription().units(), 1);

        let _ = widget.reduce(ContainersIntent::Leave);
        assert_eq!(widget.subscription().units(), 0);
    }

    #[test]
    fn a_folder_row_adds_the_folder_timer_to_containers_mode() {
        let mut widget = loaded();
        let _ = widget.reduce(ContainersIntent::OpenCreate);
        for edit in [
            RunEdit::AddVolume,
            RunEdit::VolumeSource(0, String::from("/srv/site")),
        ] {
            let _ = widget.reduce(ContainersIntent::RunEdited(edit));
        }

        assert_eq!(widget.subscription().units(), 2);

        let _ = widget.reduce(ContainersIntent::Leave);
        assert_eq!(widget.subscription().units(), 0);
    }
}
