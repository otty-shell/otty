#![forbid(clippy::disallowed_methods)]

use otty_app_i18n::{self as i18n, Key};

use super::event::{CreateIntent, CreateOutcome};
use super::state::{CreateState, Form, FormKind};
use super::types::FormField;
use crate::types::{ActionRequest, ContainersError, PullHandle};

/// Reduce an intent of the create dialogs (spec Decision 11): an open intent
/// opens a dialog, typing fills it, the action button pulls the image or
/// hands the volume or network call to the parent, and the pull reports its
/// progress until it is done or cancelled.
pub(crate) fn reduce(
    state: &mut CreateState,
    intent: CreateIntent,
) -> Vec<CreateOutcome> {
    match intent {
        CreateIntent::OpenPull { engine, image, tag } => {
            state.open_form(Form::pull(engine, &image, &tag));
            Vec::new()
        },
        CreateIntent::OpenVolume(engine) => {
            state.open_form(Form::create_volume(engine));
            Vec::new()
        },
        CreateIntent::OpenNetwork(engine) => {
            state.open_form(Form::create_network(engine));
            Vec::new()
        },
        CreateIntent::Changed(field, value) => {
            if let Some(form) = state.form_mut() {
                form.set_value(field, value);
            }
            Vec::new()
        },
        CreateIntent::Submitted => submit(state),
        CreateIntent::Cancelled => cancel(state),
        CreateIntent::PullStarted(pull) => {
            keep_pull(state, pull);
            Vec::new()
        },
        CreateIntent::PullProgressed(progress) => {
            if let Some(form) = state.form_mut() {
                form.set_progress(progress);
            }
            Vec::new()
        },
        CreateIntent::PullFinished(result) => finish_pull(state, result),
    }
}

/// Press the action button: a pull starts and the dialog stays open with
/// its progress; a create closes the dialog and asks the parent to run its
/// call. Nothing happens while the form cannot be sent.
fn submit(state: &mut CreateState) -> Vec<CreateOutcome> {
    let Some(form) = state.form_mut() else {
        return Vec::new();
    };
    if !form.can_submit() {
        return Vec::new();
    }

    if form.kind() == FormKind::Pull {
        let pull = CreateOutcome::StartPull {
            engine: form.engine().clone(),
            image: form.image().to_string(),
            tag: form.tag().to_string(),
        };
        form.start_pull();
        return vec![pull];
    }

    let Some(form) = state.take_form() else {
        return Vec::new();
    };
    let Some(action) = form.create_action() else {
        return Vec::new();
    };
    let name = form.value(FormField::Name).trim().to_string();

    let request = ActionRequest::new(form.engine().clone(), action, name);
    vec![CreateOutcome::ActionRequested(request)]
}

/// Close the dialog; a running pull is stopped through its abort handle. A
/// pull whose handle has not come yet stops when the handle comes and is
/// dropped.
fn cancel(state: &mut CreateState) -> Vec<CreateOutcome> {
    let Some(form) = state.take_form() else {
        return Vec::new();
    };
    if !form.is_pulling() {
        return Vec::new();
    }

    state
        .take_pull()
        .map(CreateOutcome::CancelPull)
        .into_iter()
        .collect()
}

/// Keep the abort handle of the dialog's pull while the dialog pulls. A
/// handle that comes after Cancel is dropped, which stops its pull.
fn keep_pull(state: &mut CreateState, pull: PullHandle) {
    if !state.form().is_some_and(Form::is_pulling) {
        log::debug!("containers pull handle without a pulling dialog dropped");
        return;
    }

    state.keep_pull(pull);
}

/// Take the end of the running pull and forget its abort handle: the dialog
/// closes; success asks the parent to reload the engine's lists, a failure to
/// tell the user. A result without a running pull (cancelled meanwhile) is
/// dropped.
fn finish_pull(
    state: &mut CreateState,
    result: Result<(), ContainersError>,
) -> Vec<CreateOutcome> {
    state.forget_pull();

    if !state.form().is_some_and(Form::is_pulling) {
        return Vec::new();
    }
    let Some(form) = state.take_form() else {
        return Vec::new();
    };

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
    vec![outcome]
}

#[cfg(test)]
mod tests {
    use otty_containers::{EngineInfo, EngineKind, fixtures};

    use super::*;
    use crate::event::{ContainersIntent, ContainersOutcome};
    use crate::reducer::reduce as reduce_containers;
    use crate::state::ContainersState;
    use crate::types::{
        DEFAULT_TAG, EngineAction, LoadedLists, Place, PullHandle, ResourceKind,
    };

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    /// Reduce `intent` in the create dialogs alone: the engine work they ask
    /// for and what they tell the parent.
    fn reduce_child(
        state: &mut CreateState,
        intent: CreateIntent,
    ) -> Vec<CreateOutcome> {
        reduce(state, intent)
    }

