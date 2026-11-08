//! Routes the Containers widget: reduces its intents and runs its effects. The
//! engine search, the list loads and the folder checks run here, off the
//! reducer, through `Task::perform`; their results come back as intents. So
//! do the two waits around a folder check.

use std::time::Duration;

use iced::Task;
use iced::futures::future::join3;
use iced::futures::{SinkExt, Stream, StreamExt};
use otty_containers::{
    Container, ContainerRun, Engine, EngineInfo, ImageDefaults, RunRequest,
    discover, host_folder_exists, wait_for,
};
use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

use super::AppEvent;
use crate::app::App;
use crate::domain::exit_behavior::ExitBehavior;
use crate::services::{self, terminal_settings_for_session};
use crate::widgets::containers::actions::{ActionRequest, EngineAction};
use crate::widgets::containers::event::{
    ContainersEffect, ContainersEvent, ContainersIntent,
};
use crate::widgets::containers::inspector::{InspectTarget, LogTarget};
use crate::widgets::containers::types::{
    ContainersError, LoadedList, LoadedLists, ResourceKind,
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

/// Route a Containers event: reduce its intents and run its effects.
pub(crate) fn route(app: &mut App, event: ContainersEvent) -> Task<AppEvent> {
    match event {
        ContainersEvent::Intent(intent) => app
            .widgets
            .containers
            .reduce(intent)
            .map(AppEvent::Containers),
        ContainersEvent::Effect(ContainersEffect::StartPull {
            engine,
            image,
            tag,
        }) => {
            let (task, handle) =
                Task::run(pull_updates(engine, image, tag), intent).abortable();
            app.widgets.containers.set_pull_task(handle);
            task
        },
        ContainersEvent::Effect(ContainersEffect::CancelPull) => {
            app.widgets.containers.cancel_pull();
            Task::none()
        },
        ContainersEvent::Effect(ContainersEffect::OpenShell {
            title,
            program,
            args,
        }) => open_shell_tab(&app.terminal_settings, title, program, args),
        ContainersEvent::Effect(effect) => handle_effect(effect),
    }
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

/// Run the effects that need no app state; the pull effects are run by
/// [`route`], which keeps the pull's abort handle.
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
            services::open_url(&url);
            Task::none()
        },
        ContainersEffect::LoadInspect { engine, target } => {
            let shown = target.clone();

            Task::perform(load_inspect(engine, target), move |result| {
                intent(ContainersIntent::InspectLoaded {
                    target: shown.clone(),
                    result,
                })
            })
        },
        ContainersEffect::CheckLogRun(target) => {
            let asked = target.clone();

            Task::perform(load_log_run(target), move |result| {
                intent(ContainersIntent::LogRunChecked {
                    target: asked.clone(),
                    result,
                })
            })
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
        ContainersEffect::RunAction(request) => {
            Task::perform(run_action(request.clone()), move |result| {
                intent(ContainersIntent::ActionFinished { request, result })
            })
        },
        ContainersEffect::LoadImageDefaults { engine, reference } => {
            let address = engine.address().clone();
            let asked = reference.clone();

            Task::perform(
                load_image_defaults(engine, reference),
                move |result| {
                    intent(ContainersIntent::ImageDefaultsLoaded {
                        engine: address.clone(),
                        reference: asked.clone(),
                        result,
                    })
                },
            )
        },
        ContainersEffect::RunContainer { engine, request } => {
            let address = engine.address().clone();

            Task::perform(run_container(engine, request), move |result| {
                intent(ContainersIntent::RunFinished {
                    engine: address.clone(),
                    result,
                })
            })
        },
        ContainersEffect::CheckFolder(path) => {
            let asked = path.clone();

            Task::perform(
                async move { host_folder_exists(&path).await },
                move |exists| {
                    intent(ContainersIntent::FolderChecked {
                        path: asked,
                        exists,
                    })
                },
            )
        },
        ContainersEffect::WaitForTypingPause(path) => folder_wait(
            FOLDER_TYPING_PAUSE,
            ContainersIntent::FolderTypingPaused(path),
        ),
        ContainersEffect::WaitForLateAnswer(path) => folder_wait(
            FOLDER_ANSWER_LIMIT,
            ContainersIntent::FolderAnswerLate(path),
        ),
        ContainersEffect::RevealInspector => Task::done(AppEvent::Sidebar(
            SidebarEvent::Intent(SidebarIntent::RevealRight),
        )),
        ContainersEffect::StartPull { .. }
        | ContainersEffect::CancelPull
        | ContainersEffect::OpenShell { .. } => Task::none(),
    }
}

