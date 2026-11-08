//! How the Run container form goes, as the reducer runs it (spec Decision
//! 18): it opens from "Run container" and from Run on an image, asks the
//! engine about the typed image, pulls a missing image when the switch is
//! on, creates and starts the container, then shows it selected in the
//! Containers table. An engine error keeps the form as typed.

use std::path::{Path, PathBuf};

use iced::Task;
use otty_containers::{EngineAddress, EngineInfo, ImageDefaults, PullProgress};

use super::action_flow::reload;
use super::event::{ContainersEffect, ContainersEvent};
use super::run_checks::check;
use super::run_form::{ImageLookup, RunEdit, RunForm, RunPhase};
use super::state::{ContainersState, EngineState};
use super::types::{ContainersError, ResourceKind, split_reference};
use crate::i18n::{self, Key};

/// The network a new container joins unless another one is chosen.
const DEFAULT_NETWORK: &str = "bridge";

/// Open the Run form on `engine` with `image` typed (empty from "Run
/// container"). A typed image is looked up at once.
pub(super) fn open(
    state: &mut ContainersState,
    engine: EngineInfo,
    image: &str,
) -> Task<ContainersEvent> {
    let network = state
        .engine(engine.address())
        .map(default_network)
        .unwrap_or_default();

    let mut form = RunForm::new(
        engine,
        image,
        &network,
        state.is_selinux_enabled(),
        state.home_folder(),
    );
    let lookup = lookup(&mut form);

    state.open_run_form(form);
    lookup
}

/// Take one edit of the form; a changed image is looked up again and a
/// volume source that is a folder waits for a pause in typing before it is
/// asked about on disk, or joins a question that is already out.
pub(super) fn edit(
    state: &mut ContainersState,
    edit: RunEdit,
) -> Task<ContainersEvent> {
    let Some(form) = state.run_form_mut() else {
        return Task::none();
    };
    let before = form.reference().to_string();
    let source_row = match &edit {
        RunEdit::VolumeSource(index, _) => Some(*index),
        _ => None,
    };

    form.apply(edit);

    let folder_wait = source_row
        .map_or_else(Task::none, |index| wait_for_folder(form, index));
    if form.reference() == before {
        return folder_wait;
    }
    Task::batch([lookup(form), folder_wait])
}

/// Run on another engine: its default network, and the image looked up
/// there.
pub(super) fn choose_engine(
    state: &mut ContainersState,
    address: &EngineAddress,
) -> Task<ContainersEvent> {
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
    state: &mut ContainersState,
    engine: &EngineAddress,
    reference: &str,
    result: Result<Option<ImageDefaults>, ContainersError>,
) -> Task<ContainersEvent> {
    let is_taken = state
        .run_form_mut()
        .is_some_and(|form| form.finish_lookup(engine, reference, result));
    if !is_taken {
        log::debug!("containers image lookup of {reference} no longer asked");
    }

    Task::none()
}

/// Take the disk's answer about a folder of the form.
pub(super) fn folder_checked(
    state: &mut ContainersState,
    path: &Path,
    exists: bool,
) -> Task<ContainersEvent> {
    let is_taken = state
        .run_form_mut()
        .is_some_and(|form| form.finish_folder_check(path, exists));
    if !is_taken {
        let path = path.display();
        log::debug!("containers folder check of {path} no longer asked");
    }

    Task::none()
}

/// Ask again about each folder of the form that has no question out, so a
/// folder created or removed elsewhere shows by itself.
pub(super) fn recheck_folders(
    state: &mut ContainersState,
) -> Task<ContainersEvent> {
    let Some(form) = state.run_form_mut() else {
        return Task::none();
    };

    let folders = form.start_folder_rechecks();
    Task::batch(folders.into_iter().map(|path| ask_folder(form, path)))
}

/// Typing stopped on `path`: ask the disk about it, unless the text moved on,
/// the disk already answered or a question is out.
pub(super) fn folder_typing_paused(
    state: &mut ContainersState,
    path: &Path,
) -> Task<ContainersEvent> {
    let Some(form) = state.run_form_mut() else {
        return Task::none();
    };
    if !form.start_folder_check(path) {
        return Task::none();
    }

    ask_folder(form, path.to_path_buf())
}

