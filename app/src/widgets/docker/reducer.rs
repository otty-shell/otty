use iced::Task;

use super::event::{DockerEffect, DockerEvent, DockerIntent};
use super::state::DockerState;

/// Reduce a Docker intent into state changes and effects. Engine work
/// (search, list loads) is only asked for while Docker mode is active
/// (spec Decision 14); the reducer itself does no I/O.
pub(crate) fn reduce(
    state: &mut DockerState,
    intent: DockerIntent,
) -> Task<DockerEvent> {
    match intent {
        DockerIntent::Enter => {
            state.set_active(true);
            state.start_search();
            effect(DockerEffect::Discover)
        },
        DockerIntent::Leave => {
            state.set_active(false);
            Task::none()
        },
        DockerIntent::DiscoveryFinished(discovery) => {
            state.finish_search(discovery);
            load_every_engine(state)
        },
        DockerIntent::ListsLoaded { engine, lists } => {
            if !state.apply_lists(&engine, lists) {
                log::debug!("docker lists of a gone engine dropped: {engine}");
            }
            Task::none()
        },
        DockerIntent::Open(place) => {
            state.open(place);
            Task::none()
        },
        DockerIntent::ClearComposeProject => {
            state.clear_compose_project();
            Task::none()
        },
        DockerIntent::SelectRow(index) => {
            let _ = state.select_shown_row(index);
            Task::none()
        },
        DockerIntent::FilterChanged(filter) => {
            state.set_filter(filter);
            Task::none()
        },
        DockerIntent::Refresh => load_every_engine(state),
        DockerIntent::CheckAgain => {
            state.start_search();
            effect(DockerEffect::Discover)
        },
        DockerIntent::CopyCommand(command) => {
            effect(DockerEffect::CopyToClipboard(command))
        },
        DockerIntent::OpenLink(url) => effect(DockerEffect::OpenUrl(url)),
    }
}

/// Ask to load the lists of every engine, only while Docker mode is
/// active: a search that finishes after the user left starts nothing.
fn load_every_engine(state: &DockerState) -> Task<DockerEvent> {
    if !state.is_active() {
        return Task::none();
    }

    Task::batch(
        state.engines().iter().map(|engine| {
            effect(DockerEffect::LoadLists(engine.info().clone()))
        }),
    )
}

fn effect(effect: DockerEffect) -> Task<DockerEvent> {
    Task::done(DockerEvent::Effect(effect))
}

