//! View models of the actions: the inspector buttons, the row menu, the
//! confirmation dialog and the Prune button.

use iced::Point;
use otty_docker::{Container, EngineKind, Image, Network, Volume};

use super::actions::{
    Confirm, EngineAction, ImageRemoval, REMOVE_ONLY, RowAction,
    container_actions, image_actions, network_remove_hint, volume_remove_hint,
};
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
/// nothing is selected. Every button is disabled while an action runs on
/// the row; Remove is disabled, with the hint saying why, when the engine
/// would refuse it (a volume or network in use, a built-in network).
pub(crate) fn action_buttons(state: &DockerState) -> Option<ActionsViewModel> {
    row_actions(state, state.selected_id()?)
}

/// The open row menu with the actions of its row (the selected row after a
/// right press, any row after a press on its "⋯" button).
pub(crate) fn row_menu(state: &DockerState) -> Option<RowMenuViewModel> {
    let position = state.row_menu()?;
    let actions = row_actions(state, state.row_menu_target()?)?;

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
        EngineAction::RemoveImage { .. } => {
            let (title_key, text_key) = image_remove_texts(confirm);
            (
                i18n::docker_named(title_key, request.name()),
                i18n::t(text_key).to_string(),
                Some(Key::DockerForceRemoveImage),
                Key::DockerActionRemove,
            )
        },
        EngineAction::RemoveVolume(_) => (
            i18n::docker_named(Key::TplDockerRemoveVolumeTitle, request.name()),
            i18n::t(Key::DockerRemoveVolumeText).to_string(),
            None,
            Key::DockerActionRemove,
        ),
        EngineAction::RemoveNetwork(_) => (
            i18n::docker_named(
                Key::TplDockerRemoveNetworkTitle,
                request.name(),
            ),
            i18n::t(Key::DockerRemoveNetworkText).to_string(),
            None,
            Key::DockerActionRemove,
        ),
        EngineAction::PruneContainers => prune_texts(
            Key::DockerPruneContainersTitle,
            Key::TplDockerPruneContainersText,
            confirm.count(),
        ),
        EngineAction::PruneImages => prune_texts(
            Key::DockerPruneImagesTitle,
            Key::TplDockerPruneImagesText,
            confirm.count(),
        ),
        EngineAction::PruneVolumes { .. } => {
            let text_key = if confirm.switch() == Some(true) {
                Key::TplDockerPruneVolumesText
            } else {
                Key::TplDockerPruneAnonymousVolumesText
            };
            let (title, text, _, confirm_key) = prune_texts(
                Key::DockerPruneVolumesTitle,
                text_key,
                confirm.count(),
            );
            (
                title,
                text,
                Some(Key::DockerAlsoRemoveNamedVolumes),
                confirm_key,
            )
        },
        EngineAction::PruneNetworks => prune_texts(
            Key::DockerPruneNetworksTitle,
            Key::TplDockerPruneNetworksText,
            confirm.count(),
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
        ResourceKind::Images => Key::DockerPruneDangling,
        ResourceKind::Volumes | ResourceKind::Networks => {
            Key::DockerPruneUnused
        },
    };

    Some(PruneViewModel {
        label: i18n::t(label),
        is_enabled: !state.is_busy(place.engine(), place.resource(), None),
    })
}

/// Whether the create button above the table works: Run container, Pull
/// image, Create volume and Create network need an open engine.
pub(crate) fn can_create(state: &DockerState) -> bool {
    state.open_engine().is_some() && state.place().is_some()
}