/// The time for an answer about `path` is over: rows still waiting for it say
/// the folder is not answering. Run stays disabled until the answer comes.
pub(super) fn folder_answer_late(
    state: &mut ContainersState,
    path: &Path,
) -> Task<ContainersEvent> {
    let is_marked = state
        .run_form_mut()
        .is_some_and(|form| form.mark_folder_slow(path));
    if !is_marked {
        let path = path.display();
        log::debug!("containers folder {path} no longer waits for an answer");
    }

    Task::none()
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
pub(super) fn submit(state: &mut ContainersState) -> Task<ContainersEvent> {
    let Some(form) = state.run_form_mut() else {
        return Task::none();
    };
    if !can_run(form) {
        return Task::none();
    }

    let engine = form.engine().clone();
    if form.needs_pull() {
        let (image, tag) = split_reference(form.reference());
        let pull = ContainersEffect::StartPull {
            engine,
            image: image.to_string(),
            tag: tag.to_string(),
        };
        form.start_pull();
        return Task::done(ContainersEvent::Effect(pull));
    }

    start_run(form)
}

/// Whether the Run form is pulling its image, so the pull results are
/// its own and not the pull dialog's.
pub(super) fn is_pulling(state: &ContainersState) -> bool {
    state
        .run_form()
        .is_some_and(|form| matches!(form.phase(), RunPhase::Pulling(_)))
}

/// Take the newest progress of the form's pull.
pub(super) fn pull_progressed(
    state: &mut ContainersState,
    progress: PullProgress,
) {
    if let Some(form) = state.run_form_mut() {
        let _ = form.set_progress(progress);
    }
}

/// The form's pull ended: create and start the container, or show the
/// engine's message and keep the form.
pub(super) fn finish_pull(
    state: &mut ContainersState,
    result: Result<(), ContainersError>,
) -> Task<ContainersEvent> {
    let Some(form) = state.run_form_mut() else {
        return Task::none();
    };

    match result {
        Ok(()) => start_run(form),
        Err(err) => {
            let notice = i18n::containers_action_failed(
                Key::TplContainersPullFailed,
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
    state: &mut ContainersState,
    engine: &EngineAddress,
    result: Result<String, ContainersError>,
) -> Task<ContainersEvent> {
    let is_running = state.run_form().is_some_and(|form| {
        form.phase() == &RunPhase::Starting && form.engine().address() == engine
    });
    if !is_running {
        log::debug!("containers run result without a running form dropped");
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
                let notice = i18n::containers_action_failed(
                    Key::TplContainersRunFailed,
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
pub(super) fn close(state: &mut ContainersState) -> Task<ContainersEvent> {
    let Some(form) = state.take_run_form() else {
        return Task::none();
    };

    if matches!(form.phase(), RunPhase::Pulling(_)) {
        Task::done(ContainersEvent::Effect(ContainersEffect::CancelPull))
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
fn lookup(form: &mut RunForm) -> Task<ContainersEvent> {
    let Some(reference) = form.start_lookup() else {
        return Task::none();
    };

    Task::done(ContainersEvent::Effect(
        ContainersEffect::LoadImageDefaults {
            engine: form.engine().clone(),
            reference,
        },
    ))
}

/// What the row at `index` waits for now that its source was typed: a pause in
/// typing, or the answer to a question another row already has out.
fn wait_for_folder(form: &mut RunForm, index: usize) -> Task<ContainersEvent> {
    if let Some(path) = form.start_typing_wait(index) {
        return effect(ContainersEffect::WaitForTypingPause(path));
    }

    form.join_folder_question(index)
        .map_or_else(Task::none, |path| {
            effect(ContainersEffect::WaitForLateAnswer(path))
        })
}

/// Ask the disk whether `path` exists. A folder with no answer yet also gets
/// a time limit; asking again about an answered one needs none, because it
/// keeps its answer and Run until the new answer comes.
fn ask_folder(form: &RunForm, path: PathBuf) -> Task<ContainersEvent> {
    if !form.is_folder_unanswered(&path) {
        return effect(ContainersEffect::CheckFolder(path));
    }

    Task::batch([
        effect(ContainersEffect::WaitForLateAnswer(path.clone())),
        effect(ContainersEffect::CheckFolder(path)),
    ])
}

/// Hand `effect` to the app.
fn effect(effect: ContainersEffect) -> Task<ContainersEvent> {
    Task::done(ContainersEvent::Effect(effect))
}

/// Create and start the container of the typed form.
fn start_run(form: &mut RunForm) -> Task<ContainersEvent> {
    let run = ContainersEffect::RunContainer {
        engine: form.engine().clone(),
        request: form.request(),
    };

    form.start_run();
    Task::done(ContainersEvent::Effect(run))
}

#[cfg(test)]
mod tests {
    use otty_containers::{
        Discovery, EngineKind, MountSource, PortProtocol, PublishedPort,
        RestartPolicy, RunRequest, VolumeMount, fixtures,
    };
    use otty_ui_kit::components::tag_search::TagSearchAction;

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::containers::event::ContainersIntent;
    use crate::widgets::containers::forms::Form;
    use crate::widgets::containers::reducer::reduce;
    use crate::widgets::containers::types::{LoadedLists, Place, ResourceKind};

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn podman() -> EngineInfo {
        fixtures::engine(EngineKind::Podman)
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

    fn lists(kind: EngineKind) -> LoadedLists {
        let resources = fixtures::resources(kind);
        LoadedLists {
            containers: Ok(resources.containers().to_vec()),
            images: Ok(resources.images().to_vec()),
            volumes: Ok(resources.volumes().to_vec()),
            networks: Ok(resources.networks().to_vec()),
        }
    }

    /// Containers mode active with the fixture engines of `kinds` loaded and
    /// `resource` of the first one open.
    fn state_on(
        kinds: &[EngineKind],
        resource: ResourceKind,
    ) -> ContainersState {
        state_after(fixtures::discovery(kinds), kinds, resource)
    }

    /// Like [`state_on`], after this engine search.
    fn state_after(
        discovery: Discovery,
        kinds: &[EngineKind],
        resource: ResourceKind,
    ) -> ContainersState {
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ =
            reduce(&mut state, ContainersIntent::DiscoveryFinished(discovery));
        for kind in kinds {
            let _ = reduce(
                &mut state,
                ContainersIntent::ListsLoaded {
                    engine: fixtures::engine(*kind).address().clone(),
                    lists: lists(*kind),
                },
            );
        }
        let _ = reduce(
            &mut state,
            ContainersIntent::Open(Place::new(
                fixtures::engine(kinds[0]).address().clone(),
                resource,
            )),
        );
        state
    }

    fn type_in(
        state: &mut ContainersState,
        edit: RunEdit,
    ) -> Vec<ContainersEffect> {
        effects(reduce(state, ContainersIntent::RunEdited(edit)))
    }

    /// The Run form on Docker with `image` typed and the engine's answer
    /// `found` (`None`: not on this computer).
    fn form_with(image: &str, found: Option<ImageDefaults>) -> ContainersState {
        let mut state =
            state_on(&[EngineKind::Docker], ResourceKind::Containers);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = type_in(&mut state, RunEdit::Image(image.to_string()));
        let _ = reduce(
            &mut state,
            ContainersIntent::ImageDefaultsLoaded {
                engine: docker().address().clone(),
                reference: image.to_string(),
                result: Ok(found),
            },
        );
        state
    }

    fn run_form(state: &ContainersState) -> &RunForm {
        state.run_form().expect("an open run form")
    }

    #[test]
    fn run_container_opens_an_empty_form_on_the_open_engine() {
        let mut state =
            state_on(&[EngineKind::Docker], ResourceKind::Containers);

        let effects = effects(reduce(&mut state, ContainersIntent::OpenCreate));

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
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::Act(
                crate::widgets::containers::actions::RowAction::Run,
            ),
        ));

        let form = run_form(&state);
        assert!(!form.image().is_empty());
        assert_eq!(
            effects,
            vec![ContainersEffect::LoadImageDefaults {
                engine: docker(),
                reference: form.image().to_string(),
            }]
        );
    }

    #[test]
    fn typing_an_image_looks_it_up_and_a_late_answer_is_dropped() {
        let mut state =
            state_on(&[EngineKind::Docker], ResourceKind::Containers);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        let first = type_in(&mut state, RunEdit::Image(String::from("ngin")));
        let second = type_in(&mut state, RunEdit::Image(String::from("nginx")));
        let _ = reduce(
            &mut state,
            ContainersIntent::ImageDefaultsLoaded {
                engine: docker().address().clone(),
                reference: String::from("ngin"),
                result: Ok(None),
            },
        );

        assert_eq!(
            first,
            vec![ContainersEffect::LoadImageDefaults {
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

        let effects =
            effects(reduce(&mut state, ContainersIntent::RunSubmitted));

        assert!(effects.is_empty(), "{effects:?}");
        assert!(!can_run(run_form(&state)));
        assert!(run_form(&state).is_editing());
    }

    #[test]
    fn run_waits_for_the_engines_answer_about_the_image() {
        let mut state =
            state_on(&[EngineKind::Docker], ResourceKind::Containers);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = type_in(&mut state, RunEdit::Image(String::from("nginx")));

        let effects =
            effects(reduce(&mut state, ContainersIntent::RunSubmitted));

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

        let effects =
            effects(reduce(&mut state, ContainersIntent::RunSubmitted));

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
            vec![ContainersEffect::RunContainer {
                engine: docker(),
                request: expected,
            }]
        );
        assert_eq!(run_form(&state).phase(), &RunPhase::Starting);
    }

    #[test]
    fn a_missing_image_is_pulled_first_with_its_progress_in_the_form() {
        let mut state = form_with("nginx:1.27-alpine", None);

        let started =
            effects(reduce(&mut state, ContainersIntent::RunSubmitted));
        let _ = reduce(
            &mut state,
            ContainersIntent::PullProgressed(fixtures::pull_progress()),
        );

        assert_eq!(
            started,
            vec![ContainersEffect::StartPull {
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
        let _ = reduce(&mut state, ContainersIntent::RunSubmitted);

        let effects =
            effects(reduce(&mut state, ContainersIntent::PullFinished(Ok(()))));

        assert!(matches!(
            effects.as_slice(),
            [ContainersEffect::RunContainer { request, .. }]
                if request.image() == "nginx:1.27-alpine"
        ));
        assert_eq!(run_form(&state).phase(), &RunPhase::Starting);
    }

    #[test]
    fn a_failed_pull_keeps_the_form_with_a_notice() {
        let mut state = form_with("nginx:9", None);
        let _ = type_in(&mut state, RunEdit::Name(String::from("web")));
        let _ = reduce(&mut state, ContainersIntent::RunSubmitted);

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::PullFinished(Err(ContainersError::new(
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
                i18n::containers_action_failed(
                    Key::TplContainersPullFailed,
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

        let effects =
            effects(reduce(&mut state, ContainersIntent::RunSubmitted));

        assert!(matches!(
            effects.as_slice(),
            [ContainersEffect::RunContainer { .. }]
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
            let _ = reduce(&mut state, ContainersIntent::TagSearch(action));
        }
        let _ = reduce(&mut state, ContainersIntent::RunSubmitted);

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::RunFinished {
                engine: docker().address().clone(),
                result: Ok(String::from("4f1c2b9a8e7d")),
            },
        ));

        assert!(state.run_form().is_none());
        let place = state.place().expect("a place");
        assert_eq!(place.resource(), ResourceKind::Containers);
        assert_eq!(place.engine(), docker().address());
        assert_eq!(state.table_search(place).input(), "");
        let selected = state
            .engine(docker().address())
            .and_then(|engine| engine.containers().selected_id());
        assert_eq!(selected, Some("4f1c2b9a8e7d"));
        assert_eq!(effects, vec![ContainersEffect::LoadLists(docker())]);
    }

    #[test]
    fn a_run_on_a_closed_engine_opens_it_with_the_new_container_shown() {
        let mut state = state_on(
            &[EngineKind::Docker, EngineKind::Podman],
            ResourceKind::Containers,
        );
        let _ = reduce(
            &mut state,
            ContainersIntent::ToggleEngine(podman().address().clone()),
        );
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = type_in(&mut state, RunEdit::Image(String::from("nginx")));
        let _ = reduce(
            &mut state,
            ContainersIntent::RunEngineChosen(podman().address().clone()),
        );
        let _ = reduce(
            &mut state,
            ContainersIntent::ImageDefaultsLoaded {
                engine: podman().address().clone(),
                reference: String::from("nginx"),
                result: Ok(Some(fixtures::image_defaults())),
            },
        );
        let _ = reduce(&mut state, ContainersIntent::RunSubmitted);

        let _ = reduce(
            &mut state,
            ContainersIntent::RunFinished {
                engine: podman().address().clone(),
                result: Ok(String::from("4f1c2b9a8e7d")),
            },
        );

        assert_eq!(state.place().map(Place::engine), Some(podman().address()));
        assert!(!state.is_engine_closed(podman().address()));
    }

    #[test]
    fn a_press_on_an_engine_root_keeps_the_form_on_screen() {
        let mut state = form_with("nginx:9", None);

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::ToggleEngine(docker().address().clone()),
        ));

        assert!(state.run_form().is_some());
        assert!(effects.is_empty());
    }

    #[test]
    fn an_engine_error_keeps_every_field_and_shows_a_notice() {
        let mut state =
            form_with("nginx:1.27", Some(fixtures::image_defaults()));
        let _ = type_in(&mut state, RunEdit::Name(String::from("web")));
        let _ = type_in(&mut state, RunEdit::AddEnv);
        let _ = type_in(&mut state, RunEdit::EnvName(0, String::from("A")));
        let _ = reduce(&mut state, ContainersIntent::RunSubmitted);

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::RunFinished {
                engine: docker().address().clone(),
                result: Err(ContainersError::new("port is already allocated")),
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
                i18n::containers_action_failed(
                    Key::TplContainersRunFailed,
                    "web",
                    "port is already allocated"
                )
                .as_str()
            )
        );

        let _ = reduce(&mut state, ContainersIntent::RunNoticeDismissed);
        assert_eq!(run_form(&state).notice(), None);
    }

    #[test]
    fn a_failure_without_a_name_names_the_image() {
        let mut state =
            form_with("nginx:1.27", Some(fixtures::image_defaults()));
        let _ = reduce(&mut state, ContainersIntent::RunSubmitted);

        let _ = reduce(
            &mut state,
            ContainersIntent::RunFinished {
                engine: docker().address().clone(),
                result: Err(ContainersError::new("boom")),
            },
        );

        assert_eq!(
            run_form(&state).notice(),
            Some(
                i18n::containers_action_failed(
                    Key::TplContainersRunFailed,
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
            ContainersIntent::RunFinished {
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
        let _ = reduce(&mut state, ContainersIntent::RunSubmitted);

        let effects =
            effects(reduce(&mut state, ContainersIntent::RunCancelled));

        assert!(state.run_form().is_none());
        assert_eq!(effects, vec![ContainersEffect::CancelPull]);
    }

    #[test]
    fn cancel_before_a_run_only_closes_the_form() {
        let mut state = form_with("nginx:9", None);

        let effects =
            effects(reduce(&mut state, ContainersIntent::RunCancelled));

        assert!(state.run_form().is_none());
        assert!(effects.is_empty());
    }

    #[test]
    fn opening_another_list_closes_the_form() {
        let mut state = form_with("nginx:9", None);

        let _ = reduce(
            &mut state,
            ContainersIntent::Open(Place::new(
                docker().address().clone(),
                ResourceKind::Images,
            )),
        );

        assert!(state.run_form().is_none());
    }

    #[test]
    fn the_pull_dialog_keeps_its_own_pull_results() {
        let mut state = state_on(&[EngineKind::Docker], ResourceKind::Images);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = reduce(
            &mut state,
            ContainersIntent::FormChanged(
                crate::widgets::containers::forms::FormField::Name,
                String::from("nginx"),
            ),
        );
        let _ = reduce(&mut state, ContainersIntent::FormSubmitted);

        let _ = reduce(
            &mut state,
            ContainersIntent::PullProgressed(fixtures::pull_progress()),
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
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = type_in(&mut state, RunEdit::Image(String::from("nginx")));
        let _ = type_in(&mut state, RunEdit::Network(String::from("none")));

        let effects = effects(reduce(
            &mut state,
            ContainersIntent::RunEngineChosen(podman().address().clone()),
        ));

        let form = run_form(&state);
        assert_eq!(form.engine(), &podman());
        assert_eq!(form.network(), "bridge");
        assert_eq!(
            effects,
            vec![ContainersEffect::LoadImageDefaults {
                engine: podman(),
                reference: String::from("nginx"),
            }]
        );
    }

    #[test]
    fn a_failed_lookup_shows_the_engines_message() {
        let mut state =
            state_on(&[EngineKind::Docker], ResourceKind::Containers);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = type_in(&mut state, RunEdit::Image(String::from("nginx")));

        let _ = reduce(
            &mut state,
            ContainersIntent::ImageDefaultsLoaded {
                engine: docker().address().clone(),
                reference: String::from("nginx"),
                result: Err(ContainersError::new("store is locked")),
            },
        );

        assert_eq!(
            run_form(&state).lookup(),
            &ImageLookup::Failed(String::from("store is locked"))
        );
        assert!(can_run(run_form(&state)));
    }

    /// The effect that asks about the folder `path`.
    fn asks_about(path: &str) -> ContainersEffect {
        ContainersEffect::CheckFolder(PathBuf::from(path))
    }

    /// The effect that waits for a pause in typing `path`.
    fn waits_for_typing(path: &str) -> ContainersEffect {
        ContainersEffect::WaitForTypingPause(PathBuf::from(path))
    }

    /// The effect that gives the disk time to answer about `path`.
    fn waits_for_answer(path: &str) -> ContainersEffect {
        ContainersEffect::WaitForLateAnswer(PathBuf::from(path))
    }

    /// Add a volume row to `/data` with `source` typed last; the effects of
    /// typing it.
    fn add_volume(
        state: &mut ContainersState,
        source: &str,
    ) -> Vec<ContainersEffect> {
        let index = run_form(state).volumes().len();
        let _ = type_in(state, RunEdit::AddVolume);
        let _ =
            type_in(state, RunEdit::VolumeTarget(index, String::from("/data")));

        type_in(state, RunEdit::VolumeSource(index, source.to_string()))
    }

    fn answer(state: &mut ContainersState, path: &str, exists: bool) {
        let effects = effects(reduce(
            state,
            ContainersIntent::FolderChecked {
                path: PathBuf::from(path),
                exists,
            },
        ));

        assert!(effects.is_empty(), "{effects:?}");
    }

    fn recheck(state: &mut ContainersState) -> Vec<ContainersEffect> {
        effects(reduce(state, ContainersIntent::RecheckFolders))
    }

    /// Typing stopped on `path`; the effects of that.
    fn pause_typing(
        state: &mut ContainersState,
        path: &str,
    ) -> Vec<ContainersEffect> {
        effects(reduce(
            state,
            ContainersIntent::FolderTypingPaused(PathBuf::from(path)),
        ))
    }

    /// The time for an answer about `path` is over.
    fn answer_is_late(state: &mut ContainersState, path: &str) {
        let effects = effects(reduce(
            state,
            ContainersIntent::FolderAnswerLate(PathBuf::from(path)),
        ));

        assert!(effects.is_empty(), "{effects:?}");
    }

    /// A form with a row from `path` and a question about it out, with no
    /// answer yet.
    fn asked_form(path: &str) -> ContainersState {
        let mut state = ready_form();
        let _ = add_volume(&mut state, path);
        let _ = pause_typing(&mut state, path);
        state
    }

    fn ready_form() -> ContainersState {
        form_with("nginx:1.27", Some(fixtures::image_defaults()))
    }

    #[test]
    fn typing_a_folder_sends_the_wait_and_not_the_question() {
        let mut state = ready_form();

        let typed = add_volume(&mut state, "/srv/site");

        assert_eq!(typed, vec![waits_for_typing("/srv/site")]);
    }

    #[test]
    fn a_pause_for_the_typed_folder_asks_once_and_not_again_while_in_flight() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/site");

        let paused = pause_typing(&mut state, "/srv/site");
        let paused_again = pause_typing(&mut state, "/srv/site");
        let retyped = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from(" /srv/site ")),
        );
        let target =
            type_in(&mut state, RunEdit::VolumeTarget(0, String::from("/d")));

        assert_eq!(paused.len(), 2, "{paused:?}");
        assert!(paused.contains(&asks_about("/srv/site")), "{paused:?}");
        assert!(
            paused.contains(&waits_for_answer("/srv/site")),
            "{paused:?}"
        );
        assert!(paused_again.is_empty(), "{paused_again:?}");
        assert!(retyped.is_empty(), "{retyped:?}");
        assert!(target.is_empty(), "{target:?}");
    }

    #[test]
    fn a_pause_for_a_folder_the_text_has_left_is_dropped() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/sit");
        let _ = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from("/srv/site")),
        );

        let paused = pause_typing(&mut state, "/srv/sit");

        assert!(paused.is_empty(), "{paused:?}");
    }

    #[test]
    fn a_typed_path_costs_one_question_not_one_per_character() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/s");
        for text in ["/sr", "/srv"] {
            let ticked = recheck(&mut state);
            let _ =
                type_in(&mut state, RunEdit::VolumeSource(0, text.to_string()));

            assert!(ticked.is_empty(), "the timer asked: {ticked:?}");
        }
        let ticked = recheck(&mut state);
        assert!(ticked.is_empty(), "the timer asked: {ticked:?}");

        let questions: Vec<ContainersEffect> = ["/s", "/sr", "/srv"]
            .into_iter()
            .flat_map(|text| pause_typing(&mut state, text))
            .filter(|effect| matches!(effect, ContainersEffect::CheckFolder(_)))
            .collect();

        assert_eq!(questions, vec![asks_about("/srv")]);
    }

    #[test]
    fn a_pause_without_a_form_asks_nothing() {
        let mut state = state_on(&[EngineKind::Docker], ResourceKind::Images);

        let paused = pause_typing(&mut state, "/srv/site");

        assert!(paused.is_empty(), "{paused:?}");
    }

    #[test]
    fn a_pause_for_a_folder_with_an_answer_asks_nothing() {
        let mut state = asked_form("/srv/site");
        answer(&mut state, "/srv/site", true);

        let paused = pause_typing(&mut state, "/srv/site");

        assert!(paused.is_empty(), "{paused:?}");
    }

    #[test]
    fn typing_a_named_volume_or_nothing_sends_nothing() {
        let mut state = ready_form();

        let named = add_volume(&mut state, "otty-fx-cache");
        let blank =
            type_in(&mut state, RunEdit::VolumeSource(0, String::from("   ")));

        assert!(named.is_empty(), "{named:?}");
        assert!(blank.is_empty(), "{blank:?}");
    }

    #[test]
    fn each_new_text_of_a_folder_waits_for_its_own_pause() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv");

        let next = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from("/srv/site")),
        );

        assert_eq!(next, vec![waits_for_typing("/srv/site")]);
    }

    #[test]
    fn run_stays_disabled_until_the_folder_is_answered() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/site");

        let pressed =
            effects(reduce(&mut state, ContainersIntent::RunSubmitted));

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
            Some(Key::ContainersRunErrSourceMissing)
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
            Some(Key::ContainersRunErrSourceMissing)
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

        let _ = add_volume(&mut state, "/srv/site");
        let _ = add_volume(&mut state, "/srv/site");
        let first = pause_typing(&mut state, "/srv/site");
        let second = pause_typing(&mut state, "/srv/site");
        answer(&mut state, "/srv/site", false);

        assert!(first.contains(&asks_about("/srv/site")), "{first:?}");
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
        let _ = pause_typing(&mut state, "/srv/in-flight");
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
        let pressed =
            effects(reduce(&mut state, ContainersIntent::RunSubmitted));
        assert!(matches!(
            pressed.as_slice(),
            [ContainersEffect::RunContainer { .. }]
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
    fn a_pause_lost_on_its_way_is_made_up_by_the_recheck() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/site");

        let first_tick = recheck(&mut state);
        let second_tick = recheck(&mut state);
        let third_tick = recheck(&mut state);

        assert!(first_tick.is_empty(), "the pause may still come");
        assert_eq!(second_tick.len(), 2, "{second_tick:?}");
        assert!(second_tick.contains(&asks_about("/srv/site")));
        assert!(second_tick.contains(&waits_for_answer("/srv/site")));
        assert!(third_tick.is_empty(), "{third_tick:?}");
    }

    #[test]
    fn a_pause_that_comes_after_a_tick_still_asks() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/site");
        let _ = recheck(&mut state);

        let paused = pause_typing(&mut state, "/srv/site");

        assert!(paused.contains(&asks_about("/srv/site")), "{paused:?}");
    }

    #[test]
    fn a_tick_during_typing_skips_only_the_row_being_typed() {
        let mut state = asked_form("/srv/answered");
        answer(&mut state, "/srv/answered", true);
        let _ = add_volume(&mut state, "/srv/typ");

        let ticked = recheck(&mut state);

        assert_eq!(ticked, vec![asks_about("/srv/answered")]);
    }

    #[test]
    fn a_second_row_joining_a_hung_recheck_gets_the_hint_in_time() {
        let mut state = asked_form("/srv/site");
        answer(&mut state, "/srv/site", true);
        let asked = recheck(&mut state);
        assert_eq!(asked, vec![asks_about("/srv/site")]);

        let joined = add_volume(&mut state, "/srv/site");
        let retyped = type_in(
            &mut state,
            RunEdit::VolumeSource(1, String::from(" /srv/site ")),
        );
        let ticked = recheck(&mut state);

        assert_eq!(joined, vec![waits_for_answer("/srv/site")]);
        assert!(retyped.is_empty(), "{retyped:?}");
        assert!(ticked.is_empty(), "a hung folder piles up: {ticked:?}");
        assert!(!can_run(run_form(&state)));

        answer_is_late(&mut state, "/srv/site");

        assert_eq!(row_message(&state, 0), None);
        assert_eq!(
            row_message(&state, 1),
            Some(Key::ContainersRunErrSourceNotAnswering)
        );

        answer(&mut state, "/srv/site", true);

        assert_eq!(row_message(&state, 1), None);
        assert!(can_run(run_form(&state)));
    }

    #[test]
    fn an_old_time_limit_shows_nothing_before_the_new_question_goes_out() {
        let mut state = asked_form("/srv/site");
        for text in ["/srv/other", "/srv/site"] {
            let _ =
                type_in(&mut state, RunEdit::VolumeSource(0, text.to_string()));
        }

        answer_is_late(&mut state, "/srv/site");

        assert_eq!(row_message(&state, 0), None);

        let paused = pause_typing(&mut state, "/srv/site");
        answer_is_late(&mut state, "/srv/site");

        assert!(
            paused.contains(&waits_for_answer("/srv/site")),
            "{paused:?}"
        );
        assert_eq!(
            row_message(&state, 0),
            Some(Key::ContainersRunErrSourceNotAnswering)
        );
    }

    #[test]
    fn an_old_time_limit_shows_nothing_in_a_reopened_form() {
        let mut state = asked_form("/srv/site");
        let _ = reduce(&mut state, ContainersIntent::RunCancelled);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = add_volume(&mut state, "/srv/site");

        answer_is_late(&mut state, "/srv/site");

        assert_eq!(row_message(&state, 0), None);
    }

    /// The key of the problem shown under the volume row at `index`.
    fn row_message(state: &ContainersState, index: usize) -> Option<Key> {
        check(run_form(state)).volume(index).map(|p| p.message())
    }

    #[test]
    fn a_folder_with_no_answer_in_time_shows_the_hint_and_run_stays_disabled() {
        let mut state = asked_form("/srv/site");
        assert_eq!(row_message(&state, 0), None);

        answer_is_late(&mut state, "/srv/site");

        assert_eq!(
            row_message(&state, 0),
            Some(Key::ContainersRunErrSourceNotAnswering)
        );
        assert!(!can_run(run_form(&state)));
        let pressed =
            effects(reduce(&mut state, ContainersIntent::RunSubmitted));
        assert!(pressed.is_empty(), "{pressed:?}");
    }

    #[test]
    fn a_late_answer_about_a_folder_the_text_has_left_shows_nothing() {
        let mut state = asked_form("/srv/old");
        let _ = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from("/srv/new")),
        );

        answer_is_late(&mut state, "/srv/old");

        assert_eq!(row_message(&state, 0), None);
        assert!(!can_run(run_form(&state)), "/srv/new has no answer yet");
    }

    #[test]
    fn a_late_answer_after_the_answer_shows_nothing() {
        let mut state = asked_form("/srv/site");
        answer(&mut state, "/srv/site", true);

        answer_is_late(&mut state, "/srv/site");

        assert_eq!(row_message(&state, 0), None);
        assert!(can_run(run_form(&state)));
    }

    #[test]
    fn a_late_answer_without_a_form_is_dropped() {
        let mut state = state_on(&[EngineKind::Docker], ResourceKind::Images);

        answer_is_late(&mut state, "/srv/site");

        assert!(state.run_form().is_none());
    }

    #[test]
    fn the_answer_clears_the_hint() {
        let mut state = asked_form("/srv/site");
        answer_is_late(&mut state, "/srv/site");

        answer(&mut state, "/srv/site", true);

        assert_eq!(row_message(&state, 0), None);
        assert!(can_run(run_form(&state)));
    }

    #[test]
    fn a_missing_answer_replaces_the_hint() {
        let mut state = asked_form("/srv/site");
        answer_is_late(&mut state, "/srv/site");

        answer(&mut state, "/srv/site", false);

        assert_eq!(
            row_message(&state, 0),
            Some(Key::ContainersRunErrSourceMissing)
        );
    }

    #[test]
    fn changing_the_text_clears_the_hint_and_waits_again() {
        let mut state = asked_form("/srv/site");
        answer_is_late(&mut state, "/srv/site");

        let typed = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from("/srv/other")),
        );

        assert_eq!(row_message(&state, 0), None);
        assert_eq!(typed, vec![waits_for_typing("/srv/other")]);
        assert!(!can_run(run_form(&state)));
    }

    #[test]
    fn blanks_around_the_same_folder_keep_the_hint() {
        let mut state = asked_form("/srv/site");
        answer_is_late(&mut state, "/srv/site");

        let _ = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from(" /srv/site ")),
        );

        assert_eq!(
            row_message(&state, 0),
            Some(Key::ContainersRunErrSourceNotAnswering)
        );
    }

    #[test]
    fn the_hint_goes_before_a_missing_answer_about_earlier_text() {
        let mut state = asked_form("/srv/sit");
        answer(&mut state, "/srv/sit", false);
        let _ = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from("/srv/site")),
        );
        let _ = pause_typing(&mut state, "/srv/site");
        assert_eq!(
            row_message(&state, 0),
            Some(Key::ContainersRunErrSourceMissing)
        );

        answer_is_late(&mut state, "/srv/site");

        assert_eq!(
            row_message(&state, 0),
            Some(Key::ContainersRunErrSourceNotAnswering)
        );
        assert!(!can_run(run_form(&state)));
    }

    #[test]
    fn a_hung_folder_is_not_asked_about_again_while_the_hint_shows() {
        let mut state = asked_form("/srv/site");
        answer_is_late(&mut state, "/srv/site");

        // The first tick also spends the mark of the typing wait, so only the
        // question in flight can keep the second one quiet.
        let first_tick = recheck(&mut state);
        let second_tick = recheck(&mut state);

        assert!(first_tick.is_empty(), "{first_tick:?}");
        assert!(second_tick.is_empty(), "{second_tick:?}");
    }

    #[test]
    fn the_recheck_of_an_answered_folder_sends_no_wait_and_shows_no_hint() {
        let mut state = asked_form("/srv/site");
        answer(&mut state, "/srv/site", true);

        let asked = recheck(&mut state);
        answer_is_late(&mut state, "/srv/site");

        assert_eq!(asked, vec![asks_about("/srv/site")]);
        assert_eq!(row_message(&state, 0), None);
        assert!(can_run(run_form(&state)));
    }

    #[test]
    fn entering_containers_mode_asks_again_about_the_folders_of_the_open_form()
    {
        let mut state = asked_form("/srv/site");
        answer(&mut state, "/srv/site", true);
        let _ = reduce(&mut state, ContainersIntent::Leave);

        let entered = effects(reduce(&mut state, ContainersIntent::Enter));

        assert_eq!(entered.len(), 2, "{entered:?}");
        assert!(entered.contains(&ContainersEffect::Discover), "{entered:?}");
        assert!(entered.contains(&asks_about("/srv/site")), "{entered:?}");
    }

    #[test]
    fn entering_containers_mode_skips_a_folder_that_is_in_flight() {
        let mut state = asked_form("/srv/site");
        // This tick spends the mark of the typing wait, so only the question
        // in flight can keep Enter from asking.
        let _ = recheck(&mut state);
        let _ = reduce(&mut state, ContainersIntent::Leave);

        let entered = effects(reduce(&mut state, ContainersIntent::Enter));

        assert_eq!(entered, vec![ContainersEffect::Discover]);
    }

    #[test]
    fn entering_containers_mode_leaves_a_folder_being_typed_to_its_pause() {
        let mut state = ready_form();
        let _ = add_volume(&mut state, "/srv/site");
        let _ = reduce(&mut state, ContainersIntent::Leave);

        let entered = effects(reduce(&mut state, ContainersIntent::Enter));

        assert_eq!(entered, vec![ContainersEffect::Discover]);
    }

    #[test]
    fn entering_containers_mode_without_a_form_asks_about_no_folder() {
        let mut state = state_on(&[EngineKind::Docker], ResourceKind::Images);
        let _ = reduce(&mut state, ContainersIntent::Leave);

        let entered = effects(reduce(&mut state, ContainersIntent::Enter));

        assert_eq!(entered, vec![ContainersEffect::Discover]);
    }

    #[test]
    fn the_form_with_its_answers_stays_while_another_mode_is_active() {
        let mut state = asked_form("/srv/site");
        answer(&mut state, "/srv/site", true);

        let _ = reduce(&mut state, ContainersIntent::Leave);

        assert!(state.run_form().is_some());
        assert!(can_run(run_form(&state)));
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
            let _ = reduce(state, ContainersIntent::OpenCreate);
            let _ = add_volume(state, "/srv/site");
        }

        assert!(run_form(&with_selinux).offers_relabel(0));
        assert!(!run_form(&without).offers_relabel(0));
    }

    /// Open the Run form after `discovery` and add a row from `~/site`.
    fn form_with_home_source(discovery: Discovery) -> ContainersState {
        let kinds = [EngineKind::Docker];
        let mut state =
            state_after(discovery, &kinds, ResourceKind::Containers);

        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = type_in(&mut state, RunEdit::Image(String::from("nginx:1.27")));
        let _ = reduce(
            &mut state,
            ContainersIntent::ImageDefaultsLoaded {
                engine: docker().address().clone(),
                reference: String::from("nginx:1.27"),
                result: Ok(Some(fixtures::image_defaults())),
            },
        );
        let _ = add_volume(&mut state, "~/site");

        state
    }

    #[test]
    fn the_form_uses_the_home_folder_of_the_search() {
        let state =
            form_with_home_source(fixtures::discovery(&[EngineKind::Docker]));

        assert_eq!(
            run_form(&state).volumes()[0].host_path(),
            Some(Path::new(fixtures::HOME_FOLDER).join("site"))
        );
    }

    #[test]
    fn a_form_opened_during_a_later_search_keeps_the_home_folder() {
        let kinds = [EngineKind::Docker];
        let mut state = state_on(&kinds, ResourceKind::Containers);

        let _ = reduce(&mut state, ContainersIntent::CheckAgain);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = add_volume(&mut state, "~/site");

        assert_eq!(
            run_form(&state).volumes()[0].host_path(),
            Some(Path::new(fixtures::HOME_FOLDER).join("site"))
        );
    }

    #[test]
    fn a_later_search_without_a_home_folder_forgets_the_old_one() {
        let kinds = [EngineKind::Docker];
        let mut state = state_on(&kinds, ResourceKind::Containers);

        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(
                fixtures::discovery_without_home(&kinds),
            ),
        );

        assert_eq!(state.home_folder(), None);
    }

    #[test]
    fn a_home_source_without_a_home_folder_asks_the_disk_nothing() {
        let kinds = [EngineKind::Docker];
        let mut state = state_after(
            fixtures::discovery_without_home(&kinds),
            &kinds,
            ResourceKind::Containers,
        );
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        let typed = add_volume(&mut state, "~/site");
        let rechecked = recheck(&mut state);

        assert!(typed.is_empty(), "{typed:?}");
        assert!(rechecked.is_empty(), "{rechecked:?}");
    }

    #[test]
    fn a_home_source_without_a_home_folder_never_runs() {
        let mut state =
            form_with_home_source(fixtures::discovery_without_home(&[
                EngineKind::Docker,
            ]));

        let pressed =
            effects(reduce(&mut state, ContainersIntent::RunSubmitted));

        assert!(!can_run(run_form(&state)));
        assert!(pressed.is_empty(), "{pressed:?}");
        assert_eq!(run_form(&state).phase(), &RunPhase::Editing);
    }

    #[test]
    fn a_full_path_clears_the_problem_of_a_home_source() {
        let mut state =
            form_with_home_source(fixtures::discovery_without_home(&[
                EngineKind::Docker,
            ]));

        let _ = type_in(
            &mut state,
            RunEdit::VolumeSource(0, String::from("/srv/site")),
        );
        answer(&mut state, "/srv/site", true);

        assert!(can_run(run_form(&state)));
    }

    #[test]
    fn a_form_opened_during_a_later_search_keeps_the_selinux_fact() {
        let kinds = [EngineKind::Docker];
        let mut state = state_after(
            fixtures::discovery_with_selinux(&kinds),
            &kinds,
            ResourceKind::Containers,
        );

        let _ = reduce(&mut state, ContainersIntent::CheckAgain);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = add_volume(&mut state, "/srv/site");

        assert!(run_form(&state).offers_relabel(0));
    }
}
