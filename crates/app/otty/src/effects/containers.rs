//! The engine and disk work of the containers feature: the engine search,
//! the list loads, the actions, the pulls with their abort handles, the Run
//! form (image defaults, the run, the folder checks and the two waits around
//! a folder check), the inspect answer and the run of the followed logs, the
//! clipboard and the browser. Each runs off the update, and its answer comes
//! back as a containers intent.

#![allow(
    clippy::disallowed_methods,
    reason = "effects/ is where the binary talks to the engines and starts the browser"
)]

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use iced::Task;
use iced::futures::future::join3;
use iced::futures::{SinkExt, Stream, StreamExt};
use otty_app_containers::{
    ActionRequest, ActionsIntent, ContainersError, ContainersIntent,
    CreateIntent, EngineAction, InspectTarget, InspectorIntent, LoadedList,
    LoadedLists, LogTarget, PullHandle, ResourceKind, RunIntent,
};
use otty_containers::{
    Container, ContainerRun, Engine, EngineError, EngineInfo, ImageDefaults,
    PullProgress, RunRequest, discover, host_folder_exists, wait_for,
};

use crate::message::AppEvent;

/// How long typing must stop before a typed folder is asked about, so a typed
/// path costs one question and not one per character.
const FOLDER_TYPING_PAUSE: Duration = Duration::from_millis(300);

/// How long the disk has to answer before the row says the folder is not
/// answering.
const FOLDER_ANSWER_LIMIT: Duration = Duration::from_secs(3);

/// What one pull reports, before it is turned into an intent of the pull
/// dialog or of the Run form, whichever asked for the pull.
enum PullUpdate {
    /// The pull started; the dialog or the form keeps its abort handle.
    Started(PullHandle),
    /// The pull got further.
    Progressed(PullProgress),
    /// The pull ended; the error is the engine's message.
    Finished(Result<(), ContainersError>),
}

/// Search for the local engines; the result comes back as
/// [`ContainersIntent::DiscoveryFinished`].
pub(crate) fn discover_engines() -> Task<AppEvent> {
    Task::perform(discover(), |discovery| {
        intent(ContainersIntent::DiscoveryFinished(discovery))
    })
}

/// The same search behind the no-engine card; the result comes back as
/// [`ContainersIntent::BackgroundSearchFinished`].
pub(crate) fn discover_in_background() -> Task<AppEvent> {
    Task::perform(discover(), |discovery| {
        intent(ContainersIntent::BackgroundSearchFinished(discovery))
    })
}

/// Load the four lists of `info`; they come back as
/// [`ContainersIntent::ListsLoaded`].
pub(crate) fn load_lists(info: EngineInfo) -> Task<AppEvent> {
    let engine = info.address().clone();

    Task::perform(read_lists(info), move |lists| {
        intent(ContainersIntent::ListsLoaded {
            engine: engine.clone(),
            lists,
        })
    })
}

/// Load one list of `info` again, after an engine event; it comes back as
/// [`ContainersIntent::ListLoaded`]. "Used by" of images, volumes and
/// networks is read from `containers`.
pub(crate) fn load_list(
    info: EngineInfo,
    resource: ResourceKind,
    containers: Vec<Container>,
) -> Task<AppEvent> {
    let engine = info.address().clone();

    Task::perform(read_list(info, resource, containers), move |list| {
        intent(ContainersIntent::ListLoaded {
            engine: engine.clone(),
            list,
        })
    })
}

/// Put `text` on the clipboard.
pub(crate) fn copy_to_clipboard(text: String) -> Task<AppEvent> {
    iced::clipboard::write(text)
}

/// Open `url` in the default browser. Whether the browser could be started
/// comes back as [`ContainersIntent::LinkOpened`].
pub(crate) fn open_url(url: String) -> Task<AppEvent> {
    Task::perform(async move { start_browser(&url) }, |result| {
        intent(ContainersIntent::LinkOpened(result))
    })
}

/// Make the engine call of an action; the result comes back as
/// [`ActionsIntent::Finished`].
pub(crate) fn run_action(request: ActionRequest) -> Task<AppEvent> {
    Task::perform(call_engine(request.clone()), move |result| {
        intent(ContainersIntent::Actions(ActionsIntent::Finished {
            request: request.clone(),
            result,
        }))
    })
}

