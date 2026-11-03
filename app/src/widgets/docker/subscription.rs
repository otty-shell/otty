//! Watches the events of every reached engine while Docker mode is active
//! (spec Decision 14) and turns them into Docker intents. Also asks again
//! about the folders of the Run form while it has any.

use std::time::Duration;

use iced::Subscription;
use iced::futures::stream::{self, BoxStream, StreamExt};
use otty_docker::{Engine, EngineAddress, EngineInfo, ListKind, WatchEvent};

use super::event::{DockerEvent, DockerIntent};
use super::run_form::RunForm;
use super::state::DockerState;
use super::types::ResourceKind;

/// How often the folders of the Run form are asked about again.
const FOLDER_RECHECK_PERIOD: Duration = Duration::from_secs(2);

/// One events watch per reached engine; the widget asks for it only while
/// Docker mode is active. The watch of an engine is keyed by what the engine
/// said about itself, so it keeps running across searches that find the
/// engine again and stops when a search no longer finds it.
pub(super) fn subscription(state: &DockerState) -> Subscription<DockerEvent> {
    Subscription::batch(state.engines().iter().map(|engine| {
        Subscription::run_with(engine.info().clone(), watch_engine)
    }))
}

/// A timer that asks again about the folders of the Run form, so a folder
/// created elsewhere clears its error by itself. It runs only while Docker
/// mode is active and the open form has a folder row.
pub(super) fn folder_recheck(state: &DockerState) -> Subscription<DockerEvent> {
    let has_folder_rows =
        state.run_form().is_some_and(RunForm::has_folder_rows);
    if !state.is_active() || !has_folder_rows {
        return Subscription::none();
    }

    iced::time::every(FOLDER_RECHECK_PERIOD)
        .map(|_| DockerEvent::Intent(DockerIntent::RecheckFolders))
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
    use crate::widgets::docker::run_form::RunEdit;

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

    /// Docker mode with the Run form open and one volume row from `source`.
    fn form_with_source(source: &str) -> DockerState {
        let mut state = found(&[EngineKind::Docker]);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        for edit in [
            RunEdit::AddVolume,
            RunEdit::VolumeSource(0, source.to_string()),
        ] {
            let _ = reduce(&mut state, DockerIntent::RunEdited(edit));
        }
        state
    }

    #[test]
    fn the_folder_timer_runs_with_a_folder_row_in_docker_mode() {
        let state = form_with_source("/srv/site");

        assert_eq!(folder_recheck(&state).units(), 1);
    }

    #[test]
    fn the_folder_timer_needs_an_open_form() {
        let mut state = form_with_source("/srv/site");

        let _ = reduce(&mut state, DockerIntent::RunCancelled);

        assert_eq!(folder_recheck(&state).units(), 0);
    }

    #[test]
    fn the_folder_timer_needs_a_folder_row() {
        let mut no_rows = found(&[EngineKind::Docker]);
        let _ = reduce(&mut no_rows, DockerIntent::OpenCreate);

        assert_eq!(folder_recheck(&no_rows).units(), 0);
        assert_eq!(folder_recheck(&form_with_source("otty-data")).units(), 0);
        assert_eq!(folder_recheck(&form_with_source("  ")).units(), 0);
    }

    #[test]
    fn the_folder_timer_stops_when_docker_mode_is_left() {
        let mut state = form_with_source("/srv/site");

        let _ = reduce(&mut state, DockerIntent::Leave);

        assert!(state.run_form().is_some());
        assert_eq!(folder_recheck(&state).units(), 0);
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
