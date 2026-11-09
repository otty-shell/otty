#![forbid(clippy::disallowed_methods)]

use iced::Task;
use otty_containers::EngineAddress;

use super::super::types::{ActionRequest, ContainersError};
use super::event::{CreateEffect, CreateIntent};
use super::state::{CreateState, Form, FormKind};
use super::types::FormField;
use crate::i18n::{self, Key};

/// What a create dialog asks of the parent widget, which acts on it in the
/// same update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CreateOutcome {
    /// A volume or network dialog was sent: run this engine call the way
    /// every action runs, so its list stays busy until the call ends.
    ActionRequested(ActionRequest),
    /// The pull finished: reload the lists of this engine.
    Pulled { engine: EngineAddress },
    /// The pull failed: tell the user; carries the text in their language.
    Failed(String),
}

/// Reduce an intent of the create dialogs (spec Decision 11): an open intent
/// opens a dialog, typing fills it, the action button pulls the image or
/// hands the volume or network call to the parent, and the pull reports its
/// progress until it is done or cancelled.
pub(crate) fn reduce(
    state: &mut CreateState,
    intent: CreateIntent,
) -> (Task<CreateEffect>, Option<CreateOutcome>) {
    match intent {
        CreateIntent::OpenPull { engine, image, tag } => {
            state.open_form(Form::pull(engine, &image, &tag));
            (Task::none(), None)
        },
        CreateIntent::OpenVolume(engine) => {
            state.open_form(Form::create_volume(engine));
            (Task::none(), None)
        },
        CreateIntent::OpenNetwork(engine) => {
            state.open_form(Form::create_network(engine));
            (Task::none(), None)
        },
        CreateIntent::Changed(field, value) => {
            if let Some(form) = state.form_mut() {
                form.set_value(field, value);
            }
            (Task::none(), None)
        },
        CreateIntent::Submitted => submit(state),
        CreateIntent::Cancelled => (cancel(state), None),
        CreateIntent::PullProgressed(progress) => {
            if let Some(form) = state.form_mut() {
                form.set_progress(progress);
            }
            (Task::none(), None)
        },
        CreateIntent::PullFinished(result) => {
            (Task::none(), finish_pull(state, result))
        },
    }
}

/// Press the action button: a pull starts and the dialog stays open with
/// its progress; a create closes the dialog and asks the parent to run its
/// call. Nothing happens while the form cannot be sent.
fn submit(
    state: &mut CreateState,
) -> (Task<CreateEffect>, Option<CreateOutcome>) {
    let Some(form) = state.form_mut() else {
        return (Task::none(), None);
    };
    if !form.can_submit() {
        return (Task::none(), None);
    }

    if form.kind() == FormKind::Pull {
        let pull = CreateEffect::StartPull {
            engine: form.engine().clone(),
            image: form.image().to_string(),
            tag: form.tag().to_string(),
        };
        form.start_pull();
        return (Task::done(pull), None);
    }

    let Some(form) = state.take_form() else {
        return (Task::none(), None);
    };
    let Some(action) = form.create_action() else {
        return (Task::none(), None);
    };
    let name = form.value(FormField::Name).trim().to_string();

    let request = ActionRequest::new(form.engine().clone(), action, name);
    (Task::none(), Some(CreateOutcome::ActionRequested(request)))
}

/// Close the dialog; a running pull is stopped.
fn cancel(state: &mut CreateState) -> Task<CreateEffect> {
    let Some(form) = state.take_form() else {
        return Task::none();
    };

    if form.is_pulling() {
        Task::done(CreateEffect::CancelPull)
    } else {
        Task::none()
    }
}

/// Take the end of the running pull: the dialog closes; success asks the
/// parent to reload the engine's lists, a failure to tell the user. A result
/// without a running pull (cancelled meanwhile) is dropped.
fn finish_pull(
    state: &mut CreateState,
    result: Result<(), ContainersError>,
) -> Option<CreateOutcome> {
    if !state.form().is_some_and(Form::is_pulling) {
        return None;
    }
    let form = state.take_form()?;

    let outcome = match result {
        Ok(()) => CreateOutcome::Pulled {
            engine: form.engine().address().clone(),
        },
        Err(err) => CreateOutcome::Failed(i18n::containers_action_failed(
            Key::TplContainersPullFailed,
            &form.reference(),
            &format!("{err}"),
        )),
    };
    Some(outcome)
}

#[cfg(test)]
mod tests {
    use otty_containers::{EngineInfo, EngineKind, fixtures};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::containers::actions::event::ActionsEffect;
    use crate::widgets::containers::event::{
        ContainersEffect, ContainersEvent, ContainersIntent,
    };
    use crate::widgets::containers::reducer::reduce as reduce_containers;
    use crate::widgets::containers::state::ContainersState;
    use crate::widgets::containers::types::{
        DEFAULT_TAG, EngineAction, LoadedLists, Place, ResourceKind,
    };

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    /// Reduce `intent` in the part alone; the effects it asks for and what
    /// it tells the parent.
    fn reduce_part(
        state: &mut CreateState,
        intent: CreateIntent,
    ) -> (Vec<CreateEffect>, Option<CreateOutcome>) {
        let (effects, outcome) = reduce(state, intent);

        (collect_outputs(effects), outcome)
    }

