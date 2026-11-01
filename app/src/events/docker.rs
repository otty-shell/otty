//! Routes the Docker widget: reduces its intents and runs its effects. The
//! engine search and the list loads run here, off the reducer, through
//! `Task::perform`; their results come back as intents.

use iced::Task;
use iced::futures::future::join3;
use otty_docker::{Engine, EngineInfo, discover};

use super::AppEvent;
use crate::app::App;
use crate::services;
use crate::widgets::docker::types::LoadedLists;
use crate::widgets::docker::{DockerEffect, DockerEvent, DockerIntent};

/// Route a Docker event: reduce its intents and run its effects.
pub(crate) fn handle(app: &mut App, event: DockerEvent) -> Task<AppEvent> {
    match event {
        DockerEvent::Intent(intent) => {
            app.widgets.docker.reduce(intent).map(AppEvent::Docker)
        },
        DockerEvent::Effect(effect) => handle_effect(effect),
    }
}

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
        DockerEffect::LoadLists(info) => {
            let engine = info.address().clone();

            Task::perform(load_lists(info), move |lists| {
                intent(DockerIntent::ListsLoaded {
                    engine: engine.clone(),
                    lists,
                })
            })
        },
    }
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

fn intent(intent: DockerIntent) -> AppEvent {
    AppEvent::Docker(DockerEvent::Intent(intent))
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;

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
    fn copying_runs_one_clipboard_task() {
        let copy = handle_effect(DockerEffect::CopyToClipboard(String::from(
            "sudo systemctl start docker",
        )));

        assert_eq!(copy.units(), 1);
    }
}
