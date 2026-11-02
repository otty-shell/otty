//! What the Docker mode status bar says: the engine, the counts of the open
//! list, the other engines found, or why there is no engine (spec Decision 5).

use otty_docker::{Container, ContainerState, Image};

use super::format::size_text;
use super::model::{engine_label, no_engine_view};
use super::state::{DockerState, EngineState, ListLoad, ResourceList};
use super::types::ResourceKind;
use crate::i18n::{self, Key};

/// Separates the parts of the text next to the engine, e.g. "6 containers,
/// 3 running".
const PART_SEPARATOR: &str = ", ";

/// The texts of the Docker mode status bar. The sessions button is added by
/// the view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusViewModel {
    /// The engine of the open list, e.g. "Docker 29.9.1"; `None` when no
    /// engine is shown (the bar names the mode instead).
    pub(crate) engine: Option<String>,
    /// The counts of the open list, or the reason there is no engine.
    pub(crate) info: String,
    /// The other engines that answered, e.g. "Podman 5.8.8 also found".
    pub(crate) also_found: Option<String>,
}

/// Build the status bar texts for the current state.
pub(crate) fn status_bar(state: &DockerState) -> StatusViewModel {
    let (Some(place), Some(open)) = (state.place(), state.open_engine()) else {
        let info = match no_engine_view(state) {
            Some(card) => card.status_text,
            None => i18n::t(Key::DockerStatusSearching).to_string(),
        };

        return StatusViewModel {
            engine: None,
            info,
            also_found: None,
        };
    };

    let others: Vec<String> = state
        .engines()
        .iter()
        .filter(|engine| engine.info().address() != open.info().address())
        .map(|engine| engine_label(engine.info()))
        .collect();
    let also_found = if others.is_empty() {
        None
    } else {
        Some(i18n::docker_status_also_found(&others.join(PART_SEPARATOR)))
    };

    // The Run form replaces the table, so the counts of the table would
    // describe something that is not on screen.
    let info = if state.run_form().is_some() {
        i18n::t(Key::DockerRunContainer).to_string()
    } else {
        counts(open, place.resource())
    };

    StatusViewModel {
        engine: Some(engine_label(open.info())),
        info,
        also_found,
    }
}

/// The counts of one list. A list that loads or failed shows that, not a
/// number.
fn counts(engine: &EngineState, resource: ResourceKind) -> String {
    match resource {
        ResourceKind::Containers => {
            list_counts(engine.containers(), |containers| {
                let running = containers
                    .iter()
                    .filter(|container: &&Container| {
                        container.state() == ContainerState::Running
                    })
                    .count();
                let total = i18n::docker_status_count(
                    Key::DockerStatusOneContainer,
                    Key::TplDockerStatusContainers,
                    containers.len(),
                );
                let running =
                    i18n::with_count(Key::TplDockerRunning, running as u64);

                format!("{total}{PART_SEPARATOR}{running}")
            })
        },
        ResourceKind::Images => list_counts(engine.images(), |images| {
            let total = i18n::docker_status_count(
                Key::DockerStatusOneImage,
                Key::TplDockerStatusImages,
                images.len(),
            );
            let bytes: u64 = images.iter().map(Image::size).sum();

            format!("{total}{PART_SEPARATOR}{}", size_text(bytes))
        }),
        ResourceKind::Volumes => list_counts(engine.volumes(), |volumes| {
            i18n::docker_status_count(
                Key::DockerStatusOneVolume,
                Key::TplDockerStatusVolumes,
                volumes.len(),
            )
        }),
        ResourceKind::Networks => list_counts(engine.networks(), |networks| {
            i18n::docker_status_count(
                Key::DockerStatusOneNetwork,
                Key::TplDockerStatusNetworks,
                networks.len(),
            )
        }),
    }
}

