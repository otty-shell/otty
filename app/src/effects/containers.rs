//! Runs the effects of the Containers widget and its child widgets. The
//! engine search, the list loads and the folder checks run here, off the
//! reducer, through `Task::perform`; their results come back as intents. So
//! do the two waits around a folder check.

use std::time::Duration;

use iced::Task;
use iced::futures::future::join3;
use iced::futures::{SinkExt, Stream, StreamExt};
use otty_containers::{
    Container, ContainerRun, Engine, EngineInfo, ImageDefaults, PullProgress,
    RunRequest, discover, host_folder_exists, wait_for,
};
use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

use super::notifications;
use crate::app::App;
use crate::dispatch::AppEvent;
use crate::domain::exit_behavior::ExitBehavior;
use crate::i18n::{self, Key};
use crate::services::{self, terminal_settings_for_session};
use crate::widgets::containers::child::actions::event::{
    ActionsEffect, ActionsIntent,
};
use crate::widgets::containers::child::create::event::{
    CreateEffect, CreateIntent,
};
use crate::widgets::containers::child::inspector::event::{
    InspectorEffect, InspectorIntent,
};
use crate::widgets::containers::child::inspector::types::{
    InspectTarget, LogTarget,
};
use crate::widgets::containers::child::run::event::{RunEffect, RunIntent};
use crate::widgets::containers::event::{
    ContainersEffect, ContainersEvent, ContainersIntent,
};
use crate::widgets::containers::types::{
    ActionRequest, ContainersError, EngineAction, LoadedList, LoadedLists,
    PullHandle, ResourceKind,
};
use crate::widgets::sidebar::event::{SidebarEvent, SidebarIntent};
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
use crate::widgets::tabs::types::TabMachine;

/// How long typing must stop before a typed folder is asked about, so a typed
/// path costs one question and not one per character.
const FOLDER_TYPING_PAUSE: Duration = Duration::from_millis(300);

/// How long the disk has to answer before the row says the folder is not
/// answering.
const FOLDER_ANSWER_LIMIT: Duration = Duration::from_secs(3);

/// What one pull reports, before it is turned into an intent of the child
/// widget that asked for the pull.
enum PullUpdate {
    /// The pull started; the child widget keeps its abort handle.
    Started(PullHandle),
    /// The pull got further.
    Progressed(PullProgress),
    /// The pull ended; the error is the engine's message.
    Finished(Result<(), ContainersError>),
}

/// Run a Containers effect. The effects of the child widgets go to their
/// own functions below.
pub(crate) fn run(app: &App, effect: ContainersEffect) -> Task<AppEvent> {
    match effect {
        ContainersEffect::Create(effect) => create_effect(effect),
        ContainersEffect::Run(effect) => run_effect(effect),
        ContainersEffect::Actions(effect) => actions_effect(app, effect),
        ContainersEffect::Inspector(effect) => inspector_effect(effect),
        effect => handle_effect(effect),
    }
}

/// Run an effect of the create dialogs.
fn create_effect(effect: CreateEffect) -> Task<AppEvent> {
    match effect {
        CreateEffect::StartPull { engine, image, tag } => {
            start_pull(engine, image, tag, create_pull)
        },
        CreateEffect::CancelPull(pull) => {
            pull.abort();
            Task::none()
        },
    }
}