/// Pull `image` at `tag` on `engine` for the pull dialog: its abort handle,
/// its progress and its end come back as intents of the create dialogs.
pub(crate) fn pull_image(
    engine: EngineInfo,
    image: String,
    tag: String,
) -> Task<AppEvent> {
    start_pull(engine, image, tag, create_pull)
}

/// Pull `image` at `tag` on `engine` for the Run form: its abort handle, its
/// progress and its end come back as intents of the Run form.
pub(crate) fn pull_run_image(
    engine: EngineInfo,
    image: String,
    tag: String,
) -> Task<AppEvent> {
    start_pull(engine, image, tag, run_pull)
}

/// Stop a running pull through its abort handle.
pub(crate) fn cancel_pull(pull: &PullHandle) -> Task<AppEvent> {
    pull.abort();

    Task::none()
}

/// Ask `engine` what the image `reference` sets on its own; the answer comes
/// back as [`RunIntent::ImageDefaultsLoaded`].
pub(crate) fn load_image_defaults(
    engine: EngineInfo,
    reference: String,
) -> Task<AppEvent> {
    let address = engine.address().clone();
    let asked = reference.clone();

    Task::perform(read_image_defaults(engine, reference), move |result| {
        run_intent(RunIntent::ImageDefaultsLoaded {
            engine: address.clone(),
            reference: asked.clone(),
            result,
        })
    })
}

/// Create and start a container on `engine`; its id or the engine's message
/// comes back as [`RunIntent::Finished`].
pub(crate) fn run_container(
    engine: EngineInfo,
    request: RunRequest,
) -> Task<AppEvent> {
    let address = engine.address().clone();

    Task::perform(start_container(engine, request), move |result| {
        run_intent(RunIntent::Finished {
            engine: address.clone(),
            result,
        })
    })
}

/// Ask the disk whether `path`, a folder of the Run form, exists; the answer
/// comes back as [`RunIntent::FolderChecked`].
pub(crate) fn check_folder(path: PathBuf) -> Task<AppEvent> {
    let asked = path.clone();

    Task::perform(
        async move { host_folder_exists(&path).await },
        move |exists| {
            run_intent(RunIntent::FolderChecked {
                path: asked,
                exists,
            })
        },
    )
}

/// Wait for a pause in typing, then send [`RunIntent::FolderTypingPaused`].
pub(crate) fn wait_for_typing_pause(path: PathBuf) -> Task<AppEvent> {
    folder_wait(FOLDER_TYPING_PAUSE, RunIntent::FolderTypingPaused(path))
}

/// Give the disk time to answer, then send [`RunIntent::FolderAnswerLate`].
pub(crate) fn wait_for_late_answer(path: PathBuf) -> Task<AppEvent> {
    folder_wait(FOLDER_ANSWER_LIMIT, RunIntent::FolderAnswerLate(path))
}

/// Load the inspect JSON of `target` from `engine`; it comes back as
/// [`InspectorIntent::InspectLoaded`].
pub(crate) fn load_inspect(
    engine: EngineInfo,
    target: InspectTarget,
) -> Task<AppEvent> {
    let shown = target.clone();

    Task::perform(read_inspect(engine, target), move |result| {
        inspector_intent(InspectorIntent::InspectLoaded {
            target: shown.clone(),
            result,
        })
    })
}

/// Ask what became of the container of the logs `target`, after its engine
/// reconnected; the answer comes back as [`InspectorIntent::LogRunChecked`].
pub(crate) fn check_log_run(target: LogTarget) -> Task<AppEvent> {
    let asked = target.clone();

    Task::perform(read_log_run(target), move |result| {
        inspector_intent(InspectorIntent::LogRunChecked {
            target: asked.clone(),
            result,
        })
    })
}

/// The error of an engine as the containers feature shows it: the engine's
/// own words when it refused a request, otherwise the whole error text.
fn containers_error(err: EngineError) -> ContainersError {
    ContainersError::new(err.message())
}

