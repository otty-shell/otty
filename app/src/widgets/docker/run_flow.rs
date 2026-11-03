//! How the Run container form goes, as the reducer runs it (spec Decision
//! 18): it opens from "Run container" and from Run on an image, asks the
//! engine about the typed image, pulls a missing image when the switch is
//! on, creates and starts the container, then shows it selected in the
//! Containers table. An engine error keeps the form as typed.

use std::path::{Path, PathBuf};

use iced::Task;
use otty_docker::{EngineAddress, EngineInfo, ImageDefaults, PullProgress};

use super::action_flow::reload;
use super::errors::DockerError;
use super::event::{DockerEffect, DockerEvent};
use super::forms::split_reference;
use super::run_checks::check;
use super::run_form::{ImageLookup, RunEdit, RunForm, RunPhase};
use super::state::{DockerState, EngineState};
use super::types::ResourceKind;
use crate::i18n::{self, Key};

/// The network a new container joins unless another one is chosen.
const DEFAULT_NETWORK: &str = "bridge";

/// Open the Run form on `engine` with `image` typed (empty from "Run
/// container"). A typed image is looked up at once.
pub(super) fn open(
    state: &mut DockerState,
    engine: EngineInfo,
    image: &str,
) -> Task<DockerEvent> {
    let network = state
        .engine(engine.address())
        .map(default_network)
        .unwrap_or_default();

    let mut form =
        RunForm::new(engine, image, &network, state.is_selinux_enabled());
    let lookup = lookup(&mut form);

    state.open_run_form(form);
    lookup
}

/// Take one edit of the form; a changed image is looked up again and a
/// volume source that is a folder is asked about on disk.
pub(super) fn edit(
    state: &mut DockerState,
    edit: RunEdit,
) -> Task<DockerEvent> {
    let Some(form) = state.run_form_mut() else {
        return Task::none();
    };
    let before = form.reference().to_string();
    let source_row = match &edit {
        RunEdit::VolumeSource(index, _) => Some(*index),
        _ => None,
    };

    form.apply(edit);

    let folder_check = source_row
        .and_then(|index| form.start_folder_check(index))
        .map_or_else(Task::none, check_folder);
    if form.reference() == before {
        return folder_check;
    }
    Task::batch([lookup(form), folder_check])
}

/// Run on another engine: its default network, and the image looked up
/// there.
pub(super) fn choose_engine(
    state: &mut DockerState,
    address: &EngineAddress,
) -> Task<DockerEvent> {
    let Some(engine) = state.engine(address) else {
        return Task::none();
    };
    let info = engine.info().clone();
    let network = default_network(engine);
    let Some(form) = state.run_form_mut() else {
        return Task::none();
    };
    if form.engine().address() == address || !form.is_editing() {
        return Task::none();
    }

    form.set_engine(info, &network);
    lookup(form)
}

/// Take the engine's answer about the typed image.
pub(super) fn defaults_loaded(
    state: &mut DockerState,
    engine: &EngineAddress,
    reference: &str,
    result: Result<Option<ImageDefaults>, DockerError>,
) -> Task<DockerEvent> {
    let is_taken = state
        .run_form_mut()
        .is_some_and(|form| form.finish_lookup(engine, reference, result));
    if !is_taken {
        log::debug!("docker image lookup of {reference} no longer asked");
    }

    Task::none()
}

/// Take the disk's answer about a folder of the form.
pub(super) fn folder_checked(
    state: &mut DockerState,
    path: &Path,
    exists: bool,
) -> Task<DockerEvent> {
    let is_taken = state
        .run_form_mut()
        .is_some_and(|form| form.finish_folder_check(path, exists));
    if !is_taken {
        let path = path.display();
        log::debug!("docker folder check of {path} no longer asked");
    }

    Task::none()
}

/// Ask again about each folder of the form that has no question out, so a
/// folder created or removed elsewhere shows by itself.
pub(super) fn recheck_folders(state: &mut DockerState) -> Task<DockerEvent> {
    let Some(form) = state.run_form_mut() else {
        return Task::none();
    };

    Task::batch(form.start_folder_rechecks().into_iter().map(check_folder))
}

/// Whether Run can be pressed: nothing runs, every check passes and both
/// the engine about the image and the disk about the folders have answered.
pub(crate) fn can_run(form: &RunForm) -> bool {
    form.is_editing()
        && form.lookup() != &ImageLookup::Loading
        && form.are_folders_answered()
        && check(form).is_empty()
}

