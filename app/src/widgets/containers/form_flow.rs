//! How a create dialog goes, as the reducer runs it: the create button
//! opens it, typing fills it, the action button pulls the image or creates
//! the volume or network, and the pull reports its progress until it is
//! done or cancelled (spec Decision 11).

use iced::Task;

use super::action_flow::{reload, run};
use super::actions::ActionRequest;
use super::event::{ContainersEffect, ContainersEvent};
use super::forms::{Form, FormField, FormKind};
use super::run_flow;
use super::state::ContainersState;
use super::types::{ContainersError, ResourceKind};
use crate::i18n::{self, Key};

/// Open the create dialog of the open list: Pull image, Create volume or
/// Create network; the containers list opens the Run container form.
pub(super) fn open(state: &mut ContainersState) -> Task<ContainersEvent> {
    let (Some(place), Some(engine)) = (state.place(), state.open_engine())
    else {
        return Task::none();
    };
    let info = engine.info().clone();

    let form = match place.resource() {
        ResourceKind::Images => Form::pull(info, "", ""),
        ResourceKind::Volumes => Form::create_volume(info),
        ResourceKind::Networks => Form::create_network(info),
        ResourceKind::Containers => return run_flow::open(state, info, ""),
    };

    state.open_form(form);
    Task::none()
}

/// Press the action button: a pull starts and the dialog stays open with
/// its progress; a create runs and the dialog closes. Nothing happens
/// while the form cannot be sent.
pub(super) fn submit(state: &mut ContainersState) -> Task<ContainersEvent> {
    let Some(form) = state.form_mut() else {
        return Task::none();
    };
    if !form.can_submit() {
        return Task::none();
    }

    if form.kind() == FormKind::Pull {
        let pull = ContainersEffect::StartPull {
            engine: form.engine().clone(),
            image: form.image().to_string(),
            tag: form.tag().to_string(),
        };
        form.start_pull();
        return Task::done(ContainersEvent::Effect(pull));
    }

    let Some(form) = state.take_form() else {
        return Task::none();
    };
    let Some(action) = form.create_action() else {
        return Task::none();
    };
    let name = form.value(FormField::Name).trim().to_string();

    run(
        state,
        ActionRequest::new(form.engine().clone(), action, name),
    )
}

/// Close the dialog; a running pull is stopped.
pub(super) fn cancel(state: &mut ContainersState) -> Task<ContainersEvent> {
    let Some(form) = state.take_form() else {
        return Task::none();
    };

    if form.is_pulling() {
        Task::done(ContainersEvent::Effect(ContainersEffect::CancelPull))
    } else {
        Task::none()
    }
}

