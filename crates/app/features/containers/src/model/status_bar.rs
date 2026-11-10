use std::collections::HashSet;

use otty_app_i18n::{self as i18n, Key};
use otty_containers::{Container, ContainerState, Image};

use super::super::format::{engine_label, size_text};
use super::super::state::{
    ContainersState, EngineState, ListLoad, ResourceList,
};
use super::super::types::ResourceKind;
use super::no_engine::NoEngineViewModel;

/// Separates the parts of the status bar text next to the engine, e.g.
/// "6 containers, 3 running".
const PART_SEPARATOR: &str = ", ";

/// The texts of the Containers mode status bar: the engine on screen and the
/// counts of the open list, or why there is no engine (spec Decision 5). The
/// frame adds the sessions button and the version on the right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusBarViewModel {
    /// The engine on screen, e.g. "Docker 29.9.1": the one the Run form will
    /// run on while it is open, else the one of the open list; `None` when no
    /// engine is shown (the bar names the mode instead).
    pub(crate) engine: Option<String>,
    /// The counts of the open list, or the reason there is no engine.
    pub(crate) info: String,
}

impl StatusBarViewModel {
    /// Build the status bar texts for the current state.
    pub(in super::super) fn from_state(state: &ContainersState) -> Self {
        let (Some(place), Some(open)) = (state.place(), state.open_engine())
        else {
            let info = match NoEngineViewModel::from_state(state) {
                Some(card) => card.status_text,
                None => i18n::t(Key::ContainersStatusSearching).to_string(),
            };

            return Self { engine: None, info };
        };

        // The Run form replaces the table, so the counts of the table would
        // describe something that is not on screen. It can run on another
        // engine than the open list, and it keeps its engine when that goes
        // away; the bar names what the form names.
        if let Some(engine) = state.run().engine() {
            return Self {
                engine: Some(engine_label(engine)),
                info: i18n::t(Key::ContainersRunContainer).to_string(),
            };
        }

        Self {
            engine: Some(engine_label(open.info())),
            info: counts(open, place.resource()),
        }
    }
}

/// The status bar counts of one list. A list that loads or failed shows that,
/// not a number.
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
                let total = i18n::containers_status_count(
                    Key::ContainersStatusOneContainer,
                    Key::TplContainersStatusContainers,
                    containers.len(),
                );
                let running =
                    i18n::with_count(Key::TplContainersRunning, running as u64);

                format!("{total}{PART_SEPARATOR}{running}")
            })
        },
        ResourceKind::Images => list_counts(engine.images(), |images| {
            let total = i18n::containers_status_count(
                Key::ContainersStatusOneImage,
                Key::TplContainersStatusImages,
                images.len(),
            );
            let bytes = images_size(images);

            format!("{total}{PART_SEPARATOR}{}", size_text(bytes))
        }),
        ResourceKind::Volumes => list_counts(engine.volumes(), |volumes| {
            i18n::containers_status_count(
                Key::ContainersStatusOneVolume,
                Key::TplContainersStatusVolumes,
                volumes.len(),
            )
        }),
        ResourceKind::Networks => list_counts(engine.networks(), |networks| {
            i18n::containers_status_count(
                Key::ContainersStatusOneNetwork,
                Key::TplContainersStatusNetworks,
                networks.len(),
            )
        }),
    }
}

/// The bytes of `images` on disk; the rows of one image count once.
fn images_size(images: &[Image]) -> u64 {
    let mut counted = HashSet::new();

    images
        .iter()
        .filter(|image| counted.insert(image.id()))
        .map(Image::size)
        .sum()
}