    /// The pull dialog of `nginx:latest` on Docker, with the pull running.
    fn pulling_part() -> CreateState {
        let mut state = CreateState::default();
        let _ = reduce_child(
            &mut state,
            CreateIntent::OpenPull {
                engine: docker(),
                image: String::from("nginx"),
                tag: String::new(),
            },
        );
        let _ = reduce_child(&mut state, CreateIntent::Submitted);
        state
    }

    /// Send `intent` to the create dialogs through the feature's reducer.
    fn send(
        state: &mut ContainersState,
        intent: CreateIntent,
    ) -> Vec<ContainersOutcome> {
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

        let outcomes = reduce_child(
            &mut tagged,
            CreateIntent::OpenPull {
                engine: docker(),
                image: String::from("busybox"),
                tag: String::from("1.37"),
            },
        );
        let _ = reduce_child(
            &mut untagged,
            CreateIntent::OpenPull {
                engine: docker(),
                image: String::from("busybox"),
                tag: String::new(),
            },
        );

        assert!(outcomes.is_empty());
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
        let _ = reduce_child(&mut state, CreateIntent::OpenVolume(docker()));
        let _ = reduce_child(
            &mut state,
            CreateIntent::Changed(FormField::Name, String::from("otty-fx-vol")),
        );

        let outcomes = reduce_child(&mut state, CreateIntent::Submitted);

        assert_eq!(
            outcomes,
            vec![CreateOutcome::ActionRequested(ActionRequest::new(
                docker(),
                EngineAction::CreateVolume {
                    name: String::from("otty-fx-vol"),
                    driver: String::from("local"),
                },
                String::from("otty-fx-vol"),
            ))]
        );
        assert!(!state.is_open());
    }

    #[test]
    fn a_finished_pull_closes_the_dialog_and_names_the_engine_to_reload() {
        let mut state = pulling_part();

        let outcomes =
            reduce_child(&mut state, CreateIntent::PullFinished(Ok(())));

        assert_eq!(
            outcomes,
            vec![CreateOutcome::Pulled {
                engine: docker().address().clone(),
            }]
        );
        assert!(!state.is_open());
    }