/// Send `next` after `delay`, without holding a thread.
fn folder_wait(delay: Duration, next: ContainersIntent) -> Task<AppEvent> {
    Task::perform(wait_for(delay), move |()| intent(next))
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

/// The intents of one pull: a `PullProgressed` per line of the engine,
/// then one `PullFinished` (with the engine's message on failure).
/// Dropping the stream (Cancel) closes the request, which stops the pull.
fn pull_updates(
    info: EngineInfo,
    image: String,
    tag: String,
) -> impl Stream<Item = ContainersIntent> {
    iced::stream::channel(16, async move |mut output| {
        let engine = match Engine::connect(&info) {
            Ok(engine) => engine,
            Err(err) => {
                let _ = output
                    .send(ContainersIntent::PullFinished(Err(err.into())))
                    .await;
                return;
            },
        };

        let mut updates = engine.pull(&image, &tag);
        while let Some(update) = updates.next().await {
            let next = match update {
                Ok(progress) => ContainersIntent::PullProgressed(progress),
                Err(err) => {
                    let _ = output
                        .send(ContainersIntent::PullFinished(Err(err.into())))
                        .await;
                    return;
                },
            };
            let _ = output.send(next).await;
        }

        let _ = output.send(ContainersIntent::PullFinished(Ok(()))).await;
    })
}

/// Make the engine call of one action. The error is the engine's own
/// message, for the notice.
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
    use crate::widgets::containers::actions::{ActionRequest, EngineAction};

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
    fn an_intent_is_reduced_by_the_widget_and_its_effect_comes_back() {
        let (mut app, _) = App::new();

        let events = collect_outputs(route(
            &mut app,
            ContainersEvent::Intent(ContainersIntent::Enter),
        ));

        assert!(app.widgets.containers.is_active());
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Containers(ContainersEvent::Effect(
                ContainersEffect::Discover
            ))]
        ));
    }

    /// An app with the right panel closed.
    fn app_with_closed_right_panel() -> App {
        let (mut app, _) = App::new();
        if app.widgets.sidebar.vm().is_right_open {
            let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleRight);
        }
        assert!(!app.widgets.sidebar.vm().is_right_open);

        app
    }

    /// An app with the fixture lists of one engine loaded and the right panel
    /// closed.
    fn app_with_rows_and_closed_right_panel() -> App {
        let mut app = app_with_closed_right_panel();
        let resources = fixtures::resources(EngineKind::Docker);
        let containers = &mut app.widgets.containers;

        let _ = containers.reduce(ContainersIntent::Enter);
        let _ = containers.reduce(ContainersIntent::DiscoveryFinished(
            fixtures::discovery(&[EngineKind::Docker]),
        ));
        let _ = containers.reduce(ContainersIntent::ListsLoaded {
            engine: fixtures::engine(EngineKind::Docker).address().clone(),
            lists: LoadedLists {
                containers: Ok(resources.containers().to_vec()),
                images: Ok(resources.images().to_vec()),
                volumes: Ok(resources.volumes().to_vec()),
                networks: Ok(resources.networks().to_vec()),
            },
        });

        app
    }

    /// Route `event` and the events its tasks produce, as the iced runtime
    /// does. Only for events that end in `Task::done`: see `collect_outputs`.
    fn route_to_the_end(app: &mut App, event: AppEvent) {
        for next in collect_outputs(crate::router::route(app, event)) {
            route_to_the_end(app, next);
        }
    }

    /// Press the row at `row` of the shown table, to the end of what follows.
    fn press_row(app: &mut App, row: usize) {
        route_to_the_end(
            app,
            AppEvent::Containers(ContainersEvent::Intent(
                ContainersIntent::SelectRow(row),
            )),
        );
    }

    #[test]
    fn pressing_a_row_opens_the_closed_right_panel() {
        let mut app = app_with_rows_and_closed_right_panel();

        press_row(&mut app, 0);

        assert!(app.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_a_row_with_the_right_panel_open_leaves_it_open() {
        let mut app = app_with_rows_and_closed_right_panel();
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleRight);
        assert!(app.widgets.sidebar.vm().is_right_open);

        press_row(&mut app, 0);
        press_row(&mut app, 0);

        assert!(app.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_the_selected_row_again_opens_the_closed_right_panel() {
        let mut app = app_with_rows_and_closed_right_panel();
        press_row(&mut app, 0);
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleRight);
        assert!(!app.widgets.sidebar.vm().is_right_open);

        press_row(&mut app, 0);

        assert!(app.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn a_press_that_selects_no_row_leaves_the_right_panel_closed() {
        let mut no_rows = app_with_closed_right_panel();
        let mut past_the_last_row = app_with_rows_and_closed_right_panel();

        press_row(&mut no_rows, 0);
        press_row(&mut past_the_last_row, usize::MAX);

        assert!(!no_rows.widgets.sidebar.vm().is_right_open);
        assert!(!past_the_last_row.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_a_row_leaves_focus_mode_with_the_right_panel_visible() {
        let mut app = app_with_rows_and_closed_right_panel();
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleFocusMode);
        assert!(app.widgets.sidebar.vm().is_focus_mode);

        press_row(&mut app, 0);

        let sidebar = app.widgets.sidebar.vm();
        assert!(!sidebar.is_focus_mode);
        assert!(sidebar.is_right_open);
        assert!(app.widgets.sidebar.right_ratio() > 0.0);
    }

    #[test]
    fn a_row_press_changes_the_sidebar_only_through_the_effect() {
        let mut app = app_with_rows_and_closed_right_panel();

        let sent = collect_outputs(route(
            &mut app,
            ContainersEvent::Intent(ContainersIntent::SelectRow(0)),
        ));

        assert!(!app.widgets.sidebar.vm().is_right_open);
        assert!(matches!(
            sent.as_slice(),
            [AppEvent::Containers(ContainersEvent::Effect(
                ContainersEffect::RevealInspector
            ))]
        ));
    }

    #[test]
    fn the_reveal_effect_sends_the_sidebar_intent() {
        let (mut app, _) = App::new();

        let sent = collect_outputs(route(
            &mut app,
            ContainersEvent::Effect(ContainersEffect::RevealInspector),
        ));

        assert!(matches!(
            sent.as_slice(),
            [AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::RevealRight
            ))]
        ));
    }

    /// The reduced intents are not routed on: a port link would open a real
    /// browser page.
    #[test]
    fn the_row_menu_button_the_right_click_and_a_port_link_leave_the_panel_closed()
     {
        let mut app = app_with_rows_and_closed_right_panel();
        let rows = fixtures::resources(EngineKind::Docker).containers().len();
        let mut sent = Vec::new();

        for row in 0..rows {
            for intent in [
                ContainersIntent::OpenPortLink { row, link: 0 },
                ContainersIntent::RowMenuPressed {
                    index: row,
                    position: iced::Point::ORIGIN,
                },
                ContainersIntent::OpenRowMenu {
                    index: row,
                    position: iced::Point::ORIGIN,
                },
            ] {
                sent.extend(collect_outputs(route(
                    &mut app,
                    ContainersEvent::Intent(intent),
                )));
                assert!(!app.widgets.sidebar.vm().is_right_open);
            }
        }

        // The rows were real: the port links found their pages.
        assert!(sent.iter().any(|event| matches!(
            event,
            AppEvent::Containers(ContainersEvent::Effect(
                ContainersEffect::OpenUrl(_)
            ))
        )));
        assert!(!sent.iter().any(|event| matches!(
            event,
            AppEvent::Containers(ContainersEvent::Effect(
                ContainersEffect::RevealInspector
            )) | AppEvent::Sidebar(_)
        )));
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

        let inspect = handle_effect(ContainersEffect::LoadInspect {
            engine: docker,
            target,
        });

        assert_eq!(inspect.units(), 1);
    }

    #[test]
    fn checking_the_run_of_open_logs_runs_one_engine_task() {
        let docker = fixtures::engine(EngineKind::Docker);

        let check = handle_effect(ContainersEffect::CheckLogRun(
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
        let stop =
            handle_effect(ContainersEffect::RunAction(ActionRequest::new(
                fixtures::engine(EngineKind::Docker),
                EngineAction::StopContainer(String::from("c1")),
                String::from("otty-fx-running"),
            )));

        assert_eq!(stop.units(), 1);
    }

    #[test]
    fn the_run_form_effects_run_one_engine_task_each() {
        let lookup = handle_effect(ContainersEffect::LoadImageDefaults {
            engine: fixtures::engine(EngineKind::Docker),
            reference: String::from("nginx:1.27"),
        });
        let run = handle_effect(ContainersEffect::RunContainer {
            engine: fixtures::engine(EngineKind::Docker),
            request: otty_containers::RunRequest::new("nginx:1.27"),
        });

        assert_eq!(lookup.units(), 1);
        assert_eq!(run.units(), 1);
    }

    #[test]
    fn checking_a_folder_runs_one_task() {
        let check = handle_effect(ContainersEffect::CheckFolder(
            std::path::PathBuf::from("/srv/site"),
        ));

        assert_eq!(check.units(), 1);
    }

    #[test]
    fn the_folder_waits_run_one_task_each() {
        let typing = handle_effect(ContainersEffect::WaitForTypingPause(
            PathBuf::from("/srv/site"),
        ));
        let late = handle_effect(ContainersEffect::WaitForLateAnswer(
            PathBuf::from("/srv/site"),
        ));

        assert_eq!(typing.units(), 1);
        assert_eq!(late.units(), 1);
    }

    #[test]
    fn the_typing_wait_sends_the_pause_after_its_time() {
        let path = PathBuf::from("/srv/site");

        let (intents, took) = run_waiting_task(handle_effect(
            ContainersEffect::WaitForTypingPause(path.clone()),
        ));

        assert_eq!(intents, vec![ContainersIntent::FolderTypingPaused(path)]);
        assert!(took >= FOLDER_TYPING_PAUSE, "{took:?}");
        assert!(took < FOLDER_ANSWER_LIMIT, "{took:?}");
    }

    #[test]
    fn a_folder_wait_sends_its_intent_after_the_delay() {
        let next =
            ContainersIntent::FolderAnswerLate(PathBuf::from("/srv/site"));
        let delay = Duration::from_millis(5);

        let (intents, took) =
            run_waiting_task(folder_wait(delay, next.clone()));

        assert_eq!(intents, vec![next]);
        assert!(took >= delay, "{took:?}");
    }

    #[test]
    fn a_pull_runs_one_task_that_cancel_aborts() {
        let (mut app, _) = App::new();

        let pull = route(
            &mut app,
            ContainersEvent::Effect(ContainersEffect::StartPull {
                engine: fixtures::engine(EngineKind::Docker),
                image: String::from("nginx"),
                tag: String::from("latest"),
            }),
        );
        assert_eq!(pull.units(), 1);
        assert!(app.widgets.containers.has_pull_task());

        let cancel = route(
            &mut app,
            ContainersEvent::Effect(ContainersEffect::CancelPull),
        );

        assert_eq!(cancel.units(), 0);
        assert!(!app.widgets.containers.has_pull_task());
    }

    #[test]
    fn copying_runs_one_clipboard_task() {
        let copy = handle_effect(ContainersEffect::CopyToClipboard(
            String::from("sudo systemctl start docker"),
        ));

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
        let (mut app, _) = App::new();

        let events = collect_outputs(route(
            &mut app,
            ContainersEvent::Effect(ContainersEffect::OpenShell {
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
