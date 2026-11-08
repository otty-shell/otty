//! How an action goes, as the reducer runs it: a press on a button or a
//! menu item, the confirmation of a destructive action, the engine call and
//! its result (spec Decision 11).

use iced::{Point, Task};
use otty_containers::{
    Container, ContainerState, EngineAddress, EngineInfo, EngineKind, Image,
    Network, Volume,
};

use super::actions::{
    Confirm, ImageRemoval, REMOVE_ONLY, RowAction, container_actions,
    failure_text, image_actions, needs_stop, network_remove_hint,
    volume_remove_hint,
};
use super::create::event::CreateIntent;
use super::event::{ContainersEffect, ContainersEvent};
use super::reducer::reduce_create;
use super::run_flow;
use super::state::ContainersState;
use super::types::{
    ActionRequest, ContainersError, EngineAction, ResourceKind, image_name,
    split_reference,
};
use crate::i18n::{self, Key};

/// The row of the open list an action runs on and what it allows, copied
/// out of the state so the state can change.
struct TargetItem {
    engine: EngineInfo,
    resource: ResourceKind,
    /// What the engine call names: the row's id, or the name or id that
    /// removes an image row.
    id: String,
    /// The name the row goes by: an image row's reference.
    name: String,
    offered: &'static [RowAction],
    /// Remove is offered but the engine would refuse it.
    is_remove_blocked: bool,
    /// The first state of the remove dialog's switch ("Stop it first",
    /// "Force"); `None` without a switch.
    remove_switch: Option<bool>,
    /// What the removal of an image row does; `None` on other lists.
    image_removal: Option<ImageRemoval>,
}

/// Run `action` on the row of the open row menu, or else on the selected
/// row: a direct action asks the engine, a Remove asks the user first, Pull
/// again opens the pull dialog. An action the row does not offer, a Remove
/// the engine would refuse, or any action on a row that already runs one
/// does nothing. Closes the row menu.
pub(super) fn act(
    state: &mut ContainersState,
    action: RowAction,
) -> Task<ContainersEvent> {
    let target = state
        .row_menu_target()
        .or_else(|| state.selected_id())
        .map(str::to_string);
    state.close_row_menu();

    let Some(item) = target.and_then(|id| target_item(state, &id)) else {
        return Task::none();
    };
    let is_busy =
        state.is_busy(item.engine.address(), item.resource, Some(&item.id));
    if !item.offered.contains(&action) || is_busy {
        return Task::none();
    }

    let call = match action {
        RowAction::Remove => {
            if item.is_remove_blocked {
                return Task::none();
            }
            let request = ActionRequest::new(
                item.engine,
                remove_call(item.resource, item.id),
                item.name,
            );
            let confirm = Confirm::new(request, item.remove_switch, 0);
            state.open_confirm(match item.image_removal {
                Some(removal) => confirm.with_image_removal(removal),
                None => confirm,
            });
            return Task::none();
        },
        RowAction::PullAgain => {
            let (image, tag) = split_reference(&item.name);
            let open = CreateIntent::OpenPull {
                engine: item.engine,
                image: image.to_string(),
                tag: tag.to_string(),
            };
            return reduce_create(state, open);
        },
        RowAction::Shell => return open_shell(state, &item),
        RowAction::Run => {
            return run_flow::open(state, item.engine, &item.name);
        },
        RowAction::Start => EngineAction::StartContainer(item.id),
        RowAction::Stop => EngineAction::StopContainer(item.id),
        RowAction::Restart => EngineAction::RestartContainer(item.id),
        RowAction::Pause => EngineAction::PauseContainer(item.id),
        RowAction::Unpause => EngineAction::UnpauseContainer(item.id),
    };

    run(state, ActionRequest::new(item.engine, call, item.name))
}

/// Select the row at `index` of the shown table and open its menu at
/// `position` (a right press). Nothing opens past the last row.
pub(super) fn select_and_open_row_menu(
    state: &mut ContainersState,
    index: usize,
    position: Point,
) -> Task<ContainersEvent> {
    let _ = state.select_shown_row(index);

    open_row_menu(state, index, position)
}

/// Open the menu of the row at `index` of the shown table at `position`
/// and keep the selection (the row "⋯" button). Nothing opens past the last
/// row.
pub(super) fn open_row_menu(
    state: &mut ContainersState,
    index: usize,
    position: Point,
) -> Task<ContainersEvent> {
    if let Some(target) = state.shown_row_id(index) {
        state.open_row_menu(position, target);
    }

    Task::none()
}

/// Ask to prune the open list, with the count of what goes: the stopped
/// containers (created, exited or dead), the dangling images no container
/// uses, the anonymous volumes no container uses (the dialog switch adds the
/// named ones), or the user networks no container
/// is connected to. Nothing opens while a prune of that list runs.
pub(super) fn prune(state: &mut ContainersState) -> Task<ContainersEvent> {
    let (Some(place), Some(engine)) = (state.place(), state.open_engine())
    else {
        return Task::none();
    };
    let resource = place.resource();
    if state.is_busy(place.engine(), resource, None) {
        return Task::none();
    }

    let mut switch = None;
    let mut count_if_on = None;
    let (action, count) = match resource {
        ResourceKind::Containers => (
            EngineAction::PruneContainers,
            count(engine.containers().items(), is_stopped),
        ),
        ResourceKind::Images => (
            EngineAction::PruneImages,
            count(engine.images().items(), is_prunable_image),
        ),
        ResourceKind::Volumes => {
            // Named volumes stay unless the dialog switch is turned on.
            switch = Some(false);
            count_if_on =
                Some(count(engine.volumes().items(), is_unused_volume));
            (
                EngineAction::PruneVolumes {
                    include_named: false,
                },
                count(engine.volumes().items(), is_unused_anonymous_volume),
            )
        },
        ResourceKind::Networks => (
            EngineAction::PruneNetworks,
            count(engine.networks().items(), is_unused_network),
        ),
    };
    let request =
        ActionRequest::new(engine.info().clone(), action, String::new());

    let confirm = Confirm::new(request, switch, count)
        .with_count_if_on(count_if_on.unwrap_or(count));

    state.open_confirm(confirm);
    Task::none()
}