/// Press Run: pull a missing image first when the switch is on, else
/// create and start the container. Nothing happens while Run is disabled.
pub(super) fn submit(state: &mut DockerState) -> Task<DockerEvent> {
    let Some(form) = state.run_form_mut() else {
        return Task::none();
    };
    if !can_run(form) {
        return Task::none();
    }

    let engine = form.engine().clone();
    if form.needs_pull() {
        let (image, tag) = split_reference(form.reference());
        let pull = DockerEffect::StartPull {
            engine,
            image: image.to_string(),
            tag: tag.to_string(),
        };
        form.start_pull();
        return Task::done(DockerEvent::Effect(pull));
    }

    start_run(form)
}

/// Whether the Run form is pulling its image, so the pull results are
/// its own and not the pull dialog's.
pub(super) fn is_pulling(state: &DockerState) -> bool {
    state
        .run_form()
        .is_some_and(|form| matches!(form.phase(), RunPhase::Pulling(_)))
}

/// Take the newest progress of the form's pull.
pub(super) fn pull_progressed(state: &mut DockerState, progress: PullProgress) {
    if let Some(form) = state.run_form_mut() {
        let _ = form.set_progress(progress);
    }
}

/// The form's pull ended: create and start the container, or show the
/// engine's message and keep the form.
pub(super) fn finish_pull(
    state: &mut DockerState,
    result: Result<(), DockerError>,
) -> Task<DockerEvent> {
    let Some(form) = state.run_form_mut() else {
        return Task::none();
    };

    match result {
        Ok(()) => start_run(form),
        Err(err) => {
            let notice = i18n::docker_action_failed(
                Key::TplDockerPullFailed,
                form.reference(),
                &err.to_string(),
            );
            form.fail(notice);
            Task::none()
        },
    }
}

/// The container ran or failed. Success closes the form and shows the
/// Containers table of the engine with the new container selected;
/// failure keeps the form with a notice. A result without a running form
/// (cancelled meanwhile) is dropped.
pub(super) fn finish(
    state: &mut DockerState,
    engine: &EngineAddress,
    result: Result<String, DockerError>,
) -> Task<DockerEvent> {
    let is_running = state.run_form().is_some_and(|form| {
        form.phase() == &RunPhase::Starting && form.engine().address() == engine
    });
    if !is_running {
        log::debug!("docker run result without a running form dropped");
        return Task::none();
    }

    match result {
        Ok(id) => {
            let _ = state.take_run_form();
            state.show_item(engine, ResourceKind::Containers, id);
            reload(state, engine)
        },
        Err(err) => {
            if let Some(form) = state.run_form_mut() {
                let name = match form.name().trim() {
                    "" => form.reference().to_string(),
                    name => name.to_string(),
                };
                let notice = i18n::docker_action_failed(
                    Key::TplDockerRunFailed,
                    &name,
                    &err.to_string(),
                );
                form.fail(notice);
            }
            Task::none()
        },
    }
}

/// Close the form, from Cancel or when another place opens; a running
/// pull is stopped.
pub(super) fn close(state: &mut DockerState) -> Task<DockerEvent> {
    let Some(form) = state.take_run_form() else {
        return Task::none();
    };

    if matches!(form.phase(), RunPhase::Pulling(_)) {
        Task::done(DockerEvent::Effect(DockerEffect::CancelPull))
    } else {
        Task::none()
    }
}

/// The network a new container on `engine` joins by default: `bridge`
/// when the engine has it, else its first network.
pub(crate) fn default_network(engine: &EngineState) -> String {
    let networks = engine.networks().items();

    if networks
        .iter()
        .any(|network| network.name() == DEFAULT_NETWORK)
    {
        return DEFAULT_NETWORK.to_string();
    }
    networks
        .first()
        .map(|network| network.name().to_string())
        .unwrap_or_default()
}

/// Ask the engine about the typed image, if one is typed.
fn lookup(form: &mut RunForm) -> Task<DockerEvent> {
    let Some(reference) = form.start_lookup() else {
        return Task::none();
    };

    Task::done(DockerEvent::Effect(DockerEffect::LoadImageDefaults {
        engine: form.engine().clone(),
        reference,
    }))
}

