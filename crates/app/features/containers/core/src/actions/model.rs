//! View models of the actions: the inspector buttons, the row menu, the
//! confirmation dialog and the Prune button.

use iced::Point;
use otty_app_i18n::{self as i18n, Key};
use otty_containers::EngineKind;

use super::ctx::ActionsCtx;
use super::offers::{
    ImageRemoval, REMOVE_ONLY, container_actions, image_actions,
    network_remove_hint, volume_remove_hint,
};
use super::state::{ActionsState, Confirm};
use super::types::RowAction;
use crate::types::{EngineAction, ListRow, ResourceKind};

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

impl ActionsViewModel {
    /// Build the action buttons of the selected row of the open list; `None`
    /// when nothing is selected. Every button is disabled while an action
    /// runs on the row; Remove is disabled, with the hint saying why, when the
    /// engine would refuse it (a volume or network in use, a built-in
    /// network).
    pub(crate) fn from_state(
        state: &ActionsState,
        ctx: &ActionsCtx<'_>,
    ) -> Option<Self> {
        let selected = ctx.open?.selected_id?;

        row_actions(state, ctx, selected)
    }
}

/// The open row menu: its items at the pointer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RowMenuViewModel {
    /// Where the right press was, in window coordinates.
    pub(crate) position: Point,
    pub(crate) items: Vec<ActionButton>,
}

impl RowMenuViewModel {
    /// Build the open row menu with the actions of its row (the selected row
    /// after a right press, any row after a press on its "⋯" button).
    pub(crate) fn from_state(
        state: &ActionsState,
        ctx: &ActionsCtx<'_>,
    ) -> Option<Self> {
        let position = state.row_menu()?;
        let actions = row_actions(state, ctx, state.row_menu_target()?)?;

        Some(Self {
            position,
            items: actions.buttons,
        })
    }
}

/// The confirmation dialog of a destructive action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfirmDialogViewModel {
    pub(crate) title: String,
    /// What goes and what stays.
    pub(crate) text: String,
    /// The switch label and whether it is on; `None` without a switch.
    pub(crate) switch: Option<(&'static str, bool)>,
    /// The label of the action button.
    pub(crate) confirm_label: &'static str,
}

impl ConfirmDialogViewModel {
    /// Build the open confirmation dialog.
    pub(crate) fn from_state(state: &ActionsState) -> Option<Self> {
        let confirm = state.confirm()?;
        let request = confirm.request();

        let (title, text, switch_key, confirm_key) = match request.action() {
            EngineAction::RemoveContainer { .. } => (
                i18n::containers_named(
                    Key::TplContainersRemoveContainerTitle,
                    request.name(),
                ),
                i18n::t(Key::ContainersRemoveContainerText).to_string(),
                Some(Key::ContainersStopItFirst),
                Key::ContainersActionRemove,
            ),
            EngineAction::RemoveImage { .. } => {
                let (title_key, text_key) = image_remove_texts(confirm);
                (
                    i18n::containers_named(title_key, request.name()),
                    i18n::t(text_key).to_string(),
                    Some(Key::ContainersForceRemoveImage),
                    Key::ContainersActionRemove,
                )
            },
            EngineAction::RemoveVolume(_) => (
                i18n::containers_named(
                    Key::TplContainersRemoveVolumeTitle,
                    request.name(),
                ),
                i18n::t(Key::ContainersRemoveVolumeText).to_string(),
                None,
                Key::ContainersActionRemove,
            ),
            EngineAction::RemoveNetwork(_) => (
                i18n::containers_named(
                    Key::TplContainersRemoveNetworkTitle,
                    request.name(),
                ),
                i18n::t(Key::ContainersRemoveNetworkText).to_string(),
                None,
                Key::ContainersActionRemove,
            ),
            EngineAction::PruneContainers => prune_texts(
                Key::ContainersPruneContainersTitle,
                Key::TplContainersPruneContainersText,
                confirm.count(),
            ),
            EngineAction::PruneImages => prune_texts(
                Key::ContainersPruneImagesTitle,
                Key::TplContainersPruneImagesText,
                confirm.count(),
            ),
            EngineAction::PruneVolumes { .. } => {
                let text_key = if confirm.switch() == Some(true) {
                    Key::TplContainersPruneVolumesText
                } else {
                    Key::TplContainersPruneAnonymousVolumesText
                };
                let (title, text, _, confirm_key) = prune_texts(
                    Key::ContainersPruneVolumesTitle,
                    text_key,
                    confirm.count(),
                );
                (
                    title,
                    text,
                    Some(Key::ContainersAlsoRemoveNamedVolumes),
                    confirm_key,
                )
            },
            EngineAction::PruneNetworks => prune_texts(
                Key::ContainersPruneNetworksTitle,
                Key::TplContainersPruneNetworksText,
                confirm.count(),
            ),
            _ => return None,
        };
        let switch = switch_key
            .zip(confirm.switch())
            .map(|(key, is_on)| (i18n::t(key), is_on));

        Some(Self {
            title,
            text,
            switch,
            confirm_label: i18n::t(confirm_key),
        })
    }
}