/// Run the confirmed action with its switch applied.
pub(super) fn accept(state: &mut ContainersState) -> Task<ContainersEvent> {
    let Some(confirm) = state.take_confirm() else {
        return Task::none();
    };

    run(state, confirm.into_request())
}

/// Take the result of an action: success reloads the engine's lists (only
/// while Containers mode is active), a failure shows a notice and leaves the
/// table as it was.
pub(super) fn finish(
    state: &mut ContainersState,
    request: &ActionRequest,
    result: Result<(), ContainersError>,
) -> Task<ContainersEvent> {
    state.finish_action(request);

    if let Err(err) = result {
        state.set_notice(failure_text(request, &err.to_string()));
        return Task::none();
    }

    reload(state, request.engine().address())
}

/// Reload every list of the engine at `address`, only while Containers mode
/// is active and the engine is still known.
pub(super) fn reload(
    state: &ContainersState,
    address: &EngineAddress,
) -> Task<ContainersEvent> {
    let Some(engine) = state.engine(address) else {
        return Task::none();
    };
    if !state.is_active() {
        return Task::none();
    }

    effect(ContainersEffect::LoadLists(engine.info().clone()))
}

/// Remember the running request and ask the app to run it.
pub(super) fn run(
    state: &mut ContainersState,
    request: ActionRequest,
) -> Task<ContainersEvent> {
    state.start_action(request.clone());

    effect(ContainersEffect::RunAction(request))
}

/// Open a terminal tab with a shell in the selected container, run by the
/// engine's own command line tool on that engine's address. Does nothing when
/// the search did not find that tool.
fn open_shell(
    state: &ContainersState,
    item: &TargetItem,
) -> Task<ContainersEvent> {
    let Some(program) = state.cli_path(item.engine.kind()) else {
        return Task::none();
    };

    effect(ContainersEffect::OpenShell {
        title: i18n::containers_named(
            Key::TplContainersShellTabTitle,
            &item.name,
        ),
        program: program.to_string_lossy().into_owned(),
        args: shell_args(item.engine.kind(), item.engine.address(), &item.id),
    })
}

/// The arguments of the command that opens a shell in container `id`. Without
/// the address the command would go to its own default engine, which may not
/// be the chosen one (Colima, Docker Desktop, rootful Podman); `--` keeps an
/// id from being read as a flag.
fn shell_args(
    kind: EngineKind,
    address: &EngineAddress,
    id: &str,
) -> Vec<String> {
    let host_flag = match kind {
        EngineKind::Docker => "--host",
        EngineKind::Podman => "--url",
    };
    let host = address.host_url();

    [host_flag, host.as_str(), "exec", "-it", "--", id, "sh"]
        .map(String::from)
        .to_vec()
}

/// The row with `id` in the open list.
fn target_item(state: &ContainersState, id: &str) -> Option<TargetItem> {
    let place = state.place()?;
    let engine = state.open_engine()?;
    let info = engine.info().clone();
    let resource = place.resource();

    let item = match resource {
        ResourceKind::Containers => {
            let container = engine.containers().find(id, Container::id)?;
            TargetItem {
                engine: info,
                resource,
                id: container.id().to_string(),
                name: container.name().to_string(),
                offered: container_actions(container.state()),
                is_remove_blocked: false,
                remove_switch: needs_stop(container.state()).then_some(true),
                image_removal: None,
            }
        },
        ResourceKind::Images => {
            let image = engine.images().find(id, Image::row_key)?;
            let removal = ImageRemoval::of(
                info.kind(),
                image,
                has_running_user(image, engine.containers().items()),
            );
            TargetItem {
                engine: info,
                resource,
                id: image.remove_target().to_string(),
                name: image_name(image).to_string(),
                offered: image_actions(image),
                is_remove_blocked: false,
                // On by default, as ticket 12 set it, except where Force
                // also deletes containers: one confirm must not do that.
                remove_switch: removal
                    .offers_force()
                    .then_some(removal != ImageRemoval::ForceDeletesContainers),
                image_removal: Some(removal),
            }
        },
        ResourceKind::Volumes => {
            let volume = engine.volumes().find(id, Volume::name)?;
            TargetItem {
                engine: info,
                resource,
                id: volume.name().to_string(),
                name: volume.name().to_string(),
                offered: REMOVE_ONLY,
                is_remove_blocked: volume_remove_hint(volume).is_some(),
                remove_switch: None,
                image_removal: None,
            }
        },
        ResourceKind::Networks => {
            let network = engine.networks().find(id, Network::id)?;
            TargetItem {
                engine: info,
                resource,
                id: network.id().to_string(),
                name: network.name().to_string(),
                offered: REMOVE_ONLY,
                is_remove_blocked: network_remove_hint(network).is_some(),
                remove_switch: None,
                image_removal: None,
            }
        },
    };

    Some(item)
}

/// Whether a running, paused or restarting container uses `image`.
fn has_running_user(image: &Image, containers: &[Container]) -> bool {
    containers.iter().any(|container| {
        container.image_id() == image.id() && needs_stop(container.state())
    })
}

/// The remove call of a row of `resource` with this id (not forced; the
/// dialog switch can force it).
fn remove_call(resource: ResourceKind, id: String) -> EngineAction {
    match resource {
        ResourceKind::Containers => {
            EngineAction::RemoveContainer { id, force: false }
        },
        ResourceKind::Images => EngineAction::RemoveImage {
            reference: id,
            force: false,
        },
        ResourceKind::Volumes => EngineAction::RemoveVolume(id),
        ResourceKind::Networks => EngineAction::RemoveNetwork(id),
    }
}

/// How many of `items` a prune removes.
fn count<T>(items: &[T], is_removed: fn(&T) -> bool) -> usize {
    items.iter().filter(|item| is_removed(item)).count()
}