/// Ask the disk whether `path` exists.
fn check_folder(path: PathBuf) -> Task<DockerEvent> {
    Task::done(DockerEvent::Effect(DockerEffect::CheckFolder(path)))
}

/// Create and start the container of the typed form.
fn start_run(form: &mut RunForm) -> Task<DockerEvent> {
    let run = DockerEffect::RunContainer {
        engine: form.engine().clone(),
        request: form.request(),
    };

    form.start_run();
    Task::done(DockerEvent::Effect(run))
}

#[cfg(test)]
mod tests {
    use otty_docker::{
        Discovery, EngineKind, MountSource, PortProtocol, PublishedPort,
        RestartPolicy, RunRequest, VolumeMount, fixtures,
    };
    use otty_ui_kit::components::tag_search::TagSearchAction;

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::docker::event::DockerIntent;
    use crate::widgets::docker::forms::Form;
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::types::{LoadedLists, Place, ResourceKind};

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn podman() -> EngineInfo {
        fixtures::engine(EngineKind::Podman)
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

    fn lists(kind: EngineKind) -> LoadedLists {
        let resources = fixtures::resources(kind);
        LoadedLists {
            containers: Ok(resources.containers().to_vec()),
            images: Ok(resources.images().to_vec()),
            volumes: Ok(resources.volumes().to_vec()),
            networks: Ok(resources.networks().to_vec()),
        }
    }

    /// Docker mode active with the fixture engines of `kinds` loaded and
    /// `resource` of the first one open.
    fn state_on(kinds: &[EngineKind], resource: ResourceKind) -> DockerState {
        state_after(fixtures::discovery(kinds), kinds, resource)
    }

    /// Like [`state_on`], after this engine search.
    fn state_after(
        discovery: Discovery,
        kinds: &[EngineKind],
        resource: ResourceKind,
    ) -> DockerState {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(&mut state, DockerIntent::DiscoveryFinished(discovery));
        for kind in kinds {
            let _ = reduce(
                &mut state,
                DockerIntent::ListsLoaded {
                    engine: fixtures::engine(*kind).address().clone(),
                    lists: lists(*kind),
                },
            );
        }
        let _ = reduce(
            &mut state,
            DockerIntent::Open(Place::new(
                fixtures::engine(kinds[0]).address().clone(),
                resource,
            )),
        );
        state
    }

    fn type_in(state: &mut DockerState, edit: RunEdit) -> Vec<DockerEffect> {
        effects(reduce(state, DockerIntent::RunEdited(edit)))
    }

    /// The Run form on Docker with `image` typed and the engine's answer
    /// `found` (`None`: not on this computer).
    fn form_with(image: &str, found: Option<ImageDefaults>) -> DockerState {
        let mut state =
            state_on(&[EngineKind::Docker], ResourceKind::Containers);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        let _ = type_in(&mut state, RunEdit::Image(image.to_string()));
        let _ = reduce(
            &mut state,
            DockerIntent::ImageDefaultsLoaded {
                engine: docker().address().clone(),
                reference: image.to_string(),
                result: Ok(found),
            },
        );
        state
    }

    fn run_form(state: &DockerState) -> &RunForm {
        state.run_form().expect("an open run form")
    }

    #[test]
    fn run_container_opens_an_empty_form_on_the_open_engine() {
        let mut state =
            state_on(&[EngineKind::Docker], ResourceKind::Containers);

        let effects = effects(reduce(&mut state, DockerIntent::OpenCreate));

        assert!(effects.is_empty(), "{effects:?}");
        let form = run_form(&state);
        assert_eq!(form.engine(), &docker());
        assert_eq!(form.image(), "");
        assert_eq!(form.network(), "bridge");
        assert!(state.form().map(Form::kind).is_none());
    }

    #[test]
    fn run_on_an_image_opens_the_form_with_the_image_and_looks_it_up() {
        let mut state = state_on(&[EngineKind::Docker], ResourceKind::Images);
        let tagged = state
            .engine(docker().address())
            .and_then(|engine| {
                engine
                    .images()
                    .items()
                    .iter()
                    .position(|image| !image.is_dangling())
            })
            .expect("a tagged image");
        let position = state.shown_position(tagged);
        let _ = reduce(&mut state, DockerIntent::SelectRow(position));

        let effects = effects(reduce(
            &mut state,
            DockerIntent::Act(crate::widgets::docker::actions::RowAction::Run),
        ));

        let form = run_form(&state);
        assert!(!form.image().is_empty());
        assert_eq!(
            effects,
            vec![DockerEffect::LoadImageDefaults {
                engine: docker(),
                reference: form.image().to_string(),
            }]
        );
    }

    #[test]
    fn typing_an_image_looks_it_up_and_a_late_answer_is_dropped() {
        let mut state =
            state_on(&[EngineKind::Docker], ResourceKind::Containers);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);

        let first = type_in(&mut state, RunEdit::Image(String::from("ngin")));
        let second = type_in(&mut state, RunEdit::Image(String::from("nginx")));
        let _ = reduce(
            &mut state,
            DockerIntent::ImageDefaultsLoaded {
                engine: docker().address().clone(),
                reference: String::from("ngin"),
                result: Ok(None),
            },
        );

        assert_eq!(
            first,
            vec![DockerEffect::LoadImageDefaults {
                engine: docker(),
                reference: String::from("ngin"),
            }]
        );
        assert_eq!(second.len(), 1);
        assert_eq!(run_form(&state).lookup(), &ImageLookup::Loading);
    }