#[cfg(test)]
mod tests {
    use otty_docker::{
        Container, EngineInfo, EngineKind, Image, Network, Volume, fixtures,
    };

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::docker::state::{ListLoad, Search};
    use crate::widgets::docker::types::{LoadedLists, Place, ResourceKind};

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn podman() -> EngineInfo {
        fixtures::engine(EngineKind::Podman)
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

    fn effects(task: Task<DockerEvent>) -> Vec<DockerEffect> {
        collect_outputs(task)
            .into_iter()
            .filter_map(|event| match event {
                DockerEvent::Effect(effect) => Some(effect),
                DockerEvent::Intent(_) => None,
            })
            .collect()
    }

    /// Docker mode active, both fixture engines found, the Docker engine's
    /// lists loaded.
    fn loaded_state() -> DockerState {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        );
        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: lists(EngineKind::Docker),
            },
        );
        state
    }

    fn open(state: &mut DockerState, resource: ResourceKind) {
        let place = Place::new(docker().address().clone(), resource);
        let _ = reduce(state, DockerIntent::Open(place));
    }

    fn selected_container(state: &DockerState) -> Option<&str> {
        state.open_engine()?.containers().selected_id()
    }

    #[test]
    fn checking_again_starts_a_new_search() {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[])),
        );
        assert!(matches!(state.search(), Search::Done(_)));

        let effects = effects(reduce(&mut state, DockerIntent::CheckAgain));

        assert_eq!(state.search(), &Search::Searching);
        assert_eq!(effects, vec![DockerEffect::Discover]);
    }

    #[test]
    fn an_engine_started_meanwhile_is_found_by_checking_again() {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[])),
        );
        let _ = reduce(&mut state, DockerIntent::CheckAgain);

        let effects = effects(reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        ));

        assert_eq!(state.engines().len(), 1);
        assert_eq!(effects, vec![DockerEffect::LoadLists(docker())]);
    }

    #[test]
    fn copying_a_command_asks_for_the_clipboard() {
        let mut state = DockerState::default();

        let effects = effects(reduce(
            &mut state,
            DockerIntent::CopyCommand(String::from(
                "sudo systemctl start docker",
            )),
        ));

        assert_eq!(
            effects,
            vec![DockerEffect::CopyToClipboard(String::from(
                "sudo systemctl start docker"
            ))]
        );
    }

    #[test]
    fn opening_a_link_asks_for_the_browser() {
        let mut state = DockerState::default();

        let effects = effects(reduce(
            &mut state,
            DockerIntent::OpenLink(String::from("https://podman.io")),
        ));

        assert_eq!(
            effects,
            vec![DockerEffect::OpenUrl(String::from("https://podman.io"))]
        );
    }

    #[test]
    fn entering_docker_mode_searches_for_engines() {
        let mut state = DockerState::default();

        let effects = effects(reduce(&mut state, DockerIntent::Enter));

        assert!(state.is_active());
        assert_eq!(state.search(), &Search::Searching);
        assert_eq!(effects, vec![DockerEffect::Discover]);
    }

    #[test]
    fn found_engines_load_their_lists_and_open_the_first_engines_containers() {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);

        let effects = effects(reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        ));

        assert_eq!(
            effects,
            vec![
                DockerEffect::LoadLists(docker()),
                DockerEffect::LoadLists(podman())
            ]
        );
        assert_eq!(state.engines().len(), 2);
        assert_eq!(
            state.place(),
            Some(&Place::new(
                docker().address().clone(),
                ResourceKind::Containers
            ))
        );
    }

    #[test]
    fn no_engine_work_starts_outside_docker_mode() {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(&mut state, DockerIntent::Leave);

        let after_search = effects(reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        ));
        let after_refresh = effects(reduce(&mut state, DockerIntent::Refresh));

        assert!(!state.is_active());
        assert!(after_search.is_empty());
        assert!(after_refresh.is_empty());
        assert_eq!(state.engines().len(), 1);
    }

    #[test]
    fn no_engine_clears_the_engines_and_the_place() {
        let mut state = loaded_state();

        let effects = effects(reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[])),
        ));

        assert!(effects.is_empty());
        assert!(state.engines().is_empty());
        assert!(state.place().is_none());
        assert!(matches!(state.search(), Search::Done(_)));
    }

    #[test]
    fn a_new_search_keeps_the_lists_and_the_place_of_engines_found_again() {
        let mut state = loaded_state();
        open(&mut state, ResourceKind::Images);

        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );

        let engine = state.open_engine().expect("docker is still there");
        assert_eq!(engine.containers().items().len(), 6);
        assert_eq!(state.engines().len(), 1);
        assert_eq!(
            state.place().map(Place::resource),
            Some(ResourceKind::Images)
        );
    }

    #[test]
    fn loaded_lists_select_their_first_rows() {
        let state = loaded_state();

        let engine = state.open_engine().expect("docker engine");
        assert_eq!(
            selected_container(&state),
            Some(engine.containers().items()[0].id())
        );
        assert_eq!(
            engine.images().selected_id(),
            Some(engine.images().items()[0].id())
        );
        assert_eq!(
            engine.volumes().selected_id(),
            Some(engine.volumes().items()[0].name())
        );
        assert_eq!(
            engine.networks().selected_id(),
            Some(engine.networks().items()[0].id())
        );
    }

    #[test]
    fn opening_each_resource_shows_it_and_clears_the_filter() {
        let mut state = loaded_state();

        // Containers are open first, so they come back last.
        for resource in [
            ResourceKind::Images,
            ResourceKind::Volumes,
            ResourceKind::Networks,
            ResourceKind::Containers,
        ] {
            let _ = reduce(
                &mut state,
                DockerIntent::FilterChanged(String::from("otty")),
            );

            open(&mut state, resource);

            assert_eq!(state.place().map(Place::resource), Some(resource));
            assert_eq!(state.filter(), "");
        }
    }

    #[test]
    fn opening_the_shown_place_again_keeps_the_filter() {
        let mut state = loaded_state();
        let _ =
            reduce(&mut state, DockerIntent::FilterChanged(String::from("x")));

        open(&mut state, ResourceKind::Containers);

        assert_eq!(state.filter(), "x");
    }

    #[test]
    fn opening_a_compose_project_keeps_the_containers_and_clears_the_filter() {
        let mut state = loaded_state();
        let _ =
            reduce(&mut state, DockerIntent::FilterChanged(String::from("x")));
        let project = Place::compose(docker().address().clone(), "otty-fx");

        let _ = reduce(&mut state, DockerIntent::Open(project));

        let place = state.place().expect("an open place");
        assert_eq!(place.resource(), ResourceKind::Containers);
        assert_eq!(place.compose_project(), Some("otty-fx"));
        assert_eq!(state.filter(), "");
    }

    #[test]
    fn clearing_the_compose_project_opens_all_containers() {
        let mut state = loaded_state();
        let project = Place::compose(docker().address().clone(), "otty-fx");
        let _ = reduce(&mut state, DockerIntent::Open(project));

        let _ = reduce(&mut state, DockerIntent::ClearComposeProject);

        let place = state.place().expect("an open place");
        assert_eq!(place.resource(), ResourceKind::Containers);
        assert_eq!(place.compose_project(), None);
    }

    #[test]
    fn clearing_without_a_project_changes_nothing() {
        let mut state = loaded_state();
        let _ =
            reduce(&mut state, DockerIntent::FilterChanged(String::from("x")));

        let _ = reduce(&mut state, DockerIntent::ClearComposeProject);

        assert_eq!(state.filter(), "x");
    }

    #[test]
    fn a_row_press_in_a_compose_project_selects_that_project_container() {
        let mut state = loaded_state();
        let project = Place::compose(docker().address().clone(), "otty-fx");
        let _ = reduce(&mut state, DockerIntent::Open(project));

        let _ = reduce(&mut state, DockerIntent::SelectRow(0));

        let engine = state.open_engine().expect("docker engine");
        let selected = engine.containers().selected(Container::id);
        assert_eq!(selected.map(Container::name), Some("otty-fx-web-1"));
    }

    #[test]
    fn a_row_press_selects_that_row_of_the_filtered_table() {
        let mut state = loaded_state();
        let _ = reduce(
            &mut state,
            DockerIntent::FilterChanged(String::from("exit")),
        );

        let _ = reduce(&mut state, DockerIntent::SelectRow(1));

        let engine = state.open_engine().expect("docker engine");
        let selected = engine.containers().selected(Container::id);
        assert_eq!(selected.map(Container::name), Some("otty-fx-exit0"));
    }

    #[test]
    fn a_row_press_on_each_resource_selects_its_row() {
        let mut state = loaded_state();

        for resource in ResourceKind::ALL {
            open(&mut state, resource);
            let _ = reduce(&mut state, DockerIntent::SelectRow(1));
        }

        let engine = state.open_engine().expect("docker engine");
        assert_eq!(
            engine.containers().selected(Container::id),
            engine.containers().items().get(1)
        );
        assert_eq!(
            engine.images().selected(Image::id),
            engine.images().items().get(1)
        );
        assert_eq!(
            engine.volumes().selected(Volume::name),
            engine.volumes().items().get(1)
        );
        assert_eq!(
            engine.networks().selected(Network::id),
            engine.networks().items().get(1)
        );
    }

    #[test]
    fn a_row_press_past_the_last_row_changes_nothing() {
        let mut state = loaded_state();
        let before = selected_container(&state).map(str::to_string);

        let _ = reduce(&mut state, DockerIntent::SelectRow(99));

        assert_eq!(selected_container(&state).map(str::to_string), before);
    }

    #[test]
    fn a_reload_keeps_the_selection_while_the_row_is_there() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, DockerIntent::SelectRow(2));
        let selected = selected_container(&state).map(str::to_string);
        let mut reloaded = lists(EngineKind::Docker);
        if let Ok(containers) = reloaded.containers.as_mut() {
            containers.remove(0);
        }

        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: reloaded,
            },
        );

        assert_eq!(selected_container(&state).map(str::to_string), selected);
    }

    #[test]
    fn a_reload_without_the_selected_row_selects_the_first_row() {
        let mut state = loaded_state();
        let _ = reduce(&mut state, DockerIntent::SelectRow(2));
        let mut reloaded = lists(EngineKind::Docker);
        if let Ok(containers) = reloaded.containers.as_mut() {
            containers.remove(2);
        }
        let first = match &reloaded.containers {
            Ok(containers) => containers[0].id().to_string(),
            Err(_) => String::new(),
        };

        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: reloaded,
            },
        );

        assert_eq!(selected_container(&state), Some(first.as_str()));
    }

    #[test]
    fn a_failed_list_shows_its_error_and_keeps_the_other_lists() {
        let mut state = loaded_state();
        let mut reloaded = lists(EngineKind::Docker);
        reloaded.images = Err(String::from("engine said no"));

        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: reloaded,
            },
        );

        let engine = state.open_engine().expect("docker engine");
        assert_eq!(
            engine.images().load(),
            &ListLoad::Failed(String::from("engine said no"))
        );
        assert_eq!(engine.containers().items().len(), 6);
        assert_eq!(engine.volumes().items().len(), 2);
        assert_eq!(engine.networks().items().len(), 4);
    }

    #[test]
    fn lists_of_an_engine_that_is_gone_are_dropped() {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );

        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: podman().address().clone(),
                lists: lists(EngineKind::Podman),
            },
        );

        assert_eq!(state.engines().len(), 1);
        assert!(state.engine(podman().address()).is_none());
    }

    #[test]
    fn refresh_in_docker_mode_reloads_every_engine() {
        let mut state = loaded_state();

        let effects = effects(reduce(&mut state, DockerIntent::Refresh));

        assert_eq!(
            effects,
            vec![
                DockerEffect::LoadLists(docker()),
                DockerEffect::LoadLists(podman())
            ]
        );
    }

    #[test]
    fn leaving_docker_mode_keeps_what_is_shown() {
        let mut state = loaded_state();

        let effects = effects(reduce(&mut state, DockerIntent::Leave));

        assert!(effects.is_empty());
        assert!(!state.is_active());
        assert_eq!(state.engines().len(), 2);
        assert!(state.place().is_some());
    }
}
