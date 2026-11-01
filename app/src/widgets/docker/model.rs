use otty_docker::{
    Container, ContainerState, EngineInfo, EngineKind, Image, Network, Volume,
};
use otty_ui_kit::components::data_table::{TableCell, TableColumn};
use otty_ui_kit::components::status_badge::StatusTone;

use super::action_model::{PruneViewModel, can_create, prune_button};
use super::no_engine::{NoEngineViewModel, no_engine};
use super::resource_rows::{
    columns, container_cells, container_status, image_cells, image_status,
    network_cells, network_status, volume_cells, volume_status,
};
use super::state::{DockerState, EngineState, ListLoad, ResourceList, Search};
use super::types::{
    Place, ResourceKind, compose_projects, filtered, image_name, network_name,
    shown_containers, volume_name,
};
use crate::i18n::{self, Key};

/// Brand name of a Docker engine; not translated.
const DOCKER_NAME: &str = "Docker";
/// Brand name of a Podman engine; not translated.
const PODMAN_NAME: &str = "Podman";

/// One row of the engine tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TreeRowViewModel {
    pub(crate) label: String,
    /// 0 for an engine, 1 for its lists.
    pub(crate) depth: u16,
    /// An engine root: it has a caret and a local target dot.
    pub(crate) is_engine: bool,
    /// The engine root shows its lists (its place is open).
    pub(crate) is_open: bool,
    /// What the row shows on its right.
    pub(crate) note: TreeNote,
    /// The row of the open list.
    pub(crate) is_selected: bool,
    /// What a press on the row opens.
    pub(crate) place: Place,
}

/// What a tree row shows on its right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TreeNote {
    /// Nothing, for example while the list loads.
    Nothing,
    /// The number of items in the list.
    Count(usize),
    /// A short text, for example "2 running" or "this computer".
    Text(String),
    /// The list failed to load.
    Failed,
}

/// What the main area of Docker mode shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MainViewModel<'a> {
    /// The engine search runs and no engine is known yet.
    Searching,
    /// The search found no engine; the card says why and what to do.
    NoEngine(NoEngineViewModel),
    /// The open list of an engine.
    Table(TableViewModel<'a>),
}

/// The header row and the table of the open list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TableViewModel<'a> {
    pub(crate) resource: ResourceKind,
    /// The engine chip, e.g. "Docker 29.9.1".
    pub(crate) engine_label: String,
    /// The name filter text.
    pub(crate) filter: &'a str,
    /// The "compose: <name>" chip while the containers are limited to a
    /// Compose project.
    pub(crate) compose_chip: Option<String>,
    pub(crate) columns: Vec<TableColumn>,
    /// Rows that match the filter, in the engine's order.
    pub(crate) rows: Vec<Vec<TableCell>>,
    /// Position of the selected row among `rows`.
    pub(crate) selected: Option<usize>,
    /// Shown instead of rows: loading, empty list or no match.
    pub(crate) empty_message: Option<String>,
    /// The engine's message when the list failed to load.
    pub(crate) error: Option<&'a str>,
    /// The Prune button of the list.
    pub(crate) prune: Option<PruneViewModel>,
    /// The notice of the last failed action, with a dismiss cross.
    pub(crate) notice: Option<&'a str>,
    /// Whether the create button opens a dialog.
    pub(crate) can_create: bool,
}

/// Header of the inspector: the selected row's name and its status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectorViewModel {
    pub(crate) title: String,
    pub(crate) status: String,
    pub(crate) tone: StatusTone,
}

/// What every table of the main area needs besides its list.
struct Shown<'a> {
    resource: ResourceKind,
    engine_label: String,
    filter: &'a str,
    compose_chip: Option<String>,
}

/// The rows of the engine tree, top to bottom: each engine, and the four
/// lists of the engine whose place is open. Containers are followed by the
/// Compose projects, when there are any.
pub(crate) fn tree_rows(state: &DockerState) -> Vec<TreeRowViewModel> {
    let open = state.place();
    let mut rows = Vec::new();

    for engine in state.engines() {
        let address = engine.info().address();
        let is_open = open.is_some_and(|place| place.engine() == address);

        rows.push(TreeRowViewModel {
            label: engine_label(engine.info()),
            depth: 0,
            is_engine: true,
            is_open,
            note: TreeNote::Text(i18n::t(Key::ThisComputerNote).to_string()),
            is_selected: false,
            place: Place::new(address.clone(), ResourceKind::Containers),
        });
        if !is_open {
            continue;
        }

        for resource in ResourceKind::ALL {
            let place = Place::new(address.clone(), resource);
            rows.push(TreeRowViewModel {
                label: i18n::t(resource.title_key()).to_string(),
                depth: 1,
                is_engine: false,
                is_open: false,
                note: list_note(engine, resource),
                is_selected: open == Some(&place),
                place,
            });
            if resource == ResourceKind::Containers {
                rows.extend(compose_rows(engine, open));
            }
        }
    }

    rows
}

