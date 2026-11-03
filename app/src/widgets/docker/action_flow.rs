//! How an action goes, as the reducer runs it: a press on a button or a
//! menu item, the confirmation of a destructive action, the engine call and
//! its result (spec Decision 11).

use iced::{Point, Task};
use otty_docker::{
    Container, ContainerState, EngineAddress, EngineInfo, Image, Network,
    Volume,
};

use super::actions::{
    ActionRequest, Confirm, EngineAction, REMOVE_ONLY, RowAction,
    container_actions, failure_text, image_actions, needs_stop,
    network_remove_hint, volume_remove_hint,
};
use super::errors::DockerError;
use super::event::{DockerEffect, DockerEvent};
use super::forms::{Form, split_reference};
use super::run_flow;
use super::state::DockerState;
use super::types::{ResourceKind, image_name};
use crate::i18n::{self, Key};

/// The row of the open list an action runs on and what it allows, copied
/// out of the state so the state can change.
struct TargetItem {
    engine: EngineInfo,
    resource: ResourceKind,
    id: String,
    /// The name the table shows (the first tag of an image).
    name: String,
    offered: &'static [RowAction],
    /// Remove is offered but the engine would refuse it.
    is_remove_blocked: bool,
    /// The first state of the remove dialog's switch ("Stop it first",
    /// "Force"); `None` without a switch.
    remove_switch: Option<bool>,
}

/// Run `action` on the row of the open row menu, or else on the selected
/// row: a direct action asks the engine, a Remove asks the user first, Pull
/// again opens the pull dialog. An action the row does not offer, a Remove
/// the engine would refuse, or any action on a row that already runs one
/// does nothing. Closes the row menu.
pub(super) fn act(
    state: &mut DockerState,
    action: RowAction,
) -> Task<DockerEvent> {
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
            state.open_confirm(Confirm::new(request, item.remove_switch, 0));
            return Task::none();
        },
        RowAction::PullAgain => {
            let (image, tag) = split_reference(&item.name);
            state.open_form(Form::pull(item.engine, image, tag));
            return Task::none();
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
    state: &mut DockerState,
    index: usize,
    position: Point,
) -> Task<DockerEvent> {
    let _ = state.select_shown_row(index);

    open_row_menu(state, index, position)
}

/// Open the menu of the row at `index` of the shown table at `position`
/// and keep the selection (the row "⋯" button). Nothing opens past the last
/// row.
pub(super) fn open_row_menu(
    state: &mut DockerState,
    index: usize,
    position: Point,
) -> Task<DockerEvent> {
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
pub(super) fn prune(state: &mut DockerState) -> Task<DockerEvent> {
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
pub(super) fn accept(state: &mut DockerState) -> Task<DockerEvent> {
    let Some(confirm) = state.take_confirm() else {
        return Task::none();
    };

    run(state, confirm.into_request())
}

/// Take the result of an action: success reloads the engine's lists (only
/// while Docker mode is active), a failure shows a notice and leaves the
/// table as it was.
pub(super) fn finish(
    state: &mut DockerState,
    request: &ActionRequest,
    result: Result<(), DockerError>,
) -> Task<DockerEvent> {
    state.finish_action(request);

    if let Err(err) = result {
        state.set_notice(failure_text(request, &err.to_string()));
        return Task::none();
    }

    reload(state, request.engine().address())
}

/// Reload every list of the engine at `address`, only while Docker mode
/// is active and the engine is still known.
pub(super) fn reload(
    state: &DockerState,
    address: &EngineAddress,
) -> Task<DockerEvent> {
    let Some(engine) = state.engine(address) else {
        return Task::none();
    };
    if !state.is_active() {
        return Task::none();
    }

    effect(DockerEffect::LoadLists(engine.info().clone()))
}

/// Remember the running request and ask the app to run it.
pub(super) fn run(
    state: &mut DockerState,
    request: ActionRequest,
) -> Task<DockerEvent> {
    state.start_action(request.clone());

    effect(DockerEffect::RunAction(request))
}

/// Open a terminal tab with a shell in the selected container, run by the
/// engine's own command line tool. Does nothing when the search did not find
/// that tool.
fn open_shell(state: &DockerState, item: &TargetItem) -> Task<DockerEvent> {
    let Some(program) = state.cli_path(item.engine.kind()) else {
        return Task::none();
    };

    effect(DockerEffect::OpenShell {
        title: i18n::docker_named(Key::TplDockerShellTabTitle, &item.name),
        program: program.to_string_lossy().into_owned(),
        args: ["exec", "-it", item.id.as_str(), "sh"]
            .map(String::from)
            .to_vec(),
    })
}

/// The row with `id` in the open list.
fn target_item(state: &DockerState, id: &str) -> Option<TargetItem> {
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
            }
        },
        ResourceKind::Images => {
            let image = engine.images().find(id, Image::id)?;
            TargetItem {
                engine: info,
                resource,
                id: image.id().to_string(),
                name: image_name(image).to_string(),
                offered: image_actions(image),
                is_remove_blocked: false,
                remove_switch: (image.containers() > 0).then_some(true),
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
            }
        },
    };

    Some(item)
}

