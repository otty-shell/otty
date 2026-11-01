//! View models of the actions: the inspector buttons, the row menu, the
//! confirmation dialog and the Prune button.

use iced::Point;
use otty_docker::Container;

use super::actions::{EngineAction, RowAction, container_actions};
use super::state::DockerState;
use super::types::ResourceKind;
use crate::i18n::{self, Key};

/// One action button of the inspector, or one item of the row menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionButton {
    pub(crate) action: RowAction,
    pub(crate) label: &'static str,
    /// Drawn in the danger color (Remove).
    pub(crate) is_danger: bool,
    /// `false` while an action runs on the row.
    pub(crate) is_enabled: bool,
}

/// The action buttons of the selected row and the hint under them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionsViewModel {
    pub(crate) buttons: Vec<ActionButton>,
    /// Why a button is disabled.
    pub(crate) hint: Option<String>,
}

/// The open row menu: its items at the pointer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RowMenuViewModel {
    /// Where the right press was, in window coordinates.
    pub(crate) position: Point,
    pub(crate) items: Vec<ActionButton>,
}

/// The confirmation dialog of a destructive action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfirmViewModel {
    pub(crate) title: String,
    /// What goes and what stays.
    pub(crate) text: String,
    /// The switch label and whether it is on; `None` without a switch.
    pub(crate) switch: Option<(&'static str, bool)>,
    /// The label of the action button.
    pub(crate) confirm_label: &'static str,
}

/// The Prune button above the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PruneViewModel {
    pub(crate) label: &'static str,
    /// `false` while the prune runs.
    pub(crate) is_enabled: bool,
}

/// The action buttons of the selected row of the open list; `None` when
/// nothing is selected.
pub(crate) fn action_buttons(state: &DockerState) -> Option<ActionsViewModel> {
    let place = state.place()?;
    let engine = state.open_engine()?;

    match place.resource() {
        ResourceKind::Containers => {
            let container = engine.containers().selected(Container::id)?;
            let is_enabled = !state.is_busy(
                place.engine(),
                ResourceKind::Containers,
                Some(container.id()),
            );

            Some(ActionsViewModel {
                buttons: buttons(
                    container_actions(container.state()),
                    is_enabled,
                ),
                hint: None,
            })
        },
        _ => None,
    }
}

/// The open row menu with the actions of the selected row.
pub(crate) fn row_menu(state: &DockerState) -> Option<RowMenuViewModel> {
    let position = state.row_menu()?;
    let actions = action_buttons(state)?;

    Some(RowMenuViewModel {
        position,
        items: actions.buttons,
    })
}

/// The open confirmation dialog.
pub(crate) fn confirm_dialog(state: &DockerState) -> Option<ConfirmViewModel> {
    let confirm = state.confirm()?;
    let request = confirm.request();

    let (title, text, switch_key, confirm_key) = match request.action() {
        EngineAction::RemoveContainer { .. } => (
            i18n::docker_named(
                Key::TplDockerRemoveContainerTitle,
                request.name(),
            ),
            i18n::t(Key::DockerRemoveContainerText).to_string(),
            Some(Key::DockerStopItFirst),
            Key::DockerActionRemove,
        ),
        EngineAction::PruneContainers => (
            i18n::t(Key::DockerPruneContainersTitle).to_string(),
            i18n::with_count(
                Key::TplDockerPruneContainersText,
                confirm.count() as u64,
            ),
            None,
            Key::DockerPrune,
        ),
        _ => return None,
    };
    let switch = switch_key
        .zip(confirm.switch())
        .map(|(key, is_on)| (i18n::t(key), is_on));

    Some(ConfirmViewModel {
        title,
        text,
        switch,
        confirm_label: i18n::t(confirm_key),
    })
}

/// The Prune button of the open list; `None` for a list without one.
pub(crate) fn prune_button(state: &DockerState) -> Option<PruneViewModel> {
    let place = state.place()?;

    let label = match place.resource() {
        ResourceKind::Containers => Key::DockerPruneStopped,
        _ => return None,
    };

    Some(PruneViewModel {
        label: i18n::t(label),
        is_enabled: !state.is_busy(place.engine(), place.resource(), None),
    })
}

