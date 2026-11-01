//! How a create dialog goes, as the reducer runs it: the create button
//! opens it, typing fills it, the action button pulls the image or creates
//! the volume or network, and the pull reports its progress until it is
//! done or cancelled (spec Decision 11).

use iced::Task;

use super::action_flow::{reload, run};
use super::actions::ActionRequest;
use super::event::{DockerEffect, DockerEvent};
use super::forms::{Form, FormField, FormKind};
use super::state::DockerState;
use super::types::ResourceKind;
use crate::i18n::{self, Key};

/// Open the create dialog of the open list: Pull image, Create volume or
/// Create network. The containers list gets its Run form with ticket 14.
pub(super) fn open(state: &mut DockerState) -> Task<DockerEvent> {
    let (Some(place), Some(engine)) = (state.place(), state.open_engine())
    else {
        return Task::none();
    };
    let info = engine.info().clone();

    let form = match place.resource() {
        ResourceKind::Images => Form::pull(info, "", ""),
        ResourceKind::Volumes => Form::create_volume(info),
        ResourceKind::Networks => Form::create_network(info),
        ResourceKind::Containers => return Task::none(),
    };

    state.open_form(form);
    Task::none()
}

/// Press the action button: a pull starts and the dialog stays open with
/// its progress; a create runs and the dialog closes. Nothing happens
/// while the form cannot be sent.
pub(super) fn submit(state: &mut DockerState) -> Task<DockerEvent> {
    let Some(form) = state.form_mut() else {
        return Task::none();
    };
    if !form.can_submit() {
        return Task::none();
    }

    if form.kind() == FormKind::Pull {
        let pull = DockerEffect::StartPull {
            engine: form.engine().clone(),
            image: form.image().to_string(),
            tag: form.tag().to_string(),
        };
        form.start_pull();
        return Task::done(DockerEvent::Effect(pull));
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
pub(super) fn cancel(state: &mut DockerState) -> Task<DockerEvent> {
    let Some(form) = state.take_form() else {
        return Task::none();
    };

    if form.is_pulling() {
        Task::done(DockerEvent::Effect(DockerEffect::CancelPull))
    } else {
        Task::none()
    }
}

/// Take the end of the running pull: the dialog closes; success reloads
/// the engine's lists, a failure shows a notice. A result without a
/// running pull (cancelled meanwhile) is dropped.
pub(super) fn finish_pull(
    state: &mut DockerState,
    result: Result<(), String>,
) -> Task<DockerEvent> {
    if !state.form().is_some_and(Form::is_pulling) {
        return Task::none();
    }
    let Some(form) = state.take_form() else {
        return Task::none();
    };

    match result {
        Ok(()) => reload(state, form.engine().address()),
        Err(message) => {
            state.set_notice(i18n::docker_action_failed(
                Key::TplDockerPullFailed,
                &form.reference(),
                &message,
            ));
            Task::none()
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineInfo, EngineKind, fixtures};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::docker::actions::{ActionRequest, EngineAction};
    use crate::widgets::docker::event::{
        DockerEffect, DockerEvent, DockerIntent,
    };
    use crate::widgets::docker::forms::{DEFAULT_TAG, FormField};
    use crate::widgets::docker::reducer::reduce;
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

    /// Docker mode active with the Docker fixture loaded and `resource`
    /// open.
    fn state_on(resource: ResourceKind) -> DockerState {
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
        let _ = reduce(
            &mut state,
            DockerIntent::Open(Place::new(
                docker().address().clone(),
                resource,
            )),
        );
        state
    }

    fn type_in(state: &mut DockerState, field: FormField, value: &str) {
        let _ = reduce(state, DockerIntent::FormChanged(field, value.into()));
    }

    /// The images list with a pull of `nginx:latest` running.
    fn pulling() -> DockerState {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "nginx");
        let _ = reduce(&mut state, DockerIntent::FormSubmitted);
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

            let effects = effects(reduce(&mut state, DockerIntent::OpenCreate));

            assert!(effects.is_empty());
            assert_eq!(state.form().map(Form::kind), Some(kind));
        }
    }

    #[test]
    fn the_containers_create_button_waits_for_the_run_form() {
        let mut state = state_on(ResourceKind::Containers);

        let _ = reduce(&mut state, DockerIntent::OpenCreate);

        assert!(state.form().is_none());
    }

    #[test]
    fn typing_fills_the_open_form() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);

        type_in(&mut state, FormField::Name, "nginx");
        type_in(&mut state, FormField::Tag, "1.27-alpine");

        let form = state.form().expect("an open form");
        assert_eq!(form.value(FormField::Name), "nginx");
        assert_eq!(form.value(FormField::Tag), "1.27-alpine");
    }

    #[test]
    fn a_pull_without_a_name_does_not_start() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);

        let effects = effects(reduce(&mut state, DockerIntent::FormSubmitted));

        assert!(effects.is_empty());
        assert!(!state.form().is_some_and(Form::is_pulling));
    }

    #[test]
    fn pull_starts_the_pull_and_keeps_the_dialog_open() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "nginx");

        let effects = effects(reduce(&mut state, DockerIntent::FormSubmitted));

        assert_eq!(
            effects,
            vec![DockerEffect::StartPull {
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
            DockerIntent::PullProgressed(fixtures::pull_progress()),
        );
        let again = effects(reduce(&mut state, DockerIntent::FormSubmitted));

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
            effects(reduce(&mut state, DockerIntent::PullFinished(Ok(()))));

        assert!(state.form().is_none());
        assert_eq!(effects, vec![DockerEffect::LoadLists(docker())]);
        assert!(state.notice().is_none());
    }

    #[test]
    fn a_failed_pull_closes_the_dialog_and_shows_a_notice() {
        let mut state = pulling();

        let effects = effects(reduce(
            &mut state,
            DockerIntent::PullFinished(Err(String::from("manifest unknown"))),
        ));

        assert!(effects.is_empty());
        assert!(state.form().is_none());
        assert_eq!(
            state.notice().map(str::to_string),
            Some(i18n::docker_action_failed(
                Key::TplDockerPullFailed,
                "nginx:latest",
                "manifest unknown"
            ))
        );
    }

    #[test]
    fn cancel_during_a_pull_stops_it() {
        let mut state = pulling();

        let effects = effects(reduce(&mut state, DockerIntent::FormCancelled));

        assert_eq!(effects, vec![DockerEffect::CancelPull]);
        assert!(state.form().is_none());
    }

    #[test]
    fn cancel_before_a_pull_only_closes_the_dialog() {
        let mut state = state_on(ResourceKind::Volumes);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);

        let effects = effects(reduce(&mut state, DockerIntent::FormCancelled));

        assert!(effects.is_empty());
        assert!(state.form().is_none());
    }

    #[test]
    fn pull_results_without_a_running_pull_are_dropped() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);

        let _ = reduce(
            &mut state,
            DockerIntent::PullProgressed(fixtures::pull_progress()),
        );
        let finished =
            effects(reduce(&mut state, DockerIntent::PullFinished(Ok(()))));

        assert!(finished.is_empty());
        assert!(state.form().is_some_and(|form| !form.is_pulling()));
    }

    #[test]
    fn create_volume_runs_the_call_and_closes_the_dialog() {
        let mut state = state_on(ResourceKind::Volumes);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "otty-fx-vol");

        let effects = effects(reduce(&mut state, DockerIntent::FormSubmitted));

        assert_eq!(
            effects,
            vec![DockerEffect::RunAction(ActionRequest::new(
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
        let _ = reduce(&mut state, DockerIntent::OpenCreate);

        let effects = effects(reduce(&mut state, DockerIntent::FormSubmitted));

        assert!(effects.is_empty());
        assert!(state.form().is_some());
    }

    #[test]
    fn create_network_runs_the_call_with_the_typed_driver() {
        let mut state = state_on(ResourceKind::Networks);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "otty-fx-net2");
        type_in(&mut state, FormField::Driver, "macvlan");

        let effects = effects(reduce(&mut state, DockerIntent::FormSubmitted));

        assert_eq!(
            effects,
            vec![DockerEffect::RunAction(ActionRequest::new(
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

        let effects = effects(reduce(&mut state, DockerIntent::FormSubmitted));
        type_in(&mut state, FormField::Name, "nginx");

        assert!(effects.is_empty());
        assert!(state.form().is_none());
    }
}