/// The remove call of a row of `resource` with this id (not forced; the
/// dialog switch can force it).
fn remove_call(resource: ResourceKind, id: String) -> EngineAction {
    match resource {
        ResourceKind::Containers => {
            EngineAction::RemoveContainer { id, force: false }
        },
        ResourceKind::Images => EngineAction::RemoveImage { id, force: false },
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

fn effect(effect: DockerEffect) -> Task<DockerEvent> {
    Task::done(DockerEvent::Effect(effect))
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use otty_docker::{Container, EngineInfo, EngineKind, fixtures};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::docker::event::{
        DockerEffect, DockerEvent, DockerIntent,
    };
    use crate::widgets::docker::forms::{FormField, FormKind};
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::state::DockerState;
    use crate::widgets::docker::types::{
        LoadedLists, Place, ResourceKind, image_name,
    };

    /// Where the tests find the `docker` command: an install folder, not
    /// `PATH`.
    const DOCKER_CLI: &str = "/Users/ann/.docker/bin/docker";

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn effects(task: Task<DockerEvent>) -> Vec<DockerEffect> {
        collect_outputs(task)
            .into_iter()
            .filter_map(|event| match event {
                DockerEvent::Effect(effect) => Some(effect),
                DockerEvent::Intent(_) => None,
            })
            .collect()
    }

    /// Docker mode active, the Docker fixture engine found and loaded, its
    /// containers open.
    fn loaded_state() -> DockerState {
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery_with_cli(
                &[EngineKind::Docker],
                Some(DOCKER_CLI),
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

    fn container(name: &str) -> Container {
        fixtures::resources(EngineKind::Docker)
            .containers()
            .iter()
            .find(|container| container.name() == name)
            .cloned()
            .expect("fixture container")
    }

    /// Select the container `name` in the containers table.
    fn select(state: &mut DockerState, name: &str) -> String {
        let resources = fixtures::resources(EngineKind::Docker);
        let index = resources
            .containers()
            .iter()
            .position(|container| container.name() == name)
            .expect("fixture container");

        let position = state.shown_position(index);
        let _ = reduce(state, DockerIntent::SelectRow(position));
        container(name).id().to_string()
    }

    fn run(action: EngineAction, name: &str) -> DockerEffect {
        DockerEffect::RunAction(ActionRequest::new(
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
                effects(reduce(&mut state, DockerIntent::Act(action)));

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

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Start)));

        assert!(effects.is_empty());
        assert!(state.running().is_empty());
    }

    #[test]
    fn while_an_action_runs_its_container_takes_no_other_action() {
        let mut state = loaded_state();
        let _ = select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Stop));

        let again =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Restart)));
        let remove =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Remove)));

        assert!(again.is_empty());
        assert!(remove.is_empty());
        assert!(state.confirm().is_none());
        assert_eq!(state.running().len(), 1);
    }

    #[test]
    fn another_container_can_act_while_one_runs_an_action() {
        let mut state = loaded_state();
        let _ = select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Stop));
        let id = select(&mut state, "otty-fx-exit0");

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Start)));

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

        let asked =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Remove)));
        assert!(asked.is_empty());
        assert!(state.confirm().is_some());

        let cancelled =
            effects(reduce(&mut state, DockerIntent::ConfirmCancelled));

        assert!(cancelled.is_empty());
        assert!(state.confirm().is_none());
        assert!(state.running().is_empty());
    }

    #[test]
    fn removing_a_running_container_stops_it_first_by_default() {
        let mut state = loaded_state();
        let id = select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));
        assert_eq!(state.confirm().and_then(Confirm::switch), Some(true));

        let effects =
            effects(reduce(&mut state, DockerIntent::ConfirmAccepted));

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
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));

        let _ = reduce(&mut state, DockerIntent::ConfirmSwitchToggled(false));
        let effects =
            effects(reduce(&mut state, DockerIntent::ConfirmAccepted));

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
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));
        assert_eq!(state.confirm().and_then(Confirm::switch), None);

        let effects =
            effects(reduce(&mut state, DockerIntent::ConfirmAccepted));

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

        let asked = effects(reduce(&mut state, DockerIntent::Prune));
        // Created, exited 0 and exited 3.
        assert_eq!(state.confirm().map(Confirm::count), Some(3));
        let effects =
            effects(reduce(&mut state, DockerIntent::ConfirmAccepted));

        assert!(asked.is_empty());
        assert_eq!(effects, vec![run(EngineAction::PruneContainers, "")]);
    }

    #[test]
    fn a_second_prune_waits_for_the_first() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, DockerIntent::Prune);
        let _ = reduce(&mut state, DockerIntent::ConfirmAccepted);

        let _ = reduce(&mut state, DockerIntent::Prune);

        assert!(state.confirm().is_none());
    }

    #[test]
    fn a_confirmation_without_a_dialog_runs_nothing() {
        let mut state = loaded_state();

        let accepted =
            effects(reduce(&mut state, DockerIntent::ConfirmAccepted));
        let _ = reduce(&mut state, DockerIntent::ConfirmSwitchToggled(true));

        assert!(accepted.is_empty());
        assert!(state.confirm().is_none());
    }

    #[test]
    fn a_finished_action_reloads_the_engines_lists() {
        let mut state = loaded_state();
        let id = select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Stop));
        let request = ActionRequest::new(
            docker(),
            EngineAction::StopContainer(id),
            String::from("otty-fx-running"),
        );

        let effects = effects(reduce(
            &mut state,
            DockerIntent::ActionFinished {
                request,
                result: Ok(()),
            },
        ));

        assert_eq!(effects, vec![DockerEffect::LoadLists(docker())]);
        assert!(state.running().is_empty());
        assert!(state.notice().is_none());
    }

    #[test]
    fn a_failed_action_shows_a_notice_and_keeps_the_table() {
        let mut state = loaded_state();
        let id = select(&mut state, "otty-fx-running");
        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Stop));
        let request = ActionRequest::new(
            docker(),
            EngineAction::StopContainer(id),
            String::from("otty-fx-running"),
        );

        let effects = effects(reduce(
            &mut state,
            DockerIntent::ActionFinished {
                request: request.clone(),
                result: Err(DockerError::new("it is restarting")),
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
            DockerIntent::ActionFinished {
                request: ActionRequest::new(
                    docker(),
                    EngineAction::PruneContainers,
                    String::new(),
                ),
                result: Err(DockerError::new("busy")),
            },
        );

        let _ = reduce(&mut state, DockerIntent::DismissNotice);

        assert!(state.notice().is_none());
    }

    #[test]
    fn opening_another_list_hides_the_notice() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            DockerIntent::ActionFinished {
                request: ActionRequest::new(
                    docker(),
                    EngineAction::PruneContainers,
                    String::new(),
                ),
                result: Err(DockerError::new("busy")),
            },
        );

        let _ = reduce(
            &mut state,
            DockerIntent::Open(Place::new(
                docker().address().clone(),
                ResourceKind::Images,
            )),
        );

        assert!(state.notice().is_none());
    }

    #[test]
    fn a_finished_action_outside_docker_mode_does_not_reload() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, DockerIntent::Leave);

        let effects = effects(reduce(
            &mut state,
            DockerIntent::ActionFinished {
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

        let _ =
            reduce(&mut state, DockerIntent::OpenRowMenu { index, position });

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
            DockerIntent::OpenRowMenu {
                index,
                position: Point::ORIGIN,
            },
        );

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Start)));

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
            DockerIntent::OpenRowMenu {
                index: 0,
                position: Point::ORIGIN,
            },
        );

        let effects = effects(reduce(&mut state, DockerIntent::CloseRowMenu));

        assert!(effects.is_empty());
        assert_eq!(state.row_menu(), None);
    }

    #[test]
    fn a_right_press_past_the_last_row_opens_no_menu() {
        let mut state = loaded_state();

        let _ = reduce(
            &mut state,
            DockerIntent::OpenRowMenu {
                index: 99,
                position: Point::ORIGIN,
            },
        );

        assert_eq!(state.row_menu(), None);
    }

    /// Return the position of the container `name` in the shown table.
    fn shown_index(state: &DockerState, name: &str) -> usize {
        let index = fixtures::resources(EngineKind::Docker)
            .containers()
            .iter()
            .position(|container| container.name() == name)
            .expect("fixture container");

        state.shown_position(index)
    }

    /// Return the id of the selected row, the one the inspector shows.
    fn selected_id(state: &DockerState) -> Option<String> {
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
            DockerIntent::RowMenuPressed { index, position },
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
            DockerIntent::RowMenuPressed {
                index,
                position: Point::ORIGIN,
            },
        );

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Start)));

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
            DockerIntent::RowMenuPressed {
                index,
                position: Point::ORIGIN,
            },
        );

        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));

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
            DockerIntent::RowMenuPressed {
                index,
                position: Point::ORIGIN,
            },
        );
        let _ = reduce(&mut state, DockerIntent::CloseRowMenu);

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Stop)));

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
            DockerIntent::RowMenuPressed {
                index: 99,
                position: Point::ORIGIN,
            },
        );

        assert_eq!(state.row_menu(), None);
    }

    /// Replace the Docker engine's lists with the idle fixture lists: no
    /// container, so nothing uses an image, a volume or a network.
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

    /// Open `resource` and select the row named `name` (the first tag of
    /// an image).
    fn select_in(state: &mut DockerState, resource: ResourceKind, name: &str) {
        let _ = reduce(
            state,
            DockerIntent::Open(Place::new(
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
        let _ = reduce(state, DockerIntent::SelectRow(position));
    }

    fn image_id(name: &str) -> String {
        fixtures::resources(EngineKind::Docker)
            .images()
            .iter()
            .find(|image| image_name(image) == name)
            .map(|image| image.id().to_string())
            .expect("fixture image")
    }

    #[test]
    fn removing_an_image_a_container_uses_offers_force_on() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Images, "busybox:1.37");

        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));
        assert_eq!(state.confirm().and_then(Confirm::switch), Some(true));
        let effects =
            effects(reduce(&mut state, DockerIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::RemoveImage {
                    id: image_id("busybox:1.37"),
                    force: true,
                },
                "busybox:1.37"
            )]
        );
    }

    #[test]
    fn removing_an_unused_image_has_no_switch() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Images, "otty-fx-img:1");

        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));
        assert_eq!(state.confirm().and_then(Confirm::switch), None);
        let effects =
            effects(reduce(&mut state, DockerIntent::ConfirmAccepted));

        assert_eq!(
            effects,
            vec![run(
                EngineAction::RemoveImage {
                    id: image_id("otty-fx-img:1"),
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
            DockerIntent::Act(RowAction::PullAgain),
        ));

        assert!(effects.is_empty());
        let form = state.form().expect("the pull dialog");
        assert_eq!(form.kind(), FormKind::Pull);
        assert_eq!(form.value(FormField::Name), "busybox");
        assert_eq!(form.value(FormField::Tag), "1.37");
    }

    #[test]
    fn a_dangling_image_cannot_be_pulled_again() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Images, "<none>:<none>");

        let _ = reduce(&mut state, DockerIntent::Act(RowAction::PullAgain));

        assert!(state.form().is_none());
    }

    #[test]
    fn a_volume_a_container_uses_is_not_removed() {
        let mut state = loaded_state();
        select_in(&mut state, ResourceKind::Volumes, "otty-fx-data");

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Remove)));

        assert!(effects.is_empty());
        assert!(state.confirm().is_none());
    }

    #[test]
    fn an_unused_volume_is_removed_after_confirming() {
        let mut state = loaded_state();
        load_idle(&mut state);
        select_in(&mut state, ResourceKind::Volumes, "otty-fx-data");

        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));
        let effects =
            effects(reduce(&mut state, DockerIntent::ConfirmAccepted));

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

            let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));

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

        let _ = reduce(&mut state, DockerIntent::Act(RowAction::Remove));
        let effects =
            effects(reduce(&mut state, DockerIntent::ConfirmAccepted));

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
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Stop)));

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
                DockerIntent::Open(Place::new(
                    docker().address().clone(),
                    resource,
                )),
            );

            let _ = reduce(&mut state, DockerIntent::Prune);
            assert_eq!(
                state.confirm().map(Confirm::count),
                Some(count),
                "{resource:?}"
            );
            let effects =
                effects(reduce(&mut state, DockerIntent::ConfirmAccepted));

            assert_eq!(effects, vec![run(action, "")], "{resource:?}");
        }
    }

    fn open_volume_prune(state: &mut DockerState) {
        load_idle(state);
        let _ = reduce(
            state,
            DockerIntent::Open(Place::new(
                docker().address().clone(),
                ResourceKind::Volumes,
            )),
        );
        let _ = reduce(state, DockerIntent::Prune);
    }

    #[test]
    fn volume_prune_keeps_named_volumes_by_default() {
        let mut state = loaded_state();
        open_volume_prune(&mut state);

        assert_eq!(state.confirm().and_then(Confirm::switch), Some(false));
        assert_eq!(state.confirm().map(Confirm::count), Some(1));
        let effects =
            effects(reduce(&mut state, DockerIntent::ConfirmAccepted));

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

        let _ = reduce(&mut state, DockerIntent::ConfirmSwitchToggled(true));
        assert_eq!(state.confirm().map(Confirm::count), Some(2));
        let effects =
            effects(reduce(&mut state, DockerIntent::ConfirmAccepted));

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

        let _ = reduce(&mut state, DockerIntent::ConfirmSwitchToggled(true));
        let _ = reduce(&mut state, DockerIntent::ConfirmSwitchToggled(false));

        assert_eq!(state.confirm().map(Confirm::count), Some(1));
    }

    /// Podman mode active, the Podman fixture engine found and loaded, with
    /// the given commands found.
    fn podman_state(
        docker_cli: Option<&str>,
        podman_cli: Option<&str>,
    ) -> DockerState {
        let engine = fixtures::engine(EngineKind::Podman);
        let resources = fixtures::resources(EngineKind::Podman);
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery_with_cli(
                &[EngineKind::Podman],
                docker_cli,
                podman_cli,
            )),
        );
        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
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
        let _ = reduce(&mut state, DockerIntent::SelectRow(position));
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

    fn shell(program: &str, id: &str) -> DockerEffect {
        DockerEffect::OpenShell {
            title: String::from("shell otty-fx-running"),
            program: program.to_string(),
            args: ["exec", "-it", id, "sh"].map(String::from).to_vec(),
        }
    }

    #[test]
    fn shell_opens_docker_exec_with_the_command_found_in_an_install_folder() {
        let mut state = loaded_state();
        let id = select(&mut state, "otty-fx-running");

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Shell)));

        assert_eq!(effects, vec![shell(DOCKER_CLI, &id)]);
    }

    #[test]
    fn shell_on_a_podman_engine_opens_podman_exec() {
        let mut state = podman_state(None, Some("/opt/podman/bin/podman"));

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Shell)));

        assert_eq!(
            effects,
            vec![shell("/opt/podman/bin/podman", &podman_container_id())]
        );
    }

    #[test]
    fn shell_on_a_podman_engine_runs_the_podman_docker_wrapper() {
        // The wrapper is the only command found: it counts as Podman, so the
        // cli reports it as the Podman command and not as Docker.
        let mut state = podman_state(None, Some("/usr/bin/docker"));

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Shell)));

        assert_eq!(
            effects,
            vec![shell("/usr/bin/docker", &podman_container_id())]
        );
    }

    #[test]
    fn shell_on_a_podman_engine_ignores_a_real_docker_command() {
        let mut state = podman_state(Some(DOCKER_CLI), None);

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Shell)));

        assert!(effects.is_empty());
    }

    #[test]
    fn shell_without_the_command_does_nothing() {
        let mut state = podman_state(None, None);

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Shell)));

        assert!(effects.is_empty());
    }

    #[test]
    fn shell_on_a_stopped_container_does_nothing() {
        let mut state = loaded_state();
        let _ = select(&mut state, "otty-fx-exit0");

        let effects =
            effects(reduce(&mut state, DockerIntent::Act(RowAction::Shell)));

        assert!(effects.is_empty());
    }
}