    #[test]
    fn a_failed_pull_closes_the_dialog_and_tells_the_parent_why() {
        let mut state = pulling_part();

        let outcomes = reduce_child(
            &mut state,
            CreateIntent::PullFinished(Err(ContainersError::new(
                "manifest unknown",
            ))),
        );

        assert_eq!(
            outcomes,
            vec![CreateOutcome::Failed(i18n::containers_action_failed(
                Key::TplContainersPullFailed,
                "nginx:latest",
                "manifest unknown"
            ))]
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

        let outcomes = send(&mut state, CreateIntent::Submitted);

        assert!(outcomes.is_empty());
        assert!(!state.create().form().is_some_and(Form::is_pulling));
    }

    #[test]
    fn pull_starts_the_pull_and_keeps_the_dialog_open() {
        let mut state = state_on(ResourceKind::Images);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "nginx");

        let outcomes = send(&mut state, CreateIntent::Submitted);

        assert_eq!(
            outcomes,
            vec![ContainersOutcome::PullImage {
                engine: docker(),
                image: String::from("nginx"),
                tag: String::from(DEFAULT_TAG),
            }]
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
        let again = send(&mut state, CreateIntent::Submitted);

        assert!(again.is_empty());
        assert_eq!(
            state.create().form().and_then(Form::progress),
            Some(&fixtures::pull_progress())
        );
    }

    #[test]
    fn a_finished_pull_closes_the_dialog_and_reloads() {
        let mut state = pulling();

        let outcomes = send(&mut state, CreateIntent::PullFinished(Ok(())));

        assert!(!state.create().is_open());
        assert_eq!(outcomes, vec![ContainersOutcome::LoadLists(docker())]);
    }

    #[test]
    fn a_failed_pull_closes_the_dialog_and_tells_the_user() {
        let mut state = pulling();

        let outcomes = send(
            &mut state,
            CreateIntent::PullFinished(Err(ContainersError::new(
                "manifest unknown",
            ))),
        );

        assert!(!state.create().is_open());
        assert_eq!(
            outcomes,
            vec![ContainersOutcome::Failed(i18n::containers_action_failed(
                Key::TplContainersPullFailed,
                "nginx:latest",
                "manifest unknown"
            ))]
        );
    }

    #[test]
    fn a_started_pull_keeps_its_handle() {
        let mut state = pulling_part();
        let pull = PullHandle::detached();

        let outcomes =
            reduce_child(&mut state, CreateIntent::PullStarted(pull.clone()));

        assert!(outcomes.is_empty(), "{outcomes:?}");
        assert_eq!(state.pull(), Some(&pull));
    }

    #[test]
    fn a_pull_handle_without_a_running_pull_is_not_kept() {
        let mut state = CreateState::default();
        let _ = reduce_child(
            &mut state,
            CreateIntent::OpenPull {
                engine: docker(),
                image: String::from("nginx"),
                tag: String::new(),
            },
        );

        let _ = reduce_child(
            &mut state,
            CreateIntent::PullStarted(PullHandle::detached()),
        );

        assert_eq!(state.pull(), None);
    }

    #[test]
    fn a_new_pull_replaces_the_handle_of_the_one_before() {
        let mut state = pulling_part();
        let first = PullHandle::detached();
        let second = PullHandle::detached();
        let _ = reduce_child(&mut state, CreateIntent::PullStarted(first));

        for intent in [
            CreateIntent::OpenPull {
                engine: docker(),
                image: String::from("redis"),
                tag: String::new(),
            },
            CreateIntent::Submitted,
            CreateIntent::PullStarted(second.clone()),
        ] {
            let _ = reduce_child(&mut state, intent);
        }

        assert_eq!(state.pull(), Some(&second));
    }

    #[test]
    fn cancel_during_a_pull_stops_it() {
        let mut state = pulling();
        let pull = PullHandle::detached();
        let _ = send(&mut state, CreateIntent::PullStarted(pull.clone()));

        let outcomes = send(&mut state, CreateIntent::Cancelled);

        assert_eq!(outcomes, vec![ContainersOutcome::CancelPull(pull)]);
        assert!(!state.create().is_open());
        assert_eq!(state.create().pull(), None);
    }

    #[test]
    fn a_pull_handle_that_comes_after_cancel_is_not_kept() {
        let mut state = pulling_part();

        let cancelled = reduce_child(&mut state, CreateIntent::Cancelled);
        let _ = reduce_child(
            &mut state,
            CreateIntent::PullStarted(PullHandle::detached()),
        );

        assert!(cancelled.is_empty(), "{cancelled:?}");
        assert_eq!(state.pull(), None);
    }

    #[test]
    fn a_finished_or_failed_pull_forgets_its_handle() {
        let mut finished = pulling_part();
        let mut failed = pulling_part();
        for state in [&mut finished, &mut failed] {
            let _ = reduce_child(
                state,
                CreateIntent::PullStarted(PullHandle::detached()),
            );
        }

        let _ = reduce_child(&mut finished, CreateIntent::PullFinished(Ok(())));
        let _ = reduce_child(
            &mut failed,
            CreateIntent::PullFinished(Err(ContainersError::new(
                "manifest unknown",
            ))),
        );

        assert_eq!(finished.pull(), None);
        assert_eq!(failed.pull(), None);
    }

    #[test]
    fn cancel_before_a_pull_only_closes_the_dialog() {
        let mut state = state_on(ResourceKind::Volumes);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);

        let outcomes = send(&mut state, CreateIntent::Cancelled);

        assert!(outcomes.is_empty());
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
        let finished = send(&mut state, CreateIntent::PullFinished(Ok(())));

        assert!(finished.is_empty());
        assert!(state.create().form().is_some_and(|form| !form.is_pulling()));
    }

    #[test]
    fn create_volume_runs_the_call_and_closes_the_dialog() {
        let mut state = state_on(ResourceKind::Volumes);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "otty-fx-vol");

        let outcomes = send(&mut state, CreateIntent::Submitted);

        assert_eq!(
            outcomes,
            vec![ContainersOutcome::RunAction(ActionRequest::new(
                docker(),
                EngineAction::CreateVolume {
                    name: String::from("otty-fx-vol"),
                    driver: String::from("local"),
                },
                String::from("otty-fx-vol"),
            ))]
        );
        assert!(!state.create().is_open());
    }

    #[test]
    fn create_network_without_a_name_does_nothing() {
        let mut state = state_on(ResourceKind::Networks);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);

        let outcomes = send(&mut state, CreateIntent::Submitted);

        assert!(outcomes.is_empty());
        assert!(state.create().is_open());
    }

    #[test]
    fn create_network_runs_the_call_with_the_typed_driver() {
        let mut state = state_on(ResourceKind::Networks);
        let _ = reduce_containers(&mut state, ContainersIntent::OpenCreate);
        type_in(&mut state, FormField::Name, "otty-fx-net2");
        type_in(&mut state, FormField::Driver, "macvlan");

        let outcomes = send(&mut state, CreateIntent::Submitted);

        assert_eq!(
            outcomes,
            vec![ContainersOutcome::RunAction(ActionRequest::new(
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

        let outcomes = send(&mut state, CreateIntent::Submitted);
        type_in(&mut state, FormField::Name, "nginx");

        assert!(outcomes.is_empty());
        assert!(!state.create().is_open());
    }
}
