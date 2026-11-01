//! What the user can do with a row or a list (spec Decision 11): the
//! actions each container state offers, the engine call each one makes and
//! the confirmation a destructive one asks for.

use otty_docker::{ContainerState, EngineAddress, EngineInfo};

use super::types::ResourceKind;
use crate::i18n::{self, Key};

/// What a button in the inspector or an item of the row menu does with the
/// selected row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowAction {
    Start,
    Stop,
    Restart,
    Pause,
    Unpause,
    /// Asks first.
    Remove,
}

impl RowAction {
    /// Translation key of the button and the menu item.
    pub(crate) fn label_key(self) -> Key {
        match self {
            Self::Start => Key::DockerActionStart,
            Self::Stop => Key::DockerActionStop,
            Self::Restart => Key::DockerActionRestart,
            Self::Pause => Key::DockerActionPause,
            Self::Unpause => Key::DockerActionUnpause,
            Self::Remove => Key::DockerActionRemove,
        }
    }

    /// Whether the action deletes something; it is drawn in the danger
    /// color.
    pub(crate) fn is_danger(self) -> bool {
        self == Self::Remove
    }
}

/// One engine call the app makes for an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EngineAction {
    /// Start the container with this id.
    StartContainer(String),
    StopContainer(String),
    RestartContainer(String),
    PauseContainer(String),
    UnpauseContainer(String),
    /// Remove the container; `force` kills a running one first.
    RemoveContainer {
        id: String,
        force: bool,
    },
    /// Remove every stopped container.
    PruneContainers,
}

impl EngineAction {
    /// The list the action changes.
    pub(crate) fn resource(&self) -> ResourceKind {
        match self {
            Self::StartContainer(_)
            | Self::StopContainer(_)
            | Self::RestartContainer(_)
            | Self::PauseContainer(_)
            | Self::UnpauseContainer(_)
            | Self::RemoveContainer { .. }
            | Self::PruneContainers => ResourceKind::Containers,
        }
    }

    /// The id of the row the action works on; `None` for a prune, which
    /// works on the whole list.
    pub(crate) fn target(&self) -> Option<&str> {
        match self {
            Self::StartContainer(id)
            | Self::StopContainer(id)
            | Self::RestartContainer(id)
            | Self::PauseContainer(id)
            | Self::UnpauseContainer(id)
            | Self::RemoveContainer { id, .. } => Some(id),
            Self::PruneContainers => None,
        }
    }

    /// Translation key of the notice shown when the engine refuses it.
    pub(crate) fn failure_key(&self) -> Key {
        match self {
            Self::StartContainer(_) => Key::TplDockerStartFailed,
            Self::StopContainer(_) => Key::TplDockerStopFailed,
            Self::RestartContainer(_) => Key::TplDockerRestartFailed,
            Self::PauseContainer(_) => Key::TplDockerPauseFailed,
            Self::UnpauseContainer(_) => Key::TplDockerUnpauseFailed,
            Self::RemoveContainer { .. } => Key::TplDockerRemoveContainerFailed,
            Self::PruneContainers => Key::TplDockerPruneContainersFailed,
        }
    }

    /// The same action with the confirmation switch applied: it forces a
    /// removal. Other actions have no switch.
    fn with_switch(self, is_on: bool) -> Self {
        match self {
            Self::RemoveContainer { id, .. } => {
                Self::RemoveContainer { id, force: is_on }
            },
            other => other,
        }
    }
}

/// An action on one engine, with the name of its row for the failure
/// notice (empty for a prune).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionRequest {
    engine: EngineInfo,
    action: EngineAction,
    name: String,
}

impl ActionRequest {
    /// The engine the action runs on.
    pub(crate) fn engine(&self) -> &EngineInfo {
        &self.engine
    }

    /// The engine call.
    pub(crate) fn action(&self) -> &EngineAction {
        &self.action
    }

    /// The name of the row, as the table shows it.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// An `action` on `engine` for the row called `name`.
    pub(crate) fn new(
        engine: EngineInfo,
        action: EngineAction,
        name: String,
    ) -> Self {
        Self {
            engine,
            action,
            name,
        }
    }

    /// Whether the request works on the row `target` (or, with `None`, on
    /// the whole list) of the `resource` list of the engine at `address`.
    pub(crate) fn acts_on(
        &self,
        address: &EngineAddress,
        resource: ResourceKind,
        target: Option<&str>,
    ) -> bool {
        self.engine.address() == address
            && self.action.resource() == resource
            && self.action.target() == target
    }
}

