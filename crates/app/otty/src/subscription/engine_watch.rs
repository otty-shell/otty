//! The events of the engines the containers feature watches, turned into
//! containers intents. One watch runs per engine, keyed by what the engine
//! said about itself, so it keeps running across searches that find the
//! engine again and stops when a search no longer finds it.

use iced::Subscription;
use iced::futures::stream::{self, BoxStream, StreamExt};
use otty_app_containers::{ContainersIntent, ResourceKind};
use otty_containers::{
    Engine, EngineAddress, EngineInfo, ListKind, WatchEvent,
};

use crate::message::AppEvent;

/// One events watch for each of `engines`.
pub(super) fn subscription(
    engines: Vec<&EngineInfo>,
) -> Subscription<AppEvent> {
    Subscription::batch(
        engines
            .into_iter()
            .map(|engine| Subscription::run_with(engine.clone(), watch_engine)),
    )
}

/// The events of one engine as containers intents. An engine OTTY cannot even
/// make a client for (its socket file is gone) counts as unreachable.
fn watch_engine(info: &EngineInfo) -> BoxStream<'static, AppEvent> {
    let address = info.address().clone();

    match Engine::connect(info) {
        Ok(engine) => engine
            .watch()
            .filter_map(move |event| {
                let intent = watch_intent(&address, event);
                async move { intent.map(AppEvent::Containers) }
            })
            .boxed(),
        Err(err) => {
            log::debug!("containers engine {address} cannot be watched: {err}");
            let intent = ContainersIntent::EngineUnreachable(address);
            stream::once(async move { AppEvent::Containers(intent) }).boxed()
        },
    }
}

/// The intent an event of the engine at `engine` asks for. A started
/// container restarts the followed logs of the inspector, which comes with
/// issue 14 (old `InspectorIntent::ContainerStarted`); until then it asks
/// for nothing.
fn watch_intent(
    engine: &EngineAddress,
    event: WatchEvent,
) -> Option<ContainersIntent> {
    match event {
        WatchEvent::Changed(list) => Some(ContainersIntent::ListChanged {
            engine: engine.clone(),
            resource: resource_of(list),
        }),
        WatchEvent::Reconnected => {
            Some(ContainersIntent::EngineReconnected(engine.clone()))
        },
        WatchEvent::Unreachable => {
            Some(ContainersIntent::EngineUnreachable(engine.clone()))
        },
        _ => None,
    }
}

/// The Containers mode list for an engine list.
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
    use otty_containers::{EngineKind, fixtures};

    use super::*;

    #[test]
    fn each_engine_gets_its_own_watch() {
        let docker = fixtures::engine(EngineKind::Docker);
        let podman = fixtures::engine(EngineKind::Podman);

        assert_eq!(subscription(vec![&docker]).units(), 1);
        assert_eq!(subscription(vec![&docker, &podman]).units(), 2);
        assert_eq!(subscription(Vec::new()).units(), 0);
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
                Some(ContainersIntent::ListChanged {
                    engine: engine.clone(),
                    resource,
                })
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
            Some(ContainersIntent::EngineReconnected(engine.clone()))
        );
        assert_eq!(
            unreachable,
            Some(ContainersIntent::EngineUnreachable(engine))
        );
    }

    #[test]
    fn a_started_container_asks_for_nothing_until_the_inspector_comes() {
        let engine = fixtures::engine(EngineKind::Docker).address().clone();

        let intent = watch_intent(
            &engine,
            WatchEvent::ContainerStarted(String::from("c178")),
        );

        assert_eq!(intent, None);
    }
}
