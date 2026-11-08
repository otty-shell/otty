use otty_containers::ContainerState;

use super::super::format::engine_label;
use super::super::state::{ContainersState, EngineState, ListLoad};
use super::super::types::{Place, ResourceKind};
use super::no_engine::NoEngineViewModel;
use crate::i18n::{self, Key};

/// The engine tree of the left panel: its rows and the short text under them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EngineTreeViewModel {
    /// Each engine and the four lists of each engine that is not closed.
    pub(crate) rows: Vec<EngineTreeRowViewModel>,
    /// The short text under the rows while no engine is usable, such as why
    /// none is shown; `None` while the search runs or an engine answered.
    pub(crate) note: Option<&'static str>,
}

impl EngineTreeViewModel {
    /// Build the rows of the engine tree and the note under them.
    pub(in super::super) fn from_state(state: &ContainersState) -> Self {
        Self {
            rows: tree_rows(state),
            note: NoEngineViewModel::from_state(state)
                .map(|card| card.panel_text),
        }
    }
}

/// One row of the engine tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EngineTreeRowViewModel {
    pub(crate) label: String,
    /// 0 for an engine, 1 for its lists.
    pub(crate) depth: u16,
    /// An engine root: it has a caret and a local target dot.
    pub(crate) is_engine: bool,
    /// The engine root shows its lists (the user did not close it).
    pub(crate) is_open: bool,
    /// What the row shows on its right.
    pub(crate) note: TreeNote,
    /// The row of the list the main area shows.
    pub(crate) is_selected: bool,
    /// On a list row, the list a press shows. On an engine root, only its
    /// engine counts: a press opens or closes that engine.
    pub(crate) place: Place,
}

/// What a tree row shows on its right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TreeNote {
    /// Nothing, for example while the list loads.
    Nothing,
    /// The number of items in the list.
    Count(usize),
    /// A short text, for example "2 running".
    Text(String),
    /// The list failed to load.
    Failed,
}

/// The rows of the engine tree, top to bottom: each engine, and the four
/// lists of each engine that is not closed. An engine opens and closes on its
/// own, whichever list the main area shows; while the engine of the shown
/// list is closed, no row is highlighted.
fn tree_rows(state: &ContainersState) -> Vec<EngineTreeRowViewModel> {
    let open = state.place();
    let mut rows = Vec::new();

    for engine in state.engines() {
        let address = engine.info().address();
        let is_open = !state.is_engine_closed(address);

        rows.push(EngineTreeRowViewModel {
            label: engine_label(engine.info()),
            depth: 0,
            is_engine: true,
            is_open,
            note: TreeNote::Nothing,
            is_selected: false,
            place: Place::new(address.clone(), ResourceKind::Containers),
        });
        if !is_open {
            continue;
        }

        for resource in ResourceKind::ALL {
            let place = Place::new(address.clone(), resource);
            let is_shown = open.is_some_and(|open| {
                open.engine() == address && open.resource() == resource
            });
            rows.push(EngineTreeRowViewModel {
                label: i18n::t(resource.title_key()).to_string(),
                depth: 1,
                is_engine: false,
                is_open: false,
                note: list_note(engine, resource),
                is_selected: is_shown,
                place,
            });
        }
    }

    rows
}

/// The note on the right of a list row: the count (running containers for
/// Containers), nothing while it loads, or that it failed.
fn list_note(engine: &EngineState, resource: ResourceKind) -> TreeNote {
    match resource {
        ResourceKind::Containers => {
            note(engine.containers().load(), |containers| {
                let running = containers
                    .iter()
                    .filter(|container| {
                        container.state() == ContainerState::Running
                    })
                    .count();
                TreeNote::Text(i18n::with_count(
                    Key::TplContainersRunning,
                    running as u64,
                ))
            })
        },
        ResourceKind::Images => note(engine.images().load(), |images| {
            TreeNote::Count(images.len())
        }),
        ResourceKind::Volumes => note(engine.volumes().load(), |volumes| {
            TreeNote::Count(volumes.len())
        }),
        ResourceKind::Networks => note(engine.networks().load(), |networks| {
            TreeNote::Count(networks.len())
        }),
    }
}

