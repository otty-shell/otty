use otty_docker::{
    Container, ContainerState, EngineInfo, EngineKind, Image, Network, Volume,
};
use otty_ui_kit::components::data_table::{TableCell, TableColumn};
use otty_ui_kit::components::status_badge::StatusTone;
use otty_ui_kit::components::tag_search::{Suggestion, TagSearch};

use super::action_model::{PruneViewModel, can_create, prune_button};
use super::actions::container_actions;
use super::no_engine::{NoEngineViewModel, no_engine};
use super::resource_rows::{
    columns, container_cells, container_status, image_cells, image_status,
    network_cells, network_status, volume_cells, volume_status,
};
use super::sort::TableSort;
use super::state::{DockerState, EngineState, ListLoad, ResourceList, Search};
use super::types::{
    Place, ResourceKind, image_name, shown_containers, shown_images,
    shown_networks, shown_volumes,
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
    /// A short text, for example "2 running".
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
    /// The tag search of this table: the field's text and the tags.
    pub(crate) search: &'a TagSearch,
    /// What the open suggestion list shows; empty while it is closed.
    pub(crate) suggestions: Vec<Suggestion>,
    /// The column the table is sorted by and the way.
    pub(crate) sort: TableSort,
    pub(crate) columns: Vec<TableColumn>,
    /// Rows that pass the search, in the order of `sort`.
    pub(crate) rows: Vec<Vec<TableCell>>,
    /// Positions among `rows` of the items without actions (a container
    /// being removed); these rows have no "⋯" button.
    pub(crate) rows_without_actions: Vec<usize>,
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
    search: &'a TagSearch,
    suggestions: Vec<Suggestion>,
    sort: TableSort,
}