    /// The pull dialog of `nginx:latest` on Docker, with the pull running.
    fn pulling_part() -> CreateState {
        let mut state = CreateState::default();
        let _ = reduce_part(
            &mut state,
            CreateIntent::OpenPull {
                engine: docker(),
                image: String::from("nginx"),
                tag: String::new(),
            },
        );
        let _ = reduce_part(&mut state, CreateIntent::Submitted);
        state
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

    /// Send `intent` to the create dialogs through the widget's reducer.
    fn send(
        state: &mut ContainersState,
        intent: CreateIntent,
    ) -> Task<ContainersEvent> {
        reduce_containers(state, ContainersIntent::Create(intent))
    }

    /// Containers mode active with the Docker fixture loaded and `resource`
    /// open.
    fn state_on(resource: ResourceKind) -> ContainersState {
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = ContainersState::default();
        let _ = reduce_containers(&mut state, ContainersIntent::Enter);
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
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
        let _ = reduce_containers(
            &mut state,
            ContainersIntent::Open(Place::new(
                docker().address().clone(),
                resource,
            )),
        );
        state
    }

    fn type_in(state: &mut ContainersState, field: FormField, value: &str) {
        let _ = send(state, CreateIntent::Changed(field, value.into()));
    }

    /// The images list with a pull of `nginx:latest` running.
    fn pulling() -> ContainersState {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "nginx");
        let _ = send(&mut state, CreateIntent::Submitted);
        state
    }

    #[test]
    fn open_pull_fills_the_image_and_the_tag() {
        let mut tagged = CreateState::default();
        let mut untagged = CreateState::default();

        let (effects, outcome) = reduce_part(
            &mut tagged,
            CreateIntent::OpenPull {
                engine: docker(),
                image: String::from("busybox"),
                tag: String::from("1.37"),
            },
        );
        let _ = reduce_part(
            &mut untagged,
            CreateIntent::OpenPull {
                engine: docker(),
                image: String::from("busybox"),
                tag: String::new(),
            },
        );

        assert!(effects.is_empty());
        assert_eq!(outcome, None);
        let form = tagged.form().expect("the pull dialog");
        assert_eq!(form.kind(), FormKind::Pull);
        assert_eq!(form.engine(), &docker());
        assert_eq!(form.value(FormField::Name), "busybox");
        assert_eq!(form.value(FormField::Tag), "1.37");
        assert_eq!(
            untagged.form().map(|form| form.value(FormField::Tag)),
            Some(DEFAULT_TAG)
        );
    }

    #[test]
    fn a_sent_volume_dialog_closes_and_asks_the_parent_to_run_its_call() {
        let mut state = CreateState::default();
        let _ = reduce_part(&mut state, CreateIntent::OpenVolume(docker()));
        let _ = reduce_part(
            &mut state,
            CreateIntent::Changed(FormField::Name, String::from("otty-fx-vol")),
        );

        let (effects, outcome) =
            reduce_part(&mut state, CreateIntent::Submitted);

        assert!(effects.is_empty());
        assert_eq!(
            outcome,
            Some(CreateOutcome::ActionRequested(ActionRequest::new(
                docker(),
                EngineAction::CreateVolume {
                    name: String::from("otty-fx-vol"),
                    driver: String::from("local"),
                },
                String::from("otty-fx-vol"),
            )))
        );
        assert!(!state.is_open());
    }

    #[test]
    fn a_finished_pull_closes_the_dialog_and_names_the_engine_to_reload() {
        let mut state = pulling_part();

        let (effects, outcome) =
            reduce_part(&mut state, CreateIntent::PullFinished(Ok(())));

        assert!(effects.is_empty());
        assert_eq!(
            outcome,
            Some(CreateOutcome::Pulled {
                engine: docker().address().clone(),
            })
        );
        assert!(!state.is_open());
    }

    #[test]
    fn a_failed_pull_closes_the_dialog_and_tells_the_parent_why() {
        let mut state = pulling_part();

        let (effects, outcome) = reduce_part(
            &mut state,
            CreateIntent::PullFinished(Err(ContainersError::new(
                "manifest unknown",
            ))),
        );

        assert!(effects.is_empty());
        assert_eq!(
            outcome,
            Some(CreateOutcome::Failed(i18n::containers_action_failed(
                Key::TplContainersPullFailed,
                "nginx:latest",
                "manifest unknown"
            )))
        );
        assert!(!state.is_open());
    }