/// The Prune button above the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PruneViewModel {
    pub(crate) label: &'static str,
    /// `false` while the prune runs.
    pub(crate) is_enabled: bool,
}

impl PruneViewModel {
    /// Build the Prune button of the open list; `None` without one. It is
    /// disabled while a prune or a create runs on that list.
    pub(crate) fn from_state(
        state: &ActionsState,
        ctx: &ActionsCtx<'_>,
    ) -> Option<Self> {
        let open = ctx.open?;

        let label = match open.resource {
            ResourceKind::Containers => Key::ContainersPruneStopped,
            ResourceKind::Images => Key::ContainersPruneDangling,
            ResourceKind::Volumes | ResourceKind::Networks => {
                Key::ContainersPruneUnused
            },
        };

        Some(Self {
            label: i18n::t(label),
            is_enabled: !state.is_busy(
                open.engine.address(),
                open.resource,
                None,
            ),
        })
    }
}

/// The title and text of an image remove dialog: what goes away with the
/// switch as it is now.
fn image_remove_texts(confirm: &Confirm) -> (Key, Key) {
    let removal = confirm.image_removal().unwrap_or(ImageRemoval::WholeImage);

    let title = match removal {
        ImageRemoval::OnlyThisTag | ImageRemoval::ForceUntags => {
            Key::TplContainersRemoveTagTitle
        },
        _ => Key::TplContainersRemoveImageTitle,
    };
    let text = match removal {
        ImageRemoval::OnlyThisTag => Key::ContainersRemoveTagText,
        ImageRemoval::WholeImage => Key::ContainersRemoveImageText,
        ImageRemoval::Refused => Key::ContainersRemoveImageRefusedText,
        _ if confirm.switch() != Some(true) => {
            Key::ContainersRemoveImageNeedsForceText
        },
        ImageRemoval::ForceUntags => Key::ContainersForceUntagsImageText,
        ImageRemoval::ForceKeepsContainers => {
            Key::ContainersForceDeletesImageText
        },
        _ => Key::ContainersForceDeletesContainersText,
    };

    (title, text)
}