/// The buttons of `actions`, all enabled or all disabled.
fn buttons(actions: &[RowAction], is_enabled: bool) -> Vec<ActionButton> {
    actions
        .iter()
        .map(|&action| ActionButton {
            action,
            label: i18n::t(action.label_key()),
            is_danger: action.is_danger(),
            is_enabled,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use otty_docker::{EngineInfo, EngineKind, fixtures};

    use super::*;
    use crate::widgets::docker::actions::RowAction;
    use crate::widgets::docker::event::DockerIntent;
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::state::DockerState;
    use crate::widgets::docker::types::{LoadedLists, Place, ResourceKind};

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn loaded_state() -> DockerState {
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        );
        state
    }

    fn select(state: &mut DockerState, name: &str) {
        let index = fixtures::resources(EngineKind::Docker)
            .containers()
            .iter()
            .position(|container| container.name() == name)
            .expect("fixture container");

        let _ = reduce(state, DockerIntent::SelectRow(index));
    }

    fn button(action: RowAction, is_enabled: bool) -> ActionButton {
        ActionButton {
            action,
            label: i18n::t(action.label_key()),
            is_danger: action == RowAction::Remove,
            is_enabled,
        }
    }

    #[test]
    fn the_inspector_offers_the_actions_of_the_selected_containers_state() {
        let mut state = loaded_state();
        select(&mut state, "otty-fx-running");
        let running = action_buttons(&state);
        select(&mut state, "otty-fx-paused");
        let paused = action_buttons(&state);
        select(&mut state, "otty-fx-exit0");
        let exited = action_buttons(&state);

        assert_eq!(
            running.map(|vm| vm.buttons),
            Some(vec![
                button(RowAction::Restart, true),
                button(RowAction::Stop, true),
                button(RowAction::Pause, true),
                button(RowAction::Remove, true),
            ])
        );
        assert_eq!(
            paused.map(|vm| vm.buttons),
            Some(vec![
                button(RowAction::Unpause, true),
                button(RowAction::Stop, true),
                button(RowAction::Remove, true),
            ])
        );
        assert_eq!(
            exited.as_ref().map(|vm| vm.buttons.clone()),
            Some(vec![
                button(RowAction::Start, true),
                button(RowAction::Remove, true),
            ])
        );
        assert_eq!(exited.and_then(|vm| vm.hint), None);
    }

    #[test]
    fn the_buttons_of_a_busy_container_are_disabled() {
        let mut state = loaded_state();
        select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Stop));

        let vm = action_buttons(&state).expect("a selected container");

        assert!(vm.buttons.iter().all(|button| !button.is_enabled));
    }

    #[test]
    fn nothing_selected_has_no_buttons() {
        let state = DockerState::default();

        assert_eq!(action_buttons(&state), None);
        assert_eq!(row_menu(&state), None);
        assert_eq!(confirm_dialog(&state), None);
    }

    #[test]
    fn the_row_menu_lists_the_same_actions_at_the_pointer() {
        let mut state = loaded_state();
        let position = Point::new(40.0, 60.0);
        let _ = reduce(
            &mut state,
            DockerIntent::OpenRowMenu { index: 3, position },
        );

        let menu = row_menu(&state).expect("an open menu");

        assert_eq!(menu.position, position);
        assert_eq!(
            menu.items,
            vec![
                button(RowAction::Start, true),
                button(RowAction::Remove, true)
            ]
        );
    }

    #[test]
    fn the_remove_dialog_names_the_container_and_offers_stop_it_first() {
        let mut state = loaded_state();
        select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));

        let dialog = confirm_dialog(&state).expect("an open dialog");

        assert_eq!(
            dialog,
            ConfirmViewModel {
                title: i18n::docker_named(
                    Key::TplDockerRemoveContainerTitle,
                    "otty-fx-running"
                ),
                text: i18n::t(Key::DockerRemoveContainerText).to_string(),
                switch: Some((i18n::t(Key::DockerStopItFirst), true)),
                confirm_label: i18n::t(Key::DockerActionRemove),
            }
        );
    }

    #[test]
    fn the_switch_shows_its_new_state() {
        let mut state = loaded_state();
        select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));

        let _ = reduce(&mut state, DockerIntent::ConfirmSwitchToggled(false));

        let dialog = confirm_dialog(&state).expect("an open dialog");
        assert_eq!(
            dialog.switch,
            Some((i18n::t(Key::DockerStopItFirst), false))
        );
    }

    #[test]
    fn a_stopped_container_dialog_has_no_switch() {
        let mut state = loaded_state();
        select(&mut state, "otty-fx-exit0");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));

        let dialog = confirm_dialog(&state).expect("an open dialog");

        assert_eq!(dialog.switch, None);
    }

    #[test]
    fn the_prune_dialog_counts_the_stopped_containers() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, DockerIntent::Prune);

        let dialog = confirm_dialog(&state).expect("an open dialog");

        assert_eq!(dialog.title, i18n::t(Key::DockerPruneContainersTitle));
        assert_eq!(
            dialog.text,
            i18n::with_count(Key::TplDockerPruneContainersText, 3)
        );
        assert_eq!(dialog.switch, None);
        assert_eq!(dialog.confirm_label, i18n::t(Key::DockerPrune));
    }

    #[test]
    fn the_containers_table_has_a_prune_button_until_the_prune_runs() {
        let mut state = loaded_state();
        let idle = prune_button(&state);
        let _ = reduce(&mut state, DockerIntent::Prune);
        let _ = reduce(&mut state, DockerIntent::ConfirmAccepted);

        let busy = prune_button(&state);

        assert_eq!(
            idle,
            Some(PruneViewModel {
                label: i18n::t(Key::DockerPruneStopped),
                is_enabled: true,
            })
        );
        assert_eq!(busy.map(|vm| vm.is_enabled), Some(false));
    }

    #[test]
    fn the_images_table_does_not_prune_containers() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, DockerIntent::Prune);
        let _ = reduce(&mut state, DockerIntent::ConfirmAccepted);

        let _ = reduce(
            &mut state,
            DockerIntent::Open(Place::new(
                docker().address().clone(),
                ResourceKind::Images,
            )),
        );

        assert_ne!(
            prune_button(&state).map(|vm| vm.label),
            Some(i18n::t(Key::DockerPruneStopped))
        );
    }
}
