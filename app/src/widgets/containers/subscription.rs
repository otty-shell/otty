//! Watches the events of every reached engine while Containers mode is active
//! (spec Decision 14) and turns them into Containers intents. Also asks again
//! about the folders of the Run form while it has any, and looks for engines
//! while the no-engine card shows (spec Decision 9).

use std::time::Duration;

use iced::Subscription;
use iced::futures::stream::{self, BoxStream, StreamExt};
use otty_containers::{
    Engine, EngineAddress, EngineInfo, ListKind, WatchEvent,
};

use super::event::{ContainersEvent, ContainersIntent};
use super::run_form::RunForm;
use super::state::ContainersState;
use super::types::ResourceKind;

/// How often the folders of the Run form are asked about again.
const FOLDER_RECHECK_PERIOD: Duration = Duration::from_secs(2);

/// How often the engines are looked for while the no-engine card shows.
const ENGINE_SEARCH_PERIOD: Duration = Duration::from_secs(5);

/// One events watch per reached engine; the widget asks for it only while
/// Containers mode is active. The watch of an engine is keyed by what the engine
/// said about itself, so it keeps running across searches that find the
/// engine again and stops when a search no longer finds it.
pub(super) fn subscription(
    state: &ContainersState,
) -> Subscription<ContainersEvent> {
    Subscription::batch(state.engines().iter().map(|engine| {
        Subscription::run_with(engine.info().clone(), watch_engine)
    }))
}

/// A timer that asks again about the folders of the Run form, so a folder
/// created elsewhere clears its error by itself. It runs only while Containers
/// mode is active and the open form has a folder row.
pub(super) fn folder_recheck(
    state: &ContainersState,
) -> Subscription<ContainersEvent> {
    let has_folder_rows =
        state.run_form().is_some_and(RunForm::has_folder_rows);
    if !state.is_active() || !has_folder_rows {
        return Subscription::none();
    }

    iced::time::every(FOLDER_RECHECK_PERIOD)
        .map(|_| ContainersEvent::Intent(ContainersIntent::RecheckFolders))
}

/// A timer that looks for engines while the no-engine card is on screen in
/// active Containers mode, so an engine that starts takes over by itself. It stops
/// as soon as the card goes.
pub(super) fn engine_search_timer(
    state: &ContainersState,
) -> Subscription<ContainersEvent> {
    if !state.is_active() || !state.shows_no_engine_card() {
        return Subscription::none();
    }

    iced::time::every(ENGINE_SEARCH_PERIOD)
        .map(|_| ContainersEvent::Intent(ContainersIntent::EngineSearchTick))
}

/// The intent an event of the engine at `engine` asks for.
fn watch_intent(engine: &EngineAddress, event: WatchEvent) -> ContainersIntent {
    match event {
        WatchEvent::Changed(list) => ContainersIntent::ListChanged {
            engine: engine.clone(),
            resource: resource_of(list),
        },
        WatchEvent::ContainerStarted(id) => {
            ContainersIntent::ContainerStarted {
                engine: engine.clone(),
                id,
            }
        },
        WatchEvent::Reconnected => {
            ContainersIntent::EngineReconnected(engine.clone())
        },
        WatchEvent::Unreachable => {
            ContainersIntent::EngineUnreachable(engine.clone())
        },
    }
}