/// Run an effect of the Run container form. Its pull, its engine calls, its
/// folder checks and the two waits around a folder check come back as
/// intents of the form.
fn run_effect(effect: RunEffect) -> Task<AppEvent> {
    match effect {
        RunEffect::StartPull { engine, image, tag } => {
            start_pull(engine, image, tag, run_pull)
        },
        RunEffect::CancelPull(pull) => {
            pull.abort();
            Task::none()
        },
        RunEffect::LoadImageDefaults { engine, reference } => {
            let address = engine.address().clone();
            let asked = reference.clone();

            Task::perform(
                load_image_defaults(engine, reference),
                move |result| {
                    run_intent(RunIntent::ImageDefaultsLoaded {
                        engine: address.clone(),
                        reference: asked.clone(),
                        result,
                    })
                },
            )
        },
        RunEffect::RunContainer { engine, request } => {
            let address = engine.address().clone();

            Task::perform(run_container(engine, request), move |result| {
                run_intent(RunIntent::Finished {
                    engine: address.clone(),
                    result,
                })
            })
        },
        RunEffect::CheckFolder(path) => {
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
        },
        RunEffect::WaitForTypingPause(path) => folder_wait(
            FOLDER_TYPING_PAUSE,
            RunIntent::FolderTypingPaused(path),
        ),
        RunEffect::WaitForLateAnswer(path) => {
            folder_wait(FOLDER_ANSWER_LIMIT, RunIntent::FolderAnswerLate(path))
        },
    }
}

/// Run an effect of the actions: the engine call of an action, whose result
/// comes back as an intent of the actions, or the tab of a shell.
fn actions_effect(app: &App, effect: ActionsEffect) -> Task<AppEvent> {
    match effect {
        ActionsEffect::RunAction(request) => {
            Task::perform(run_action(request.clone()), move |result| {
                actions_intent(ActionsIntent::Finished { request, result })
            })
        },
        ActionsEffect::OpenShell {
            title,
            program,
            args,
        } => open_shell_tab(&app.terminal_settings, title, program, args),
    }
}

/// Run an effect of the inspector: the inspect JSON of a row and the run of
/// the container of the open logs come back as intents of the inspector.
fn inspector_effect(effect: InspectorEffect) -> Task<AppEvent> {
    match effect {
        InspectorEffect::LoadInspect { engine, target } => {
            let shown = target.clone();

            Task::perform(load_inspect(engine, target), move |result| {
                inspector_intent(InspectorIntent::InspectLoaded {
                    target: shown.clone(),
                    result,
                })
            })
        },
        InspectorEffect::CheckLogRun(target) => {
            let asked = target.clone();

            Task::perform(load_log_run(target), move |result| {
                inspector_intent(InspectorIntent::LogRunChecked {
                    target: asked.clone(),
                    result,
                })
            })
        },
        InspectorEffect::CopyToClipboard(text) => iced::clipboard::write(text),
    }
}

/// Pull `image` at `tag` on `engine` and hand the pull's abort handle to the
/// child widget that asked for the pull, which keeps it in its state;
/// `to_event` turns the handle and each update into that child's intent.
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

/// Open a command tab on this computer running `program` with `args`, the
/// way a quick launch opens one. The tabs handler shows Terminal mode, like
/// for every opened tab.
fn open_shell_tab(
    base: &Settings,
    title: String,
    program: String,
    args: Vec<String>,
) -> Task<AppEvent> {
    let options = LocalSessionOptions::default()
        .with_program(&program)
        .with_args(args);
    let settings = terminal_settings_for_session(
        base,
        SessionKind::from_local_options(options),
    );

    Task::done(AppEvent::Tabs(TabsEvent::Intent(
        TabsIntent::OpenCommandTab {
            title,
            settings: Box::new(settings),
            exit_behavior: ExitBehavior::default(),
            machine: TabMachine::ThisComputer,
        },
    )))
}