/// A destructive action waiting for the user to confirm it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Confirm {
    request: ActionRequest,
    /// The dialog switch ("Stop it first"); `None` when the dialog has none.
    switch: Option<bool>,
    /// How many rows a prune deletes.
    count: usize,
}

impl Confirm {
    /// The action that runs on confirm, before the switch is applied.
    pub(crate) fn request(&self) -> &ActionRequest {
        &self.request
    }

    /// Whether the switch is on; `None` when the dialog has no switch.
    pub(crate) fn switch(&self) -> Option<bool> {
        self.switch
    }

    /// How many rows a prune deletes.
    pub(crate) fn count(&self) -> usize {
        self.count
    }

    /// Ask to confirm `request`, with a switch in its first state, or
    /// none.
    pub(crate) fn new(
        request: ActionRequest,
        switch: Option<bool>,
        count: usize,
    ) -> Self {
        Self {
            request,
            switch,
            count,
        }
    }

    /// Turn the switch on or off; a dialog without a switch keeps none.
    pub(crate) fn set_switch(&mut self, is_on: bool) {
        if self.switch.is_some() {
            self.switch = Some(is_on);
        }
    }

    /// The request to run, with the switch applied.
    pub(crate) fn into_request(self) -> ActionRequest {
        let Some(is_on) = self.switch else {
            return self.request;
        };

        ActionRequest {
            action: self.request.action.with_switch(is_on),
            ..self.request
        }
    }
}

/// The actions a container in `state` offers, in button order (spec
/// Decision 11). Shell joins the running ones with ticket 15.
pub(crate) fn container_actions(state: ContainerState) -> &'static [RowAction] {
    use RowAction::*;

    match state {
        ContainerState::Running => &[Restart, Stop, Pause, Remove],
        ContainerState::Paused => &[Unpause, Stop, Remove],
        ContainerState::Restarting => &[Stop, Remove],
        ContainerState::Exited | ContainerState::Created => &[Start, Remove],
        ContainerState::Removing => &[],
        _ => &[Remove],
    }
}

/// Whether the engine refuses to remove a container in `state` unless it
/// is stopped first: the remove dialog then offers "Stop it first".
pub(crate) fn needs_stop(state: ContainerState) -> bool {
    matches!(
        state,
        ContainerState::Running
            | ContainerState::Paused
            | ContainerState::Restarting
    )
}

/// The notice text of a failed `request`: the action, the row and the
/// engine's `message`.
pub(crate) fn failure_text(request: &ActionRequest, message: &str) -> String {
    i18n::docker_action_failed(
        request.action.failure_key(),
        &request.name,
        message,
    )
}

#[cfg(test)]
mod tests {
    use otty_docker::{ContainerState, EngineKind, fixtures};

    use super::*;
    use crate::i18n;

    fn request(action: EngineAction) -> ActionRequest {
        ActionRequest::new(
            fixtures::engine(EngineKind::Docker),
            action,
            String::from("otty-fx-running"),
        )
    }

    #[test]
    fn each_container_state_offers_its_actions() {
        use RowAction::*;

        let offered = |state| container_actions(state).to_vec();

        assert_eq!(
            offered(ContainerState::Running),
            vec![Restart, Stop, Pause, Remove]
        );
        assert_eq!(
            offered(ContainerState::Paused),
            vec![Unpause, Stop, Remove]
        );
        assert_eq!(offered(ContainerState::Restarting), vec![Stop, Remove]);
        assert_eq!(offered(ContainerState::Exited), vec![Start, Remove]);
        assert_eq!(offered(ContainerState::Created), vec![Start, Remove]);
        assert_eq!(offered(ContainerState::Dead), vec![Remove]);
        assert_eq!(offered(ContainerState::Unknown), vec![Remove]);
        assert!(offered(ContainerState::Removing).is_empty());
    }

    #[test]
    fn only_remove_is_destructive() {
        use RowAction::*;

        for action in [Start, Stop, Restart, Pause, Unpause] {
            assert!(!action.is_danger(), "{action:?}");
        }
        assert!(Remove.is_danger());
    }