/// The title and text of an image remove dialog: what goes away with the
/// switch as it is now.
fn image_remove_texts(confirm: &Confirm) -> (Key, Key) {
    let removal = confirm.image_removal().unwrap_or(ImageRemoval::WholeImage);

    let title = match removal {
        ImageRemoval::OnlyThisTag | ImageRemoval::ForceUntags => {
            Key::TplDockerRemoveTagTitle
        },
        _ => Key::TplDockerRemoveImageTitle,
    };
    let text = match removal {
        ImageRemoval::OnlyThisTag => Key::DockerRemoveTagText,
        ImageRemoval::WholeImage => Key::DockerRemoveImageText,
        ImageRemoval::Refused => Key::DockerRemoveImageRefusedText,
        _ if confirm.switch() != Some(true) => {
            Key::DockerRemoveImageNeedsForceText
        },
        ImageRemoval::ForceUntags => Key::DockerForceUntagsImageText,
        ImageRemoval::ForceKeepsContainers => Key::DockerForceDeletesImageText,
        _ => Key::DockerForceDeletesContainersText,
    };

    (title, text)
}

/// The action buttons of the row with `id` in the open list and the hint
/// under them, by the rules of [`action_buttons`]; `None` when the row is
/// not in the loaded list.
fn row_actions(state: &DockerState, id: &str) -> Option<ActionsViewModel> {
    let place = state.place()?;
    let engine = state.open_engine()?;

    // What a running action on this row names: the id, but an image row goes
    // by what removes only that row.
    let (offered, remove_hint, action_target) = match place.resource() {
        ResourceKind::Containers => {
            let container = engine.containers().find(id, Container::id)?;
            (container_actions(container.state()), None, id)
        },
        ResourceKind::Images => {
            let image = engine.images().find(id, Image::row_key)?;
            (image_actions(image), None, image.remove_target())
        },
        ResourceKind::Volumes => {
            let volume = engine.volumes().find(id, Volume::name)?;
            (REMOVE_ONLY, volume_remove_hint(volume), id)
        },
        ResourceKind::Networks => {
            let network = engine.networks().find(id, Network::id)?;
            (REMOVE_ONLY, network_remove_hint(network), id)
        },
    };
    let is_busy =
        state.is_busy(place.engine(), place.resource(), Some(action_target));
    let shell_hint = shell_hint(state, engine.info().kind(), offered);

    let buttons = offered
        .iter()
        .map(|&action| {
            let is_blocked = (action == RowAction::Remove
                && remove_hint.is_some())
                || (action == RowAction::Shell && shell_hint.is_some());
            ActionButton {
                action,
                label: i18n::t(action.label_key()),
                is_danger: action.is_danger(),
                is_enabled: !is_busy && !is_blocked,
            }
        })
        .collect();

    Some(ActionsViewModel {
        buttons,
        hint: shell_hint.or(remove_hint),
    })
}

/// Why Shell is disabled: the search did not find the engine's command line
/// tool. `None` when the row offers no Shell or the tool was found.
fn shell_hint(
    state: &DockerState,
    kind: EngineKind,
    offered: &[RowAction],
) -> Option<String> {
    if !offered.contains(&RowAction::Shell) || state.cli_path(kind).is_some() {
        return None;
    }

    let key = match kind {
        EngineKind::Docker => Key::DockerShellNeedsDocker,
        EngineKind::Podman => Key::DockerShellNeedsPodman,
    };

    Some(i18n::t(key).to_string())
}