/// Load the four lists of one engine. The containers come first, because
/// "used by" of images, volumes and networks is read from them; then the
/// other three load at the same time. Each list keeps its own error, so one
/// failed list does not hide the others.
async fn read_lists(info: EngineInfo) -> LoadedLists {
    let engine = match Engine::connect(&info) {
        Ok(engine) => engine,
        Err(err) => return LoadedLists::failed(&containers_error(err)),
    };

    let containers = engine.containers().await.map_err(containers_error);

    let users = containers.as_deref().unwrap_or_default();
    let (images, volumes, networks) = join3(
        engine.images(users),
        engine.volumes(users),
        engine.networks(users),
    )
    .await;

    LoadedLists {
        images: images.map_err(containers_error),
        volumes: volumes.map_err(containers_error),
        networks: networks.map_err(containers_error),
        containers,
    }
}

/// Load one list of one engine again. "Used by" of images, volumes and
/// networks is read from `containers`.
async fn read_list(
    info: EngineInfo,
    resource: ResourceKind,
    containers: Vec<Container>,
) -> LoadedList {
    let engine = match Engine::connect(&info) {
        Ok(engine) => engine,
        Err(err) => return failed_list(resource, containers_error(err)),
    };

    match resource {
        ResourceKind::Containers => LoadedList::Containers(
            engine.containers().await.map_err(containers_error),
        ),
        ResourceKind::Images => LoadedList::Images(
            engine.images(&containers).await.map_err(containers_error),
        ),
        ResourceKind::Volumes => LoadedList::Volumes(
            engine.volumes(&containers).await.map_err(containers_error),
        ),
        ResourceKind::Networks => LoadedList::Networks(
            engine.networks(&containers).await.map_err(containers_error),
        ),
    }
}

/// Pull `image` at `tag` on `engine` and hand the pull's abort handle to the
/// form that asked for the pull, which keeps it in its state; `to_event`
/// turns the handle and each update into that form's intent.
fn start_pull(
    engine: EngineInfo,
    image: String,
    tag: String,
    to_event: fn(PullUpdate) -> AppEvent,
) -> Task<AppEvent> {
    let (pull, handle) =
        Task::run(pull_updates(engine, image, tag), to_event).abortable();

    let started = to_event(PullUpdate::Started(PullHandle::new(handle)));
    Task::batch([Task::done(started), pull])
}

/// An update of the Run form's pull, as an intent of the Run form.
fn run_pull(update: PullUpdate) -> AppEvent {
    let run = match update {
        PullUpdate::Started(pull) => RunIntent::PullStarted(pull),
        PullUpdate::Progressed(progress) => RunIntent::PullProgressed(progress),
        PullUpdate::Finished(result) => RunIntent::PullFinished(result),
    };

    run_intent(run)
}

/// An update of the pull dialog's pull, as an intent of the create dialogs.
fn create_pull(update: PullUpdate) -> AppEvent {
    let create = match update {
        PullUpdate::Started(pull) => CreateIntent::PullStarted(pull),
        PullUpdate::Progressed(progress) => {
            CreateIntent::PullProgressed(progress)
        },
        PullUpdate::Finished(result) => CreateIntent::PullFinished(result),
    };

    intent(ContainersIntent::Create(create))
}

/// Send `intent` to the Run form after `delay`, without holding a thread.
fn folder_wait(delay: Duration, intent: RunIntent) -> Task<AppEvent> {
    Task::perform(wait_for(delay), move |()| run_intent(intent))
}

/// The updates of one pull: one `Progressed` per line of the engine, then
/// one `Finished` (with the engine's message on failure). Dropping the
/// stream (Cancel) closes the request, which stops the pull.
fn pull_updates(
    info: EngineInfo,
    image: String,
    tag: String,
) -> impl Stream<Item = PullUpdate> {
    iced::stream::channel(16, async move |mut output| {
        let engine = match Engine::connect(&info) {
            Ok(engine) => engine,
            Err(err) => {
                let _ = output
                    .send(PullUpdate::Finished(Err(containers_error(err))))
                    .await;
                return;
            },
        };

        let mut updates = engine.pull(&image, &tag);
        while let Some(update) = updates.next().await {
            let next = match update {
                Ok(progress) => PullUpdate::Progressed(progress),
                Err(err) => {
                    let _ = output
                        .send(PullUpdate::Finished(Err(containers_error(err))))
                        .await;
                    return;
                },
            };
            let _ = output.send(next).await;
        }

        let _ = output.send(PullUpdate::Finished(Ok(()))).await;
    })
}