/// The text of a list in each load state; `loaded` gives it for the items.
fn list_counts<T>(
    list: &ResourceList<T>,
    loaded: impl FnOnce(&[T]) -> String,
) -> String {
    match list.load() {
        ListLoad::Loaded(items) => loaded(items),
        ListLoad::Loading => i18n::t(Key::DockerLoading).to_string(),
        ListLoad::Failed(_) => i18n::t(Key::DockerListFailedShort).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineInfo, EngineKind, fixtures};

    use super::*;
    use crate::widgets::docker::event::DockerIntent;
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::types::{LoadedLists, Place};

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn found(kinds: &[EngineKind]) -> DockerState {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(kinds)),
        );
        state
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

    fn loaded(kinds: &[EngineKind]) -> DockerState {
        let mut state = found(kinds);
        for kind in kinds {
            let _ = reduce(
                &mut state,
                DockerIntent::ListsLoaded {
                    engine: fixtures::engine(*kind).address().clone(),
                    lists: lists(*kind),
                },
            );
        }
        state
    }

    fn open(state: &mut DockerState, resource: ResourceKind) {
        let place = Place::new(docker().address().clone(), resource);
        let _ = reduce(state, DockerIntent::Open(place));
    }

    #[test]
    fn while_searching_the_bar_says_it_looks_for_engines() {
        let state = DockerState::default();

        let vm = status_bar(&state);

        assert_eq!(vm.engine, None);
        assert_eq!(vm.info, "Looking for engines…");
        assert_eq!(vm.also_found, None);
    }

    #[test]
    fn while_the_run_form_is_open_the_bar_says_run_container() {
        let mut state = loaded(&[EngineKind::Docker]);
        let _ = reduce(&mut state, DockerIntent::OpenCreate);

        let vm = status_bar(&state);

        assert_eq!(vm.engine.as_deref(), Some("Docker 29.9.1"));
        assert_eq!(vm.info, "Run container");
    }

    #[test]
    fn containers_show_the_total_and_the_running_ones() {
        let state = loaded(&[EngineKind::Docker]);
        let containers = fixtures::resources(EngineKind::Docker);
        let total = containers.containers().len();
        let running = containers
            .containers()
            .iter()
            .filter(|c| c.state() == ContainerState::Running)
            .count();

        let vm = status_bar(&state);

        assert_eq!(vm.engine.as_deref(), Some("Docker 29.9.1"));
        assert_eq!(vm.info, format!("{total} containers, {running} running"));
        assert_eq!(vm.also_found, None);
    }

    #[test]
    fn images_show_the_total_and_the_size() {
        let mut state = loaded(&[EngineKind::Docker]);
        open(&mut state, ResourceKind::Images);
        let resources = fixtures::resources(EngineKind::Docker);
        let count = resources.images().len();
        let bytes: u64 = resources.images().iter().map(Image::size).sum();

        let vm = status_bar(&state);

        assert_eq!(vm.info, format!("{count} images, {}", size_text(bytes)));
    }

    #[test]
    fn volumes_and_networks_show_their_count() {
        let mut state = loaded(&[EngineKind::Docker]);
        let resources = fixtures::resources(EngineKind::Docker);

        open(&mut state, ResourceKind::Volumes);
        let volumes = status_bar(&state).info;
        open(&mut state, ResourceKind::Networks);
        let networks = status_bar(&state).info;

        assert_eq!(volumes, format!("{} volumes", resources.volumes().len()));
        assert_eq!(
            networks,
            format!("{} networks", resources.networks().len())
        );
    }

    #[test]
    fn a_count_of_one_is_singular() {
        let mut state = found(&[EngineKind::Docker]);
        let resources = fixtures::resources(EngineKind::Docker);
        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: LoadedLists {
                    containers: Ok(vec![resources.containers()[0].clone()]),
                    images: Ok(vec![resources.images()[0].clone()]),
                    volumes: Ok(vec![resources.volumes()[0].clone()]),
                    networks: Ok(vec![resources.networks()[0].clone()]),
                },
            },
        );

        let containers = status_bar(&state).info;
        open(&mut state, ResourceKind::Images);
        let images = status_bar(&state).info;
        open(&mut state, ResourceKind::Volumes);
        let volumes = status_bar(&state).info;
        open(&mut state, ResourceKind::Networks);
        let networks = status_bar(&state).info;

        assert!(containers.starts_with("1 container, "), "{containers}");
        assert!(images.starts_with("1 image, "), "{images}");
        assert_eq!(volumes, "1 volume");
        assert_eq!(networks, "1 network");
    }

    #[test]
    fn with_two_engines_the_other_one_is_also_found() {
        let state = loaded(&[EngineKind::Docker, EngineKind::Podman]);

        let vm = status_bar(&state);

        assert_eq!(vm.engine.as_deref(), Some("Docker 29.9.1"));
        assert_eq!(vm.also_found.as_deref(), Some("Podman 5.8.8 also found"));
    }

    #[test]
    fn a_list_that_loads_shows_no_number() {
        let state = found(&[EngineKind::Docker]);

        let vm = status_bar(&state);

        assert_eq!(vm.engine.as_deref(), Some("Docker 29.9.1"));
        assert_eq!(vm.info, "Loading…");
    }

    #[test]
    fn a_list_that_failed_shows_no_number() {
        let mut state = found(&[EngineKind::Docker]);
        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: LoadedLists {
                    containers: Err(String::from("boom")),
                    images: Ok(Vec::new()),
                    volumes: Ok(Vec::new()),
                    networks: Ok(Vec::new()),
                },
            },
        );

        let vm = status_bar(&state);

        assert_eq!(vm.info, "failed");
    }

    #[test]
    fn without_an_engine_the_bar_gives_the_reason() {
        let state = found(&[]);

        let vm = status_bar(&state);

        assert_eq!(vm.engine, None);
        assert_eq!(vm.info, "no engine found");
        assert_eq!(vm.also_found, None);
    }
}
