//! How an action goes, as the reducer runs it: a press on a button or a
//! menu item, the confirmation of a destructive action, the engine call and
//! its result (spec Decision 11).

use iced::{Point, Task};
use otty_docker::{Container, ContainerState, EngineInfo};

use super::actions::{
    ActionRequest, Confirm, EngineAction, RowAction, container_actions,
    failure_text, needs_stop,
};
use super::event::{DockerEffect, DockerEvent};
use super::state::DockerState;
use super::types::ResourceKind;

/// The selected container of the open list, copied out of the state so the
/// state can change.
struct SelectedContainer {
    engine: EngineInfo,
    id: String,
    name: String,
    state: ContainerState,
}

/// Run `action` on the selected row: a direct action asks the engine, a
/// Remove asks the user first. An action the row does not offer, or one on
/// a row that already runs an action, does nothing. Closes the row menu.
pub(super) fn act(
    state: &mut DockerState,
    action: RowAction,
) -> Task<DockerEvent> {
    state.close_row_menu();

    let Some(container) = selected_container(state) else {
        return Task::none();
    };
    let is_offered = container_actions(container.state).contains(&action);
    let is_busy = state.is_busy(
        container.engine.address(),
        ResourceKind::Containers,
        Some(&container.id),
    );
    if !is_offered || is_busy {
        return Task::none();
    }

    let call = match action {
        RowAction::Start => EngineAction::StartContainer(container.id),
        RowAction::Stop => EngineAction::StopContainer(container.id),
        RowAction::Restart => EngineAction::RestartContainer(container.id),
        RowAction::Pause => EngineAction::PauseContainer(container.id),
        RowAction::Unpause => EngineAction::UnpauseContainer(container.id),
        RowAction::Remove => {
            let remove = EngineAction::RemoveContainer {
                id: container.id,
                force: false,
            };
            let request =
                ActionRequest::new(container.engine, remove, container.name);
            let switch = needs_stop(container.state).then_some(true);
            state.open_confirm(Confirm::new(request, switch, 0));
            return Task::none();
        },
    };

    run(
        state,
        ActionRequest::new(container.engine, call, container.name),
    )
}

/// Select the row at `index` of the shown table and open its menu at
/// `position`. Nothing opens past the last row.
pub(super) fn open_row_menu(
    state: &mut DockerState,
    index: usize,
    position: Point,
) -> Task<DockerEvent> {
    if state.select_shown_row(index) {
        state.open_row_menu(position);
    }

    Task::none()
}

/// Ask to prune the open list. Containers: every stopped one (created,
/// exited or dead). Nothing opens while a prune of that list runs.
pub(super) fn prune(state: &mut DockerState) -> Task<DockerEvent> {
    let (Some(place), Some(engine)) = (state.place(), state.open_engine())
    else {
        return Task::none();
    };
    if place.resource() != ResourceKind::Containers
        || state.is_busy(place.engine(), ResourceKind::Containers, None)
    {
        return Task::none();
    }

    let stopped = engine
        .containers()
        .items()
        .iter()
        .filter(|container| is_stopped(container))
        .count();
    let request = ActionRequest::new(
        engine.info().clone(),
        EngineAction::PruneContainers,
        String::new(),
    );

    state.open_confirm(Confirm::new(request, None, stopped));
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
    result: Result<(), String>,
) -> Task<DockerEvent> {
    state.finish_action(request);

    if let Err(message) = result {
        state.set_notice(failure_text(request, &message));
        return Task::none();
    }

    let address = request.engine().address();
    let Some(engine) = state.engine(address) else {
        return Task::none();
    };
    if !state.is_active() {
        return Task::none();
    }

    effect(DockerEffect::LoadLists(engine.info().clone()))
}

/// Remember the running request and ask the app to run it.
fn run(state: &mut DockerState, request: ActionRequest) -> Task<DockerEvent> {
    state.start_action(request.clone());

    effect(DockerEffect::RunAction(request))
}

/// The selected container of the open containers list.
fn selected_container(state: &DockerState) -> Option<SelectedContainer> {
    let place = state.place()?;
    if place.resource() != ResourceKind::Containers {
        return None;
    }
    let engine = state.open_engine()?;
    let container = engine.containers().selected(Container::id)?;

    Some(SelectedContainer {
        engine: engine.info().clone(),
        id: container.id().to_string(),
        name: container.name().to_string(),
        state: container.state(),
    })
}

/// Whether a container prune removes `container`.
fn is_stopped(container: &Container) -> bool {
    matches!(
        container.state(),
        ContainerState::Created | ContainerState::Exited | ContainerState::Dead
    )
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
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::state::DockerState;
    use crate::widgets::docker::types::{LoadedLists, Place, ResourceKind};

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

        let _ = reduce(state, DockerIntent::SelectRow(index));
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
                result: Err(String::from("it is restarting")),
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
                result: Err(String::from("busy")),
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
                result: Err(String::from("busy")),
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

        let _ = reduce(
            &mut state,
            DockerIntent::OpenRowMenu { index: 2, position },
        );

        let engine = state.open_engine().expect("docker engine");
        let selected = engine.containers().selected(Container::id);
        assert_eq!(selected.map(Container::name), Some("otty-fx-exit3"));
        assert_eq!(state.row_menu(), Some(position));
    }

    #[test]
    fn a_menu_item_closes_the_menu_and_runs_its_action() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            DockerIntent::OpenRowMenu {
                index: 2,
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
}