/// The "Compose projects" row and one row per project under it; none when
/// no container belongs to a project. The group row opens the containers,
/// the same as the Containers row.
fn compose_rows(
    engine: &EngineState,
    open: Option<&Place>,
) -> Vec<TreeRowViewModel> {
    let address = engine.info().address();
    let projects = compose_projects(engine.containers().items());
    if projects.is_empty() {
        return Vec::new();
    }

    let mut rows = vec![TreeRowViewModel {
        label: i18n::t(Key::DockerComposeProjects).to_string(),
        depth: 1,
        is_engine: false,
        is_open: false,
        note: TreeNote::Count(projects.len()),
        is_selected: false,
        place: Place::new(address.clone(), ResourceKind::Containers),
    }];
    for (name, count) in projects {
        let place = Place::compose(address.clone(), name);
        rows.push(TreeRowViewModel {
            label: name.to_string(),
            depth: 2,
            is_engine: false,
            is_open: false,
            note: TreeNote::Count(count),
            is_selected: open == Some(&place),
            place,
        });
    }

    rows
}

/// What the main area shows; ages are counted up to `now` (Unix seconds).
pub(crate) fn main_area(state: &DockerState, now: i64) -> MainViewModel<'_> {
    let (Some(place), Some(engine)) = (state.place(), state.open_engine())
    else {
        return no_engine_view(state)
            .map_or(MainViewModel::Searching, MainViewModel::NoEngine);
    };

    let shown = Shown {
        resource: place.resource(),
        engine_label: engine_label(engine.info()),
        filter: state.filter(),
        compose_chip: place.compose_project().map(i18n::docker_compose_chip),
    };
    let filter = shown.filter;

    let table = match place.resource() {
        ResourceKind::Containers => {
            let items = engine.containers().items();
            table(
                shown,
                engine.containers(),
                shown_containers(items, filter, place.compose_project()),
                Container::id,
                |container| container_cells(container, now),
            )
        },
        ResourceKind::Images => {
            let items = engine.images().items();
            table(
                shown,
                engine.images(),
                filtered(items, filter, image_name),
                Image::id,
                |image| image_cells(image, now),
            )
        },
        ResourceKind::Volumes => {
            let items = engine.volumes().items();
            table(
                shown,
                engine.volumes(),
                filtered(items, filter, volume_name),
                Volume::name,
                |volume| volume_cells(volume, now),
            )
        },
        ResourceKind::Networks => {
            let items = engine.networks().items();
            table(
                shown,
                engine.networks(),
                filtered(items, filter, network_name),
                Network::id,
                |network| network_cells(network, now),
            )
        },
    };
    MainViewModel::Table(TableViewModel {
        prune: prune_button(state),
        notice: state.notice(),
        can_create: can_create(state),
        ..table
    })
}

/// The short text of the left panel while no engine is usable; `None` when
/// the search runs or an engine answered.
pub(crate) fn panel_note(state: &DockerState) -> Option<&'static str> {
    no_engine_view(state).map(|vm| vm.panel_text)
}

/// The inspector header of the selected row of the open list; `None` when
/// nothing is selected.
pub(crate) fn inspector(state: &DockerState) -> Option<InspectorViewModel> {
    let place = state.place()?;
    let engine = state.open_engine()?;

    let (title, (status, tone)) = match place.resource() {
        ResourceKind::Containers => {
            let container = engine.containers().selected(Container::id)?;
            (container.name(), container_status(container))
        },
        ResourceKind::Images => {
            let image = engine.images().selected(Image::id)?;
            (image_name(image), image_status(image))
        },
        ResourceKind::Volumes => {
            let volume = engine.volumes().selected(Volume::name)?;
            (volume.name(), volume_status(volume))
        },
        ResourceKind::Networks => {
            let network = engine.networks().selected(Network::id)?;
            (network.name(), network_status(network))
        },
    };

    Some(InspectorViewModel {
        title: title.to_string(),
        status,
        tone,
    })
}