    #[test]
    fn other_edits_do_not_look_the_image_up_again() {
        let mut state = form_with("nginx:1.27", None);

        let effects = type_in(&mut state, RunEdit::Name(String::from("web")));

        assert!(effects.is_empty(), "{effects:?}");
        assert_eq!(run_form(&state).name(), "web");
    }

    #[test]
    fn run_is_disabled_while_a_check_fails() {
        let mut state =
            form_with("nginx:1.27", Some(fixtures::image_defaults()));
        let _ = type_in(&mut state, RunEdit::AddPort);
        let _ =
            type_in(&mut state, RunEdit::PortHost(0, String::from("80800")));
        let _ =
            type_in(&mut state, RunEdit::PortContainer(0, String::from("80")));

        let effects = effects(reduce(&mut state, DockerIntent::RunSubmitted));

        assert!(effects.is_empty(), "{effects:?}");
        assert!(!can_run(run_form(&state)));
        assert!(run_form(&state).is_editing());
    }

    #[test]
    fn run_waits_for_the_engines_answer_about_the_image() {
        let mut state =
            state_on(&[EngineKind::Docker], ResourceKind::Containers);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        let _ = type_in(&mut state, RunEdit::Image(String::from("nginx")));

        let effects = effects(reduce(&mut state, DockerIntent::RunSubmitted));

        assert!(effects.is_empty(), "{effects:?}");
        assert!(!can_run(run_form(&state)));
    }

