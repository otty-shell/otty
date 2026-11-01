pub(crate) mod event;
pub(crate) mod format;
pub(crate) mod inspector;
pub(crate) mod inspector_tabs;
mod logs;
pub(crate) mod model;
pub(crate) mod no_engine;
pub(crate) mod overview;
mod reducer;
pub(crate) mod resource_rows;
mod state;
pub(crate) mod status;
pub(crate) mod types;
pub(crate) mod view;

pub(crate) use event::{DockerEffect, DockerEvent, DockerIntent};
use iced::{Subscription, Task};
use inspector_tabs::InspectorTabsViewModel;
use model::{InspectorViewModel, MainViewModel, TreeRowViewModel};
use state::DockerState;
use status::StatusViewModel;

/// Docker widget: the engines found on this computer, their containers,
/// images, volumes and networks, the open list, the selected rows and the
/// name filter (spec Decisions 5, 8 and 10). It asks for engine work only
/// while Docker mode is active.
pub(crate) struct DockerWidget {
    state: DockerState,
}

impl DockerWidget {
    /// Create the widget before any engine search.
    pub(crate) fn new() -> Self {
        Self {
            state: DockerState::default(),
        }
    }

    /// Reduce an intent into state changes and effect events.
    pub(crate) fn reduce(&mut self, intent: DockerIntent) -> Task<DockerEvent> {
        reducer::reduce(&mut self.state, intent)
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

    /// Follow the logs of the selected container while the Logs tab is open
    /// in Docker mode; nothing otherwise (spec Decisions 12 and 14).
    pub(crate) fn subscription(&self) -> Subscription<DockerEvent> {
        let target = self.state.inspector().logs().map(|logs| logs.target());

        logs::subscription(target)
    }
}

#[cfg(test)]
impl DockerWidget {
    /// Return whether Docker mode is the active mode.
    pub(crate) fn is_active(&self) -> bool {
        self.state.is_active()
    }
}

#[cfg(test)]
mod tests {
    use std::hash::Hasher as _;

    use iced::advanced::subscription::{Hasher, into_recipes};
    use otty_docker::{EngineKind, fixtures};

    use super::*;
    use crate::widgets::docker::inspector::InspectorTab;
    use crate::widgets::docker::types::LoadedLists;

    /// Docker mode active with the fixture Docker engine and its lists.
    fn loaded() -> DockerWidget {
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let mut widget = DockerWidget::new();
        let _ = widget.reduce(DockerIntent::Enter);
        let _ = widget.reduce(DockerIntent::DiscoveryFinished(
            fixtures::discovery(&[EngineKind::Docker]),
        ));
        let _ = widget.reduce(DockerIntent::ListsLoaded {
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

    /// The identity of every running subscription.
    fn subscriptions(widget: &DockerWidget) -> Vec<u64> {
        into_recipes(widget.subscription())
            .iter()
            .map(|recipe| {
                let mut hasher = Hasher::default();
                recipe.hash(&mut hasher);
                hasher.finish()
            })
            .collect()
    }

    #[test]
    fn the_logs_subscription_runs_only_on_the_logs_tab_of_a_container() {
        let mut widget = loaded();
        let on_overview = subscriptions(&widget);

        let _ =
            widget.reduce(DockerIntent::SelectInspectorTab(InspectorTab::Logs));

        assert!(on_overview.is_empty());
        assert_eq!(subscriptions(&widget).len(), 1);
    }

    #[test]
    fn the_logs_subscription_stops_when_another_tab_opens() {
        let mut widget = loaded();
        let _ =
            widget.reduce(DockerIntent::SelectInspectorTab(InspectorTab::Logs));

        let _ = widget
            .reduce(DockerIntent::SelectInspectorTab(InspectorTab::Inspect));

        assert!(subscriptions(&widget).is_empty());
    }

    #[test]
    fn the_logs_subscription_is_replaced_when_the_selection_changes() {
        let mut widget = loaded();
        let _ =
            widget.reduce(DockerIntent::SelectInspectorTab(InspectorTab::Logs));
        let first = subscriptions(&widget);

        let _ = widget.reduce(DockerIntent::SelectRow(1));
        let second = subscriptions(&widget);
        let _ = widget.reduce(DockerIntent::SelectRow(1));

        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert_ne!(first, second);
        assert_eq!(subscriptions(&widget), second);
    }

    #[test]
    fn the_logs_subscription_stops_when_docker_mode_is_left() {
        let mut widget = loaded();
        let _ =
            widget.reduce(DockerIntent::SelectInspectorTab(InspectorTab::Logs));

        let _ = widget.reduce(DockerIntent::Leave);

        assert!(subscriptions(&widget).is_empty());
    }
}