    #[test]
    fn a_running_paused_or_restarting_container_is_stopped_before_removal() {
        assert!(needs_stop(ContainerState::Running));
        assert!(needs_stop(ContainerState::Paused));
        assert!(needs_stop(ContainerState::Restarting));
        assert!(!needs_stop(ContainerState::Exited));
        assert!(!needs_stop(ContainerState::Created));
        assert!(!needs_stop(ContainerState::Dead));
    }

    #[test]
    fn each_action_names_its_list_and_its_row() {
        let id = String::from("c1");
        let actions = [
            EngineAction::StartContainer(id.clone()),
            EngineAction::StopContainer(id.clone()),
            EngineAction::RestartContainer(id.clone()),
            EngineAction::PauseContainer(id.clone()),
            EngineAction::UnpauseContainer(id.clone()),
            EngineAction::RemoveContainer {
                id: id.clone(),
                force: true,
            },
        ];

        for action in actions {
            assert_eq!(action.resource(), ResourceKind::Containers);
            assert_eq!(action.target(), Some("c1"), "{action:?}");
        }
        assert_eq!(
            EngineAction::PruneContainers.resource(),
            ResourceKind::Containers
        );
        assert_eq!(EngineAction::PruneContainers.target(), None);
    }

    #[test]
    fn a_request_runs_on_its_engine_list_and_row() {
        let stop = request(EngineAction::StopContainer(String::from("c1")));
        let docker = fixtures::engine(EngineKind::Docker);
        let podman = fixtures::engine(EngineKind::Podman);

        assert!(stop.acts_on(
            docker.address(),
            ResourceKind::Containers,
            Some("c1")
        ));
        assert!(!stop.acts_on(
            docker.address(),
            ResourceKind::Containers,
            Some("c2")
        ));
        assert!(!stop.acts_on(
            podman.address(),
            ResourceKind::Containers,
            Some("c1")
        ));
        assert!(!stop.acts_on(
            docker.address(),
            ResourceKind::Containers,
            None
        ));
    }

    #[test]
    fn the_switch_of_a_remove_confirmation_forces_the_removal() {
        let remove = request(EngineAction::RemoveContainer {
            id: String::from("c1"),
            force: false,
        });
        let mut confirm = Confirm::new(remove, Some(true), 0);

        let forced = confirm.clone().into_request();
        confirm.set_switch(false);
        let gentle = confirm.into_request();

        assert_eq!(
            forced.action(),
            &EngineAction::RemoveContainer {
                id: String::from("c1"),
                force: true,
            }
        );
        assert_eq!(
            gentle.action(),
            &EngineAction::RemoveContainer {
                id: String::from("c1"),
                force: false,
            }
        );
    }

    #[test]
    fn a_confirmation_without_a_switch_keeps_it_hidden() {
        let mut confirm =
            Confirm::new(request(EngineAction::PruneContainers), None, 3);

        confirm.set_switch(true);

        assert_eq!(confirm.switch(), None);
        assert_eq!(confirm.count(), 3);
        assert_eq!(
            confirm.into_request().action(),
            &EngineAction::PruneContainers
        );
    }

    #[test]
    fn a_failure_names_the_action_the_row_and_the_engine_message() {
        let stop = request(EngineAction::StopContainer(String::from("c1")));
        let prune = request(EngineAction::PruneContainers);

        let stop_text = failure_text(&stop, "it is restarting");
        let prune_text = failure_text(&prune, "engine is busy");

        assert_eq!(
            stop_text,
            i18n::docker_action_failed(
                Key::TplDockerStopFailed,
                "otty-fx-running",
                "it is restarting"
            )
        );
        assert!(stop_text.contains("otty-fx-running"), "{stop_text}");
        assert!(stop_text.contains("it is restarting"), "{stop_text}");
        assert!(prune_text.contains("engine is busy"), "{prune_text}");
    }

    #[test]
    fn every_failure_has_its_own_text() {
        let id = || String::from("c1");
        let keys: Vec<Key> = [
            EngineAction::StartContainer(id()),
            EngineAction::StopContainer(id()),
            EngineAction::RestartContainer(id()),
            EngineAction::PauseContainer(id()),
            EngineAction::UnpauseContainer(id()),
            EngineAction::RemoveContainer {
                id: id(),
                force: false,
            },
            EngineAction::PruneContainers,
        ]
        .iter()
        .map(EngineAction::failure_key)
        .collect();

        for (position, key) in keys.iter().enumerate() {
            assert!(!keys[position + 1..].contains(key), "{key:?} repeats");
        }
    }
}