/// The engine name and version for the tree root and the chip, e.g.
/// "Docker 29.9.1" or "Podman 5.8.8".
pub(crate) fn engine_label(info: &EngineInfo) -> String {
    let kind = match info.kind() {
        EngineKind::Docker => DOCKER_NAME,
        EngineKind::Podman => PODMAN_NAME,
    };

    match info.version() {
        Some(version) => format!("{kind} {version}"),
        None => kind.to_string(),
    }
}

/// The note on the right of a list row: the count (running containers for
/// Containers), nothing while it loads, or that it failed.
/// The no-engine card of a finished search; `None` while it runs.
pub(super) fn no_engine_view(state: &DockerState) -> Option<NoEngineViewModel> {
    match state.search() {
        Search::Done(discovery) => no_engine(discovery),
        _ => None,
    }
}

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
                    Key::TplDockerRunning,
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

/// The table of one list: the rows of `items` (the items that match the
/// filter), the position of the selected row among them, and the message or
/// error shown instead of rows. `id_of` gives the selection id.
fn table<'a, T>(
    shown: Shown<'a>,
    list: &'a ResourceList<T>,
    items: Vec<&'a T>,
    id_of: fn(&T) -> &str,
    cells: impl Fn(&T) -> Vec<TableCell>,
) -> TableViewModel<'a> {
    let selected = list
        .selected_id()
        .and_then(|id| items.iter().position(|item| id_of(item) == id));
    let rows: Vec<Vec<TableCell>> = items.into_iter().map(cells).collect();

    let (error, empty_message) = match list.load() {
        ListLoad::Failed(message) => (Some(message.as_str()), None),
        ListLoad::Loading => {
            (None, Some(i18n::t(Key::DockerLoading).to_string()))
        },
        ListLoad::Loaded(all) if all.is_empty() => {
            (None, Some(i18n::t(shown.resource.empty_key()).to_string()))
        },
        ListLoad::Loaded(_) if rows.is_empty() => {
            (None, Some(i18n::docker_nothing_matches(shown.filter)))
        },
        ListLoad::Loaded(_) => (None, None),
    };

    TableViewModel {
        resource: shown.resource,
        engine_label: shown.engine_label,
        filter: shown.filter,
        compose_chip: shown.compose_chip,
        columns: columns(shown.resource),
        rows,
        selected,
        empty_message,
        error,
        prune: None,
        notice: None,
        can_create: false,
    }
}

#[cfg(test)]
mod tests {
    use otty_docker::{Container, EngineKind, fixtures};

    use super::*;
    use crate::i18n::{self, Key};
    use crate::widgets::docker::event::DockerIntent;
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::resource_rows;
    use crate::widgets::docker::types::LoadedLists;

    const NOW: i64 = 1_793_557_960;