/// Whether a container prune removes `container`.
fn is_stopped(container: &Container) -> bool {
    matches!(
        container.state(),
        ContainerState::Created | ContainerState::Exited | ContainerState::Dead
    )
}

/// Whether a dangling image prune removes `image`.
fn is_prunable_image(image: &Image) -> bool {
    image.is_dangling() && image.containers() == 0
}

/// Whether a volume prune that includes named volumes removes `volume`.
fn is_unused_volume(volume: &Volume) -> bool {
    volume.containers().is_empty()
}

/// Whether the default volume prune removes `volume`: unused and
/// anonymous.
fn is_unused_anonymous_volume(volume: &Volume) -> bool {
    volume.is_anonymous() && is_unused_volume(volume)
}

/// Whether a network prune removes `network`.
fn is_unused_network(network: &Network) -> bool {
    !network.is_built_in() && network.containers().is_empty()
}

fn effect(effect: ContainersEffect) -> Task<ContainersEvent> {
    Task::done(ContainersEvent::Effect(effect))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::Point;
    use otty_containers::{Container, EngineInfo, EngineKind, fixtures};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::containers::create::FormViewModel;
    use crate::widgets::containers::create::types::FormField;
    use crate::widgets::containers::event::{
        ContainersEffect, ContainersEvent, ContainersIntent,
    };
    use crate::widgets::containers::reducer::reduce;
    use crate::widgets::containers::state::ContainersState;
    use crate::widgets::containers::types::{
        LoadedLists, Place, ResourceKind, image_name,
    };

    /// Where the tests find the `docker` command: an install folder, not
    /// `PATH`.
    const DOCKER_CLI: &str = "/Users/ann/.docker/bin/docker";

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn effects(task: Task<ContainersEvent>) -> Vec<ContainersEffect> {
        collect_outputs(task)
            .into_iter()
            .filter_map(|event| match event {
                ContainersEvent::Effect(effect) => Some(effect),
                ContainersEvent::Intent(_) => None,
            })
            .collect()
    }

    /// Containers mode active, the Docker fixture engine found and loaded, its
    /// containers open.
    fn loaded_state() -> ContainersState {
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery_with_cli(
                &[EngineKind::Docker],
                Some(DOCKER_CLI),
                None,
            )),
        );
        let _ = reduce(
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

    fn container(name: &str) -> Container {
        fixtures::resources(EngineKind::Docker)
            .containers()
            .iter()
            .find(|container| container.name() == name)
            .cloned()
            .expect("fixture container")
    }

    /// Select the container `name` in the containers table.
    fn select(state: &mut ContainersState, name: &str) -> String {
        let resources = fixtures::resources(EngineKind::Docker);
        let index = resources
            .containers()
            .iter()
            .position(|container| container.name() == name)
            .expect("fixture container");

        let position = state.shown_position(index);
        let _ = reduce(state, ContainersIntent::SelectRow(position));
        container(name).id().to_string()
    }

    fn run(action: EngineAction, name: &str) -> ContainersEffect {
        ContainersEffect::RunAction(ActionRequest::new(
            docker(),
            action,
            name.to_string(),
        ))
    }

    #[test]
    fn each_direct_action_runs_on_the_selected_container() {
        let cases = [
            ("otty-fx-exit0", RowAction::Start, "start"),
            ("otty-fx-running", RowAction::Stop, "stop"),
            ("otty-fx-running", RowAction::Restart, "restart"),
            ("otty-fx-running", RowAction::Pause, "pause"),
            ("otty-fx-paused", RowAction::Unpause, "unpause"),
        ];

        for (name, action, call) in cases {
            let mut state = loaded_state();
            let id = select(&mut state, name);

            let effects =
                effects(reduce(&mut state, ContainersIntent::Act(action)));

            let expected = match call {
                "start" => EngineAction::StartContainer(id),
                "stop" => EngineAction::StopContainer(id),
                "restart" => EngineAction::RestartContainer(id),
                "pause" => EngineAction::PauseContainer(id),
                _ => EngineAction::UnpauseContainer(id),
            };
            assert_eq!(effects, vec![run(expected, name)], "{action:?}");
        }
    }

    #[test]
    fn an_action_the_state_does_not_offer_does_nothing() {
        let mut state = loaded_state();
        let _ = select(&mut state, "otty-fx-running");

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Start),
        ));

        assert!(effects.is_empty());
        assert!(state.running().is_empty());
    }

    #[test]
    fn while_an_action_runs_its_container_takes_no_other_action() {
        let mut state = loaded_state();
        let _ = select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Stop));

        let again = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Restart),
        ));
        let remove = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Remove),
        ));

        assert!(again.is_empty());
        assert!(remove.is_empty());
        assert!(state.confirm().is_none());
        assert_eq!(state.running().len(), 1);
    }

    #[test]
    fn another_container_can_act_while_one_runs_an_action() {
        let mut state = loaded_state();
        let _ = select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Stop));
        let id = select(&mut state, "otty-fx-exit0");

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Start),
        ));

        assert_eq!(
            effects,
            vec![run(EngineAction::StartContainer(id), "otty-fx-exit0")]
        );
        assert_eq!(state.running().len(), 2);
    }

    #[test]
    fn remove_asks_first_and_cancel_runs_nothing() {
        let mut state = loaded_state();
        let _ = select(&mut state, "otty-fx-exit0");

        let asked = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Remove),
        ));
        assert!(asked.is_empty());
        assert!(state.confirm().is_some());

        let cancelled =
            effects(reduce(&mut state, ContainersIntent::ConfirmCancelled));

        assert!(cancelled.is_empty());
        assert!(state.confirm().is_none());
        assert!(state.running().is_empty());
    }

    #[test]
    fn removing_a_running_container_stops_it_first_by_default() {
        let mut state = loaded_state();
        let id = select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));
        assert_eq!(state.confirm().and_then(Confirm::switch), Some(true));

        let effects =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::RemoveContainer { id, force: true },
                "otty-fx-running"
            )]
        );
        assert!(state.confirm().is_none());
        assert_eq!(state.running().len(), 1);
    }

    #[test]
    fn turning_stop_it_first_off_removes_without_force() {
        let mut state = loaded_state();
        let id = select(&mut state, "otty-fx-paused");
        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));

        let _ =
            reduce(&mut state, ContainersIntent::ConfirmSwitchToggled(false));
        let effects =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::RemoveContainer { id, force: false },
                "otty-fx-paused"
            )]
        );
    }

    #[test]
    fn a_stopped_container_is_removed_without_the_switch() {
        let mut state = loaded_state();
        let id = select(&mut state, "otty-fx-exit3");
        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));
        assert_eq!(state.confirm().and_then(Confirm::switch), None);

        let effects =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::RemoveContainer { id, force: false },
                "otty-fx-exit3"
            )]
        );
    }

    #[test]
    fn prune_asks_first_then_prunes_the_stopped_containers() {
        let mut state = loaded_state();

        let asked = effects(reduce(&mut state, ContainersIntent::Prune));
        // Created, exited 0 and exited 3.
        assert_eq!(state.confirm().map(Confirm::count), Some(3));
        let effects =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

        assert!(asked.is_empty());
        assert_eq!(effects, vec![run(EngineAction::PruneContainers, "")]);
    }

    #[test]
    fn a_second_prune_waits_for_the_first() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, ContainersIntent::Prune);
        let _ = reduce(&mut state, ContainersIntent::ConfirmAccepted);

        let _ = reduce(&mut state, ContainersIntent::Prune);

        assert!(state.confirm().is_none());
    }

    #[test]
    fn a_confirmation_without_a_dialog_runs_nothing() {
        let mut state = loaded_state();

        let accepted =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));
        let _ =
            reduce(&mut state, ContainersIntent::ConfirmSwitchToggled(true));

        assert!(accepted.is_empty());
        assert!(state.confirm().is_none());
    }

    #[test]
    fn a_finished_action_reloads_the_engines_lists() {
        let mut state = loaded_state();
        let id = select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Stop));
        let request = ActionRequest::new(
            docker(),
            EngineAction::StopContainer(id),
            String::from("otty-fx-running"),
        );

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::ActionFinished {
                request,
                result: Ok(()),
            },
        ));

        assert_eq!(effects, vec![ContainersEffect::LoadLists(docker())]);
        assert!(state.running().is_empty());
        assert!(state.notice().is_none());
    }

    #[test]
    fn a_failed_action_shows_a_notice_and_keeps_the_table() {
        let mut state = loaded_state();
        let id = select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Stop));
        let request = ActionRequest::new(
            docker(),
            EngineAction::StopContainer(id),
            String::from("otty-fx-running"),
        );

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::ActionFinished {
                request: request.clone(),
                result: Err(ContainersError::new("it is restarting")),
            },
        ));

        assert!(effects.is_empty());
        assert!(state.running().is_empty());
        assert_eq!(
            state.notice(),
            Some(failure_text(&request, "it is restarting").as_str())
        );
        let engine = state.open_engine().expect("docker engine");
        assert_eq!(engine.containers().items().len(), 6);
    }

    #[test]
    fn dismissing_the_notice_hides_it() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            ContainersIntent::ActionFinished {
                request: ActionRequest::new(
                    docker(),
                    EngineAction::PruneContainers,
                    String::new(),
                ),
                result: Err(ContainersError::new("busy")),
            },
        );

        let _ = reduce(&mut state, ContainersIntent::DismissNotice);

        assert!(state.notice().is_none());
    }

    #[test]
    fn opening_another_list_hides_the_notice() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            ContainersIntent::ActionFinished {
                request: ActionRequest::new(
                    docker(),
                    EngineAction::PruneContainers,
                    String::new(),
                ),
                result: Err(ContainersError::new("busy")),
            },
        );

        let _ = reduce(
            &mut state,
            ContainersIntent::Open(Place::new(
                docker().address().clone(),
                ResourceKind::Images,
            )),
        );

        assert!(state.notice().is_none());
    }

    #[test]
    fn a_finished_action_outside_containers_mode_does_not_reload() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, ContainersIntent::Leave);

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::ActionFinished {
                request: ActionRequest::new(
                    docker(),
                    EngineAction::PruneContainers,
                    String::new(),
                ),
                result: Ok(()),
            },
        ));

        assert!(effects.is_empty());
    }

    #[test]
    fn a_right_press_selects_the_row_and_opens_its_menu_there() {
        let mut state = loaded_state();
        let position = Point::new(120.0, 80.0);
        let index = state.shown_position(2);

        let _ = reduce(
            &mut state,
            ContainersIntent::OpenRowMenu { index, position },
        );

        let engine = state.open_engine().expect("docker engine");
        let selected = engine.containers().selected(Container::id);
        assert_eq!(selected.map(Container::name), Some("otty-fx-exit3"));
        assert_eq!(state.row_menu(), Some(position));
    }

    #[test]
    fn a_menu_item_closes_the_menu_and_runs_its_action() {
        let mut state = loaded_state();
        let index = state.shown_position(2);
        let _ = reduce(
            &mut state,
            ContainersIntent::OpenRowMenu {
                index,
                position: Point::ORIGIN,
            },
        );

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Start),
        ));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::StartContainer(
                    container("otty-fx-exit3").id().to_string()
                ),
                "otty-fx-exit3"
            )]
        );
        assert_eq!(state.row_menu(), None);
    }

    #[test]
    fn closing_the_menu_runs_nothing() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            ContainersIntent::OpenRowMenu {
                index: 0,
                position: Point::ORIGIN,
            },
        );

        let effects =
            effects(reduce(&mut state, ContainersIntent::CloseRowMenu));

        assert!(effects.is_empty());
        assert_eq!(state.row_menu(), None);
    }

    #[test]
    fn a_right_press_past_the_last_row_opens_no_menu() {
        let mut state = loaded_state();

        let _ = reduce(
            &mut state,
            ContainersIntent::OpenRowMenu {
                index: 99,
                position: Point::ORIGIN,
            },
        );

        assert_eq!(state.row_menu(), None);
    }

    /// Return the position of the container `name` in the shown table.
    fn shown_index(state: &ContainersState, name: &str) -> usize {
        let index = fixtures::resources(EngineKind::Docker)
            .containers()
            .iter()
            .position(|container| container.name() == name)
            .expect("fixture container");

        state.shown_position(index)
    }

    /// Return the id of the selected row, the one the inspector shows.
    fn selected_id(state: &ContainersState) -> Option<String> {
        state.selected_row().map(|(_, row)| row.id().to_string())
    }

    #[test]
    fn the_row_menu_button_opens_that_rows_menu_and_keeps_the_selection() {
        let mut state = loaded_state();
        let running = select(&mut state, "otty-fx-running");
        let position = Point::new(900.0, 120.0);
        let index = shown_index(&state, "otty-fx-exit3");

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::RowMenuPressed { index, position },
        ));

        assert!(effects.is_empty());
        assert_eq!(state.row_menu(), Some(position));
        assert_eq!(selected_id(&state), Some(running));
    }

    #[test]
    fn an_item_of_the_row_button_menu_runs_on_that_row_not_the_selected_one() {
        let mut state = loaded_state();
        let running = select(&mut state, "otty-fx-running");
        let index = shown_index(&state, "otty-fx-exit3");
        let _ = reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index,
                position: Point::ORIGIN,
            },
        );

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Start),
        ));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::StartContainer(
                    container("otty-fx-exit3").id().to_string()
                ),
                "otty-fx-exit3"
            )]
        );
        assert_eq!(state.row_menu(), None);
        assert_eq!(selected_id(&state), Some(running));
    }

    #[test]
    fn remove_from_the_row_button_menu_asks_about_that_row() {
        let mut state = loaded_state();
        let _ = select(&mut state, "otty-fx-running");
        let index = shown_index(&state, "otty-fx-exit3");
        let _ = reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index,
                position: Point::ORIGIN,
            },
        );

        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));

        let confirm = state.confirm().expect("the remove dialog");
        assert_eq!(confirm.request().name(), "otty-fx-exit3");
        assert_eq!(
            confirm.request().action(),
            &EngineAction::RemoveContainer {
                id: container("otty-fx-exit3").id().to_string(),
                force: false
            }
        );
    }

    #[test]
    fn after_the_row_button_menu_closes_the_inspector_buttons_act_on_the_selection()
     {
        let mut state = loaded_state();
        let running = select(&mut state, "otty-fx-running");
        let index = shown_index(&state, "otty-fx-exit3");
        let _ = reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index,
                position: Point::ORIGIN,
            },
        );
        let _ = reduce(&mut state, ContainersIntent::CloseRowMenu);

        let effects =
            effects(reduce(&mut state, ContainersIntent::Act(RowAction::Stop)));

        assert_eq!(
            effects,
            vec![run(EngineAction::StopContainer(running), "otty-fx-running")]
        );
    }

    #[test]
    fn the_row_menu_button_past_the_last_row_opens_no_menu() {
        let mut state = loaded_state();

        let _ = reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index: 99,
                position: Point::ORIGIN,
            },
        );

        assert_eq!(state.row_menu(), None);
    }

    /// Replace the Docker engine's lists with the idle fixture lists: no
    /// container, so nothing uses an image, a volume or a network.
    fn load_idle(state: &mut ContainersState) {
        let idle = fixtures::idle_resources(EngineKind::Docker);
        let _ = reduce(
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

    /// Open `resource` and select the row named `name` (the first tag of
    /// an image).
    fn select_in(
        state: &mut ContainersState,
        resource: ResourceKind,
        name: &str,
    ) {
        let _ = reduce(
            state,
            ContainersIntent::Open(Place::new(
                docker().address().clone(),
                resource,
            )),
        );
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
        let _ = reduce(state, ContainersIntent::SelectRow(position));
    }

    #[test]
    fn removing_an_image_a_container_uses_offers_force_on() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Images, "busybox:1.37");

        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));
        assert_eq!(state.confirm().and_then(Confirm::switch), Some(true));
        let effects =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::RemoveImage {
                    reference: String::from("busybox:1.37"),
                    force: true,
                },
                "busybox:1.37"
            )]
        );
    }

    const APP_ID: &str = "sha256:app";

    /// `kind` with its images open, listing the rows of an image with the id
    /// [`APP_ID`], `names` and `digests`, used by a container in each of
    /// `user_states` (for example `exited`).
    fn image_state(
        kind: EngineKind,
        names: &[&str],
        digests: &[&str],
        user_states: &[&str],
    ) -> ContainersState {
        let info = fixtures::engine(kind);
        let resources = fixtures::resources(kind);
        let users: Vec<Container> = user_states
            .iter()
            .enumerate()
            .map(|(index, state)| {
                fixtures::container_of_image(
                    &format!("user-{index}"),
                    state,
                    "app:1",
                    APP_ID,
                )
            })
            .collect();
        let images = fixtures::image_rows(APP_ID, names, digests, &users);

        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[kind])),
        );
        let _ = reduce(
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
        let _ = reduce(
            &mut state,
            ContainersIntent::Open(Place::new(
                info.address().clone(),
                ResourceKind::Images,
            )),
        );

        state
    }

    /// Select the row at `index` of the shown images and press Remove;
    /// returns the switch of the dialog and the call a confirm then makes.
    fn remove_row(
        state: &mut ContainersState,
        index: usize,
    ) -> (Option<bool>, EngineAction) {
        let _ = reduce(state, ContainersIntent::SelectRow(index));
        let _ = reduce(state, ContainersIntent::Act(RowAction::Remove));
        let switch = state.confirm().and_then(Confirm::switch);
        let call = state
            .take_confirm()
            .expect("the remove dialog")
            .into_request()
            .action()
            .clone();

        (switch, call)
    }

    fn remove_call_of(reference: &str, force: bool) -> EngineAction {
        EngineAction::RemoveImage {
            reference: reference.to_string(),
            force,
        }
    }

    #[test]
    fn removing_one_of_several_tags_names_it_and_has_no_switch() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            for users in [&[][..], &["exited"][..], &["running"][..]] {
                let mut state =
                    image_state(kind, &["app:1", "app:2"], &[], users);

                let removed = remove_row(&mut state, 1);

                assert_eq!(
                    removed,
                    (None, remove_call_of("app:2", false)),
                    "{kind:?} {users:?}"
                );
            }
        }
    }

    #[test]
    fn removing_the_last_tag_of_an_unused_image_names_it_and_has_no_switch() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            let mut state = image_state(kind, &["app:1"], &[], &[]);

            let removed = remove_row(&mut state, 0);

            assert_eq!(removed, (None, remove_call_of("app:1", false)));
        }
    }

    #[test]
    fn removing_an_untagged_unused_image_goes_by_id_with_no_switch() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            let mut state = image_state(kind, &[], &[], &[]);

            let removed = remove_row(&mut state, 0);

            assert_eq!(removed, (None, remove_call_of(APP_ID, false)));
        }
    }

    #[test]
    fn the_last_tag_of_a_used_image_on_docker_can_be_forced() {
        for users in [&["exited"][..], &["created"][..], &["running"][..]] {
            let mut state =
                image_state(EngineKind::Docker, &["app:1"], &[], users);

            let removed = remove_row(&mut state, 0);

            assert_eq!(
                removed,
                (Some(true), remove_call_of("app:1", true)),
                "{users:?}"
            );
        }
    }

    #[test]
    fn an_untagged_image_stopped_containers_use_on_docker_can_be_forced() {
        let mut state = image_state(EngineKind::Docker, &[], &[], &["exited"]);

        let removed = remove_row(&mut state, 0);

        assert_eq!(removed, (Some(true), remove_call_of(APP_ID, true)));
    }

    #[test]
    fn an_untagged_image_a_running_container_uses_on_docker_is_not_forced() {
        for user in ["running", "paused", "restarting"] {
            let mut state = image_state(EngineKind::Docker, &[], &[], &[user]);

            let removed = remove_row(&mut state, 0);

            assert_eq!(
                removed,
                (None, remove_call_of(APP_ID, false)),
                "{user}"
            );
        }
    }

    #[test]
    fn an_image_only_stopped_containers_use_on_podman_starts_with_force_off() {
        // Force would delete those containers too, so one confirm must not.
        let mut tagged =
            image_state(EngineKind::Podman, &["app:1"], &[], &["exited"]);
        let mut untagged =
            image_state(EngineKind::Podman, &[], &[], &["exited"]);

        assert_eq!(
            remove_row(&mut tagged, 0),
            (Some(false), remove_call_of("app:1", false))
        );
        assert_eq!(
            remove_row(&mut untagged, 0),
            (Some(false), remove_call_of(APP_ID, false))
        );
    }

    #[test]
    fn turning_force_on_on_podman_forces_the_removal() {
        let mut state =
            image_state(EngineKind::Podman, &["app:1"], &[], &["exited"]);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(0));
        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));

        let _ =
            reduce(&mut state, ContainersIntent::ConfirmSwitchToggled(true));
        let effects =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![ContainersEffect::RunAction(ActionRequest::new(
                fixtures::engine(EngineKind::Podman),
                remove_call_of("app:1", true),
                String::from("app:1"),
            ))]
        );
    }

    #[test]
    fn a_podman_refusal_of_a_removal_left_without_force_reaches_the_notice() {
        let mut state =
            image_state(EngineKind::Podman, &["app:1"], &[], &["exited"]);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(0));
        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));
        let request = ActionRequest::new(
            fixtures::engine(EngineKind::Podman),
            remove_call_of("app:1", false),
            String::from("app:1"),
        );

        let started =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));
        let _ = reduce(
            &mut state,
            ContainersIntent::ActionFinished {
                request: request.clone(),
                result: Err(ContainersError::new("image is in use")),
            },
        );

        assert_eq!(started, vec![ContainersEffect::RunAction(request.clone())]);
        assert_eq!(
            state.notice(),
            Some(failure_text(&request, "image is in use").as_str())
        );
    }

    #[test]
    fn a_container_that_runs_on_podman_keeps_force_away_from_its_stopped_ones()
    {
        // Force would delete the stopped container and then fail anyway.
        let mut state = image_state(
            EngineKind::Podman,
            &["app:1"],
            &[],
            &["exited", "running"],
        );

        let removed = remove_row(&mut state, 0);

        assert_eq!(removed, (None, remove_call_of("app:1", false)));
    }

    #[test]
    fn a_digest_row_beside_a_tag_goes_by_its_digest_and_an_only_one_by_id() {
        let mut beside = image_state(
            EngineKind::Docker,
            &["app:1"],
            &["reg/app@sha256:aa"],
            &[],
        );
        let mut only =
            image_state(EngineKind::Docker, &[], &["reg/app@sha256:aa"], &[]);

        assert_eq!(
            remove_row(&mut beside, 1),
            (None, remove_call_of("reg/app@sha256:aa", false))
        );
        assert_eq!(
            remove_row(&mut only, 0),
            (None, remove_call_of(APP_ID, false))
        );
    }

    #[test]
    fn the_row_menu_of_a_tag_acts_on_that_tag() {
        let mut state =
            image_state(EngineKind::Docker, &["app:1", "app:2"], &[], &[]);
        let _ = reduce(
            &mut state,
            ContainersIntent::RowMenuPressed {
                index: 1,
                position: Point::ORIGIN,
            },
        );

        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));

        let confirm = state.confirm().expect("the remove dialog");
        assert_eq!(confirm.request().name(), "app:2");
        assert_eq!(confirm.request().action(), &remove_call_of("app:2", false));
    }

    #[test]
    fn removing_a_tag_blocks_only_that_row() {
        let mut state =
            image_state(EngineKind::Docker, &["app:1", "app:2"], &[], &[]);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(0));
        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));
        let _ = reduce(&mut state, ContainersIntent::ConfirmAccepted);

        let again =
            reduce(&mut state, ContainersIntent::Act(RowAction::Remove));
        let _ = collect_outputs(again);
        assert!(state.confirm().is_none());

        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));
        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));
        assert!(state.confirm().is_some());
    }

    #[test]
    fn a_refused_tag_removal_reaches_the_notice_and_names_the_tag() {
        let mut state = image_state(EngineKind::Docker, &["app:1"], &[], &[]);
        let (_, call) = remove_row(&mut state, 0);
        let request = ActionRequest::new(
            fixtures::engine(EngineKind::Docker),
            call,
            String::from("app:1"),
        );

        let _ = reduce(
            &mut state,
            ContainersIntent::ActionFinished {
                request: request.clone(),
                result: Err(ContainersError::new("must be forced")),
            },
        );

        let notice = state.notice().expect("a notice");
        assert_eq!(notice, failure_text(&request, "must be forced"));
        assert!(notice.contains("app:1"), "{notice}");
    }

    #[test]
    fn run_on_a_tag_opens_the_form_with_that_tag() {
        let mut state =
            image_state(EngineKind::Docker, &["app:1", "app:2"], &[], &[]);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Run));

        assert_eq!(state.run_form().map(|form| form.image()), Some("app:2"));
    }

    #[test]
    fn pull_again_on_a_tag_opens_the_dialog_with_that_tag() {
        let mut state =
            image_state(EngineKind::Docker, &["app:1", "app:2"], &[], &[]);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::PullAgain));

        let form = state.create().form().expect("the pull dialog");
        assert_eq!(form.value(FormField::Name), "app");
        assert_eq!(form.value(FormField::Tag), "2");
    }

    #[test]
    fn removing_an_unused_image_has_no_switch() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Images, "otty-fx-img:1");

        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));
        assert_eq!(state.confirm().and_then(Confirm::switch), None);
        let effects =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::RemoveImage {
                    reference: String::from("otty-fx-img:1"),
                    force: false,
                },
                "otty-fx-img:1"
            )]
        );
    }

    #[test]
    fn pull_again_opens_the_pull_dialog_with_the_image_name_and_tag() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Images, "busybox:1.37");

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::PullAgain),
        ));

        assert!(effects.is_empty());
        assert_eq!(
            FormViewModel::from_state(state.create()).map(|vm| vm.title),
            Some(i18n::t(Key::ContainersPullImage))
        );
        let form = state.create().form().expect("the pull dialog");
        assert_eq!(form.value(FormField::Name), "busybox");
        assert_eq!(form.value(FormField::Tag), "1.37");
    }

    #[test]
    fn a_dangling_image_cannot_be_pulled_again() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Images, "<none>:<none>");

        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::PullAgain));

        assert!(!state.create().is_open());
    }

    #[test]
    fn a_volume_a_container_uses_is_not_removed() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Volumes, "otty-fx-data");

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Remove),
        ));

        assert!(effects.is_empty());
        assert!(state.confirm().is_none());
    }

    #[test]
    fn an_unused_volume_is_removed_after_confirming() {
        let mut state = loaded_state();
        load_idle(&mut state);
        select_in(&mut state, ResourceKind::Volumes, "otty-fx-data");

        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));
        let effects =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::RemoveVolume(String::from("otty-fx-data")),
                "otty-fx-data"
            )]
        );
    }

    #[test]
    fn a_built_in_or_connected_network_is_not_removed() {
        for name in ["bridge", "host", "none", "otty-fx-net"] {
            let mut state = loaded_state();
            select_in(&mut state, ResourceKind::Networks, name);

            let _ =
                reduce(&mut state, ContainersIntent::Act(RowAction::Remove));

            assert!(state.confirm().is_none(), "{name}");
        }
    }

    #[test]
    fn an_unused_network_is_removed_by_id_after_confirming() {
        let mut state = loaded_state();
        load_idle(&mut state);
        select_in(&mut state, ResourceKind::Networks, "otty-fx-net");
        let id = state
            .open_engine()
            .and_then(|engine| engine.networks().selected_id())
            .expect("selected network")
            .to_string();

        let _ = reduce(&mut state, ContainersIntent::Act(RowAction::Remove));
        let effects =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![run(EngineAction::RemoveNetwork(id), "otty-fx-net")]
        );
    }

    #[test]
    fn container_actions_do_nothing_on_other_lists() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Images, "busybox:1.37");

        let effects =
            effects(reduce(&mut state, ContainersIntent::Act(RowAction::Stop)));

        assert!(effects.is_empty());
        assert!(state.confirm().is_none());
    }

    #[test]
    fn each_list_prunes_after_confirming_with_its_count() {
        let cases = [
            (ResourceKind::Images, EngineAction::PruneImages, 1),
            (ResourceKind::Networks, EngineAction::PruneNetworks, 1),
        ];

        for (resource, action, count) in cases {
            let mut state = loaded_state();
            load_idle(&mut state);
            let _ = reduce(
                &mut state,
                ContainersIntent::Open(Place::new(
                    docker().address().clone(),
                    resource,
                )),
            );

            let _ = reduce(&mut state, ContainersIntent::Prune);
            assert_eq!(
                state.confirm().map(Confirm::count),
                Some(count),
                "{resource:?}"
            );
            let effects =
                effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

            assert_eq!(effects, vec![run(action, "")], "{resource:?}");
        }
    }

    fn open_volume_prune(state: &mut ContainersState) {
        load_idle(state);
        let _ = reduce(
            state,
            ContainersIntent::Open(Place::new(
                docker().address().clone(),
                ResourceKind::Volumes,
            )),
        );
        let _ = reduce(state, ContainersIntent::Prune);
    }

    #[test]
    fn volume_prune_keeps_named_volumes_by_default() {
        let mut state = loaded_state();
        open_volume_prune(&mut state);

        assert_eq!(state.confirm().and_then(Confirm::switch), Some(false));
        assert_eq!(state.confirm().map(Confirm::count), Some(1));
        let effects =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::PruneVolumes {
                    include_named: false
                },
                ""
            )]
        );
    }

    #[test]
    fn volume_prune_switch_adds_named_volumes_and_their_count() {
        let mut state = loaded_state();
        open_volume_prune(&mut state);

        let _ =
            reduce(&mut state, ContainersIntent::ConfirmSwitchToggled(true));
        assert_eq!(state.confirm().map(Confirm::count), Some(2));
        let effects =
            effects(reduce(&mut state, ContainersIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::PruneVolumes {
                    include_named: true
                },
                ""
            )]
        );
    }

    #[test]
    fn volume_prune_count_goes_back_when_the_switch_is_turned_off() {
        let mut state = loaded_state();
        open_volume_prune(&mut state);

        let _ =
            reduce(&mut state, ContainersIntent::ConfirmSwitchToggled(true));
        let _ =
            reduce(&mut state, ContainersIntent::ConfirmSwitchToggled(false));

        assert_eq!(state.confirm().map(Confirm::count), Some(1));
    }

    /// Podman mode active, the Podman fixture engine found and loaded, with
    /// the given commands found.
    fn podman_state(
        docker_cli: Option<&str>,
        podman_cli: Option<&str>,
    ) -> ContainersState {
        let engine = fixtures::engine(EngineKind::Podman);
        let resources = fixtures::resources(EngineKind::Podman);
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery_with_cli(
                &[EngineKind::Podman],
                docker_cli,
                podman_cli,
            )),
        );
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: engine.address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        );
        let index = resources
            .containers()
            .iter()
            .position(|container| container.name() == "otty-fx-running")
            .expect("fixture container");
        let position = state.shown_position(index);
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));
        state
    }

    fn podman_container_id() -> String {
        fixtures::resources(EngineKind::Podman)
            .containers()
            .iter()
            .find(|container| container.name() == "otty-fx-running")
            .map(|container| container.id().to_string())
            .expect("fixture container")
    }

    fn shell(
        program: &str,
        flag: &str,
        socket: &str,
        id: &str,
    ) -> ContainersEffect {
        let host = format!("unix://{socket}");

        ContainersEffect::OpenShell {
            title: String::from("shell otty-fx-running"),
            program: program.to_string(),
            args: [flag, host.as_str(), "exec", "-it", "--", id, "sh"]
                .map(String::from)
                .to_vec(),
        }
    }

    fn podman_shell(program: &str) -> ContainersEffect {
        shell(
            program,
            "--url",
            fixtures::PODMAN_SOCKET,
            &podman_container_id(),
        )
    }

    #[test]
    fn shell_opens_docker_exec_with_the_command_found_in_an_install_folder() {
        let mut state = loaded_state();
        let id = select(&mut state, "otty-fx-running");

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Shell),
        ));

        assert_eq!(
            effects,
            vec![shell(DOCKER_CLI, "--host", fixtures::DOCKER_SOCKET, &id)]
        );
    }

    #[test]
    fn shell_on_a_podman_engine_opens_podman_exec() {
        let mut state = podman_state(None, Some("/opt/podman/bin/podman"));

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Shell),
        ));

        assert_eq!(effects, vec![podman_shell("/opt/podman/bin/podman")]);
    }

    #[test]
    fn shell_on_a_podman_engine_runs_the_podman_docker_wrapper() {
        // The wrapper is the only command found: it counts as Podman, so the
        // cli reports it as the Podman command and not as Docker.
        let mut state = podman_state(None, Some("/usr/bin/docker"));

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Shell),
        ));

        assert_eq!(effects, vec![podman_shell("/usr/bin/docker")]);
    }

    #[test]
    fn shell_args_name_a_docker_compatible_socket_that_is_not_the_default() {
        let colima = EngineAddress::Unix(PathBuf::from(
            "/home/ann/.colima/default/docker.sock",
        ));

        let args = shell_args(EngineKind::Docker, &colima, "abc123");

        assert_eq!(
            args,
            [
                "--host",
                "unix:///home/ann/.colima/default/docker.sock",
                "exec",
                "-it",
                "--",
                "abc123",
                "sh",
            ]
        );
    }

    #[test]
    fn shell_args_name_the_rootful_podman_socket() {
        let rootful =
            EngineAddress::Unix(PathBuf::from("/run/podman/podman.sock"));

        let args = shell_args(EngineKind::Podman, &rootful, "abc123");

        assert_eq!(
            args,
            [
                "--url",
                "unix:///run/podman/podman.sock",
                "exec",
                "-it",
                "--",
                "abc123",
                "sh",
            ]
        );
    }

    #[test]
    fn shell_args_name_a_tcp_engine() {
        let tcp = EngineAddress::Tcp(String::from("localhost:2375"));

        let args = shell_args(EngineKind::Docker, &tcp, "abc123");

        assert_eq!(args[..2], ["--host", "tcp://localhost:2375"]);
    }

    #[test]
    fn shell_args_never_read_an_id_as_a_flag() {
        let tcp = EngineAddress::Tcp(String::from("localhost:2375"));

        let args = shell_args(EngineKind::Docker, &tcp, "--privileged");

        assert_eq!(args[4..], ["--", "--privileged", "sh"]);
    }

    #[test]
    fn shell_on_a_podman_engine_ignores_a_real_docker_command() {
        let mut state = podman_state(Some(DOCKER_CLI), None);

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Shell),
        ));

        assert!(effects.is_empty());
    }

    #[test]
    fn shell_without_the_command_does_nothing() {
        let mut state = podman_state(None, None);

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Shell),
        ));

        assert!(effects.is_empty());
    }

    #[test]
    fn shell_on_a_stopped_container_does_nothing() {
        let mut state = loaded_state();
        let _ = select(&mut state, "otty-fx-exit0");

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(RowAction::Shell),
        ));

        assert!(effects.is_empty());
    }
}