/// The rows of the engine tree, top to bottom: each engine, and the four
/// lists of the engine whose place is open.
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
            rows.push(TreeRowViewModel {
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

/// What the main area shows; ages are counted up to `now` (Unix seconds).
pub(crate) fn main_area(state: &DockerState, now: i64) -> MainViewModel<'_> {
    let (Some(place), Some(engine)) = (state.place(), state.open_engine())
    else {
        return no_engine_view(state)
            .map_or(MainViewModel::Searching, MainViewModel::NoEngine);
    };

    let search = state.table_search(place.resource());
    let sort = state.sort(place.resource());
    let clock = state.age_clock();
    let shown = Shown {
        resource: place.resource(),
        search,
        suggestions: suggestions(state, search),
        sort,
    };

    let table = match place.resource() {
        ResourceKind::Containers => {
            let items = engine.containers().items();
            table(
                shown,
                engine.containers(),
                shown_containers(items, search, clock, sort),
                Container::id,
                |container| !container_actions(container.state()).is_empty(),
                |container| container_cells(container, now),
            )
        },
        ResourceKind::Images => {
            let items = engine.images().items();
            table(
                shown,
                engine.images(),
                shown_images(items, search, clock, sort),
                Image::id,
                |_| true,
                |image| image_cells(image, now),
            )
        },
        ResourceKind::Volumes => {
            let items = engine.volumes().items();
            table(
                shown,
                engine.volumes(),
                shown_volumes(items, search, clock, sort),
                Volume::name,
                |_| true,
                |volume| volume_cells(volume, now),
            )
        },
        ResourceKind::Networks => {
            let items = engine.networks().items();
            table(
                shown,
                engine.networks(),
                shown_networks(items, search, clock, sort),
                Network::id,
                |_| true,
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

/// The engine name and version for the tree root, e.g.
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

/// What the suggestion list of the open table shows. The rows are read only
/// while the list is open.
fn suggestions(state: &DockerState, search: &TagSearch) -> Vec<Suggestion> {
    if !search.is_open() {
        return Vec::new();
    }

    state
        .open_table_schema()
        .map_or_else(Vec::new, |schema| search.suggestions(&schema))
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

/// The table of one list: the rows of `items` (the items that pass the
/// search), the position of the selected row among them, the rows whose
/// item has no actions, and the message or error shown instead of rows.
/// `id_of` gives the selection id; `has_actions` tells whether an item has
/// a row menu.
fn table<'a, T>(
    shown: Shown<'a>,
    list: &'a ResourceList<T>,
    items: Vec<&'a T>,
    id_of: fn(&T) -> &str,
    has_actions: fn(&T) -> bool,
    cells: impl Fn(&T) -> Vec<TableCell>,
) -> TableViewModel<'a> {
    let selected = list
        .selected_id()
        .and_then(|id| items.iter().position(|item| id_of(item) == id));
    let rows_without_actions = items
        .iter()
        .enumerate()
        .filter(|(_, item)| !has_actions(item))
        .map(|(position, _)| position)
        .collect();
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
            (None, Some(i18n::t(Key::DockerNothingMatches).to_string()))
        },
        ListLoad::Loaded(_) => (None, None),
    };

    TableViewModel {
        resource: shown.resource,
        search: shown.search,
        suggestions: shown.suggestions,
        sort: shown.sort,
        columns: columns(shown.resource),
        rows,
        rows_without_actions,
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
    use otty_ui_kit::components::tag_search::TagSearchAction;

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
                        TreeNote::Nothing,
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
                        TreeNote::Nothing,
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
    fn the_tree_has_no_compose_group_even_with_compose_containers() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));

        let vm = build(&state);

        let labels: Vec<&str> =
            vm.tree.iter().map(|row| row.label.as_str()).collect();
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
        assert!(vm.tree.iter().all(|row| row.depth < 2));
    }

    #[test]
    fn a_loading_engine_shows_only_its_lists() {
        let state = found(&[EngineKind::Docker]);

        let vm = build(&state);

        assert_eq!(vm.tree.len(), 5);
    }

    #[test]
    fn the_containers_table_has_a_compose_column_after_image_with_links() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));

        let vm = build(&state);
        let table = table(&vm);

        assert_eq!(
            table.columns[3],
            TableColumn::new(i18n::t(Key::DockerFieldCompose), 2)
                .min_width(80)
                .sortable(true)
        );
        let projects: Vec<&TableCell> =
            table.rows.iter().map(|cells| &cells[3]).collect();
        assert!(projects.contains(&&TableCell::link("otty-fx")));
        assert!(projects.contains(&&TableCell::muted("")));
    }

    fn search_does(state: &mut DockerState, action: TagSearchAction) {
        let _ = reduce(state, DockerIntent::TagSearch(action));
    }

    fn type_text(state: &mut DockerState, text: &str) {
        search_does(state, TagSearchAction::InputChanged(text.to_string()));
    }

    /// Adds the typed `key:text` as it is, as Enter does with no suggestion.
    fn add_tag(state: &mut DockerState, text: &str) {
        type_text(state, text);
        search_does(state, TagSearchAction::Close);
        search_does(state, TagSearchAction::Apply);
    }

    /// The texts of the suggestions the table shows.
    fn suggested(state: &DockerState) -> Vec<String> {
        let vm = build(state);

        table(&vm)
            .suggestions
            .iter()
            .map(|suggestion| suggestion.text().to_string())
            .collect()
    }

    #[test]
    fn a_compose_tag_keeps_the_containers_row_selected_and_limits_the_rows() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        add_tag(&mut state, "compose:otty-fx");

        let vm = build(&state);
        let table = table(&vm);

        let selected: Vec<&str> = vm
            .tree
            .iter()
            .filter(|row| row.is_selected)
            .map(|row| row.label.as_str())
            .collect();
        assert_eq!(selected, vec!["Containers"]);
        assert_eq!(table.resource, ResourceKind::Containers);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.search.tags().len(), 1);
        assert_eq!(table.empty_message, None);
    }

    #[test]
    fn a_tag_works_with_the_name_text() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        add_tag(&mut state, "compose:otty-fx");
        type_text(&mut state, "exit");

        let vm = build(&state);

        assert!(table(&vm).rows.is_empty());
        assert!(table(&vm).empty_message.is_some());
    }

    #[test]
    fn clear_all_shows_every_row_again() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        add_tag(&mut state, "compose:otty-fx");

        search_does(&mut state, TagSearchAction::ClearTags);

        let vm = build(&state);
        assert!(table(&vm).search.tags().is_empty());
        assert!(table(&vm).rows.len() > 1);
    }

    #[test]
    fn a_closed_search_has_no_suggestions() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));

        assert!(suggested(&state).is_empty());
    }

    #[test]
    fn a_focused_empty_search_suggests_the_keys_of_each_table() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));

        for resource in ResourceKind::ALL {
            open(&mut state, resource);
            search_does(&mut state, TagSearchAction::Focused);

            assert_eq!(
                suggested(&state),
                crate::widgets::docker::table_search::keys(resource),
                "{resource:?}"
            );
        }
    }

    #[test]
    fn values_of_a_key_come_from_each_table() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        // (list, typed text, expected first value)
        let cases = [
            (ResourceKind::Images, "repository:busy", "busybox:1.37"),
            (ResourceKind::Volumes, "name:otty-fx", "otty-fx-data"),
            (ResourceKind::Networks, "name:bri", "bridge"),
            (ResourceKind::Containers, "ports:18080", "18080:80/tcp"),
        ];

        for (resource, text, expected) in cases {
            open(&mut state, resource);
            type_text(&mut state, text);

            // The typed text is the first row; the values follow it.
            assert_eq!(
                suggested(&state).get(1).map(String::as_str),
                Some(expected),
                "{text}"
            );
        }
    }

    #[test]
    fn values_of_a_key_come_from_the_rows_the_other_tags_leave() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        add_tag(&mut state, "compose:otty-fx");

        type_text(&mut state, "name:");

        assert_eq!(suggested(&state), vec!["otty-fx-web-1"]);
    }

    #[test]
    fn a_tag_filters_each_table_by_its_own_values() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        open(&mut state, ResourceKind::Networks);

        add_tag(&mut state, "driver:null");

        let vm = build(&state);
        assert_eq!(table(&vm).rows.len(), 1);
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

        assert_eq!(vm.tree[3].note, TreeNote::Failed);
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
        assert_eq!(
            table.columns,
            resource_rows::columns(ResourceKind::Containers)
        );
        assert_eq!(table.rows.len(), 6);
        let selected = table.selected.expect("a selected row");
        assert_eq!(
            table.rows[selected],
            resource_rows::container_cells(&resources.containers()[0], NOW)
        );
        assert_eq!(table.empty_message, None);
        assert_eq!(table.error, None);
    }

    #[test]
    fn the_containers_are_shown_running_first_by_default() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));

        let vm = build(&state);
        let table = table(&vm);

        assert_eq!(table.sort, state.sort(ResourceKind::Containers));
        assert_eq!(table.sort.column(), 1);
        let running = i18n::t(Key::DockerStateRunning);
        let first_status = &table.rows[0][1];
        assert_eq!(first_status, &TableCell::badge(running, StatusTone::Ok));
    }

    #[test]
    fn a_sort_press_reorders_the_rows_and_keeps_the_selected_item() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let _ = reduce(&mut state, DockerIntent::SelectRow(2));
        let selected_name = |vm: &Vm<'_>| {
            let table = table(vm);
            table.rows[table.selected.expect("a selected row")][0].clone()
        };
        let before = selected_name(&build(&state));

        let _ = reduce(&mut state, DockerIntent::SortBy(0));
        let vm = build(&state);
        let table = table(&vm);

        assert_eq!(table.sort.column(), 0);
        let names: Vec<&TableCell> =
            table.rows.iter().map(|cells| &cells[0]).collect();
        let mut sorted = names.clone();
        sorted.sort_by_key(|cell| format!("{cell:?}"));
        assert_eq!(names, sorted);
        assert_eq!(selected_name(&vm), before);
    }

    #[test]
    fn a_row_press_after_a_sort_selects_the_row_that_is_shown() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let _ = reduce(&mut state, DockerIntent::SortBy(0));
        let vm = build(&state);
        let expected = table(&vm).rows[3][0].clone();

        let _ = reduce(&mut state, DockerIntent::SelectRow(3));

        let vm = build(&state);
        let table = table(&vm);
        assert_eq!(table.selected, Some(3));
        assert_eq!(table.rows[3][0], expected);
    }

    #[test]
    fn sorting_applies_inside_the_search() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        type_text(&mut state, "exit");
        let _ = reduce(&mut state, DockerIntent::SortBy(0));
        let _ = reduce(&mut state, DockerIntent::SortBy(0));

        let vm = build(&state);
        let table = table(&vm);

        assert!(table.rows.len() > 1);
        let names: Vec<String> = table
            .rows
            .iter()
            .map(|cells| format!("{:?}", cells[0]))
            .collect();
        let mut reversed = names.clone();
        reversed.sort();
        reversed.reverse();
        assert_eq!(names, reversed);
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
        let position = state.shown_position(3);
        let _ = reduce(&mut state, DockerIntent::SelectRow(position));
        type_text(&mut state, "exit");

        let vm = build(&state);
        let table = table(&vm);

        assert_eq!(table.search.input(), "exit");
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0][0], TableCell::mono("otty-fx-exit0"));
        assert_eq!(table.selected, Some(0));
    }

    #[test]
    fn a_selected_row_outside_the_filter_is_not_highlighted() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        type_text(&mut state, "paused");

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
    fn a_search_without_matches_says_nothing_matches() {
        let mut tagged = found(&[EngineKind::Docker]);
        load(&mut tagged, &docker(), lists(EngineKind::Docker));
        add_tag(&mut tagged, "status:zzz");
        let mut typed = found(&[EngineKind::Docker]);
        load(&mut typed, &docker(), lists(EngineKind::Docker));
        type_text(&mut typed, "zzz");
        let expected = Some(i18n::t(Key::DockerNothingMatches).to_string());

        for state in [&tagged, &typed] {
            let vm = build(state);

            assert!(table(&vm).rows.is_empty());
            assert_eq!(table(&vm).empty_message, expected);
        }
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
