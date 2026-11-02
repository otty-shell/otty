//! Routes the Docker widget: reduces its intents and runs its effects. The
//! engine search and the list loads run here, off the reducer, through
//! `Task::perform`; their results come back as intents.

use iced::Task;
use iced::futures::future::join3;
use iced::futures::{SinkExt, Stream, StreamExt};
use otty_docker::{
    Container, Engine, EngineInfo, ImageDefaults, RunRequest, discover,
};
use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

use super::AppEvent;
use crate::app::App;
use crate::domain::exit_behavior::ExitBehavior;
use crate::services::{self, terminal_settings_for_session};
use crate::widgets::docker::actions::{ActionRequest, EngineAction};
use crate::widgets::docker::inspector::InspectTarget;
use crate::widgets::docker::types::{LoadedList, LoadedLists, ResourceKind};
use crate::widgets::docker::{DockerEffect, DockerEvent, DockerIntent};
use crate::widgets::sidebar::SidebarIntent;
use crate::widgets::tabs::types::TabMachine;
use crate::widgets::tabs::{TabsEvent, TabsIntent};

/// Route a Docker event: reduce its intents and run its effects.
pub(crate) fn handle(app: &mut App, event: DockerEvent) -> Task<AppEvent> {
    match event {
        DockerEvent::Intent(intent) => {
            // A press on a table row shows it in the inspector, so a closed
            // right panel opens. The row "⋯" button and the right click
            // send other intents and leave the panel as it is.
            let is_row_press = matches!(intent, DockerIntent::SelectRow(_));
            let docker_task =
                app.widgets.docker.reduce(intent).map(AppEvent::Docker);
            if !is_row_press || app.widgets.sidebar.vm().is_right_open {
                return docker_task;
            }

            let open_panel = app
                .widgets
                .sidebar
                .reduce(SidebarIntent::ToggleRight)
                .map(AppEvent::Sidebar);

            Task::batch([docker_task, open_panel])
        },
        DockerEvent::Effect(DockerEffect::StartPull { engine, image, tag }) => {
            let (task, handle) =
                Task::run(pull_updates(engine, image, tag), intent).abortable();
            app.widgets.docker.set_pull_task(handle);
            task
        },
        DockerEvent::Effect(DockerEffect::CancelPull) => {
            app.widgets.docker.cancel_pull();
            Task::none()
        },
        DockerEvent::Effect(DockerEffect::OpenShell {
            title,
            program,
            args,
        }) => open_shell_tab(&app.terminal_settings, title, program, args),
        DockerEvent::Effect(effect) => handle_effect(effect),
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
/// [`handle`], which keeps the pull's abort handle.
fn handle_effect(effect: DockerEffect) -> Task<AppEvent> {
    match effect {
        DockerEffect::Discover => Task::perform(discover(), |discovery| {
            intent(DockerIntent::DiscoveryFinished(discovery))
        }),
        DockerEffect::CopyToClipboard(text) => iced::clipboard::write(text),
        DockerEffect::OpenUrl(url) => {
            services::open_url(&url);
            Task::none()
        },
        DockerEffect::LoadInspect { engine, target } => {
            let shown = target.clone();

            Task::perform(load_inspect(engine, target), move |result| {
                intent(DockerIntent::InspectLoaded {
                    target: shown.clone(),
                    result,
                })
            })
        },
        DockerEffect::LoadLists(info) => {
            let engine = info.address().clone();

            Task::perform(load_lists(info), move |lists| {
                intent(DockerIntent::ListsLoaded {
                    engine: engine.clone(),
                    lists,
                })
            })
        },
        DockerEffect::LoadList {
            info,
            resource,
            containers,
        } => {
            let engine = info.address().clone();

            Task::perform(load_list(info, resource, containers), move |list| {
                intent(DockerIntent::ListLoaded {
                    engine: engine.clone(),
                    list,
                })
            })
        },
        DockerEffect::RunAction(request) => {
            Task::perform(run_action(request.clone()), move |result| {
                intent(DockerIntent::ActionFinished { request, result })
            })
        },
        DockerEffect::LoadImageDefaults { engine, reference } => {
            let address = engine.address().clone();
            let asked = reference.clone();

            Task::perform(
                load_image_defaults(engine, reference),
                move |result| {
                    intent(DockerIntent::ImageDefaultsLoaded {
                        engine: address.clone(),
                        reference: asked.clone(),
                        result,
                    })
                },
            )
        },
        DockerEffect::RunContainer { engine, request } => {
            let address = engine.address().clone();

            Task::perform(run_container(engine, request), move |result| {
                intent(DockerIntent::RunFinished {
                    engine: address.clone(),
                    result,
                })
            })
        },
        DockerEffect::StartPull { .. }
        | DockerEffect::CancelPull
        | DockerEffect::OpenShell { .. } => Task::none(),
    }
}

/// Ask the engine what an image sets on its own; `None` when the image is
/// not on this computer. The error is the engine's message.
async fn load_image_defaults(
    info: EngineInfo,
    reference: String,
) -> Result<Option<ImageDefaults>, String> {
    let engine = Engine::connect(&info).map_err(|err| err.message())?;

    engine
        .image_defaults(&reference)
        .await
        .map_err(|err| err.message())
}

/// Create and start a container; its id, or the engine's message.
async fn run_container(
    info: EngineInfo,
    request: RunRequest,
) -> Result<String, String> {
    let engine = Engine::connect(&info).map_err(|err| err.message())?;

    engine
        .run_container(&request)
        .await
        .map_err(|err| err.message())
}

/// The intents of one pull: a `PullProgressed` per line of the engine,
/// then one `PullFinished` (with the engine's message on failure).
/// Dropping the stream (Cancel) closes the request, which stops the pull.
fn pull_updates(
    info: EngineInfo,
    image: String,
    tag: String,
) -> impl Stream<Item = DockerIntent> {
    iced::stream::channel(16, async move |mut output| {
        let engine = match Engine::connect(&info) {
            Ok(engine) => engine,
            Err(err) => {
                let _ = output
                    .send(DockerIntent::PullFinished(Err(err.message())))
                    .await;
                return;
            },
        };

        let mut updates = engine.pull(&image, &tag);
        while let Some(update) = updates.next().await {
            let next = match update {
                Ok(progress) => DockerIntent::PullProgressed(progress),
                Err(err) => {
                    let _ = output
                        .send(DockerIntent::PullFinished(Err(err.message())))
                        .await;
                    return;
                },
            };
            let _ = output.send(next).await;
        }

        let _ = output.send(DockerIntent::PullFinished(Ok(()))).await;
    })
}

/// Make the engine call of one action. The error is the engine's own
/// message, for the notice.
async fn run_action(request: ActionRequest) -> Result<(), String> {
    let engine =
        Engine::connect(request.engine()).map_err(|err| err.message())?;

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
        EngineAction::RemoveImage { id, force } => {
            engine.remove_image(id, *force).await
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

    result.map_err(|err| err.message())
}

/// Load the four lists of one engine. The containers come first, because
/// "used by" of images, volumes and networks is read from them; then the
/// other three load at the same time. Each list keeps its own error, so one
/// failed list does not hide the others.
async fn load_lists(info: EngineInfo) -> LoadedLists {
    let engine = match Engine::connect(&info) {
        Ok(engine) => engine,
        Err(err) => return LoadedLists::failed(&format!("{err}")),
    };

    let containers = engine.containers().await.map_err(|err| format!("{err}"));

    let users = containers.as_deref().unwrap_or_default();
    let (images, volumes, networks) = join3(
        engine.images(users),
        engine.volumes(users),
        engine.networks(users),
    )
    .await;

    LoadedLists {
        images: images.map_err(|err| format!("{err}")),
        volumes: volumes.map_err(|err| format!("{err}")),
        networks: networks.map_err(|err| format!("{err}")),
        containers,
    }
}

/// Load the inspect JSON of one row; the error is the engine's message.
async fn load_inspect(
    info: EngineInfo,
    target: InspectTarget,
) -> Result<String, String> {
    let engine = Engine::connect(&info).map_err(|err| format!("{err}"))?;

    let id = target.id();
    let json = match target.resource() {
        ResourceKind::Containers => engine.inspect_container(id).await,
        ResourceKind::Images => engine.inspect_image(id).await,
        ResourceKind::Volumes => engine.inspect_volume(id).await,
        ResourceKind::Networks => engine.inspect_network(id).await,
    };

    json.map_err(|err| format!("{err}"))
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
        Err(err) => return failed_list(resource, format!("{err}")),
    };

    match resource {
        ResourceKind::Containers => LoadedList::Containers(
            engine.containers().await.map_err(|err| format!("{err}")),
        ),
        ResourceKind::Images => LoadedList::Images(
            engine
                .images(&containers)
                .await
                .map_err(|err| format!("{err}")),
        ),
        ResourceKind::Volumes => LoadedList::Volumes(
            engine
                .volumes(&containers)
                .await
                .map_err(|err| format!("{err}")),
        ),
        ResourceKind::Networks => LoadedList::Networks(
            engine
                .networks(&containers)
                .await
                .map_err(|err| format!("{err}")),
        ),
    }
}

/// `resource` failed to load with `message`.
fn failed_list(resource: ResourceKind, message: String) -> LoadedList {
    match resource {
        ResourceKind::Containers => LoadedList::Containers(Err(message)),
        ResourceKind::Images => LoadedList::Images(Err(message)),
        ResourceKind::Volumes => LoadedList::Volumes(Err(message)),
        ResourceKind::Networks => LoadedList::Networks(Err(message)),
    }
}

fn intent(intent: DockerIntent) -> AppEvent {
    AppEvent::Docker(DockerEvent::Intent(intent))
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::docker::actions::{ActionRequest, EngineAction};

    #[test]
    fn an_intent_is_reduced_by_the_widget_and_its_effect_comes_back() {
        let (mut app, _) = App::new();

        let events = collect_outputs(handle(
            &mut app,
            DockerEvent::Intent(DockerIntent::Enter),
        ));

        assert!(app.widgets.docker.is_active());
        assert!(matches!(
            events.as_slice(),
            [AppEvent::Docker(DockerEvent::Effect(
                DockerEffect::Discover
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

    #[test]
    fn pressing_a_row_opens_the_closed_right_panel() {
        let mut app = app_with_closed_right_panel();

        let _ =
            handle(&mut app, DockerEvent::Intent(DockerIntent::SelectRow(0)));

        assert!(app.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn pressing_a_row_with_the_right_panel_open_leaves_it_open() {
        let mut app = app_with_closed_right_panel();
        let _ = app.widgets.sidebar.reduce(SidebarIntent::ToggleRight);
        assert!(app.widgets.sidebar.vm().is_right_open);

        for _ in 0..2 {
            let _ = handle(
                &mut app,
                DockerEvent::Intent(DockerIntent::SelectRow(0)),
            );
        }

        assert!(app.widgets.sidebar.vm().is_right_open);
    }

    #[test]
    fn the_row_menu_button_and_the_right_click_leave_the_panel_closed() {
        let mut app = app_with_closed_right_panel();

        for intent in [
            DockerIntent::RowMenuPressed {
                index: 0,
                position: iced::Point::ORIGIN,
            },
            DockerIntent::OpenRowMenu {
                index: 0,
                position: iced::Point::ORIGIN,
            },
        ] {
            let _ = handle(&mut app, DockerEvent::Intent(intent));
            assert!(!app.widgets.sidebar.vm().is_right_open);
        }
    }

    #[test]
    fn each_effect_runs_one_engine_task() {
        let search = handle_effect(DockerEffect::Discover);
        let load = handle_effect(DockerEffect::LoadLists(fixtures::engine(
            EngineKind::Docker,
        )));

        assert_eq!(search.units(), 1);
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

        let inspect = handle_effect(DockerEffect::LoadInspect {
            engine: docker,
            target,
        });

        assert_eq!(inspect.units(), 1);
    }

    #[test]
    fn reloading_one_list_runs_one_engine_task() {
        let reload = handle_effect(DockerEffect::LoadList {
            info: fixtures::engine(EngineKind::Docker),
            resource: ResourceKind::Images,
            containers: Vec::new(),
        });

        assert_eq!(reload.units(), 1);
    }

    #[test]
    fn an_action_runs_one_engine_task() {
        let stop = handle_effect(DockerEffect::RunAction(ActionRequest::new(
            fixtures::engine(EngineKind::Docker),
            EngineAction::StopContainer(String::from("c1")),
            String::from("otty-fx-running"),
        )));

        assert_eq!(stop.units(), 1);
    }

    #[test]
    fn the_run_form_effects_run_one_engine_task_each() {
        let lookup = handle_effect(DockerEffect::LoadImageDefaults {
            engine: fixtures::engine(EngineKind::Docker),
            reference: String::from("nginx:1.27"),
        });
        let run = handle_effect(DockerEffect::RunContainer {
            engine: fixtures::engine(EngineKind::Docker),
            request: otty_docker::RunRequest::new("nginx:1.27"),
        });

        assert_eq!(lookup.units(), 1);
        assert_eq!(run.units(), 1);
    }

    #[test]
    fn a_pull_runs_one_task_that_cancel_aborts() {
        let (mut app, _) = App::new();

        let pull = handle(
            &mut app,
            DockerEvent::Effect(DockerEffect::StartPull {
                engine: fixtures::engine(EngineKind::Docker),
                image: String::from("nginx"),
                tag: String::from("latest"),
            }),
        );
        assert_eq!(pull.units(), 1);
        assert!(app.widgets.docker.has_pull_task());

        let cancel =
            handle(&mut app, DockerEvent::Effect(DockerEffect::CancelPull));

        assert_eq!(cancel.units(), 0);
        assert!(!app.widgets.docker.has_pull_task());
    }

    #[test]
    fn copying_runs_one_clipboard_task() {
        let copy = handle_effect(DockerEffect::CopyToClipboard(String::from(
            "sudo systemctl start docker",
        )));

        assert_eq!(copy.units(), 1);
    }

    #[test]
    fn the_shell_effect_opens_one_command_tab() {
        let (mut app, _) = App::new();

        let events = collect_outputs(handle(
            &mut app,
            DockerEvent::Effect(DockerEffect::OpenShell {
                title: String::from("shell web-1"),
                program: String::from("/opt/homebrew/bin/docker"),
                args: ["exec", "-it", "abc123", "sh"]
                    .map(String::from)
                    .to_vec(),
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
        assert_eq!(options.args(), ["exec", "-it", "abc123", "sh"]);
    }
}