/// The events of one engine as Containers intents. An engine OTTY cannot even
/// make a client for (its socket file is gone) counts as unreachable.
fn watch_engine(info: &EngineInfo) -> BoxStream<'static, ContainersEvent> {
    let address = info.address().clone();

    match Engine::connect(info) {
        Ok(engine) => engine
            .watch()
            .map(move |event| {
                ContainersEvent::Intent(watch_intent(&address, event))
            })
            .boxed(),
        Err(err) => {
            log::debug!("containers engine {address} cannot be watched: {err}");
            let intent = ContainersIntent::EngineUnreachable(address);
            stream::once(async move { ContainersEvent::Intent(intent) }).boxed()
        },
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
    use crate::widgets::containers::reducer::reduce;
    use crate::widgets::containers::run_form::RunEdit;

    fn found(kinds: &[EngineKind]) -> ContainersState {
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(kinds)),
        );
        state
    }

    #[test]
    fn containers_mode_watches_every_reached_engine() {
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

    /// Containers mode with the Run form open and one volume row from `source`.
    fn form_with_source(source: &str) -> ContainersState {
        let mut state = found(&[EngineKind::Docker]);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        for edit in [
            RunEdit::AddVolume,
            RunEdit::VolumeSource(0, source.to_string()),
        ] {
            let _ = reduce(&mut state, ContainersIntent::RunEdited(edit));
        }
        state
    }

    #[test]
    fn the_folder_timer_runs_with_a_folder_row_in_containers_mode() {
        let state = form_with_source("/srv/site");

        assert_eq!(folder_recheck(&state).units(), 1);
    }

    #[test]
    fn the_folder_timer_needs_an_open_form() {
        let mut state = form_with_source("/srv/site");

        let _ = reduce(&mut state, ContainersIntent::RunCancelled);

        assert_eq!(folder_recheck(&state).units(), 0);
    }

    #[test]
    fn the_folder_timer_needs_a_folder_row() {
        let mut no_rows = found(&[EngineKind::Docker]);
        let _ = reduce(&mut no_rows, ContainersIntent::OpenCreate);

        assert_eq!(folder_recheck(&no_rows).units(), 0);
        assert_eq!(folder_recheck(&form_with_source("otty-data")).units(), 0);
        assert_eq!(folder_recheck(&form_with_source("  ")).units(), 0);
    }

    #[test]
    fn the_folder_timer_stops_when_containers_mode_is_left() {
        let mut state = form_with_source("/srv/site");

        let _ = reduce(&mut state, ContainersIntent::Leave);

        assert!(state.run_form().is_some());
        assert_eq!(folder_recheck(&state).units(), 0);
    }

    #[test]
    fn the_engine_timer_runs_while_the_card_shows() {
        let state = found(&[]);

        assert_eq!(engine_search_timer(&state).units(), 1);
    }

    #[test]
    fn the_engine_timer_runs_for_every_card() {
        for discovery in [
            fixtures::discovery_with_cli(&[], Some("/bin/docker"), None),
            fixtures::discovery_with_cli(&[], None, Some("/bin/podman")),
            fixtures::discovery_with_cli(
                &[],
                Some("/bin/docker"),
                Some("/bin/podman"),
            ),
            fixtures::denied_discovery(
                fixtures::DOCKER_SOCKET,
                otty_containers::CandidateSource::Docker,
            ),
        ] {
            let mut state = found(&[]);

            let _ = reduce(
                &mut state,
                ContainersIntent::DiscoveryFinished(discovery),
            );

            assert_eq!(engine_search_timer(&state).units(), 1);
        }
    }

    #[test]
    fn the_engine_timer_does_not_run_while_engines_are_shown() {
        let state = found(&[EngineKind::Docker]);

        assert_eq!(engine_search_timer(&state).units(), 0);
    }

    #[test]
    fn the_engine_timer_does_not_run_before_the_first_search_finishes() {
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);

        assert_eq!(engine_search_timer(&state).units(), 0);
    }

    #[test]
    fn the_engine_timer_stops_when_containers_mode_is_left() {
        let mut state = found(&[]);

        let _ = reduce(&mut state, ContainersIntent::Leave);

        assert_eq!(engine_search_timer(&state).units(), 0);
    }

    #[test]
    fn the_engine_timer_stops_as_soon_as_the_card_goes() {
        let mut state = found(&[]);

        let _ = reduce(
            &mut state,
            ContainersIntent::BackgroundSearchFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );

        assert_eq!(engine_search_timer(&state).units(), 0);
    }

    /// While Check again runs, the card is replaced by "searching".
    #[test]
    fn the_engine_timer_waits_while_check_again_runs() {
        let mut state = found(&[]);

        let _ = reduce(&mut state, ContainersIntent::CheckAgain);

        assert_eq!(engine_search_timer(&state).units(), 0);
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
                ContainersIntent::ListChanged {
                    engine: engine.clone(),
                    resource,
                }
            );
        }
    }

    #[test]
    fn a_started_container_names_its_engine_and_id() {
        let engine = fixtures::engine(EngineKind::Docker).address().clone();

        let intent = watch_intent(
            &engine,
            WatchEvent::ContainerStarted(String::from("c178")),
        );

        assert_eq!(
            intent,
            ContainersIntent::ContainerStarted {
                engine,
                id: String::from("c178"),
            }
        );
    }

    #[test]
    fn a_reconnect_or_a_lost_engine_names_the_engine() {
        let engine = fixtures::engine(EngineKind::Podman).address().clone();

        let reconnected = watch_intent(&engine, WatchEvent::Reconnected);
        let unreachable = watch_intent(&engine, WatchEvent::Unreachable);

        assert_eq!(
            reconnected,
            ContainersIntent::EngineReconnected(engine.clone())
        );
        assert_eq!(unreachable, ContainersIntent::EngineUnreachable(engine));
    }
}