/// Run the effects that need no app state; the effects of the child widgets
/// are run by [`run`]: the create dialogs, the Run form, the actions, whose
/// shell tab takes the terminal settings, and the inspector.
fn handle_effect(effect: ContainersEffect) -> Task<AppEvent> {
    match effect {
        ContainersEffect::Discover => Task::perform(discover(), |discovery| {
            intent(ContainersIntent::DiscoveryFinished(discovery))
        }),
        ContainersEffect::DiscoverInBackground => {
            Task::perform(discover(), |discovery| {
                intent(ContainersIntent::BackgroundSearchFinished(discovery))
            })
        },
        ContainersEffect::CopyToClipboard(text) => iced::clipboard::write(text),
        ContainersEffect::OpenUrl(url) => {
            link_opened(&url, services::open_url(&url))
        },
        ContainersEffect::LoadLists(info) => {
            let engine = info.address().clone();

            Task::perform(load_lists(info), move |lists| {
                intent(ContainersIntent::ListsLoaded {
                    engine: engine.clone(),
                    lists,
                })
            })
        },
        ContainersEffect::LoadList {
            info,
            resource,
            containers,
        } => {
            let engine = info.address().clone();

            Task::perform(load_list(info, resource, containers), move |list| {
                intent(ContainersIntent::ListLoaded {
                    engine: engine.clone(),
                    list,
                })
            })
        },
        ContainersEffect::RevealInspector => Task::done(AppEvent::Sidebar(
            SidebarEvent::Intent(SidebarIntent::RevealRight),
        )),
        ContainersEffect::Failed(text) => notifications::push(text),
        ContainersEffect::Create(_)
        | ContainersEffect::Run(_)
        | ContainersEffect::Actions(_)
        | ContainersEffect::Inspector(_) => Task::none(),
    }
}

/// What an attempt to open a link leads to: nothing when the browser
/// started. An opener that could not be started is logged and told in a
/// toast.
fn link_opened(url: &str, result: Result<(), String>) -> Task<AppEvent> {
    let Err(error) = result else {
        return Task::none();
    };

    log::warn!("opening {url} failed: {error}");
    notifications::push(i18n::with_error(Key::TplLinkOpenFailed, &error))
}

/// Send `intent` to the Run form after `delay`, without holding a thread.
fn folder_wait(delay: Duration, intent: RunIntent) -> Task<AppEvent> {
    Task::perform(wait_for(delay), move |()| run_intent(intent))
}

/// Ask the engine what an image sets on its own; `None` when the image is
/// not on this computer. The error is the engine's message.
async fn load_image_defaults(
    info: EngineInfo,
    reference: String,
) -> Result<Option<ImageDefaults>, ContainersError> {
    let engine = Engine::connect(&info)?;

    Ok(engine.image_defaults(&reference).await?)
}