/// Take the end of the running pull: the dialog closes; success reloads
/// the engine's lists, a failure shows a notice. A result without a
/// running pull (cancelled meanwhile) is dropped.
pub(super) fn finish_pull(
    state: &mut ContainersState,
    result: Result<(), ContainersError>,
) -> Task<ContainersEvent> {
    if !state.form().is_some_and(Form::is_pulling) {
        return Task::none();
    }
    let Some(form) = state.take_form() else {
        return Task::none();
    };

    match result {
        Ok(()) => reload(state, form.engine().address()),
        Err(err) => {
            state.set_notice(i18n::containers_action_failed(
                Key::TplContainersPullFailed,
                &form.reference(),
                &err.to_string(),
            ));
            Task::none()
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_containers::{EngineInfo, EngineKind, fixtures};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::containers::actions::{ActionRequest, EngineAction};
    use crate::widgets::containers::event::{
        ContainersEffect, ContainersEvent, ContainersIntent,
    };
    use crate::widgets::containers::forms::{DEFAULT_TAG, FormField};
    use crate::widgets::containers::reducer::reduce;
    use crate::widgets::containers::types::{LoadedLists, Place, ResourceKind};

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

    /// Containers mode active with the Docker fixture loaded and `resource`
    /// open.
    fn state_on(resource: ResourceKind) -> ContainersState {
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
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
        let _ = reduce(
            &mut state,
            ContainersIntent::Open(Place::new(
                docker().address().clone(),
                resource,
            )),
        );
        state
    }

    fn type_in(state: &mut ContainersState, field: FormField, value: &str) {
        let _ =
            reduce(state, ContainersIntent::FormChanged(field, value.into()));
    }

    /// The images list with a pull of `nginx:latest` running.
    fn pulling() -> ContainersState {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "nginx");
        let _ = reduce(&mut state, ContainersIntent::FormSubmitted);
        state
    }

    #[test]
    fn the_create_button_opens_the_dialog_of_each_list() {
        let cases = [
            (ResourceKind::Images, FormKind::Pull),
            (ResourceKind::Volumes, FormKind::CreateVolume),
            (ResourceKind::Networks, FormKind::CreateNetwork),
        ];

        for (resource, kind) in cases {
            let mut state = state_on(resource);

            let effects =
                effects(reduce(&mut state, ContainersIntent::OpenCreate));

            assert!(effects.is_empty());
            assert_eq!(state.form().map(Form::kind), Some(kind));
        }
    }

    #[test]
    fn the_containers_create_button_opens_the_run_form() {
        let mut state = state_on(ResourceKind::Containers);

        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        assert!(state.form().is_none());
        assert!(state.run_form().is_some());
    }

    #[test]
    fn typing_fills_the_open_form() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        type_in(&mut state, FormField::Name, "nginx");
        type_in(&mut state, FormField::Tag, "1.27-alpine");

        let form = state.form().expect("an open form");
        assert_eq!(form.value(FormField::Name), "nginx");
        assert_eq!(form.value(FormField::Tag), "1.27-alpine");
    }

    #[test]
    fn a_pull_without_a_name_does_not_start() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        let effects =
            effects(reduce(&mut state, ContainersIntent::FormSubmitted));

        assert!(effects.is_empty());
        assert!(!state.form().is_some_and(Form::is_pulling));
    }

    #[test]
    fn pull_starts_the_pull_and_keeps_the_dialog_open() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "nginx");

        let effects =
            effects(reduce(&mut state, ContainersIntent::FormSubmitted));

        assert_eq!(
            effects,
            vec![ContainersEffect::StartPull {
                engine: docker(),
                image: String::from("nginx"),
                tag: String::from(DEFAULT_TAG),
            }]
        );
        assert!(state.form().is_some_and(Form::is_pulling));
    }

    #[test]
    fn a_running_pull_takes_each_progress_and_ignores_a_second_submit() {
        let mut state = pulling();

        let _ = reduce(
            &mut state,
            ContainersIntent::PullProgressed(fixtures::pull_progress()),
        );
        let again =
            effects(reduce(&mut state, ContainersIntent::FormSubmitted));

        assert!(again.is_empty());
        assert_eq!(
            state.form().and_then(Form::progress),
            Some(&fixtures::pull_progress())
        );
    }

    #[test]
    fn a_finished_pull_closes_the_dialog_and_reloads() {
        let mut state = pulling();

        let effects =
            effects(reduce(&mut state, ContainersIntent::PullFinished(Ok(()))));

        assert!(state.form().is_none());
        assert_eq!(effects, vec![ContainersEffect::LoadLists(docker())]);
        assert!(state.notice().is_none());
    }

    #[test]
    fn a_failed_pull_closes_the_dialog_and_shows_a_notice() {
        let mut state = pulling();

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::PullFinished(Err(ContainersError::new(
                "manifest unknown",
            ))),
        ));

        assert!(effects.is_empty());
        assert!(state.form().is_none());
        assert_eq!(
            state.notice().map(str::to_string),
            Some(i18n::containers_action_failed(
                Key::TplContainersPullFailed,
                "nginx:latest",
                "manifest unknown"
            ))
        );
    }

    #[test]
    fn cancel_during_a_pull_stops_it() {
        let mut state = pulling();

        let effects =
            effects(reduce(&mut state, ContainersIntent::FormCancelled));

        assert_eq!(effects, vec![ContainersEffect::CancelPull]);
        assert!(state.form().is_none());
    }

    #[test]
    fn cancel_before_a_pull_only_closes_the_dialog() {
        let mut state = state_on(ResourceKind::Volumes);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        let effects =
            effects(reduce(&mut state, ContainersIntent::FormCancelled));

        assert!(effects.is_empty());
        assert!(state.form().is_none());
    }

    #[test]
    fn pull_results_without_a_running_pull_are_dropped() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        let _ = reduce(
            &mut state,
            ContainersIntent::PullProgressed(fixtures::pull_progress()),
        );
        let finished =
            effects(reduce(&mut state, ContainersIntent::PullFinished(Ok(()))));

        assert!(finished.is_empty());
        assert!(state.form().is_some_and(|form| !form.is_pulling()));
    }

    #[test]
    fn create_volume_runs_the_call_and_closes_the_dialog() {
        let mut state = state_on(ResourceKind::Volumes);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "otty-fx-vol");

        let effects =
            effects(reduce(&mut state, ContainersIntent::FormSubmitted));

        assert_eq!(
            effects,
            vec![ContainersEffect::RunAction(ActionRequest::new(
                docker(),
                EngineAction::CreateVolume {
                    name: String::from("otty-fx-vol"),
                    driver: String::from("local"),
                },
                String::from("otty-fx-vol"),
            ))]
        );
        assert!(state.form().is_none());
    }

    #[test]
    fn create_network_without_a_name_does_nothing() {
        let mut state = state_on(ResourceKind::Networks);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        let effects =
            effects(reduce(&mut state, ContainersIntent::FormSubmitted));

        assert!(effects.is_empty());
        assert!(state.form().is_some());
    }

    #[test]
    fn create_network_runs_the_call_with_the_typed_driver() {
        let mut state = state_on(ResourceKind::Networks);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "otty-fx-net2");
        type_in(&mut state, FormField::Driver, "macvlan");

        let effects =
            effects(reduce(&mut state, ContainersIntent::FormSubmitted));

        assert_eq!(
            effects,
            vec![ContainersEffect::RunAction(ActionRequest::new(
                docker(),
                EngineAction::CreateNetwork {
                    name: String::from("otty-fx-net2"),
                    driver: String::from("macvlan"),
                },
                String::from("otty-fx-net2"),
            ))]
        );
    }

    #[test]
    fn a_submit_without_a_form_does_nothing() {
        let mut state = state_on(ResourceKind::Images);

        let effects =
            effects(reduce(&mut state, ContainersIntent::FormSubmitted));
        type_in(&mut state, FormField::Name, "nginx");

        assert!(effects.is_empty());
        assert!(state.form().is_none());
    }
}