/// The status bar text of a list in each load state; `loaded` gives it for
/// the items.
fn list_counts<T>(
    list: &ResourceList<T>,
    loaded: impl FnOnce(&[T]) -> String,
) -> String {
    match list.load() {
        ListLoad::Loaded(items) => loaded(items),
        ListLoad::Loading => i18n::t(Key::ContainersLoading).to_string(),
        ListLoad::Failed(_) => {
            i18n::t(Key::ContainersListFailedShort).to_string()
        },
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;
    use otty_containers::{ContainerState, EngineKind, Image, fixtures};
    use otty_ui_kit::components::status_bar::StatusBar;
    use otty_ui_kit::components::tag_search::TagSearchAction;

    use super::super::test_support::{docker, found, lists, open, podman};
    use super::*;
    use crate::event::ContainersIntent;
    use crate::format::size_text;
    use crate::reducer::reduce;
    use crate::run::RunFormViewModel;
    use crate::run::event::RunIntent;
    use crate::state::{ContainersState, EngineState};
    use crate::types::{ContainersError, LoadedLists, ResourceKind};
    use crate::view::status_bar::items as status_items;

    // What the status bar says.

    fn loaded(kinds: &[EngineKind]) -> ContainersState {
        let mut state = found(kinds);
        for kind in kinds {
            let _ = reduce(
                &mut state,
                ContainersIntent::ListsLoaded {
                    engine: fixtures::engine(*kind).address().clone(),
                    lists: lists(*kind),
                },
            );
        }
        state
    }

    #[test]
    fn the_counts_are_of_the_whole_list_whatever_the_page_filters() {
        let mut state = loaded(&[EngineKind::Docker]);
        let whole_list = StatusBarViewModel::from_state(&state).info;

        let _ = reduce(
            &mut state,
            ContainersIntent::TagSearch(TagSearchAction::InputChanged(
                String::from("no such name"),
            )),
        );

        assert_eq!(StatusBarViewModel::from_state(&state).info, whole_list);
    }

    #[test]
    fn while_the_run_form_is_open_the_bar_says_run_container() {
        let mut state = loaded(&[EngineKind::Docker]);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        let vm = StatusBarViewModel::from_state(&state);

        assert_eq!(vm.engine.as_deref(), Some("Docker 29.9.1"));
        assert_eq!(vm.info, "Run container");
    }

    /// Both engines loaded, the Containers list of Docker open and the Run
    /// form open on Podman.
    fn run_form_on_podman() -> ContainersState {
        let mut state = loaded(&[EngineKind::Docker, EngineKind::Podman]);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        let _ = reduce(
            &mut state,
            ContainersIntent::Run(RunIntent::EngineChosen(
                podman().address().clone(),
            )),
        );
        state
    }

    #[test]
    fn the_bar_names_the_engine_the_run_form_will_run_on() {
        let state = run_form_on_podman();

        let vm = StatusBarViewModel::from_state(&state);

        assert_eq!(state.open_engine().map(EngineState::info), Some(&docker()));
        assert_eq!(vm.engine.as_deref(), Some("Podman 5.8.8"));
        assert_eq!(vm.info, "Run container");
    }

    #[test]
    fn the_bar_follows_the_run_form_when_its_engine_changes_back() {
        let mut state = run_form_on_podman();

        let _ = reduce(
            &mut state,
            ContainersIntent::Run(RunIntent::EngineChosen(
                docker().address().clone(),
            )),
        );

        assert_eq!(
            StatusBarViewModel::from_state(&state).engine.as_deref(),
            Some("Docker 29.9.1")
        );
    }

    #[test]
    fn the_bar_goes_back_to_the_open_list_when_the_run_form_closes() {
        let mut state = run_form_on_podman();

        let _ = reduce(&mut state, ContainersIntent::Run(RunIntent::Close));

        let vm = StatusBarViewModel::from_state(&state);
        assert_eq!(vm.engine.as_deref(), Some("Docker 29.9.1"));
        assert!(vm.info.contains("containers"), "{}", vm.info);
    }

    /// The form stays on the engine that went away: it names that engine, a
    /// run goes there and fails, and the bar must not name another one.
    #[test]
    fn the_bar_and_the_form_keep_naming_an_engine_that_went_away() {
        let mut state = run_form_on_podman();

        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );

        let vm = StatusBarViewModel::from_state(&state);
        assert!(state.engine(podman().address()).is_none());
        assert_eq!(vm.engine.as_deref(), Some("Podman 5.8.8"));
        assert_eq!(vm.info, "Run container");
        let form = RunFormViewModel::from_state(state.run(), &state.run_ctx())
            .expect("the form stays");
        assert_eq!(vm.engine.as_ref(), Some(&form.engine.label));
    }

    /// Without any engine the form waits behind the no-engine card, so the bar
    /// gives the reason there is none, as the card does.
    #[test]
    fn with_no_engine_left_the_bar_gives_the_reason_while_the_form_waits() {
        let mut state = run_form_on_podman();

        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[])),
        );

        let vm = StatusBarViewModel::from_state(&state);
        assert!(state.run().is_open());
        assert_eq!(vm.engine, None);
        assert_eq!(vm.info, "no engine found");
    }

    #[test]
    fn while_searching_the_bar_says_it_looks_for_engines() {
        let state = ContainersState::default();

        let vm = StatusBarViewModel::from_state(&state);

        assert_eq!(vm.engine, None);
        assert_eq!(vm.info, "Looking for engines…");
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

        let vm = StatusBarViewModel::from_state(&state);

        assert_eq!(vm.engine.as_deref(), Some("Docker 29.9.1"));
        assert_eq!(vm.info, format!("{total} containers, {running} running"));
    }

    #[test]
    fn images_count_every_row_and_the_size_of_each_image_once() {
        let mut state = loaded(&[EngineKind::Docker]);
        let two_tags =
            fixtures::image_rows("sha256:app", &["app:1", "app:2"], &[], &[]);
        let bytes = two_tags[0].size();
        let resources = fixtures::resources(EngineKind::Docker);
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: fixtures::engine(EngineKind::Docker).address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(two_tags),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        );
        open(&mut state, ResourceKind::Images);

        let vm = StatusBarViewModel::from_state(&state);

        assert_eq!(vm.info, format!("2 images, {}", size_text(bytes)));
    }

    #[test]
    fn images_show_the_total_and_the_size() {
        let mut state = loaded(&[EngineKind::Docker]);
        open(&mut state, ResourceKind::Images);
        let resources = fixtures::resources(EngineKind::Docker);
        let count = resources.images().len();
        let bytes: u64 = resources.images().iter().map(Image::size).sum();

        let vm = StatusBarViewModel::from_state(&state);

        assert_eq!(vm.info, format!("{count} images, {}", size_text(bytes)));
    }

    #[test]
    fn volumes_and_networks_show_their_count() {
        let mut state = loaded(&[EngineKind::Docker]);
        let resources = fixtures::resources(EngineKind::Docker);

        open(&mut state, ResourceKind::Volumes);
        let volumes = StatusBarViewModel::from_state(&state).info;
        open(&mut state, ResourceKind::Networks);
        let networks = StatusBarViewModel::from_state(&state).info;

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
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: LoadedLists {
                    containers: Ok(vec![resources.containers()[0].clone()]),
                    images: Ok(vec![resources.images()[0].clone()]),
                    volumes: Ok(vec![resources.volumes()[0].clone()]),
                    networks: Ok(vec![resources.networks()[0].clone()]),
                },
            },
        );

        let containers = StatusBarViewModel::from_state(&state).info;
        open(&mut state, ResourceKind::Images);
        let images = StatusBarViewModel::from_state(&state).info;
        open(&mut state, ResourceKind::Volumes);
        let volumes = StatusBarViewModel::from_state(&state).info;
        open(&mut state, ResourceKind::Networks);
        let networks = StatusBarViewModel::from_state(&state).info;

        assert!(containers.starts_with("1 container, "), "{containers}");
        assert!(images.starts_with("1 image, "), "{images}");
        assert_eq!(volumes, "1 volume");
        assert_eq!(networks, "1 network");
    }

    /// Every engine shows in the tree, so the bar speaks only of the open one.
    #[test]
    fn with_two_engines_the_bar_names_only_the_open_one() {
        let themes = ThemeManager::new();
        let state = loaded(&[EngineKind::Docker, EngineKind::Podman]);

        let vm = StatusBarViewModel::from_state(&state);
        let mut bar = StatusBar::<()>::new(themes.current().ui());
        for item in status_items(&vm) {
            bar = bar.left(item);
        }
        let mut ui = simulator(bar);

        ui.find("Docker 29.9.1").expect("the open engine");
        assert!(ui.find("Podman 5.8.8").is_err());
        assert!(!vm.info.contains("Podman"), "{}", vm.info);
    }

    #[test]
    fn a_list_that_loads_shows_no_number() {
        let state = found(&[EngineKind::Docker]);

        let vm = StatusBarViewModel::from_state(&state);

        assert_eq!(vm.engine.as_deref(), Some("Docker 29.9.1"));
        assert_eq!(vm.info, "Loading…");
    }

    #[test]
    fn a_list_that_failed_shows_no_number() {
        let mut state = found(&[EngineKind::Docker]);
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: LoadedLists {
                    containers: Err(ContainersError::new("boom")),
                    images: Ok(Vec::new()),
                    volumes: Ok(Vec::new()),
                    networks: Ok(Vec::new()),
                },
            },
        );

        let vm = StatusBarViewModel::from_state(&state);

        assert_eq!(vm.info, "failed");
    }

    #[test]
    fn without_an_engine_the_bar_gives_the_reason() {
        let state = found(&[]);

        let vm = StatusBarViewModel::from_state(&state);

        assert_eq!(vm.engine, None);
        assert_eq!(vm.info, "no engine found");
    }
}