/// Create and start a container; its id, or the engine's message.
async fn run_container(
    info: EngineInfo,
    request: RunRequest,
) -> Result<String, ContainersError> {
    let engine = Engine::connect(&info)?;

    Ok(engine.run_container(&request).await?)
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
                let _ =
                    output.send(PullUpdate::Finished(Err(err.into()))).await;
                return;
            },
        };

        let mut updates = engine.pull(&image, &tag);
        while let Some(update) = updates.next().await {
            let next = match update {
                Ok(progress) => PullUpdate::Progressed(progress),
                Err(err) => {
                    let _ = output
                        .send(PullUpdate::Finished(Err(err.into())))
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
async fn run_action(request: ActionRequest) -> Result<(), ContainersError> {
    let engine = Engine::connect(request.engine())?;

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

    Ok(result?)
}

/// Load the four lists of one engine. The containers come first, because
/// "used by" of images, volumes and networks is read from them; then the
/// other three load at the same time. Each list keeps its own error, so one
/// failed list does not hide the others.
async fn load_lists(info: EngineInfo) -> LoadedLists {
    let engine = match Engine::connect(&info) {
        Ok(engine) => engine,
        Err(err) => return LoadedLists::failed(&ContainersError::from(err)),
    };

    let containers = engine.containers().await.map_err(ContainersError::from);

    let users = containers.as_deref().unwrap_or_default();
    let (images, volumes, networks) = join3(
        engine.images(users),
        engine.volumes(users),
        engine.networks(users),
    )
    .await;

    LoadedLists {
        images: images.map_err(ContainersError::from),
        volumes: volumes.map_err(ContainersError::from),
        networks: networks.map_err(ContainersError::from),
        containers,
    }
}

/// Load the inspect JSON of one row; the error is the engine's message.
async fn load_inspect(
    info: EngineInfo,
    target: InspectTarget,
) -> Result<String, ContainersError> {
    let engine = Engine::connect(&info)?;

    let id = target.id();
    let json = match target.resource() {
        ResourceKind::Containers => engine.inspect_container(id).await,
        ResourceKind::Images => engine.inspect_image(id).await,
        ResourceKind::Volumes => engine.inspect_volume(id).await,
        ResourceKind::Networks => engine.inspect_network(id).await,
    };

    Ok(json?)
}

/// What became of the container of the logs `target`; the error is the
/// engine's message.
async fn load_log_run(
    target: LogTarget,
) -> Result<ContainerRun, ContainersError> {
    let engine = Engine::connect(target.engine())?;

    Ok(engine.container_run(target.container_id()).await?)
}

/// Load one list of one engine again, after an engine event. "Used by" of
/// images, volumes and networks is read from `containers`.
async fn load_list(
    info: EngineInfo,
    resource: ResourceKind,
    containers: Vec<Container>,
) -> LoadedList {
    let engine = match Engine::connect(&info) {
        Ok(engine) => engine,
        Err(err) => return failed_list(resource, ContainersError::from(err)),
    };

    match resource {
        ResourceKind::Containers => LoadedList::Containers(
            engine.containers().await.map_err(ContainersError::from),
        ),
        ResourceKind::Images => LoadedList::Images(
            engine
                .images(&containers)
                .await
                .map_err(ContainersError::from),
        ),
        ResourceKind::Volumes => LoadedList::Volumes(
            engine
                .volumes(&containers)
                .await
                .map_err(ContainersError::from),
        ),
        ResourceKind::Networks => LoadedList::Networks(
            engine
                .networks(&containers)
                .await
                .map_err(ContainersError::from),
        ),
    }
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

fn intent(intent: ContainersIntent) -> AppEvent {
    AppEvent::Containers(ContainersEvent::Intent(intent))
}

fn run_intent(intent: RunIntent) -> AppEvent {
    AppEvent::Containers(ContainersEvent::Intent(ContainersIntent::Run(intent)))
}

fn actions_intent(intent: ActionsIntent) -> AppEvent {
    AppEvent::Containers(ContainersEvent::Intent(ContainersIntent::Actions(
        intent,
    )))
}

fn inspector_intent(intent: InspectorIntent) -> AppEvent {
    AppEvent::Containers(ContainersEvent::Intent(ContainersIntent::Inspector(
        intent,
    )))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Instant;

    use iced::futures::StreamExt;
    use iced_test::runtime::Action;
    use iced_test::runtime::task::into_stream;
    use otty_containers::{EngineKind, fixtures};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::containers::types::{
        ActionRequest, EngineAction, PullHandle,
    };
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };

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
                Action::Output(AppEvent::Containers(
                    ContainersEvent::Intent(intent),
                )) => Some(intent),
                _ => None,
            })
            .collect();

        (intents, took)
    }

    #[test]
    fn a_failure_becomes_a_notification() {
        let (app, _) = App::new();

        let events = collect_outputs(run(
            &app,
            ContainersEffect::Failed(String::from(
                "Could not stop web-1: busy",
            )),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::Push(text)
            ))] if text == "Could not stop web-1: busy"
        ));
    }

    /// A browser that cannot be started tells the user why.
    #[test]
    fn a_link_that_cannot_be_opened_tells_the_user() {
        let events = collect_outputs(link_opened(
            "http://localhost:8080",
            Err(String::from("cannot run xdg-open: not found")),
        ));

        let expected = i18n::with_error(
            Key::TplLinkOpenFailed,
            "cannot run xdg-open: not found",
        );
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::Push(text)
            ))] if *text == expected
        ));
    }

    #[test]
    fn an_opened_link_tells_nothing() {
        let events =
            collect_outputs(link_opened("http://localhost:8080", Ok(())));

        assert!(events.is_empty());
    }

    #[test]
    fn the_reveal_effect_sends_the_sidebar_intent() {
        let (app, _) = App::new();

        let sent =
            collect_outputs(run(&app, ContainersEffect::RevealInspector));

        assert!(matches!(
            sent.as_slice(),
            [AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::RevealRight
            ))]
        ));
    }

    #[test]
    fn each_effect_runs_one_engine_task() {
        let search = handle_effect(ContainersEffect::Discover);
        let background = handle_effect(ContainersEffect::DiscoverInBackground);
        let load = handle_effect(ContainersEffect::LoadLists(
            fixtures::engine(EngineKind::Docker),
        ));

        assert_eq!(search.units(), 1);
        assert_eq!(background.units(), 1);
        assert_eq!(load.units(), 1);
    }

    #[test]
    fn loading_an_inspect_runs_one_engine_task() {
        let docker = fixtures::engine(EngineKind::Docker);
        let target = InspectTarget::new(
            docker.address().clone(),
            ResourceKind::Containers,
            "850b6438a531",
        );

        let inspect = inspector_effect(InspectorEffect::LoadInspect {
            engine: docker,
            target,
        });

        assert_eq!(inspect.units(), 1);
    }

    #[test]
    fn checking_the_run_of_open_logs_runs_one_engine_task() {
        let docker = fixtures::engine(EngineKind::Docker);

        let check = inspector_effect(InspectorEffect::CheckLogRun(
            LogTarget::new(docker, "850b6438a531"),
        ));

        assert_eq!(check.units(), 1);
    }

    #[test]
    fn reloading_one_list_runs_one_engine_task() {
        let reload = handle_effect(ContainersEffect::LoadList {
            info: fixtures::engine(EngineKind::Docker),
            resource: ResourceKind::Images,
            containers: Vec::new(),
        });

        assert_eq!(reload.units(), 1);
    }

    #[test]
    fn an_action_runs_one_engine_task() {
        let (app, _) = App::new();

        let stop = actions_effect(
            &app,
            ActionsEffect::RunAction(ActionRequest::new(
                fixtures::engine(EngineKind::Docker),
                EngineAction::StopContainer(String::from("c1")),
                String::from("otty-fx-running"),
            )),
        );

        assert_eq!(stop.units(), 1);
    }

    #[test]
    fn the_run_form_effects_run_one_engine_task_each() {
        let lookup = run_effect(RunEffect::LoadImageDefaults {
            engine: fixtures::engine(EngineKind::Docker),
            reference: String::from("nginx:1.27"),
        });
        let run = run_effect(RunEffect::RunContainer {
            engine: fixtures::engine(EngineKind::Docker),
            request: otty_containers::RunRequest::new("nginx:1.27"),
        });

        assert_eq!(lookup.units(), 1);
        assert_eq!(run.units(), 1);
    }

    #[test]
    fn checking_a_folder_runs_one_task() {
        let check =
            run_effect(RunEffect::CheckFolder(PathBuf::from("/srv/site")));

        assert_eq!(check.units(), 1);
    }

    #[test]
    fn the_folder_waits_run_one_task_each() {
        let typing = run_effect(RunEffect::WaitForTypingPause(PathBuf::from(
            "/srv/site",
        )));
        let late = run_effect(RunEffect::WaitForLateAnswer(PathBuf::from(
            "/srv/site",
        )));

        assert_eq!(typing.units(), 1);
        assert_eq!(late.units(), 1);
    }

    #[test]
    fn the_typing_wait_sends_the_pause_after_its_time() {
        let path = PathBuf::from("/srv/site");

        let (intents, took) = run_waiting_task(run_effect(
            RunEffect::WaitForTypingPause(path.clone()),
        ));

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
        let (app, _) = App::new();

        let run_pull = run(
            &app,
            ContainersEffect::Run(RunEffect::StartPull {
                engine: fixtures::engine(EngineKind::Docker),
                image: String::from("nginx"),
                tag: String::from("latest"),
            }),
        );
        let create_pull = run(
            &app,
            ContainersEffect::Create(CreateEffect::StartPull {
                engine: fixtures::engine(EngineKind::Docker),
                image: String::from("nginx"),
                tag: String::from("latest"),
            }),
        );

        assert_eq!(run_pull.units(), 2);
        assert_eq!(create_pull.units(), 2);
    }

    #[test]
    fn cancelling_a_run_pull_aborts_its_handle() {
        let (app, _) = App::new();
        let pull = PullHandle::detached();

        let cancel = run(
            &app,
            ContainersEffect::Run(RunEffect::CancelPull(pull.clone())),
        );

        assert_eq!(cancel.units(), 0);
        assert!(pull.is_aborted());
    }

    #[test]
    fn cancelling_a_create_pull_aborts_its_handle() {
        let (app, _) = App::new();
        let pull = PullHandle::detached();

        let cancel = run(
            &app,
            ContainersEffect::Create(CreateEffect::CancelPull(pull.clone())),
        );

        assert_eq!(cancel.units(), 0);
        assert!(pull.is_aborted());
    }

    #[test]
    fn a_create_pull_reports_to_the_create_dialogs() {
        let handle = PullHandle::detached();
        let started = create_pull(PullUpdate::Started(handle.clone()));
        let progressed =
            create_pull(PullUpdate::Progressed(fixtures::pull_progress()));
        let failed = create_pull(PullUpdate::Finished(Err(
            ContainersError::new("manifest unknown"),
        )));

        assert!(matches!(
            started,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Create(CreateIntent::PullStarted(pull))
            )) if pull == handle
        ));
        assert!(matches!(
            progressed,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Create(CreateIntent::PullProgressed(progress))
            )) if progress == fixtures::pull_progress()
        ));
        assert!(matches!(
            failed,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Create(CreateIntent::PullFinished(Err(error)))
            )) if error == ContainersError::new("manifest unknown")
        ));
    }

    #[test]
    fn a_run_pull_reports_to_the_run_form() {
        let handle = PullHandle::detached();
        let started = run_pull(PullUpdate::Started(handle.clone()));
        let progressed =
            run_pull(PullUpdate::Progressed(fixtures::pull_progress()));
        let failed = run_pull(PullUpdate::Finished(Err(ContainersError::new(
            "manifest unknown",
        ))));

        assert!(matches!(
            started,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Run(RunIntent::PullStarted(pull))
            )) if pull == handle
        ));
        assert!(matches!(
            progressed,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Run(RunIntent::PullProgressed(progress))
            )) if progress == fixtures::pull_progress()
        ));
        assert!(matches!(
            failed,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::Run(RunIntent::PullFinished(Err(error)))
            )) if error == ContainersError::new("manifest unknown")
        ));
    }

    #[test]
    fn copying_runs_one_clipboard_task() {
        let copy = handle_effect(ContainersEffect::CopyToClipboard(
            String::from("sudo systemctl start docker"),
        ));

        assert_eq!(copy.units(), 1);
    }

    #[test]
    fn copying_the_inspect_json_runs_one_clipboard_task() {
        let (app, _) = App::new();

        let copy = run(
            &app,
            ContainersEffect::Inspector(InspectorEffect::CopyToClipboard(
                String::from("{}"),
            )),
        );

        assert_eq!(copy.units(), 1);
    }

    #[test]
    fn the_shell_effect_opens_one_command_tab() {
        const SHELL_ARGS: [&str; 7] = [
            "--host",
            "unix:///var/run/docker.sock",
            "exec",
            "-it",
            "--",
            "abc123",
            "sh",
        ];
        let (app, _) = App::new();

        let events = collect_outputs(run(
            &app,
            ContainersEffect::Actions(ActionsEffect::OpenShell {
                title: String::from("shell web-1"),
                program: String::from("/opt/homebrew/bin/docker"),
                args: SHELL_ARGS.map(String::from).to_vec(),
            }),
        ));

        let [
            AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenCommandTab {
                title,
                settings,
                machine,
                ..
            })),
        ] = events.as_slice()
        else {
            panic!("expected one command tab");
        };
        assert_eq!(title, "shell web-1");
        assert_eq!(machine, &TabMachine::ThisComputer);
        let SessionKind::Local(options) = &settings.backend.session else {
            panic!("expected a local session");
        };
        assert_eq!(options.program(), "/opt/homebrew/bin/docker");
        assert_eq!(options.args(), SHELL_ARGS);
    }
}