    #[test]
    fn typing_fills_the_open_form() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);

        type_in(&mut state, FormField::Name, "nginx");
        type_in(&mut state, FormField::Tag, "1.27-alpine");

        let form = state.create().form().expect("an open form");
        assert_eq!(form.value(FormField::Name), "nginx");
        assert_eq!(form.value(FormField::Tag), "1.27-alpine");
    }

    #[test]
    fn a_pull_without_a_name_does_not_start() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);

        let effects = effects(send(&mut state, CreateIntent::Submitted));

        assert!(effects.is_empty());
        assert!(!state.create().form().is_some_and(Form::is_pulling));
    }

    #[test]
    fn pull_starts_the_pull_and_keeps_the_dialog_open() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "nginx");

        let effects = effects(send(&mut state, CreateIntent::Submitted));

        assert_eq!(
            effects,
            vec![ContainersEffect::Create(CreateEffect::StartPull {
                engine: docker(),
                image: String::from("nginx"),
                tag: String::from(DEFAULT_TAG),
            })]
        );
        assert!(state.create().form().is_some_and(Form::is_pulling));
    }

    #[test]
    fn a_running_pull_takes_each_progress_and_ignores_a_second_submit() {
        let mut state = pulling();

        let _ = send(
            &mut state,
            CreateIntent::PullProgressed(fixtures::pull_progress()),
        );
        let again = effects(send(&mut state, CreateIntent::Submitted));

        assert!(again.is_empty());
        assert_eq!(
            state.create().form().and_then(Form::progress),
            Some(&fixtures::pull_progress())
        );
    }

    #[test]
    fn a_finished_pull_closes_the_dialog_and_reloads() {
        let mut state = pulling();

        let effects =
            effects(send(&mut state, CreateIntent::PullFinished(Ok(()))));

        assert!(!state.create().is_open());
        assert_eq!(effects, vec![ContainersEffect::LoadLists(docker())]);
    }

    #[test]
    fn a_failed_pull_closes_the_dialog_and_tells_the_user() {
        let mut state = pulling();

        let effects = effects(send(
            &mut state,
            CreateIntent::PullFinished(Err(ContainersError::new(
                "manifest unknown",
            ))),
        ));

        assert!(!state.create().is_open());
        assert_eq!(
            effects,
            vec![ContainersEffect::Failed(i18n::containers_action_failed(
                Key::TplContainersPullFailed,
                "nginx:latest",
                "manifest unknown"
            ))]
        );
    }

    #[test]
    fn cancel_during_a_pull_stops_it() {
        let mut state = pulling();

        let effects = effects(send(&mut state, CreateIntent::Cancelled));

        assert_eq!(
            effects,
            vec![ContainersEffect::Create(CreateEffect::CancelPull)]
        );
        assert!(!state.create().is_open());
    }

    #[test]
    fn cancel_before_a_pull_only_closes_the_dialog() {
        let mut state = state_on(ResourceKind::Volumes);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);

        let effects = effects(send(&mut state, CreateIntent::Cancelled));

        assert!(effects.is_empty());
        assert!(!state.create().is_open());
    }

    #[test]
    fn pull_results_without_a_running_pull_are_dropped() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);

        let _ = send(
            &mut state,
            CreateIntent::PullProgressed(fixtures::pull_progress()),
        );
        let finished =
            effects(send(&mut state, CreateIntent::PullFinished(Ok(()))));

        assert!(finished.is_empty());
        assert!(state.create().form().is_some_and(|form| !form.is_pulling()));
    }

    #[test]
    fn create_volume_runs_the_call_and_closes_the_dialog() {
        let mut state = state_on(ResourceKind::Volumes);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "otty-fx-vol");

        let effects = effects(send(&mut state, CreateIntent::Submitted));

        assert_eq!(
            effects,
            vec![ContainersEffect::Actions(ActionsEffect::RunAction(
                ActionRequest::new(
                    docker(),
                    EngineAction::CreateVolume {
                        name: String::from("otty-fx-vol"),
                        driver: String::from("local"),
                    },
                    String::from("otty-fx-vol"),
                )
            ))]
        );
        assert!(!state.create().is_open());
    }

    #[test]
    fn create_network_without_a_name_does_nothing() {
        let mut state = state_on(ResourceKind::Networks);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);

        let effects = effects(send(&mut state, CreateIntent::Submitted));

        assert!(effects.is_empty());
        assert!(state.create().is_open());
    }

    #[test]
    fn create_network_runs_the_call_with_the_typed_driver() {
        let mut state = state_on(ResourceKind::Networks);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "otty-fx-net2");
        type_in(&mut state, FormField::Driver, "macvlan");

        let effects = effects(send(&mut state, CreateIntent::Submitted));

        assert_eq!(
            effects,
            vec![ContainersEffect::Actions(ActionsEffect::RunAction(
                ActionRequest::new(
                    docker(),
                    EngineAction::CreateNetwork {
                        name: String::from("otty-fx-net2"),
                        driver: String::from("macvlan"),
                    },
                    String::from("otty-fx-net2"),
                )
            ))]
        );
    }

    #[test]
    fn a_submit_without_a_form_does_nothing() {
        let mut state = state_on(ResourceKind::Images);

        let effects = effects(send(&mut state, CreateIntent::Submitted));
        type_in(&mut state, FormField::Name, "nginx");

        assert!(effects.is_empty());
        assert!(!state.create().is_open());
    }
}