    #[test]
    fn an_image_on_this_computer_is_run_at_once_with_the_typed_settings() {
        let mut state =
            form_with("nginx:1.27", Some(fixtures::image_defaults()));
        let _ = type_in(&mut state, RunEdit::Name(String::from("otty-fx-web")));
        let _ = type_in(&mut state, RunEdit::AddPort);
        let _ = type_in(&mut state, RunEdit::PortHost(0, String::from("8081")));
        let _ =
            type_in(&mut state, RunEdit::PortContainer(0, String::from("80")));
        let _ = type_in(&mut state, RunEdit::AddVolume);
        let _ = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from("otty-fx-cache")),
        );
        let _ = type_in(
            &mut state,
            RunEdit::VolumeTarget(0, String::from("/cache")),
        );
        let _ = type_in(&mut state, RunEdit::AutoRemove(true));
        assert!(can_run(run_form(&state)));

        let effects = effects(reduce(&mut state, DockerIntent::RunSubmitted));

        let expected = RunRequest::new("nginx:1.27")
            .with_name("otty-fx-web")
            .with_port(PublishedPort::new(Some(8081), 80, PortProtocol::Tcp))
            .with_volume(VolumeMount::new(
                MountSource::Volume(String::from("otty-fx-cache")),
                "/cache",
                false,
            ))
            .with_network("bridge")
            .with_restart(RestartPolicy::No)
            .with_auto_remove(true);
        assert_eq!(
            effects,
            vec![DockerEffect::RunContainer {
                engine: docker(),
                request: expected,
            }]
        );
        assert_eq!(run_form(&state).phase(), &RunPhase::Starting);
    }

    #[test]
    fn a_missing_image_is_pulled_first_with_its_progress_in_the_form() {
        let mut state = form_with("nginx:1.27-alpine", None);

        let started = effects(reduce(&mut state, DockerIntent::RunSubmitted));
        let _ = reduce(
            &mut state,
            DockerIntent::PullProgressed(fixtures::pull_progress()),
        );

        assert_eq!(
            started,
            vec![DockerEffect::StartPull {
                engine: docker(),
                image: String::from("nginx"),
                tag: String::from("1.27-alpine"),
            }]
        );
        assert_eq!(
            run_form(&state).phase(),
            &RunPhase::Pulling(fixtures::pull_progress())
        );
    }

    #[test]
    fn after_the_pull_the_container_is_created_and_started() {
        let mut state = form_with("nginx:1.27-alpine", None);
        let _ = reduce(&mut state, DockerIntent::RunSubmitted);

        let effects =
            effects(reduce(&mut state, DockerIntent::PullFinished(Ok(()))));

        assert!(matches!(
            effects.as_slice(),
            [DockerEffect::RunContainer { request, .. }]
                if request.image() == "nginx:1.27-alpine"
        ));
        assert_eq!(run_form(&state).phase(), &RunPhase::Starting);
    }

    #[test]
    fn a_failed_pull_keeps_the_form_with_a_notice() {
        let mut state = form_with("nginx:9", None);
        let _ = type_in(&mut state, RunEdit::Name(String::from("web")));
        let _ = reduce(&mut state, DockerIntent::RunSubmitted);

        let effects = effects(reduce(
            &mut state,
            DockerIntent::PullFinished(Err(DockerError::new(
                "manifest unknown",
            ))),
        ));

        assert!(effects.is_empty(), "{effects:?}");
        let form = run_form(&state);
        assert!(form.is_editing());
        assert_eq!(form.name(), "web");
        assert_eq!(
            form.notice(),
            Some(
                i18n::docker_action_failed(
                    Key::TplDockerPullFailed,
                    "nginx:9",
                    "manifest unknown"
                )
                .as_str()
            )
        );
    }

    #[test]
    fn a_missing_image_without_the_pull_switch_is_run_as_is() {
        let mut state = form_with("nginx:9", None);
        let _ = type_in(&mut state, RunEdit::PullMissing(false));

        let effects = effects(reduce(&mut state, DockerIntent::RunSubmitted));

        assert!(matches!(
            effects.as_slice(),
            [DockerEffect::RunContainer { .. }]
        ));
    }

    #[test]
    fn a_run_shows_the_new_container_selected_in_the_containers_table() {
        let mut state =
            form_with("nginx:1.27", Some(fixtures::image_defaults()));
        for action in [
            TagSearchAction::InputChanged(String::from("x")),
            TagSearchAction::Close,
        ] {
            let _ = reduce(&mut state, DockerIntent::TagSearch(action));
        }
        let _ = reduce(&mut state, DockerIntent::RunSubmitted);

        let effects = effects(reduce(
            &mut state,
            DockerIntent::RunFinished {
                engine: docker().address().clone(),
                result: Ok(String::from("4f1c2b9a8e7d")),
            },
        ));

        assert!(state.run_form().is_none());
        let place = state.place().expect("a place");
        assert_eq!(place.resource(), ResourceKind::Containers);
        assert_eq!(place.engine(), docker().address());
        assert_eq!(state.table_search(ResourceKind::Containers).input(), "");
        let selected = state
            .engine(docker().address())
            .and_then(|engine| engine.containers().selected_id());
        assert_eq!(selected, Some("4f1c2b9a8e7d"));
        assert_eq!(effects, vec![DockerEffect::LoadLists(docker())]);
    }

    #[test]
    fn an_engine_error_keeps_every_field_and_shows_a_notice() {
        let mut state =
            form_with("nginx:1.27", Some(fixtures::image_defaults()));
        let _ = type_in(&mut state, RunEdit::Name(String::from("web")));
        let _ = type_in(&mut state, RunEdit::AddEnv);
        let _ = type_in(&mut state, RunEdit::EnvName(0, String::from("A")));
        let _ = reduce(&mut state, DockerIntent::RunSubmitted);

        let effects = effects(reduce(
            &mut state,
            DockerIntent::RunFinished {
                engine: docker().address().clone(),
                result: Err(DockerError::new("port is already allocated")),
            },
        ));

        assert!(effects.is_empty(), "{effects:?}");
        let form = run_form(&state);
        assert!(form.is_editing());
        assert_eq!(form.name(), "web");
        assert_eq!(form.env().len(), 1);
        assert_eq!(
            form.notice(),
            Some(
                i18n::docker_action_failed(
                    Key::TplDockerRunFailed,
                    "web",
                    "port is already allocated"
                )
                .as_str()
            )
        );

        let _ = reduce(&mut state, DockerIntent::RunNoticeDismissed);
        assert_eq!(run_form(&state).notice(), None);
    }

    #[test]
    fn a_failure_without_a_name_names_the_image() {
        let mut state =
            form_with("nginx:1.27", Some(fixtures::image_defaults()));
        let _ = reduce(&mut state, DockerIntent::RunSubmitted);

        let _ = reduce(
            &mut state,
            DockerIntent::RunFinished {
                engine: docker().address().clone(),
                result: Err(DockerError::new("boom")),
            },
        );

        assert_eq!(
            run_form(&state).notice(),
            Some(
                i18n::docker_action_failed(
                    Key::TplDockerRunFailed,
                    "nginx:1.27",
                    "boom"
                )
                .as_str()
            )
        );
    }

    #[test]
    fn a_result_without_a_running_form_is_dropped() {
        let mut state = state_on(&[EngineKind::Docker], ResourceKind::Images);

        let effects = effects(reduce(
            &mut state,
            DockerIntent::RunFinished {
                engine: docker().address().clone(),
                result: Ok(String::from("4f1c")),
            },
        ));

        assert!(effects.is_empty());
        assert_eq!(
            state.place().map(Place::resource),
            Some(ResourceKind::Images)
        );
    }

    #[test]
    fn cancel_closes_the_form_and_stops_a_running_pull() {
        let mut state = form_with("nginx:9", None);
        let _ = reduce(&mut state, DockerIntent::RunSubmitted);

        let effects = effects(reduce(&mut state, DockerIntent::RunCancelled));

        assert!(state.run_form().is_none());
        assert_eq!(effects, vec![DockerEffect::CancelPull]);
    }

    #[test]
    fn cancel_before_a_run_only_closes_the_form() {
        let mut state = form_with("nginx:9", None);

        let effects = effects(reduce(&mut state, DockerIntent::RunCancelled));

        assert!(state.run_form().is_none());
        assert!(effects.is_empty());
    }

    #[test]
    fn opening_another_list_closes_the_form() {
        let mut state = form_with("nginx:9", None);

        let _ = reduce(
            &mut state,
            DockerIntent::Open(Place::new(
                docker().address().clone(),
                ResourceKind::Images,
            )),
        );

        assert!(state.run_form().is_none());
    }

    #[test]
    fn the_pull_dialog_keeps_its_own_pull_results() {
        let mut state = state_on(&[EngineKind::Docker], ResourceKind::Images);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        let _ = reduce(
            &mut state,
            DockerIntent::FormChanged(
                crate::widgets::docker::forms::FormField::Name,
                String::from("nginx"),
            ),
        );
        let _ = reduce(&mut state, DockerIntent::FormSubmitted);

        let _ = reduce(
            &mut state,
            DockerIntent::PullProgressed(fixtures::pull_progress()),
        );

        assert!(state.run_form().is_none());
        assert_eq!(
            state.form().and_then(Form::progress),
            Some(&fixtures::pull_progress())
        );
    }

    #[test]
    fn another_engine_has_its_own_network_and_lookup() {
        let mut state = state_on(
            &[EngineKind::Docker, EngineKind::Podman],
            ResourceKind::Containers,
        );
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        let _ = type_in(&mut state, RunEdit::Image(String::from("nginx")));
        let _ = type_in(&mut state, RunEdit::Network(String::from("none")));

        let effects = effects(reduce(
            &mut state,
            DockerIntent::RunEngineChosen(podman().address().clone()),
        ));

        let form = run_form(&state);
        assert_eq!(form.engine(), &podman());
        assert_eq!(form.network(), "bridge");
        assert_eq!(
            effects,
            vec![DockerEffect::LoadImageDefaults {
                engine: podman(),
                reference: String::from("nginx"),
            }]
        );
    }

    #[test]
    fn a_failed_lookup_shows_the_engines_message() {
        let mut state =
            state_on(&[EngineKind::Docker], ResourceKind::Containers);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        let _ = type_in(&mut state, RunEdit::Image(String::from("nginx")));

        let _ = reduce(
            &mut state,
            DockerIntent::ImageDefaultsLoaded {
                engine: docker().address().clone(),
                reference: String::from("nginx"),
                result: Err(DockerError::new("store is locked")),
            },
        );

        assert_eq!(
            run_form(&state).lookup(),
            &ImageLookup::Failed(String::from("store is locked"))
        );
        assert!(can_run(run_form(&state)));
    }

    /// The effect that asks about the folder `path`.
    fn asks_about(path: &str) -> DockerEffect {
        DockerEffect::CheckFolder(PathBuf::from(path))
    }

    /// Add a volume row to `/data` with `source` typed last; the effects of
    /// typing it.
    fn add_volume(state: &mut DockerState, source: &str) -> Vec<DockerEffect> {
        let index = run_form(state).volumes().len();
        let _ = type_in(state, RunEdit::AddVolume);
        let _ =
            type_in(state, RunEdit::VolumeTarget(index, String::from("/data")));

        type_in(state, RunEdit::VolumeSource(index, source.to_string()))
    }

    fn answer(state: &mut DockerState, path: &str, exists: bool) {
        let effects = effects(reduce(
            state,
            DockerIntent::FolderChecked {
                path: PathBuf::from(path),
                exists,
            },
        ));

        assert!(effects.is_empty(), "{effects:?}");
    }

    fn recheck(state: &mut DockerState) -> Vec<DockerEffect> {
        effects(reduce(state, DockerIntent::RecheckFolders))
    }

    fn ready_form() -> DockerState {
        form_with("nginx:1.27", Some(fixtures::image_defaults()))
    }

    #[test]
    fn typing_a_folder_asks_about_it_once() {
        let mut state = ready_form();

        let typed = add_volume(&mut state, "/srv/site");
        let retyped = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from(" /srv/site ")),
        );
        let target =
            type_in(&mut state, RunEdit::VolumeTarget(0, String::from("/d")));

        assert_eq!(typed, vec![asks_about("/srv/site")]);
        assert!(retyped.is_empty(), "{retyped:?}");
        assert!(target.is_empty(), "{target:?}");
    }

    #[test]
    fn typing_a_named_volume_or_nothing_asks_nothing() {
        let mut state = ready_form();

        let named = add_volume(&mut state, "otty-fx-cache");
        let blank =
            type_in(&mut state, RunEdit::VolumeSource(0, String::from("   ")));

        assert!(named.is_empty(), "{named:?}");
        assert!(blank.is_empty(), "{blank:?}");
    }

    #[test]
    fn each_new_text_of_a_folder_is_asked_about() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv");

        let next = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from("/srv/site")),
        );

        assert_eq!(next, vec![asks_about("/srv/site")]);
    }

    #[test]
    fn run_stays_disabled_until_the_folder_is_answered() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/site");

        let pressed = effects(reduce(&mut state, DockerIntent::RunSubmitted));

        assert!(pressed.is_empty(), "{pressed:?}");
        assert!(!can_run(run_form(&state)));
        assert!(check(run_form(&state)).is_empty());

        answer(&mut state, "/srv/site", true);

        assert!(can_run(run_form(&state)));
    }

    #[test]
    fn a_missing_answer_shows_the_problem_and_an_existing_answer_clears_it() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/site");

        answer(&mut state, "/srv/site", false);

        assert_eq!(
            check(run_form(&state)).volume(0).map(|p| p.message()),
            Some(Key::DockerRunErrSourceMissing)
        );
        assert!(!can_run(run_form(&state)));

        answer(&mut state, "/srv/site", true);

        assert!(check(run_form(&state)).is_empty());
        assert!(can_run(run_form(&state)));
    }

    #[test]
    fn the_error_does_not_flicker_while_the_next_folder_is_asked() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/sit");
        answer(&mut state, "/srv/sit", false);

        let _ = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from("/srv/site")),
        );

        assert_eq!(
            check(run_form(&state)).volume(0).map(|p| p.message()),
            Some(Key::DockerRunErrSourceMissing)
        );
        assert!(!can_run(run_form(&state)));
    }

    #[test]
    fn an_answer_about_a_folder_no_row_has_is_dropped() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/old");
        let _ = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from("/srv/new")),
        );

        answer(&mut state, "/srv/old", false);

        assert!(check(run_form(&state)).is_empty());
        assert!(!can_run(run_form(&state)), "/srv/new has no answer yet");

        let _ = type_in(&mut state, RunEdit::RemoveVolume(0));
        answer(&mut state, "/srv/new", false);

        assert!(check(run_form(&state)).is_empty());
        assert!(can_run(run_form(&state)));
    }

    #[test]
    fn an_answer_without_a_form_is_dropped() {
        let mut state = state_on(&[EngineKind::Docker], ResourceKind::Images);

        answer(&mut state, "/srv/site", true);

        assert!(state.run_form().is_none());
    }

    #[test]
    fn rows_with_the_same_folder_share_one_question_and_its_answer() {
        let mut state = ready_form();

        let first = add_volume(&mut state, "/srv/site");
        let second = add_volume(&mut state, "/srv/site");
        answer(&mut state, "/srv/site", false);

        assert_eq!(first, vec![asks_about("/srv/site")]);
        assert!(second.is_empty(), "{second:?}");
        let problems = check(run_form(&state));
        assert!(problems.volume(0).is_some());
        assert!(problems.volume(1).is_some());
    }

    #[test]
    fn the_recheck_asks_once_per_folder_that_is_not_in_flight() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/answered");
        let _ = add_volume(&mut state, "/srv/answered");
        answer(&mut state, "/srv/answered", true);
        let _ = add_volume(&mut state, "/srv/in-flight");
        let _ = add_volume(&mut state, "otty-fx-cache");

        let first = recheck(&mut state);
        let second = recheck(&mut state);

        assert_eq!(first, vec![asks_about("/srv/answered")]);
        assert!(second.is_empty(), "a hung folder piles up: {second:?}");
    }

    #[test]
    fn the_recheck_does_not_disable_run() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/site");
        answer(&mut state, "/srv/site", true);

        let asked = recheck(&mut state);

        assert_eq!(asked, vec![asks_about("/srv/site")]);
        assert!(can_run(run_form(&state)));
        let pressed = effects(reduce(&mut state, DockerIntent::RunSubmitted));
        assert!(matches!(
            pressed.as_slice(),
            [DockerEffect::RunContainer { .. }]
        ));
    }

    #[test]
    fn a_folder_created_elsewhere_clears_its_error_on_the_recheck() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/site");
        answer(&mut state, "/srv/site", false);
        assert!(!can_run(run_form(&state)));

        let asked = recheck(&mut state);
        answer(&mut state, "/srv/site", true);

        assert_eq!(asked, vec![asks_about("/srv/site")]);
        assert!(check(run_form(&state)).is_empty());
        assert!(can_run(run_form(&state)));
    }

    #[test]
    fn the_recheck_asks_nothing_without_a_form() {
        let mut state = state_on(&[EngineKind::Docker], ResourceKind::Images);

        assert!(recheck(&mut state).is_empty());
    }

    #[test]
    fn the_form_uses_the_selinux_fact_of_the_search() {
        let kinds = [EngineKind::Docker];
        let mut with_selinux = state_after(
            fixtures::discovery_with_selinux(&kinds),
            &kinds,
            ResourceKind::Containers,
        );
        let mut without = state_after(
            fixtures::discovery(&kinds),
            &kinds,
            ResourceKind::Containers,
        );

        for state in [&mut with_selinux, &mut without] {
            let _ = reduce(state, DockerIntent::OpenCreate);
            let _ = add_volume(state, "/srv/site");
        }

        assert!(run_form(&with_selinux).offers_relabel(0));
        assert!(!run_form(&without).offers_relabel(0));
    }

    #[test]
    fn a_form_opened_during_a_later_search_keeps_the_selinux_fact() {
        let kinds = [EngineKind::Docker];
        let mut state = state_after(
            fixtures::discovery_with_selinux(&kinds),
            &kinds,
            ResourceKind::Containers,
        );

        let _ = reduce(&mut state, DockerIntent::CheckAgain);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        let _ = add_volume(&mut state, "/srv/site");

        assert!(run_form(&state).offers_relabel(0));
    }
}