/// The action buttons of the row with `id` in the open list and the hint
/// under them, by the rules of [`ActionsViewModel::from_state`]; `None` when
/// the row is not in the loaded list.
fn row_actions(
    state: &ActionsState,
    ctx: &ActionsCtx<'_>,
    id: &str,
) -> Option<ActionsViewModel> {
    let open = ctx.open?;

    // What a running action on this row names: the id, but an image row goes
    // by what removes only that row.
    let (offered, remove_hint, action_target) = match open.find(id)? {
        ListRow::Container(container) => {
            (container_actions(container.state()), None, id)
        },
        ListRow::Image(image) => {
            (image_actions(image), None, image.remove_target())
        },
        ListRow::Volume(volume) => {
            (REMOVE_ONLY, volume_remove_hint(volume), id)
        },
        ListRow::Network(network) => {
            (REMOVE_ONLY, network_remove_hint(network), id)
        },
    };
    let is_busy = state.is_busy(
        open.engine.address(),
        open.resource,
        Some(action_target),
    );
    let shell_hint = shell_hint(ctx, open.engine.kind(), offered);

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
    ctx: &ActionsCtx<'_>,
    kind: EngineKind,
    offered: &[RowAction],
) -> Option<String> {
    if !offered.contains(&RowAction::Shell) || ctx.cli_path(kind).is_some() {
        return None;
    }

    let key = match kind {
        EngineKind::Docker => Key::ContainersShellNeedsDocker,
        EngineKind::Podman => Key::ContainersShellNeedsPodman,
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
        Key::ContainersPrune,
    )
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use otty_containers::{Container, EngineInfo, EngineKind, fixtures};

    use super::*;
    use crate::actions::event::ActionsIntent;
    use crate::event::{ContainersIntent, ContainersOutcome};
    use crate::reducer::reduce as reduce_containers;
    use crate::state::ContainersState;
    use crate::types::{LoadedLists, Place, ResourceKind, image_name};

    /// Send `intent` to the actions through the feature's reducer.
    fn send(
        state: &mut ContainersState,
        intent: ActionsIntent,
    ) -> Vec<ContainersOutcome> {
        reduce_containers(state, ContainersIntent::Actions(intent))
    }

    fn action_buttons(state: &ContainersState) -> Option<ActionsViewModel> {
        ActionsViewModel::from_state(state.actions(), &state.actions_ctx())
    }

    fn row_menu(state: &ContainersState) -> Option<RowMenuViewModel> {
        RowMenuViewModel::from_state(state.actions(), &state.actions_ctx())
    }

    fn confirm_dialog(
        state: &ContainersState,
    ) -> Option<ConfirmDialogViewModel> {
        ConfirmDialogViewModel::from_state(state.actions())
    }

    fn prune_button(state: &ContainersState) -> Option<PruneViewModel> {
        PruneViewModel::from_state(state.actions(), &state.actions_ctx())
    }

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn loaded_state() -> ContainersState {
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = ContainersState::default();
        let _ = reduce_containers(&mut state, ContainersIntent::Enter);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery_with_cli(
                &[EngineKind::Docker],
                Some("/usr/local/bin/docker"),
                None,
            )),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListsLoaded {
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

    fn select(state: &mut ContainersState, name: &str) {
        let index = fixtures::resources(EngineKind::Docker)
            .containers()
            .iter()
            .position(|container| container.name() == name)
            .expect("fixture container");

        let position = state.shown_position(index);
        let _ = reduce_containers(state, ContainersIntent::SelectRow(position));
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
        let _ = send(&mut state, ActionsIntent::Act(RowAction::Stop));

        let vm = action_buttons(&state).expect("a selected container");

        assert!(vm.buttons.iter().all(|button| !button.is_enabled));
    }

    #[test]
    fn nothing_selected_has_no_buttons() {
        let state = ContainersState::default();

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
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::RowMenuPressed { index, position },
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
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::OpenRowMenu { index: 3, position },
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
        let _ = send(&mut state, ActionsIntent::Act(RowAction::Remove));

        let dialog = confirm_dialog(&state).expect("an open dialog");

        assert_eq!(
            dialog,
            ConfirmDialogViewModel {
                title: i18n::containers_named(
                    Key::TplContainersRemoveContainerTitle,
                    "otty-fx-running"
                ),
                text: i18n::t(Key::ContainersRemoveContainerText).to_string(),
                switch: Some((i18n::t(Key::ContainersStopItFirst), true)),
                confirm_label: i18n::t(Key::ContainersActionRemove),
            }
        );
    }

    #[test]
    fn the_switch_shows_its_new_state() {
        let mut state = loaded_state();
        select(&mut state, "otty-fx-running");
        let _ = send(&mut state, ActionsIntent::Act(RowAction::Remove));

        let _ = send(&mut state, ActionsIntent::ConfirmSwitchToggled(false));

        let dialog = confirm_dialog(&state).expect("an open dialog");
        assert_eq!(
            dialog.switch,
            Some((i18n::t(Key::ContainersStopItFirst), false))
        );
    }

    #[test]
    fn a_stopped_container_dialog_has_no_switch() {
        let mut state = loaded_state();
        select(&mut state, "otty-fx-exit0");
        let _ = send(&mut state, ActionsIntent::Act(RowAction::Remove));

        let dialog = confirm_dialog(&state).expect("an open dialog");

        assert_eq!(dialog.switch, None);
    }

    #[test]
    fn the_prune_dialog_counts_the_stopped_containers() {
        let mut state = loaded_state();
        let _ = send(&mut state, ActionsIntent::Prune);

        let dialog = confirm_dialog(&state).expect("an open dialog");

        assert_eq!(dialog.title, i18n::t(Key::ContainersPruneContainersTitle));
        assert_eq!(
            dialog.text,
            i18n::with_count(Key::TplContainersPruneContainersText, 3)
        );
        assert_eq!(dialog.switch, None);
        assert_eq!(dialog.confirm_label, i18n::t(Key::ContainersPrune));
    }

    #[test]
    fn the_containers_table_has_a_prune_button_until_the_prune_runs() {
        let mut state = loaded_state();
        let idle = prune_button(&state);
        let _ = send(&mut state, ActionsIntent::Prune);
        let _ = send(&mut state, ActionsIntent::ConfirmAccepted);

        let busy = prune_button(&state);

        assert_eq!(
            idle,
            Some(PruneViewModel {
                label: i18n::t(Key::ContainersPruneStopped),
                is_enabled: true,
            })
        );
        assert_eq!(busy.map(|vm| vm.is_enabled), Some(false));
    }

    #[test]
    fn the_images_table_does_not_prune_containers() {
        let mut state = loaded_state();
        let _ = send(&mut state, ActionsIntent::Prune);
        let _ = send(&mut state, ActionsIntent::ConfirmAccepted);

        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Open(Place::new(
                docker().address().clone(),
                ResourceKind::Images,
            )),
        );

        assert_ne!(
            prune_button(&state).map(|vm| vm.label),
            Some(i18n::t(Key::ContainersPruneStopped))
        );
    }

    /// Replace the Docker lists with the idle fixture lists (no container).
    fn load_idle(state: &mut ContainersState) {
        let idle = fixtures::idle_resources(EngineKind::Docker);
        let _ = reduce_containers(
            state,
            ContainersIntent::ListsLoaded {
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

    fn open(state: &mut ContainersState, resource: ResourceKind) {
        let _ = reduce_containers(
            state,
            ContainersIntent::Open(Place::new(
                docker().address().clone(),
                resource,
            )),
        );
    }

    /// Open `resource` and select the row called `name`.
    fn select_in(
        state: &mut ContainersState,
        resource: ResourceKind,
        name: &str,
    ) {
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
        let _ = reduce_containers(state, ContainersIntent::SelectRow(position));
    }

    fn buttons_of(
        state: &ContainersState,
    ) -> (Vec<ActionButton>, Option<String>) {
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
                Some(i18n::containers_volume_in_use("otty-fx-running"))
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
                Some(i18n::t(Key::ContainersBuiltInNetworkHint).to_string())
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
        let _ = send(&mut state, ActionsIntent::Act(RowAction::Remove));
        let image = confirm_dialog(&state).expect("image dialog");
        let _ = send(&mut state, ActionsIntent::ConfirmCancelled);
        load_idle(&mut state);
        select_in(&mut state, ResourceKind::Volumes, "otty-fx-data");
        let _ = send(&mut state, ActionsIntent::Act(RowAction::Remove));
        let volume = confirm_dialog(&state).expect("volume dialog");
        let _ = send(&mut state, ActionsIntent::ConfirmCancelled);
        select_in(&mut state, ResourceKind::Networks, "otty-fx-net");
        let _ = send(&mut state, ActionsIntent::Act(RowAction::Remove));
        let network = confirm_dialog(&state).expect("network dialog");

        assert_eq!(
            image,
            ConfirmDialogViewModel {
                title: i18n::containers_named(
                    Key::TplContainersRemoveTagTitle,
                    "busybox:1.37"
                ),
                text: i18n::t(Key::ContainersForceUntagsImageText).to_string(),
                switch: Some((i18n::t(Key::ContainersForceRemoveImage), true)),
                confirm_label: i18n::t(Key::ContainersActionRemove),
            }
        );
        assert_eq!(
            volume.title,
            i18n::containers_named(
                Key::TplContainersRemoveVolumeTitle,
                "otty-fx-data"
            )
        );
        assert_eq!(volume.text, i18n::t(Key::ContainersRemoveVolumeText));
        assert_eq!(volume.switch, None);
        assert_eq!(
            network.title,
            i18n::containers_named(
                Key::TplContainersRemoveNetworkTitle,
                "otty-fx-net"
            )
        );
        assert_eq!(network.text, i18n::t(Key::ContainersRemoveNetworkText));
    }

    /// `kind` with its images open, listing an image with `names` and a
    /// container in each of `user_states`, the row at `index` selected and its
    /// remove dialog open.
    fn image_remove_dialog(
        kind: EngineKind,
        names: &[&str],
        user_states: &[&str],
        index: usize,
    ) -> ContainersState {
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

        let mut state = ContainersState::default();
        let _ = reduce_containers(&mut state, ContainersIntent::Enter);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[kind])),
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: info.address().clone(),
                lists: LoadedLists {
                    containers: Ok(users),
                    images: Ok(images),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        );
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Open(Place::new(
                info.address().clone(),
                ResourceKind::Images,
            )),
        );
        let _ =
            reduce_containers(&mut state, ContainersIntent::SelectRow(index));
        let _ = send(&mut state, ActionsIntent::Act(RowAction::Remove));

        state
    }

    fn dialog_text(state: &ContainersState) -> String {
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
                i18n::containers_named(
                    Key::TplContainersRemoveTagTitle,
                    "app:2"
                )
            );
            assert_eq!(dialog.text, i18n::t(Key::ContainersRemoveTagText));
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
                i18n::containers_named(
                    Key::TplContainersRemoveImageTitle,
                    "app:1"
                )
            );
            assert_eq!(dialog.text, i18n::t(Key::ContainersRemoveImageText));
            assert_eq!(dialog.switch, None);
        }
    }

    #[test]
    fn force_on_docker_says_the_image_stays_without_a_tag() {
        let state =
            image_remove_dialog(EngineKind::Docker, &["app:1"], &["exited"], 0);

        let dialog = confirm_dialog(&state).expect("the remove dialog");

        assert_eq!(dialog.text, i18n::t(Key::ContainersForceUntagsImageText));
        assert_eq!(
            dialog.switch,
            Some((i18n::t(Key::ContainersForceRemoveImage), true))
        );
    }

    #[test]
    fn force_on_docker_without_a_tag_says_the_containers_stay() {
        let state =
            image_remove_dialog(EngineKind::Docker, &[], &["exited"], 0);

        assert_eq!(
            dialog_text(&state),
            i18n::t(Key::ContainersForceDeletesImageText)
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
                i18n::t(Key::ContainersRemoveImageNeedsForceText)
            );
            assert_eq!(
                dialog.switch,
                Some((i18n::t(Key::ContainersForceRemoveImage), false))
            );
        }
    }

    #[test]
    fn force_on_podman_says_the_containers_go_too() {
        let mut state =
            image_remove_dialog(EngineKind::Podman, &["app:1"], &["exited"], 0);

        let _ = send(&mut state, ActionsIntent::ConfirmSwitchToggled(true));

        assert_eq!(
            dialog_text(&state),
            i18n::t(Key::ContainersForceDeletesContainersText)
        );
    }

    #[test]
    fn with_force_off_the_dialog_says_the_engine_refuses() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            let mut state =
                image_remove_dialog(kind, &["app:1"], &["exited"], 0);

            let _ =
                send(&mut state, ActionsIntent::ConfirmSwitchToggled(false));

            assert_eq!(
                dialog_text(&state),
                i18n::t(Key::ContainersRemoveImageNeedsForceText)
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

            assert_eq!(
                dialog.text,
                i18n::t(Key::ContainersRemoveImageRefusedText)
            );
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
        let _ = send(&mut state, ActionsIntent::ConfirmAccepted);
        let enabled = |state: &ContainersState| {
            buttons_of(state).0.iter().all(|button| button.is_enabled)
        };

        let removed_row = enabled(&state);
        let _ = reduce_containers(&mut state, ContainersIntent::SelectRow(1));

        assert!(!removed_row);
        assert!(enabled(&state));
    }

    #[test]
    fn each_prune_dialog_counts_what_goes() {
        let cases = [
            (
                ResourceKind::Images,
                Key::ContainersPruneImagesTitle,
                Key::TplContainersPruneImagesText,
                1,
            ),
            (
                ResourceKind::Networks,
                Key::ContainersPruneNetworksTitle,
                Key::TplContainersPruneNetworksText,
                1,
            ),
        ];

        for (resource, title, text, count) in cases {
            let mut state = loaded_state();
            load_idle(&mut state);
            open(&mut state, resource);
            let _ = send(&mut state, ActionsIntent::Prune);

            let dialog = confirm_dialog(&state).expect("an open dialog");

            assert_eq!(dialog.title, i18n::t(title), "{resource:?}");
            assert_eq!(dialog.text, i18n::with_count(text, count));
            assert_eq!(dialog.confirm_label, i18n::t(Key::ContainersPrune));
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
                Some(i18n::t(Key::ContainersPruneStopped)),
                Some(i18n::t(Key::ContainersPruneDangling)),
                Some(i18n::t(Key::ContainersPruneUnused)),
                Some(i18n::t(Key::ContainersPruneUnused)),
            ]
        );
    }

    #[test]
    fn volume_prune_dialog_has_the_named_volumes_switch_off_and_counts_with_it()
    {
        let mut state = loaded_state();
        load_idle(&mut state);
        open(&mut state, ResourceKind::Volumes);
        let _ = send(&mut state, ActionsIntent::Prune);

        let off = confirm_dialog(&state).expect("an open dialog");
        let _ = send(&mut state, ActionsIntent::ConfirmSwitchToggled(true));
        let on = confirm_dialog(&state).expect("an open dialog");

        assert_eq!(off.title, i18n::t(Key::ContainersPruneVolumesTitle));
        assert_eq!(
            off.switch,
            Some((i18n::t(Key::ContainersAlsoRemoveNamedVolumes), false))
        );
        assert_eq!(
            off.text,
            i18n::with_count(Key::TplContainersPruneAnonymousVolumesText, 1)
        );
        assert_eq!(
            off.text,
            "Anonymous volumes no container uses: 1. They and all data in \
             them are deleted. Named volumes stay. This cannot be undone."
        );
        assert_eq!(
            on.switch,
            Some((i18n::t(Key::ContainersAlsoRemoveNamedVolumes), true))
        );
        assert_eq!(
            on.text,
            i18n::with_count(Key::TplContainersPruneVolumesText, 2)
        );
        assert_eq!(
            on.text,
            "Volumes no container uses: 2. They and all data in them are \
             deleted, named ones too. This cannot be undone."
        );
    }

    #[test]
    fn shell_is_disabled_with_a_hint_when_the_engines_command_is_missing() {
        let mut state = ContainersState::default();
        let _ = reduce_containers(&mut state, ContainersIntent::Enter);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        let resources = fixtures::resources(EngineKind::Docker);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::ListsLoaded {
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
            Some(i18n::t(Key::ContainersShellNeedsDocker))
        );
    }

    #[test]
    fn a_missing_podman_command_names_podman_in_the_shell_hint() {
        let state = ContainersState::default();

        assert_eq!(
            shell_hint(
                &state.actions_ctx(),
                EngineKind::Podman,
                &[RowAction::Shell]
            ),
            Some(i18n::t(Key::ContainersShellNeedsPodman).to_string())
        );
        assert_eq!(
            shell_hint(
                &state.actions_ctx(),
                EngineKind::Podman,
                &[RowAction::Remove]
            ),
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