/// Make the engine call of one action. The error is the engine's own
/// message, for the notification.
async fn call_engine(request: ActionRequest) -> Result<(), ContainersError> {
    let engine = Engine::connect(request.engine()).map_err(containers_error)?;

    let result = match request.action() {
        EngineAction::StartContainer(id) => engine.start_container(id).await,
        EngineAction::StopContainer(id) => engine.stop_container(id).await,
        EngineAction::RestartContainer(id) => {
            engine.restart_container(id).await
        },
        EngineAction::PauseContainer(id) => engine.pause_container(id).await,
        EngineAction::UnpauseContainer(id) => {
            engine.unpause_container(id).await
        },
        EngineAction::RemoveContainer { id, force } => {
            engine.remove_container(id, *force).await
        },
        EngineAction::PruneContainers => engine.prune_containers().await,
        EngineAction::RemoveImage { reference, force } => {
            engine.remove_image(reference, *force).await
        },
        EngineAction::PruneImages => engine.prune_images().await,
        EngineAction::CreateVolume { name, driver } => {
            engine.create_volume(name, driver).await
        },
        EngineAction::RemoveVolume(name) => engine.remove_volume(name).await,
        EngineAction::PruneVolumes { include_named } => {
            engine.prune_volumes(*include_named).await
        },
        EngineAction::CreateNetwork { name, driver } => {
            engine.create_network(name, driver).await
        },
        EngineAction::RemoveNetwork(id) => engine.remove_network(id).await,
        EngineAction::PruneNetworks => engine.prune_networks().await,
    };

    result.map_err(containers_error)
}

/// Ask the engine what an image sets on its own; `None` when the image is
/// not on this computer. The error is the engine's message.
async fn read_image_defaults(
    info: EngineInfo,
    reference: String,
) -> Result<Option<ImageDefaults>, ContainersError> {
    let engine = Engine::connect(&info).map_err(containers_error)?;

    engine
        .image_defaults(&reference)
        .await
        .map_err(containers_error)
}

/// Create and start a container; its id, or the engine's message.
async fn start_container(
    info: EngineInfo,
    request: RunRequest,
) -> Result<String, ContainersError> {
    let engine = Engine::connect(&info).map_err(containers_error)?;

    engine
        .run_container(&request)
        .await
        .map_err(containers_error)
}

/// Load the inspect JSON of one row; the error is the engine's message.
async fn read_inspect(
    info: EngineInfo,
    target: InspectTarget,
) -> Result<String, ContainersError> {
    let engine = Engine::connect(&info).map_err(containers_error)?;

    let id = target.id();
    let json = match target.resource() {
        ResourceKind::Containers => engine.inspect_container(id).await,
        ResourceKind::Images => engine.inspect_image(id).await,
        ResourceKind::Volumes => engine.inspect_volume(id).await,
        ResourceKind::Networks => engine.inspect_network(id).await,
    };

    json.map_err(containers_error)
}

/// What became of the container of the logs `target`; the error is the
/// engine's message.
async fn read_log_run(
    target: LogTarget,
) -> Result<ContainerRun, ContainersError> {
    let engine = Engine::connect(target.engine()).map_err(containers_error)?;

    engine
        .container_run(target.container_id())
        .await
        .map_err(containers_error)
}

/// `resource` failed to load with `error`.
fn failed_list(resource: ResourceKind, error: ContainersError) -> LoadedList {
    match resource {
        ResourceKind::Containers => LoadedList::Containers(Err(error)),
        ResourceKind::Images => LoadedList::Images(Err(error)),
        ResourceKind::Volumes => LoadedList::Volumes(Err(error)),
        ResourceKind::Networks => LoadedList::Networks(Err(error)),
    }
}

/// Open `url` in the default browser with `xdg-open` (Linux) or `open`
/// (macOS). The error is why the opener could not be started; it is logged
/// too. An opener that starts but then fails reports later, on a helper
/// thread, and that is only logged.
fn start_browser(url: &str) -> Result<(), String> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    let spawned = Command::new(program)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    let mut child = spawned.map_err(|err| {
        log::warn!("opening {url} failed: cannot run {program}: {err}");
        format!("cannot run {program}: {err}")
    })?;

    // Wait on a thread so the finished opener does not stay a zombie.
    std::thread::spawn(move || match child.wait() {
        Ok(status) if !status.success() => {
            log::warn!("{program} failed to open a link: {status}");
        },
        Ok(_) => {},
        Err(err) => log::warn!("{program} wait failed: {err}"),
    });

    Ok(())
}

