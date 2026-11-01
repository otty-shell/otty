//! Watches the events of every reached engine while Docker mode is active
//! (spec Decision 14) and turns them into Docker intents.

use iced::Subscription;
use iced::futures::stream::{self, BoxStream, StreamExt};
use otty_docker::{Engine, EngineAddress, EngineInfo, ListKind, WatchEvent};

use super::event::{DockerEvent, DockerIntent};
use super::state::DockerState;
use super::types::ResourceKind;

/// One events watch per reached engine; the widget asks for it only while
/// Docker mode is active. The watch of an engine is keyed by what the engine
/// said about itself, so it keeps running across searches that find the
/// engine again and stops when a search no longer finds it.
pub(super) fn subscription(state: &DockerState) -> Subscription<DockerEvent> {
    Subscription::batch(state.engines().iter().map(|engine| {
        Subscription::run_with(engine.info().clone(), watch_engine)
    }))
}

/// The intent an event of the engine at `engine` asks for.
fn watch_intent(engine: &EngineAddress, event: WatchEvent) -> DockerIntent {
    match event {
        WatchEvent::Changed(list) => DockerIntent::ListChanged {
            engine: engine.clone(),
            resource: resource_of(list),
        },
        WatchEvent::Reconnected => {
            DockerIntent::EngineReconnected(engine.clone())
        },
        WatchEvent::Unreachable => {
            DockerIntent::EngineUnreachable(engine.clone())
        },
    }
}

/// The events of one engine as Docker intents. An engine OTTY cannot even
/// make a client for (its socket file is gone) counts as unreachable.
fn watch_engine(info: &EngineInfo) -> BoxStream<'static, DockerEvent> {
    let address = info.address().clone();

    match Engine::connect(info) {
        Ok(engine) => engine
            .watch()
            .map(move |event| {
                DockerEvent::Intent(watch_intent(&address, event))
            })
            .boxed(),
        Err(err) => {
            log::debug!("docker engine {address} cannot be watched: {err}");
            let intent = DockerIntent::EngineUnreachable(address);
            stream::once(async move { DockerEvent::Intent(intent) }).boxed()
        },
    }
}

/// The Docker mode list for an engine list.
fn resource_of(list: ListKind) -> ResourceKind {
    match list {
        ListKind::Containers => ResourceKind::Containers,
        ListKind::Images => ResourceKind::Images,
        ListKind::Volumes => ResourceKind::Volumes,
        ListKind::Networks => ResourceKind::Networks,
    }
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;
    use crate::widgets::docker::reducer::reduce;

    fn found(kinds: &[EngineKind]) -> DockerState {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(kinds)),
        );
        state
    }

    #[test]
    fn docker_mode_watches_every_reached_engine() {
        let one = found(&[EngineKind::Docker]);
        let two = found(&[EngineKind::Docker, EngineKind::Podman]);

        assert_eq!(subscription(&one).units(), 1);
        assert_eq!(subscription(&two).units(), 2);
    }

    #[test]
    fn nothing_is_watched_without_an_engine() {
        let state = found(&[]);

        assert_eq!(subscription(&state).units(), 0);
    }

    #[test]
    fn a_changed_list_asks_to_reload_that_list_of_that_engine() {
        let engine = fixtures::engine(EngineKind::Docker).address().clone();
        let pairs = [
            (ListKind::Containers, ResourceKind::Containers),
            (ListKind::Images, ResourceKind::Images),
            (ListKind::Volumes, ResourceKind::Volumes),
            (ListKind::Networks, ResourceKind::Networks),
        ];

        for (list, resource) in pairs {
            let intent = watch_intent(&engine, WatchEvent::Changed(list));

            assert_eq!(
                intent,
                DockerIntent::ListChanged {
                    engine: engine.clone(),
                    resource,
                }
            );
        }
    }

    #[test]
    fn a_reconnect_or_a_lost_engine_names_the_engine() {
        let engine = fixtures::engine(EngineKind::Podman).address().clone();

        let reconnected = watch_intent(&engine, WatchEvent::Reconnected);
        let unreachable = watch_intent(&engine, WatchEvent::Unreachable);

        assert_eq!(
            reconnected,
            DockerIntent::EngineReconnected(engine.clone())
        );
        assert_eq!(unreachable, DockerIntent::EngineUnreachable(engine));
    }
}