/// The title, text, switch and action of a prune dialog that deletes
/// `count` rows.
fn prune_texts(
    title: Key,
    text: Key,
    count: usize,
) -> (String, String, Option<Key>, Key) {
    (
        i18n::t(title).to_string(),
        i18n::with_count(text, count as u64),
        None,
        Key::DockerPrune,
    )
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use otty_docker::{Container, EngineInfo, EngineKind, fixtures};

    use super::*;
    use crate::widgets::docker::actions::RowAction;
    use crate::widgets::docker::event::DockerIntent;
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::state::DockerState;
    use crate::widgets::docker::types::{
        LoadedLists, Place, ResourceKind, image_name,
    };

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn loaded_state() -> DockerState {
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery_with_cli(
                &[EngineKind::Docker],
                Some("/usr/local/bin/docker"),
                None,
            )),
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

        let position = state.shown_position(index);
        let _ = reduce(state, DockerIntent::SelectRow(position));
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
                button(RowAction::Shell, true),
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
    fn the_row_button_menu_lists_the_actions_of_its_row_not_the_selected_one() {
        let mut state = loaded_state();
        select(&mut state, "otty-fx-running");
        let exited = fixtures::resources(EngineKind::Docker)
            .containers()
            .iter()
            .position(|container| container.name() == "otty-fx-exit0")
            .expect("fixture container");
        let position = Point::new(900.0, 120.0);
        let index = state.shown_position(exited);
        let _ = reduce(
            &mut state,
            DockerIntent::RowMenuPressed { index, position },
        );

        let menu = row_menu(&state).expect("an open menu");
        let inspector = action_buttons(&state).expect("a selected container");

        assert_eq!(menu.position, position);
        assert_eq!(
            menu.items,
            vec![
                button(RowAction::Start, true),
                button(RowAction::Remove, true)
            ]
        );
        assert_eq!(inspector.buttons[0], button(RowAction::Shell, true));
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

    /// Replace the Docker lists with the idle fixture lists (no container).
    fn load_idle(state: &mut DockerState) {
        let idle = fixtures::idle_resources(EngineKind::Docker);
        let _ = reduce(
            state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: LoadedLists {
                    containers: Ok(Vec::new()),
                    images: Ok(idle.images().to_vec()),
                    volumes: Ok(idle.volumes().to_vec()),
                    networks: Ok(idle.networks().to_vec()),
                },
            },
        );
    }

    fn open(state: &mut DockerState, resource: ResourceKind) {
        let _ = reduce(
            state,
            DockerIntent::Open(Place::new(
                docker().address().clone(),
                resource,
            )),
        );
    }

    /// Open `resource` and select the row called `name`.
    fn select_in(state: &mut DockerState, resource: ResourceKind, name: &str) {
        open(state, resource);
        let engine = state.open_engine().expect("docker engine");
        let index = match resource {
            ResourceKind::Images => engine
                .images()
                .items()
                .iter()
                .position(|image| image_name(image) == name),
            ResourceKind::Volumes => engine
                .volumes()
                .items()
                .iter()
                .position(|volume| volume.name() == name),
            _ => engine
                .networks()
                .items()
                .iter()
                .position(|network| network.name() == name),
        }
        .expect("fixture row");

        let position = state.shown_position(index);
        let _ = reduce(state, DockerIntent::SelectRow(position));
    }

    fn buttons_of(state: &DockerState) -> (Vec<ActionButton>, Option<String>) {
        let vm = action_buttons(state).expect("a selected row");

        (vm.buttons, vm.hint)
    }

    #[test]
    fn a_tagged_image_offers_run_pull_again_and_remove() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Images, "busybox:1.37");

        assert_eq!(
            buttons_of(&state),
            (
                vec![
                    button(RowAction::Run, true),
                    button(RowAction::PullAgain, true),
                    button(RowAction::Remove, true)
                ],
                None
            )
        );
    }

    #[test]
    fn a_used_volume_has_a_disabled_remove_and_says_why() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Volumes, "otty-fx-data");

        assert_eq!(
            buttons_of(&state),
            (
                vec![button(RowAction::Remove, false)],
                Some(i18n::docker_volume_in_use("otty-fx-running"))
            )
        );
    }

    #[test]
    fn a_built_in_network_has_a_disabled_remove_and_says_why() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Networks, "bridge");

        assert_eq!(
            buttons_of(&state),
            (
                vec![button(RowAction::Remove, false)],
                Some(i18n::t(Key::DockerBuiltInNetworkHint).to_string())
            )
        );
    }

    #[test]
    fn an_unused_volume_and_network_can_be_removed() {
        let mut state = loaded_state();
        load_idle(&mut state);

        select_in(&mut state, ResourceKind::Volumes, "otty-fx-data");
        let volume = buttons_of(&state);
        select_in(&mut state, ResourceKind::Networks, "otty-fx-net");
        let network = buttons_of(&state);

        assert_eq!(volume, (vec![button(RowAction::Remove, true)], None));
        assert_eq!(network, (vec![button(RowAction::Remove, true)], None));
    }

    #[test]
    fn each_remove_dialog_names_its_row() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Images, "busybox:1.37");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));
        let image = confirm_dialog(&state).expect("image dialog");
        let _ = reduce(&mut state, DockerIntent::ConfirmCancelled);
        load_idle(&mut state);
        select_in(&mut state, ResourceKind::Volumes, "otty-fx-data");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));
        let volume = confirm_dialog(&state).expect("volume dialog");
        let _ = reduce(&mut state, DockerIntent::ConfirmCancelled);
        select_in(&mut state, ResourceKind::Networks, "otty-fx-net");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));
        let network = confirm_dialog(&state).expect("network dialog");

        assert_eq!(
            image,
            ConfirmViewModel {
                title: i18n::docker_named(
                    Key::TplDockerRemoveTagTitle,
                    "busybox:1.37"
                ),
                text: i18n::t(Key::DockerForceUntagsImageText).to_string(),
                switch: Some((i18n::t(Key::DockerForceRemoveImage), true)),
                confirm_label: i18n::t(Key::DockerActionRemove),
            }
        );
        assert_eq!(
            volume.title,
            i18n::docker_named(Key::TplDockerRemoveVolumeTitle, "otty-fx-data")
        );
        assert_eq!(volume.text, i18n::t(Key::DockerRemoveVolumeText));
        assert_eq!(volume.switch, None);
        assert_eq!(
            network.title,
            i18n::docker_named(Key::TplDockerRemoveNetworkTitle, "otty-fx-net")
        );
        assert_eq!(network.text, i18n::t(Key::DockerRemoveNetworkText));
    }

    /// `kind` with its images open, listing an image with `names` and a
    /// container in each of `user_states`, the row at `index` selected and its
    /// remove dialog open.
    fn image_remove_dialog(
        kind: EngineKind,
        names: &[&str],
        user_states: &[&str],
        index: usize,
    ) -> DockerState {
        let info = fixtures::engine(kind);
        let resources = fixtures::resources(kind);
        let users: Vec<Container> = user_states
            .iter()
            .enumerate()
            .map(|(number, state)| {
                fixtures::container_of_image(
                    &format!("user-{number}"),
                    state,
                    "app:1",
                    "sha256:app",
                )
            })
            .collect();
        let images = fixtures::image_rows("sha256:app", names, &[], &users);

        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[kind])),
        );
        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: info.address().clone(),
                lists: LoadedLists {
                    containers: Ok(users),
                    images: Ok(images),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        );
        let _ = reduce(
            &mut state,
            DockerIntent::Open(Place::new(
                info.address().clone(),
                ResourceKind::Images,
            )),
        );
        let _ = reduce(&mut state, DockerIntent::SelectRow(index));
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));

        state
    }

    fn dialog_text(state: &DockerState) -> String {
        confirm_dialog(state).expect("the remove dialog").text
    }

    #[test]
    fn a_tag_with_siblings_says_only_it_goes_on_any_engine() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            let state =
                image_remove_dialog(kind, &["app:1", "app:2"], &["running"], 1);

            let dialog = confirm_dialog(&state).expect("the remove dialog");

            assert_eq!(
                dialog.title,
                i18n::docker_named(Key::TplDockerRemoveTagTitle, "app:2")
            );
            assert_eq!(dialog.text, i18n::t(Key::DockerRemoveTagText));
            assert_eq!(dialog.switch, None);
        }
    }

    #[test]
    fn the_last_tag_of_an_unused_image_says_the_image_goes() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            let state = image_remove_dialog(kind, &["app:1"], &[], 0);

            let dialog = confirm_dialog(&state).expect("the remove dialog");

            assert_eq!(
                dialog.title,
                i18n::docker_named(Key::TplDockerRemoveImageTitle, "app:1")
            );
            assert_eq!(dialog.text, i18n::t(Key::DockerRemoveImageText));
            assert_eq!(dialog.switch, None);
        }
    }

    #[test]
    fn force_on_docker_says_the_image_stays_without_a_tag() {
        let state =
            image_remove_dialog(EngineKind::Docker, &["app:1"], &["exited"], 0);

        let dialog = confirm_dialog(&state).expect("the remove dialog");

        assert_eq!(dialog.text, i18n::t(Key::DockerForceUntagsImageText));
        assert_eq!(
            dialog.switch,
            Some((i18n::t(Key::DockerForceRemoveImage), true))
        );
    }

    #[test]
    fn force_on_docker_without_a_tag_says_the_containers_stay() {
        let state =
            image_remove_dialog(EngineKind::Docker, &[], &["exited"], 0);

        assert_eq!(
            dialog_text(&state),
            i18n::t(Key::DockerForceDeletesImageText)
        );
    }

    #[test]
    fn on_podman_force_starts_off_and_the_dialog_says_the_engine_refuses() {
        for names in [&["app:1"][..], &[][..]] {
            let state =
                image_remove_dialog(EngineKind::Podman, names, &["exited"], 0);

            let dialog = confirm_dialog(&state).expect("the remove dialog");

            assert_eq!(
                dialog.text,
                i18n::t(Key::DockerRemoveImageNeedsForceText)
            );
            assert_eq!(
                dialog.switch,
                Some((i18n::t(Key::DockerForceRemoveImage), false))
            );
        }
    }

    #[test]
    fn force_on_podman_says_the_containers_go_too() {
        let mut state =
            image_remove_dialog(EngineKind::Podman, &["app:1"], &["exited"], 0);

        let _ = reduce(&mut state, DockerIntent::ConfirmSwitchToggled(true));

        assert_eq!(
            dialog_text(&state),
            i18n::t(Key::DockerForceDeletesContainersText)
        );
    }

    #[test]
    fn with_force_off_the_dialog_says_the_engine_refuses() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            let mut state =
                image_remove_dialog(kind, &["app:1"], &["exited"], 0);

            let _ =
                reduce(&mut state, DockerIntent::ConfirmSwitchToggled(false));

            assert_eq!(
                dialog_text(&state),
                i18n::t(Key::DockerRemoveImageNeedsForceText)
            );
        }
    }

    #[test]
    fn when_force_cannot_help_the_dialog_says_stop_it_first() {
        let cases = [
            (EngineKind::Docker, &[][..]),
            (EngineKind::Podman, &["app:1"][..]),
            (EngineKind::Podman, &[][..]),
        ];

        for (kind, names) in cases {
            let state = image_remove_dialog(kind, names, &["running"], 0);

            let dialog = confirm_dialog(&state).expect("the remove dialog");

            assert_eq!(dialog.text, i18n::t(Key::DockerRemoveImageRefusedText));
            assert_eq!(dialog.switch, None);
        }
    }

    #[test]
    fn the_buttons_of_a_tag_are_disabled_only_while_that_tag_goes() {
        let mut state = image_remove_dialog(
            EngineKind::Docker,
            &["app:1", "app:2"],
            &[],
            0,
        );
        let _ = reduce(&mut state, DockerIntent::ConfirmAccepted);
        let enabled = |state: &DockerState| {
            buttons_of(state).0.iter().all(|button| button.is_enabled)
        };

        let removed_row = enabled(&state);
        let _ = reduce(&mut state, DockerIntent::SelectRow(1));

        assert!(!removed_row);
        assert!(enabled(&state));
    }

    #[test]
    fn each_prune_dialog_counts_what_goes() {
        let cases = [
            (
                ResourceKind::Images,
                Key::DockerPruneImagesTitle,
                Key::TplDockerPruneImagesText,
                1,
            ),
            (
                ResourceKind::Networks,
                Key::DockerPruneNetworksTitle,
                Key::TplDockerPruneNetworksText,
                1,
            ),
        ];

        for (resource, title, text, count) in cases {
            let mut state = loaded_state();
            load_idle(&mut state);
            open(&mut state, resource);
            let _ = reduce(&mut state, DockerIntent::Prune);

            let dialog = confirm_dialog(&state).expect("an open dialog");

            assert_eq!(dialog.title, i18n::t(title), "{resource:?}");
            assert_eq!(dialog.text, i18n::with_count(text, count));
            assert_eq!(dialog.confirm_label, i18n::t(Key::DockerPrune));
        }
    }

    #[test]
    fn every_list_has_its_prune_button() {
        let mut state = loaded_state();
        let mut labels = Vec::new();

        for resource in ResourceKind::ALL {
            open(&mut state, resource);
            labels.push(prune_button(&state).map(|vm| vm.label));
        }

        assert_eq!(
            labels,
            vec![
                Some(i18n::t(Key::DockerPruneStopped)),
                Some(i18n::t(Key::DockerPruneDangling)),
                Some(i18n::t(Key::DockerPruneUnused)),
                Some(i18n::t(Key::DockerPruneUnused)),
            ]
        );
    }

    #[test]
    fn every_list_has_a_create_button() {
        let mut state = loaded_state();
        let mut enabled = Vec::new();

        for resource in ResourceKind::ALL {
            open(&mut state, resource);
            enabled.push(can_create(&state));
        }

        assert_eq!(enabled, vec![true, true, true, true]);
        assert!(!can_create(&DockerState::default()));
    }

    #[test]
    fn volume_prune_dialog_has_the_named_volumes_switch_off_and_counts_with_it()
    {
        let mut state = loaded_state();
        load_idle(&mut state);
        open(&mut state, ResourceKind::Volumes);
        let _ = reduce(&mut state, DockerIntent::Prune);

        let off = confirm_dialog(&state).expect("an open dialog");
        let _ = reduce(&mut state, DockerIntent::ConfirmSwitchToggled(true));
        let on = confirm_dialog(&state).expect("an open dialog");

        assert_eq!(off.title, i18n::t(Key::DockerPruneVolumesTitle));
        assert_eq!(
            off.switch,
            Some((i18n::t(Key::DockerAlsoRemoveNamedVolumes), false))
        );
        assert_eq!(
            off.text,
            i18n::with_count(Key::TplDockerPruneAnonymousVolumesText, 1)
        );
        assert_eq!(
            off.text,
            "Anonymous volumes no container uses: 1. They and all data in \
             them are deleted. Named volumes stay. This cannot be undone."
        );
        assert_eq!(
            on.switch,
            Some((i18n::t(Key::DockerAlsoRemoveNamedVolumes), true))
        );
        assert_eq!(
            on.text,
            i18n::with_count(Key::TplDockerPruneVolumesText, 2)
        );
        assert_eq!(
            on.text,
            "Volumes no container uses: 2. They and all data in them are \
             deleted, named ones too. This cannot be undone."
        );
    }

    #[test]
    fn shell_is_disabled_with_a_hint_when_the_engines_command_is_missing() {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        let resources = fixtures::resources(EngineKind::Docker);
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
        select(&mut state, "otty-fx-running");

        let view = action_buttons(&state).expect("buttons");

        assert_eq!(view.buttons[0], button(RowAction::Shell, false));
        assert_eq!(view.buttons[1], button(RowAction::Restart, true));
        assert_eq!(
            view.hint.as_deref(),
            Some(i18n::t(Key::DockerShellNeedsDocker))
        );
    }

    #[test]
    fn a_missing_podman_command_names_podman_in_the_shell_hint() {
        let state = DockerState::default();

        assert_eq!(
            shell_hint(&state, EngineKind::Podman, &[RowAction::Shell]),
            Some(i18n::t(Key::DockerShellNeedsPodman).to_string())
        );
        assert_eq!(
            shell_hint(&state, EngineKind::Podman, &[RowAction::Remove]),
            None
        );
    }

    #[test]
    fn shell_is_enabled_without_a_hint_when_the_command_is_found() {
        let mut state = loaded_state();
        select(&mut state, "otty-fx-running");

        let view = action_buttons(&state).expect("buttons");

        assert_eq!(view.buttons[0], button(RowAction::Shell, true));
        assert_eq!(view.hint, None);
    }
}