fn intent(intent: ContainersIntent) -> AppEvent {
    AppEvent::Containers(intent)
}

fn run_intent(intent: RunIntent) -> AppEvent {
    AppEvent::Containers(ContainersIntent::Run(intent))
}

fn inspector_intent(intent: InspectorIntent) -> AppEvent {
    AppEvent::Containers(ContainersIntent::Inspector(intent))
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use iced_test::runtime::Action;
    use iced_test::runtime::task::into_stream;
    use otty_containers::{EngineKind, fixtures};

    use super::*;

    /// Run a waiting task on the runtime the app runs on. Returns the intents
    /// it sent and how long it took.
    fn run_waiting_task(
        task: Task<AppEvent>,
    ) -> (Vec<ContainersIntent>, Duration) {
        let runtime = iced::executor::Default::new().expect("a tokio runtime");
        let stream = into_stream(task).expect("a task");

        let started = Instant::now();
        let actions: Vec<Action<AppEvent>> = runtime.block_on(stream.collect());
        let took = started.elapsed();

        let intents = actions
            .into_iter()
            .filter_map(|action| match action {
                Action::Output(AppEvent::Containers(intent)) => Some(intent),
                _ => None,
            })
            .collect();

        (intents, took)
    }

    /// A pull handle of a task that never runs.
    fn detached_pull() -> PullHandle {
        let (_, handle) = Task::<()>::none().abortable();

        PullHandle::new(handle)
    }

    #[test]
    fn loading_an_inspect_runs_one_engine_task() {
        let docker = fixtures::engine(EngineKind::Docker);
        let target = InspectTarget::new(
            docker.address().clone(),
            ResourceKind::Containers,
            "850b6438a531",
        );

        let inspect = load_inspect(docker, target);

        assert_eq!(inspect.units(), 1);
    }

    #[test]
    fn checking_the_run_of_open_logs_runs_one_engine_task() {
        let docker = fixtures::engine(EngineKind::Docker);

        let check = check_log_run(LogTarget::new(docker, "850b6438a531"));

        assert_eq!(check.units(), 1);
    }

    #[test]
    fn an_action_runs_one_engine_task() {
        let stop = run_action(ActionRequest::new(
            fixtures::engine(EngineKind::Docker),
            EngineAction::StopContainer(String::from("c1")),
            String::from("otty-fx-running"),
        ));

        assert_eq!(stop.units(), 1);
    }

    #[test]
    fn the_run_form_effects_run_one_engine_task_each() {
        let lookup = load_image_defaults(
            fixtures::engine(EngineKind::Docker),
            String::from("nginx:1.27"),
        );
        let run = run_container(
            fixtures::engine(EngineKind::Docker),
            RunRequest::new("nginx:1.27"),
        );

        assert_eq!(lookup.units(), 1);
        assert_eq!(run.units(), 1);
    }

    #[test]
    fn checking_a_folder_runs_one_task() {
        let check = check_folder(PathBuf::from("/srv/site"));

        assert_eq!(check.units(), 1);
    }

    #[test]
    fn the_folder_waits_run_one_task_each() {
        let typing = wait_for_typing_pause(PathBuf::from("/srv/site"));
        let late = wait_for_late_answer(PathBuf::from("/srv/site"));

        assert_eq!(typing.units(), 1);
        assert_eq!(late.units(), 1);
    }

    #[test]
    fn the_typing_wait_sends_the_pause_after_its_time() {
        let path = PathBuf::from("/srv/site");

        let (intents, took) =
            run_waiting_task(wait_for_typing_pause(path.clone()));

        assert_eq!(
            intents,
            vec![ContainersIntent::Run(RunIntent::FolderTypingPaused(path))]
        );
        assert!(took >= FOLDER_TYPING_PAUSE, "{took:?}");
        assert!(took < FOLDER_ANSWER_LIMIT, "{took:?}");
    }

    #[test]
    fn a_folder_wait_sends_its_intent_after_the_delay() {
        let next = RunIntent::FolderAnswerLate(PathBuf::from("/srv/site"));
        let delay = Duration::from_millis(5);

        let (intents, took) =
            run_waiting_task(folder_wait(delay, next.clone()));

        assert_eq!(intents, vec![ContainersIntent::Run(next)]);
        assert!(took >= delay, "{took:?}");
    }

    #[test]
    fn a_pull_runs_with_the_intent_that_hands_over_its_handle() {
        let run_pull = pull_run_image(
            fixtures::engine(EngineKind::Docker),
            String::from("nginx"),
            String::from("latest"),
        );
        let create_pull = pull_image(
            fixtures::engine(EngineKind::Docker),
            String::from("nginx"),
            String::from("latest"),
        );

        assert_eq!(run_pull.units(), 2);
        assert_eq!(create_pull.units(), 2);
    }

    #[test]
    fn cancelling_a_pull_aborts_its_handle() {
        let pull = detached_pull();

        let cancel = cancel_pull(&pull);

        assert_eq!(cancel.units(), 0);
        assert!(pull.is_aborted());
    }

    #[test]
    fn a_create_pull_reports_to_the_create_dialogs() {
        let handle = detached_pull();
        let started = create_pull(PullUpdate::Started(handle.clone()));
        let progressed =
            create_pull(PullUpdate::Progressed(fixtures::pull_progress()));
        let failed = create_pull(PullUpdate::Finished(Err(
            ContainersError::new("manifest unknown"),
        )));

        assert!(matches!(
            started,
            AppEvent::Containers(ContainersIntent::Create(
                CreateIntent::PullStarted(pull)
            )) if pull == handle
        ));
        assert!(matches!(
            progressed,
            AppEvent::Containers(ContainersIntent::Create(
                CreateIntent::PullProgressed(progress)
            )) if progress == fixtures::pull_progress()
        ));
        assert!(matches!(
            failed,
            AppEvent::Containers(ContainersIntent::Create(
                CreateIntent::PullFinished(Err(error))
            )) if error == ContainersError::new("manifest unknown")
        ));
    }

    #[test]
    fn a_run_pull_reports_to_the_run_form() {
        let handle = detached_pull();
        let started = run_pull(PullUpdate::Started(handle.clone()));
        let progressed =
            run_pull(PullUpdate::Progressed(fixtures::pull_progress()));
        let failed = run_pull(PullUpdate::Finished(Err(ContainersError::new(
            "manifest unknown",
        ))));

        assert!(matches!(
            started,
            AppEvent::Containers(ContainersIntent::Run(
                RunIntent::PullStarted(pull)
            )) if pull == handle
        ));
        assert!(matches!(
            progressed,
            AppEvent::Containers(ContainersIntent::Run(
                RunIntent::PullProgressed(progress)
            )) if progress == fixtures::pull_progress()
        ));
        assert!(matches!(
            failed,
            AppEvent::Containers(ContainersIntent::Run(
                RunIntent::PullFinished(Err(error))
            )) if error == ContainersError::new("manifest unknown")
        ));
    }

    #[test]
    fn each_engine_effect_runs_one_engine_task() {
        let search = discover_engines();
        let background = discover_in_background();
        let load = load_lists(fixtures::engine(EngineKind::Docker));
        let reload = load_list(
            fixtures::engine(EngineKind::Docker),
            ResourceKind::Images,
            Vec::new(),
        );

        assert_eq!(search.units(), 1);
        assert_eq!(background.units(), 1);
        assert_eq!(load.units(), 1);
        assert_eq!(reload.units(), 1);
    }

    #[test]
    fn copying_runs_one_clipboard_task() {
        let copy =
            copy_to_clipboard(String::from("sudo systemctl start docker"));

        assert_eq!(copy.units(), 1);
    }

    #[test]
    fn an_engine_error_becomes_the_text_of_its_message() {
        let engine_error = EngineError::PathNotUtf8(PathBuf::from("/run/x"));
        let message = engine_error.message();

        let error = containers_error(engine_error);

        assert_eq!(error.to_string(), message);
    }

    #[test]
    fn a_failed_list_keeps_its_kind_and_the_error() {
        let error = ContainersError::new("refused");

        let lists = [
            ResourceKind::Containers,
            ResourceKind::Images,
            ResourceKind::Volumes,
            ResourceKind::Networks,
        ]
        .map(|resource| failed_list(resource, error.clone()));

        assert_eq!(
            lists,
            [
                LoadedList::Containers(Err(error.clone())),
                LoadedList::Images(Err(error.clone())),
                LoadedList::Volumes(Err(error.clone())),
                LoadedList::Networks(Err(error)),
            ]
        );
    }
}