    /// All three parts of Docker mode at [`NOW`].
    struct Vm<'a> {
        tree: Vec<TreeRowViewModel>,
        main: MainViewModel<'a>,
        inspector: Option<InspectorViewModel>,
    }

    fn build(state: &DockerState) -> Vm<'_> {
        Vm {
            tree: tree_rows(state),
            main: main_area(state, NOW),
            inspector: inspector(state),
        }
    }

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

    fn found(kinds: &[EngineKind]) -> DockerState {
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(kinds)),
        );
        state
    }

    fn load(state: &mut DockerState, info: &EngineInfo, lists: LoadedLists) {
        let _ = reduce(
            state,
            DockerIntent::ListsLoaded {
                engine: info.address().clone(),
                lists,
            },
        );
    }

    fn open(state: &mut DockerState, resource: ResourceKind) {
        let place = Place::new(docker().address().clone(), resource);
        let _ = reduce(state, DockerIntent::Open(place));
    }

    fn table<'a>(vm: &'a Vm<'_>) -> &'a TableViewModel<'a> {
        match &vm.main {
            MainViewModel::Table(table) => table,
            other => panic!("expected a table, got {other:?}"),
        }
    }

    fn tree_row(
        label: &str,
        depth: u16,
        note: TreeNote,
        is_selected: bool,
        place: Place,
    ) -> TreeRowViewModel {
        TreeRowViewModel {
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
    fn a_search_that_found_nothing_shows_the_no_engine_card_and_panel_text() {
        let state = found(&[]);

        let vm = build(&state);

        let MainViewModel::NoEngine(card) = vm.main else {
            panic!("expected the no engine card");
        };
        assert_eq!(card.title, i18n::t(Key::DockerNotInstalledTitle));
        assert_eq!(
            panel_note(&state),
            Some(i18n::t(Key::DockerPanelNotInstalled))
        );
        assert!(vm.tree.is_empty());
        assert_eq!(vm.inspector, None);
    }

    #[test]
    fn a_running_search_has_no_panel_text() {
        let state = DockerState::default();

        assert_eq!(panel_note(&state), None);
    }

    #[test]
    fn engines_are_named_by_kind_and_version() {
        assert_eq!(engine_label(&docker()), "Docker 29.9.1");
        assert_eq!(engine_label(&podman()), "Podman 5.8.8");
    }

    #[test]
    fn before_any_engine_is_known_the_main_area_says_it_searches() {
        let state = DockerState::default();

        let vm = build(&state);

        assert_eq!(vm.main, MainViewModel::Searching);
        assert!(vm.tree.is_empty());
        assert_eq!(vm.inspector, None);
    }

    #[test]
    fn the_tree_shows_each_engine_and_the_counts_of_the_open_one() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let docker_at =
            |resource| Place::new(docker().address().clone(), resource);
        let this_computer =
            TreeNote::Text(i18n::t(Key::ThisComputerNote).into());

        let vm = build(&state);

        assert_eq!(
            vm.tree,
            vec![
                TreeRowViewModel {
                    is_engine: true,
                    is_open: true,
                    ..tree_row(
                        "Docker 29.9.1",
                        0,
                        this_computer.clone(),
                        false,
                        docker_at(ResourceKind::Containers)
                    )
                },
                tree_row(
                    i18n::t(Key::DockerContainers),
                    1,
                    TreeNote::Text(i18n::with_count(Key::TplDockerRunning, 2)),
                    true,
                    docker_at(ResourceKind::Containers)
                ),
                tree_row(
                    i18n::t(Key::DockerComposeProjects),
                    1,
                    TreeNote::Count(1),
                    false,
                    docker_at(ResourceKind::Containers)
                ),
                tree_row(
                    "otty-fx",
                    2,
                    TreeNote::Count(1),
                    false,
                    Place::compose(docker().address().clone(), "otty-fx")
                ),
                tree_row(
                    i18n::t(Key::DockerImages),
                    1,
                    TreeNote::Count(3),
                    false,
                    docker_at(ResourceKind::Images)
                ),
                tree_row(
                    i18n::t(Key::DockerVolumes),
                    1,
                    TreeNote::Count(2),
                    false,
                    docker_at(ResourceKind::Volumes)
                ),
                tree_row(
                    i18n::t(Key::DockerNetworks),
                    1,
                    TreeNote::Count(4),
                    false,
                    docker_at(ResourceKind::Networks)
                ),
                TreeRowViewModel {
                    is_engine: true,
                    ..tree_row(
                        "Podman 5.8.8",
                        0,
                        this_computer,
                        false,
                        Place::new(
                            podman().address().clone(),
                            ResourceKind::Containers
                        )
                    )
                },
            ]
        );
    }

    #[test]
    fn compose_projects_are_listed_under_containers_with_their_counts() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let address = docker().address().clone();

        let vm = build(&state);

        let labels: Vec<&str> =
            vm.tree.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(
            labels,
            vec![
                "Docker 29.9.1",
                "Containers",
                "Compose projects",
                "otty-fx",
                "Images",
                "Volumes",
                "Networks",
            ]
        );
        assert_eq!(vm.tree[2].depth, 1);
        assert_eq!(vm.tree[2].note, TreeNote::Count(1));
        assert!(!vm.tree[2].is_selected);
        assert_eq!(
            vm.tree[2].place,
            Place::new(address.clone(), ResourceKind::Containers)
        );
        assert_eq!(vm.tree[3].depth, 2);
        assert_eq!(vm.tree[3].note, TreeNote::Count(1));
        assert_eq!(vm.tree[3].place, Place::compose(address, "otty-fx"));
    }

    #[test]
    fn without_compose_projects_there_is_no_compose_projects_row() {
        let mut state = found(&[EngineKind::Docker]);
        let mut plain = lists(EngineKind::Docker);
        plain.containers = Ok(plain
            .containers
            .unwrap_or_default()
            .into_iter()
            .filter(|container| container.compose_project().is_none())
            .collect());
        load(&mut state, &docker(), plain);

        let vm = build(&state);

        assert!(
            vm.tree
                .iter()
                .all(|row| row.label != i18n::t(Key::DockerComposeProjects))
        );
        assert_eq!(vm.tree.len(), 5);
    }

    #[test]
    fn a_loading_engine_has_no_compose_projects_row() {
        let state = found(&[EngineKind::Docker]);

        let vm = build(&state);

        assert_eq!(vm.tree.len(), 5);
    }

    #[test]
    fn opening_a_project_selects_its_row_and_limits_the_containers() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let project = Place::compose(docker().address().clone(), "otty-fx");
        let _ = reduce(&mut state, DockerIntent::Open(project));

        let vm = build(&state);
        let table = table(&vm);

        let selected: Vec<&str> = vm
            .tree
            .iter()
            .filter(|row| row.is_selected)
            .map(|row| row.label.as_str())
            .collect();
        assert_eq!(selected, vec!["otty-fx"]);
        assert_eq!(table.resource, ResourceKind::Containers);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(
            table.compose_chip,
            Some(i18n::docker_compose_chip("otty-fx"))
        );
        assert_eq!(table.empty_message, None);
    }

    #[test]
    fn the_project_limit_works_with_the_name_filter() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let project = Place::compose(docker().address().clone(), "otty-fx");
        let _ = reduce(&mut state, DockerIntent::Open(project));
        let _ = reduce(&mut state, DockerIntent::FilterChanged("exit".into()));

        let vm = build(&state);

        assert!(table(&vm).rows.is_empty());
        assert!(table(&vm).empty_message.is_some());
    }

    #[test]
    fn the_whole_containers_list_has_no_chip() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));

        let vm = build(&state);

        assert_eq!(table(&vm).compose_chip, None);
        assert!(table(&vm).rows.len() > 1);
    }

    #[test]
    fn clearing_the_project_shows_all_containers_again() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let project = Place::compose(docker().address().clone(), "otty-fx");
        let _ = reduce(&mut state, DockerIntent::Open(project));

        let _ = reduce(&mut state, DockerIntent::ClearComposeProject);

        let vm = build(&state);
        assert_eq!(table(&vm).compose_chip, None);
        assert!(table(&vm).rows.len() > 1);
        assert!(
            vm.tree
                .iter()
                .any(|row| row.label == "Containers" && row.is_selected)
        );
    }

    #[test]
    fn a_loading_list_has_no_count_and_a_failed_one_says_so() {
        let mut state = found(&[EngineKind::Docker]);
        let mut partial = lists(EngineKind::Docker);
        partial.volumes = Err(String::from("no"));
        load(&mut state, &docker(), partial);
        let loading = found(&[EngineKind::Docker]);

        let vm = build(&state);
        let loading_vm = build(&loading);

        assert_eq!(vm.tree[5].note, TreeNote::Failed);
        assert_eq!(loading_vm.tree[1].note, TreeNote::Nothing);
        assert_eq!(loading_vm.tree[2].note, TreeNote::Nothing);
    }

    #[test]
    fn the_containers_table_has_a_row_per_container_and_the_first_selected() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let resources = fixtures::resources(EngineKind::Docker);

        let vm = build(&state);
        let table = table(&vm);

        assert_eq!(table.resource, ResourceKind::Containers);
        assert_eq!(table.engine_label, "Docker 29.9.1");
        assert_eq!(
            table.columns,
            resource_rows::columns(ResourceKind::Containers)
        );
        assert_eq!(table.rows.len(), 6);
        assert_eq!(
            table.rows[0],
            resource_rows::container_cells(&resources.containers()[0], NOW)
        );
        assert_eq!(table.selected, Some(0));
        assert_eq!(table.empty_message, None);
        assert_eq!(table.error, None);
    }

    #[test]
    fn each_resource_table_uses_its_own_columns_and_rows() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let resources = fixtures::resources(EngineKind::Docker);

        for (resource, first_row, count) in [
            (
                ResourceKind::Images,
                resource_rows::image_cells(&resources.images()[0], NOW),
                3,
            ),
            (
                ResourceKind::Volumes,
                resource_rows::volume_cells(&resources.volumes()[0], NOW),
                2,
            ),
            (
                ResourceKind::Networks,
                resource_rows::network_cells(&resources.networks()[0], NOW),
                4,
            ),
        ] {
            open(&mut state, resource);
            let vm = build(&state);
            let table = table(&vm);

            assert_eq!(table.columns, resource_rows::columns(resource));
            assert_eq!(table.rows.len(), count);
            assert_eq!(table.rows[0], first_row);
        }
    }

    #[test]
    fn the_filter_keeps_matching_rows_and_moves_the_selected_position() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let _ = reduce(&mut state, DockerIntent::SelectRow(3));
        let _ = reduce(
            &mut state,
            DockerIntent::FilterChanged(String::from("exit")),
        );

        let vm = build(&state);
        let table = table(&vm);

        assert_eq!(table.filter, "exit");
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[1][0], TableCell::mono("otty-fx-exit0"));
        assert_eq!(table.selected, Some(1));
    }

    #[test]
    fn a_selected_row_outside_the_filter_is_not_highlighted() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let _ = reduce(
            &mut state,
            DockerIntent::FilterChanged(String::from("paused")),
        );

        let vm = build(&state);
        let table = table(&vm);

        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.selected, None);
        assert_eq!(
            vm.inspector.map(|inspector| inspector.title),
            Some(String::from("otty-fx-web-1"))
        );
    }

    #[test]
    fn a_filter_without_matches_says_nothing_matches() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let _ = reduce(
            &mut state,
            DockerIntent::FilterChanged(String::from("zzz")),
        );

        let vm = build(&state);

        assert!(table(&vm).rows.is_empty());
        assert_eq!(
            table(&vm).empty_message,
            Some(i18n::docker_nothing_matches("zzz"))
        );
    }

    #[test]
    fn an_empty_or_loading_list_says_so() {
        let mut state = found(&[EngineKind::Docker]);
        let loading = build(&state);
        let loading_message = table(&loading).empty_message.clone();
        let mut empty = lists(EngineKind::Docker);
        empty.containers = Ok(Vec::new());
        load(&mut state, &docker(), empty);

        let vm = build(&state);

        assert_eq!(
            loading_message,
            Some(i18n::t(Key::DockerLoading).to_string())
        );
        assert_eq!(
            table(&vm).empty_message,
            Some(i18n::t(Key::DockerNoContainers).to_string())
        );
        assert_eq!(vm.inspector, None);
    }

    #[test]
    fn a_failed_list_shows_the_engine_message_and_no_rows() {
        let mut state = found(&[EngineKind::Docker]);
        let mut failed = lists(EngineKind::Docker);
        failed.containers = Err(String::from("engine said no"));
        load(&mut state, &docker(), failed);

        let vm = build(&state);

        assert_eq!(table(&vm).error, Some("engine said no"));
        assert!(table(&vm).rows.is_empty());
        assert_eq!(table(&vm).empty_message, None);
        assert_eq!(vm.inspector, None);
    }

    #[test]
    fn the_inspector_shows_the_selected_row_of_each_resource() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let resources = fixtures::resources(EngineKind::Docker);
        let first: &Container = &resources.containers()[0];

        let containers = build(&state).inspector;
        open(&mut state, ResourceKind::Images);
        let images = build(&state).inspector;
        open(&mut state, ResourceKind::Volumes);
        let volumes = build(&state).inspector;
        open(&mut state, ResourceKind::Networks);
        let networks = build(&state).inspector;

        let header = |title: &str, (status, tone): (String, StatusTone)| {
            Some(InspectorViewModel {
                title: title.to_string(),
                status,
                tone,
            })
        };
        assert_eq!(
            containers,
            header(first.name(), resource_rows::container_status(first))
        );
        assert_eq!(
            images,
            header(
                "<none>:<none>",
                resource_rows::image_status(&resources.images()[0])
            )
        );
        assert_eq!(
            volumes,
            header(
                resources.volumes()[0].name(),
                resource_rows::volume_status(&resources.volumes()[0])
            )
        );
        assert_eq!(
            networks,
            header(
                "bridge",
                resource_rows::network_status(&resources.networks()[0])
            )
        );
    }
}