/// The note of a list in each load state; `loaded` gives it for the items.
fn note<T>(load: &ListLoad<T>, loaded: fn(&[T]) -> TreeNote) -> TreeNote {
    match load {
        ListLoad::Loaded(items) => loaded(items),
        ListLoad::Failed(_) => TreeNote::Failed,
        ListLoad::Loading => TreeNote::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use otty_containers::{EngineKind, fixtures};

    use super::super::test_support::{
        build, docker, found, highlighted, labels, lists, load, open_labels,
        podman, press_root,
    };
    use super::*;
    use crate::i18n::{self, Key};
    use crate::widgets::containers::event::ContainersIntent;
    use crate::widgets::containers::reducer::reduce;
    use crate::widgets::containers::state::ContainersState;
    use crate::widgets::containers::types::{
        ContainersError, Place, ResourceKind,
    };

    fn tree_row(
        label: &str,
        depth: u16,
        note: TreeNote,
        is_selected: bool,
        place: Place,
    ) -> EngineTreeRowViewModel {
        EngineTreeRowViewModel {
            label: label.to_string(),
            depth,
            is_engine: false,
            is_open: false,
            note,
            is_selected,
            place,
        }
    }

    #[test]
    fn a_running_search_has_no_panel_text() {
        let state = ContainersState::default();

        assert_eq!(EngineTreeViewModel::from_state(&state).note, None);
    }

    #[test]
    fn the_tree_has_either_the_engine_rows_or_the_note_of_the_card() {
        let with_engine = found(&[EngineKind::Docker]);
        let without_engine = found(&[]);

        let tree = EngineTreeViewModel::from_state(&with_engine);
        let card_tree = EngineTreeViewModel::from_state(&without_engine);

        assert_eq!(tree.rows.len(), 5);
        assert_eq!(tree.note, None);
        assert!(card_tree.rows.is_empty());
        assert_eq!(
            card_tree.note,
            Some(i18n::t(Key::ContainersPanelNotInstalled))
        );
    }

    #[test]
    fn every_engine_is_open_by_default() {
        let state = found(&[EngineKind::Docker, EngineKind::Podman]);

        let roots: Vec<bool> = EngineTreeViewModel::from_state(&state)
            .rows
            .iter()
            .filter(|row| row.is_engine)
            .map(|row| row.is_open)
            .collect();

        assert_eq!(roots, vec![true, true]);
        assert_eq!(
            labels(&state),
            [open_labels("Docker 29.9.1"), open_labels("Podman 5.8.8")]
                .concat()
        );
    }

    #[test]
    fn an_engine_found_later_is_open_and_a_closed_one_stays_closed() {
        let mut state = found(&[EngineKind::Docker]);
        press_root(&mut state, &docker());

        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        );

        assert_eq!(
            labels(&state),
            [
                vec![String::from("Docker 29.9.1")],
                open_labels("Podman 5.8.8")
            ]
            .concat()
        );
    }

    #[test]
    fn a_root_press_hides_or_shows_the_lists_of_that_engine_only() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);

        press_root(&mut state, &podman());
        let podman_closed = labels(&state);
        let root = EngineTreeViewModel::from_state(&state)
            .rows
            .into_iter()
            .find(|row| row.label == "Podman 5.8.8")
            .expect("the root stays");
        press_root(&mut state, &docker());
        let both_closed = labels(&state);
        press_root(&mut state, &podman());
        let docker_closed = labels(&state);
        press_root(&mut state, &docker());

        assert_eq!(
            podman_closed,
            [
                open_labels("Docker 29.9.1"),
                vec![String::from("Podman 5.8.8")]
            ]
            .concat()
        );
        assert!(!root.is_open);
        assert_eq!(both_closed, vec!["Docker 29.9.1", "Podman 5.8.8"]);
        assert_eq!(
            docker_closed,
            [
                vec![String::from("Docker 29.9.1")],
                open_labels("Podman 5.8.8")
            ]
            .concat()
        );
        assert_eq!(
            labels(&state),
            [open_labels("Docker 29.9.1"), open_labels("Podman 5.8.8")]
                .concat()
        );
    }

    #[test]
    fn each_open_engine_shows_the_counts_of_its_own_lists() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let mut podman_lists = lists(EngineKind::Podman);
        podman_lists.images = podman_lists.images.map(|mut images| {
            images.truncate(1);
            images
        });
        podman_lists.volumes = Err(ContainersError::new("no"));
        load(&mut state, &podman(), podman_lists);

        let tree = EngineTreeViewModel::from_state(&state).rows;

        assert_eq!(tree[2].note, TreeNote::Count(3));
        assert_eq!(tree[3].note, TreeNote::Count(2));
        assert_eq!(tree[7].note, TreeNote::Count(1));
        assert_eq!(tree[8].note, TreeNote::Failed);
    }

    #[test]
    fn only_the_shown_list_is_highlighted_among_the_engines() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);

        let _ = reduce(
            &mut state,
            ContainersIntent::Open(Place::new(
                podman().address().clone(),
                ResourceKind::Images,
            )),
        );

        assert_eq!(
            highlighted(&state),
            vec![(
                i18n::t(Key::ContainersImages).to_string(),
                podman().address().clone()
            )]
        );
    }

    #[test]
    fn opening_the_shown_engine_again_highlights_its_list() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);

        press_root(&mut state, &docker());
        press_root(&mut state, &docker());

        assert_eq!(
            highlighted(&state),
            vec![(
                i18n::t(Key::ContainersContainers).to_string(),
                docker().address().clone()
            )]
        );
    }

    #[test]
    fn closing_another_engine_keeps_the_highlight_of_the_shown_list() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);

        press_root(&mut state, &podman());

        assert_eq!(
            highlighted(&state),
            vec![(
                i18n::t(Key::ContainersContainers).to_string(),
                docker().address().clone()
            )]
        );
    }

    #[test]
    fn the_tree_shows_every_engine_with_its_lists_and_the_loaded_counts() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let docker_at =
            |resource| Place::new(docker().address().clone(), resource);
        let podman_at =
            |resource| Place::new(podman().address().clone(), resource);

        let vm = build(&state);

        assert_eq!(
            vm.tree.rows,
            vec![
                EngineTreeRowViewModel {
                    is_engine: true,
                    is_open: true,
                    ..tree_row(
                        "Docker 29.9.1",
                        0,
                        TreeNote::Nothing,
                        false,
                        docker_at(ResourceKind::Containers)
                    )
                },
                tree_row(
                    i18n::t(Key::ContainersContainers),
                    1,
                    TreeNote::Text(i18n::with_count(
                        Key::TplContainersRunning,
                        2
                    )),
                    true,
                    docker_at(ResourceKind::Containers)
                ),
                tree_row(
                    i18n::t(Key::ContainersImages),
                    1,
                    TreeNote::Count(3),
                    false,
                    docker_at(ResourceKind::Images)
                ),
                tree_row(
                    i18n::t(Key::ContainersVolumes),
                    1,
                    TreeNote::Count(2),
                    false,
                    docker_at(ResourceKind::Volumes)
                ),
                tree_row(
                    i18n::t(Key::ContainersNetworks),
                    1,
                    TreeNote::Count(4),
                    false,
                    docker_at(ResourceKind::Networks)
                ),
                EngineTreeRowViewModel {
                    is_engine: true,
                    is_open: true,
                    ..tree_row(
                        "Podman 5.8.8",
                        0,
                        TreeNote::Nothing,
                        false,
                        podman_at(ResourceKind::Containers)
                    )
                },
                tree_row(
                    i18n::t(Key::ContainersContainers),
                    1,
                    TreeNote::Nothing,
                    false,
                    podman_at(ResourceKind::Containers)
                ),
                tree_row(
                    i18n::t(Key::ContainersImages),
                    1,
                    TreeNote::Nothing,
                    false,
                    podman_at(ResourceKind::Images)
                ),
                tree_row(
                    i18n::t(Key::ContainersVolumes),
                    1,
                    TreeNote::Nothing,
                    false,
                    podman_at(ResourceKind::Volumes)
                ),
                tree_row(
                    i18n::t(Key::ContainersNetworks),
                    1,
                    TreeNote::Nothing,
                    false,
                    podman_at(ResourceKind::Networks)
                ),
            ]
        );
    }

    #[test]
    fn the_tree_has_no_compose_group_even_with_compose_containers() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));

        let vm = build(&state);

        let labels: Vec<&str> =
            vm.tree.rows.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(
            labels,
            vec![
                "Docker 29.9.1",
                "Containers",
                "Images",
                "Volumes",
                "Networks"
            ]
        );
        assert!(vm.tree.rows.iter().all(|row| row.depth < 2));
    }

    #[test]
    fn a_loading_engine_shows_only_its_lists() {
        let state = found(&[EngineKind::Docker]);

        let vm = build(&state);

        assert_eq!(vm.tree.rows.len(), 5);
    }

    #[test]
    fn a_loading_list_has_no_count_and_a_failed_one_says_so() {
        let mut state = found(&[EngineKind::Docker]);
        let mut partial = lists(EngineKind::Docker);
        partial.volumes = Err(ContainersError::new("no"));
        load(&mut state, &docker(), partial);
        let loading = found(&[EngineKind::Docker]);

        let vm = build(&state);
        let loading_vm = build(&loading);

        assert_eq!(vm.tree.rows[3].note, TreeNote::Failed);
        assert_eq!(loading_vm.tree.rows[1].note, TreeNote::Nothing);
        assert_eq!(loading_vm.tree.rows[2].note, TreeNote::Nothing);
    }
}
