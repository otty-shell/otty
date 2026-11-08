use std::collections::HashSet;

use otty_containers::{Container, ContainerState, Image, Network, Volume};
use otty_ui_kit::components::data_table::{TableCell, TableColumn};
use otty_ui_kit::components::status_badge::StatusTone;
use otty_ui_kit::components::tag_search::{Suggestion, TagKey, TagSearch};

use super::actions::{PruneViewModel, container_actions};
use super::format::{engine_label, size_text};
use super::no_engine::{NoEngineViewModel, no_engine};
use super::resource_rows::{
    columns, container_cells, container_status, image_cells, image_status,
    network_cells, network_status, volume_cells, volume_status,
};
use super::sort::TableSort;
use super::state::{
    ContainersState, EngineState, ListLoad, ResourceList, Search,
};
use super::table_search::keys;
use super::types::{
    Place, ResourceKind, image_name, shown_containers, shown_images,
    shown_networks, shown_volumes,
};
use crate::i18n::{self, Key};

/// Separates the parts of the status bar text next to the engine, e.g.
/// "6 containers, 3 running".
const PART_SEPARATOR: &str = ", ";

/// The engine tree of the left panel: its rows and the short text under them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EngineTreeViewModel {
    /// Each engine and the four lists of each engine that is not closed.
    pub(crate) rows: Vec<TreeRowViewModel>,
    /// The short text under the rows while no engine is usable, such as why
    /// none is shown; `None` while the search runs or an engine answered.
    pub(crate) note: Option<&'static str>,
}

impl EngineTreeViewModel {
    /// Build the rows of the engine tree and the note under them.
    pub(super) fn from_state(state: &ContainersState) -> Self {
        Self {
            rows: tree_rows(state),
            note: no_engine_view(state).map(|card| card.panel_text),
        }
    }
}

/// One row of the engine tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TreeRowViewModel {
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

/// What the main area of Containers mode shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MainViewModel<'a> {
    /// The engine search runs and no engine is known yet.
    Searching,
    /// The search found no engine; the card says why and what to do.
    NoEngine(NoEngineViewModel),
    /// The open list of an engine.
    Table(TableViewModel<'a>),
}

impl<'a> MainViewModel<'a> {
    /// Build what the main area shows; ages are counted up to `now` (Unix
    /// seconds).
    pub(super) fn from_state(state: &'a ContainersState, now: i64) -> Self {
        let (Some(place), Some(engine)) = (state.place(), state.open_engine())
        else {
            return no_engine_view(state)
                .map_or(MainViewModel::Searching, MainViewModel::NoEngine);
        };

        let search = state.table_search(place);
        let sort = state.sort(place);
        let clock = state.age_clock();
        let shown = Shown {
            page: place,
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
                    |container| {
                        !container_actions(container.state()).is_empty()
                    },
                    |container| container_cells(container, now),
                )
            },
            ResourceKind::Images => {
                let items = engine.images().items();
                table(
                    shown,
                    engine.images(),
                    shown_images(items, search, clock, sort),
                    Image::row_key,
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
            prune: PruneViewModel::from_state(
                state.actions(),
                &state.actions_ctx(),
            ),
            notice: state.notice(),
            can_create: can_create(state),
            ..table
        })
    }
}

/// The header row and the table of the open list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TableViewModel<'a> {
    /// The page this table belongs to; what the table sends carries it.
    pub(crate) page: &'a Place,
    pub(crate) resource: ResourceKind,
    /// The tag search of this page: the field's text and the tags.
    pub(crate) search: &'a TagSearch,
    /// The keys of this table in the interface language; the tags read their
    /// shown names here.
    pub(crate) keys: Vec<TagKey>,
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

impl InspectorViewModel {
    /// Build the inspector header of the selected row of the open list;
    /// `None` when nothing is selected.
    pub(super) fn from_state(state: &ContainersState) -> Option<Self> {
        let place = state.place()?;
        let engine = state.open_engine()?;

        let (title, (status, tone)) = match place.resource() {
            ResourceKind::Containers => {
                let container = engine.containers().selected(Container::id)?;
                (container.name(), container_status(container))
            },
            ResourceKind::Images => {
                let image = engine.images().selected(Image::row_key)?;
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

        Some(Self {
            title: title.to_string(),
            status,
            tone,
        })
    }
}

/// The texts of the Containers mode status bar: the engine on screen and the
/// counts of the open list, or why there is no engine (spec Decision 5). The
/// sessions button is added by the view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusViewModel {
    /// The engine on screen, e.g. "Docker 29.9.1": the one the Run form will
    /// run on while it is open, else the one of the open list; `None` when no
    /// engine is shown (the bar names the mode instead).
    pub(crate) engine: Option<String>,
    /// The counts of the open list, or the reason there is no engine.
    pub(crate) info: String,
}

impl StatusViewModel {
    /// Build the status bar texts for the current state.
    pub(super) fn from_state(state: &ContainersState) -> Self {
        let (Some(place), Some(open)) = (state.place(), state.open_engine())
        else {
            let info = match no_engine_view(state) {
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

/// What every table of the main area needs besides its list.
struct Shown<'a> {
    page: &'a Place,
    resource: ResourceKind,
    search: &'a TagSearch,
    suggestions: Vec<Suggestion>,
    sort: TableSort,
}

/// The rows of the engine tree, top to bottom: each engine, and the four
/// lists of each engine that is not closed. An engine opens and closes on its
/// own, whichever list the main area shows; while the engine of the shown
/// list is closed, no row is highlighted.
fn tree_rows(state: &ContainersState) -> Vec<TreeRowViewModel> {
    let open = state.place();
    let mut rows = Vec::new();

    for engine in state.engines() {
        let address = engine.info().address();
        let is_open = !state.is_engine_closed(address);

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

/// The no-engine card of a finished search; `None` while it runs.
fn no_engine_view(state: &ContainersState) -> Option<NoEngineViewModel> {
    match state.search() {
        Search::Done(discovery) => no_engine(discovery),
        _ => None,
    }
}

/// Whether the create button above the table works: Run container, Pull
/// image, Create volume and Create network need an open engine.
fn can_create(state: &ContainersState) -> bool {
    state.open_engine().is_some() && state.place().is_some()
}

/// What the suggestion list of the open table shows. The rows are read only
/// while the list is open.
fn suggestions(state: &ContainersState, search: &TagSearch) -> Vec<Suggestion> {
    if !search.is_open() {
        return Vec::new();
    }

    state
        .open_table_schema()
        .map_or_else(Vec::new, |schema| search.suggestions(&schema))
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
            (None, Some(i18n::t(Key::ContainersLoading).to_string()))
        },
        ListLoad::Loaded(all) if all.is_empty() => {
            (None, Some(i18n::t(shown.resource.empty_key()).to_string()))
        },
        ListLoad::Loaded(_) if rows.is_empty() => (
            None,
            Some(i18n::t(Key::ContainersNothingMatches).to_string()),
        ),
        ListLoad::Loaded(_) => (None, None),
    };

    TableViewModel {
        page: shown.page,
        resource: shown.resource,
        search: shown.search,
        keys: keys(shown.resource),
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
    use otty_containers::{
        CandidateSource, Container, Discovery, EngineAddress, EngineInfo,
        EngineKind, fixtures,
    };
    use otty_ui_kit::components::data_table::CellPart;
    use otty_ui_kit::components::tag_search::TagSearchAction;

    use super::*;
    use crate::i18n::{self, Key};
    use crate::testing::task_outputs::collect_outputs;
    use crate::theme::{ThemeManager, ThemeProps};
    use crate::widgets::containers::event::{
        ContainersEffect, ContainersEvent, ContainersIntent,
    };
    use crate::widgets::containers::format::NO_VALUE;
    use crate::widgets::containers::no_engine::NoEngineCase;
    use crate::widgets::containers::reducer::reduce;
    use crate::widgets::containers::resource_rows::{self, PORTS_COLUMN};
    use crate::widgets::containers::run::event::RunIntent;
    use crate::widgets::containers::run::types::RunEdit;
    use crate::widgets::containers::run::{
        ImagePanelViewModel, RunFormViewModel,
    };
    use crate::widgets::containers::types::{ContainersError, LoadedLists};
    use crate::widgets::containers::view::status_bar::view as status_bar_view;

    const NOW: i64 = 1_793_557_960;

    /// All three parts of Containers mode at [`NOW`].
    struct Vm<'a> {
        tree: EngineTreeViewModel,
        main: MainViewModel<'a>,
        inspector: Option<InspectorViewModel>,
    }

    fn build(state: &ContainersState) -> Vm<'_> {
        Vm {
            tree: EngineTreeViewModel::from_state(state),
            main: MainViewModel::from_state(state, NOW),
            inspector: InspectorViewModel::from_state(state),
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

    fn found(kinds: &[EngineKind]) -> ContainersState {
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(kinds)),
        );
        state
    }

    fn load(
        state: &mut ContainersState,
        info: &EngineInfo,
        lists: LoadedLists,
    ) {
        let _ = reduce(
            state,
            ContainersIntent::ListsLoaded {
                engine: info.address().clone(),
                lists,
            },
        );
    }

    fn open(state: &mut ContainersState, resource: ResourceKind) {
        let place = Place::new(docker().address().clone(), resource);
        let _ = reduce(state, ContainersIntent::Open(place));
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

    // From losing the only engine to the card that says what to do.

    const DOCKER_PATH: &str = "/usr/bin/docker";
    const PODMAN_PATH: &str = "/usr/bin/podman";

    /// What a search says after the only engine stopped, and the card it must
    /// show: the case, the commands, and how many install buttons.
    fn lost_engine_cases()
    -> [(Discovery, NoEngineCase, &'static [&'static str], usize); 6] {
        [
            (
                fixtures::discovery_with_cli(&[], Some(DOCKER_PATH), None),
                NoEngineCase::DockerNotRunning,
                &["sudo systemctl start docker"],
                0,
            ),
            (
                fixtures::discovery_with_cli(&[], None, Some(PODMAN_PATH)),
                NoEngineCase::PodmanApiOff,
                &["systemctl --user enable --now podman.socket"],
                0,
            ),
            (
                fixtures::discovery_with_cli(
                    &[],
                    Some(DOCKER_PATH),
                    Some(PODMAN_PATH),
                ),
                NoEngineCase::DockerAndPodmanOff,
                &[
                    "sudo systemctl start docker",
                    "systemctl --user enable --now podman.socket",
                ],
                0,
            ),
            (
                fixtures::denied_discovery_with_cli(
                    fixtures::DOCKER_SOCKET,
                    CandidateSource::Docker,
                    None,
                    Some(PODMAN_PATH),
                ),
                NoEngineCase::NoPermissionAndOtherOff(EngineKind::Docker),
                &[
                    "sudo usermod -aG docker $USER",
                    "systemctl --user enable --now podman.socket",
                ],
                0,
            ),
            (
                fixtures::denied_discovery_with_cli(
                    fixtures::PODMAN_SOCKET,
                    CandidateSource::Podman,
                    Some(DOCKER_PATH),
                    None,
                ),
                NoEngineCase::NoPermissionAndOtherOff(EngineKind::Podman),
                &[
                    "systemctl --user enable --now podman.socket",
                    "sudo systemctl start docker",
                ],
                0,
            ),
            (fixtures::discovery(&[]), NoEngineCase::NotInstalled, &[], 2),
        ]
    }

    /// The engine of `kind` is the only one and its lists are loaded.
    fn only_engine(kind: EngineKind) -> ContainersState {
        let mut state = found(&[kind]);
        load(&mut state, &fixtures::engine(kind), lists(kind));

        state
    }

    /// The only engine stops answering and the search that follows says
    /// `found`, as the app does it.
    fn lose_engine(
        state: &mut ContainersState,
        kind: EngineKind,
        found: Discovery,
    ) {
        let info = fixtures::engine(kind);
        let sent = collect_outputs(reduce(
            state,
            ContainersIntent::EngineUnreachable(info.address().clone()),
        ));
        let asks_for_a_search = sent.iter().any(|event| {
            matches!(event, ContainersEvent::Effect(ContainersEffect::Discover))
        });
        assert!(asks_for_a_search, "a lost engine starts a search");

        let _ = reduce(state, ContainersIntent::DiscoveryFinished(found));
    }

    /// Checks that everything says what `case` says and nothing of the engine
    /// is left.
    fn assert_only_the_card(
        state: &ContainersState,
        case: NoEngineCase,
        commands: &[&str],
        installs: usize,
    ) {
        let vm = build(state);
        let MainViewModel::NoEngine(card) = &vm.main else {
            panic!("expected the no engine card, got {:?}", vm.main);
        };

        assert_eq!(card.case, case);
        let shown: Vec<&str> =
            card.commands.iter().map(|command| command.text).collect();
        assert_eq!(shown, commands);
        assert_eq!(card.installs.len(), installs);
        assert!(vm.tree.rows.is_empty(), "stale rows: {:?}", vm.tree.rows);
        assert_eq!(vm.inspector, None);
        assert_eq!(vm.tree.note, Some(card.panel_text));
        assert_eq!(state.place(), None);

        let bar = StatusViewModel::from_state(state);
        assert_eq!(bar.engine, None);
        assert_eq!(bar.info, card.status_text);
    }

    #[test]
    fn losing_the_only_engine_shows_the_card_of_the_search() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            for (found, case, commands, installs) in lost_engine_cases() {
                let mut state = only_engine(kind);

                lose_engine(&mut state, kind, found);

                assert_only_the_card(&state, case, commands, installs);
            }
        }
    }

    #[test]
    fn losing_the_only_engine_while_it_is_closed_shows_the_same_card() {
        for (found, case, commands, installs) in lost_engine_cases() {
            let mut state = only_engine(EngineKind::Docker);
            press_root(&mut state, &docker());

            lose_engine(&mut state, EngineKind::Docker, found);

            assert_only_the_card(&state, case, commands, installs);
            assert!(state.is_engine_closed(docker().address()));
        }
    }

    #[test]
    fn the_table_stays_until_the_search_after_a_lost_engine_finishes() {
        let mut state = only_engine(EngineKind::Docker);

        let _ = reduce(
            &mut state,
            ContainersIntent::EngineUnreachable(docker().address().clone()),
        );

        assert!(matches!(build(&state).main, MainViewModel::Table(_)));
        assert!(!EngineTreeViewModel::from_state(&state).rows.is_empty());
    }

    #[test]
    fn losing_one_of_two_engines_shows_no_card_and_no_row_of_the_lost_one() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        load(&mut state, &podman(), lists(EngineKind::Podman));

        lose_engine(
            &mut state,
            EngineKind::Docker,
            fixtures::discovery(&[EngineKind::Podman]),
        );

        let vm = build(&state);
        assert!(matches!(vm.main, MainViewModel::Table(_)));
        assert_eq!(labels(&state), open_labels("Podman 5.8.8"));
        assert_eq!(EngineTreeViewModel::from_state(&state).note, None);
    }

    #[test]
    fn the_card_replaces_an_open_run_form_and_the_form_waits_for_the_engine() {
        for (found, case, commands, installs) in lost_engine_cases() {
            let mut state = only_engine(EngineKind::Docker);
            let _ = reduce(&mut state, ContainersIntent::OpenCreate);
            let _ = reduce(
                &mut state,
                ContainersIntent::Run(RunIntent::Edited(RunEdit::Image(
                    String::from("nginx"),
                ))),
            );

            lose_engine(&mut state, EngineKind::Docker, found);

            let ctx = state.run_ctx();
            let form = RunFormViewModel::from_state(state.run(), &ctx);
            let panel = ImagePanelViewModel::from_state(state.run(), &ctx);
            assert_eq!(form.map(|_| ()), None, "{case:?}");
            assert_eq!(panel.map(|_| ()), None, "{case:?}");
            assert_only_the_card(&state, case, commands, installs);

            let _ = reduce(
                &mut state,
                ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                    EngineKind::Docker,
                ])),
            );

            let form = state.run().form().expect("the form was kept");
            assert_eq!(form.reference(), "nginx");
            let vm =
                RunFormViewModel::from_state(state.run(), &state.run_ctx());
            assert!(vm.is_some(), "{case:?}");
        }
    }

    #[test]
    fn a_closed_engine_that_comes_back_shows_its_table_under_a_closed_root() {
        let mut state = only_engine(EngineKind::Docker);
        press_root(&mut state, &docker());
        lose_engine(
            &mut state,
            EngineKind::Docker,
            fixtures::discovery_with_cli(&[], Some(DOCKER_PATH), None),
        );

        let _ = reduce(&mut state, ContainersIntent::CheckAgain);
        assert_eq!(build(&state).main, MainViewModel::Searching);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        load(&mut state, &docker(), lists(EngineKind::Docker));

        let vm = build(&state);
        assert_eq!(table(&vm).rows.len(), 6);
        assert_eq!(labels(&state), vec![String::from("Docker 29.9.1")]);
        assert!(!vm.tree.rows[0].is_open);
        assert!(highlighted(&state).is_empty());
        assert_eq!(EngineTreeViewModel::from_state(&state).note, None);
    }

    #[test]
    fn an_open_engine_that_comes_back_shows_its_table_under_an_open_root() {
        let mut state = only_engine(EngineKind::Docker);
        lose_engine(
            &mut state,
            EngineKind::Docker,
            fixtures::discovery_with_cli(&[], Some(DOCKER_PATH), None),
        );

        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );

        assert_eq!(labels(&state), open_labels("Docker 29.9.1"));
        assert_eq!(highlighted(&state).len(), 1);
    }

    // One search and sort per page.

    /// Docker and Podman with one container each, named after its engine.
    fn one_named_container_each() -> ContainersState {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        for (info, name) in
            [(docker(), "only-docker"), (podman(), "only-podman")]
        {
            let lists = LoadedLists {
                containers: Ok(vec![fixtures::container_of_image(
                    name,
                    "running",
                    &format!("{name}-image"),
                    "sha256:a",
                )]),
                ..lists(EngineKind::Docker)
            };
            load(&mut state, &info, lists);
        }

        state
    }

    fn podman_containers() -> Place {
        Place::new(podman().address().clone(), ResourceKind::Containers)
    }

    fn suggestion_texts(state: &ContainersState) -> Vec<String> {
        let MainViewModel::Table(table) = MainViewModel::from_state(state, NOW)
        else {
            panic!("expected a table");
        };

        table
            .suggestions
            .iter()
            .map(|found| found.text().to_string())
            .collect()
    }

    #[test]
    fn the_names_suggested_come_from_the_rows_of_the_open_page() {
        let mut state = one_named_container_each();

        search_does(
            &mut state,
            TagSearchAction::InputChanged(String::from("only")),
        );

        let found = suggestion_texts(&state);
        assert!(found.contains(&String::from("only-docker")), "{found:?}");
        assert!(!found.contains(&String::from("only-podman")), "{found:?}");
    }

    #[test]
    fn the_values_suggested_come_from_the_rows_of_the_open_page() {
        let mut state = one_named_container_each();
        let _ = reduce(&mut state, ContainersIntent::Open(podman_containers()));

        search_does(
            &mut state,
            TagSearchAction::InputChanged(String::from("Image:")),
        );

        assert_eq!(suggestion_texts(&state), ["only-podman-image"]);
    }

    #[test]
    fn the_suggestions_of_a_page_ignore_the_tags_of_another_page() {
        let mut state = one_named_container_each();
        search_does(
            &mut state,
            TagSearchAction::InputChanged(String::from("Name:only-docker")),
        );
        search_does(&mut state, TagSearchAction::Close);
        search_does(&mut state, TagSearchAction::Apply);
        let _ = reduce(&mut state, ContainersIntent::Open(podman_containers()));

        search_does(
            &mut state,
            TagSearchAction::InputChanged(String::from("only")),
        );

        let found = suggestion_texts(&state);
        assert!(found.contains(&String::from("only-podman")), "{found:?}");
    }

    #[test]
    fn the_no_match_message_belongs_to_the_page_that_filters() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        load(&mut state, &podman(), lists(EngineKind::Podman));
        search_does(
            &mut state,
            TagSearchAction::InputChanged(String::from("Name:no-such-name")),
        );
        search_does(&mut state, TagSearchAction::Close);
        search_does(&mut state, TagSearchAction::Apply);

        {
            let vm = build(&state);
            assert_eq!(
                table(&vm).empty_message.as_deref(),
                Some(i18n::t(Key::ContainersNothingMatches))
            );
        }

        let _ = reduce(&mut state, ContainersIntent::Open(podman_containers()));

        let vm = build(&state);
        assert_eq!(table(&vm).empty_message, None);
        assert_eq!(table(&vm).rows.len(), 6);
    }

    #[test]
    fn the_selected_row_is_found_among_the_rows_of_the_open_page() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        load(&mut state, &podman(), lists(EngineKind::Podman));
        search_does(
            &mut state,
            TagSearchAction::InputChanged(String::from("Name:no-such-name")),
        );
        search_does(&mut state, TagSearchAction::Close);
        search_does(&mut state, TagSearchAction::Apply);
        let _ = reduce(&mut state, ContainersIntent::Open(podman_containers()));

        let vm = build(&state);
        assert!(table(&vm).selected.is_some());
    }

    #[test]
    fn the_table_says_which_page_it_belongs_to() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        let _ = reduce(&mut state, ContainersIntent::Open(podman_containers()));

        let vm = build(&state);
        assert_eq!(table(&vm).page, &podman_containers());
    }

    #[test]
    fn a_search_that_found_nothing_shows_the_no_engine_card_and_panel_text() {
        let state = found(&[]);

        let vm = build(&state);

        let MainViewModel::NoEngine(card) = vm.main else {
            panic!("expected the no engine card");
        };
        assert_eq!(card.title, i18n::t(Key::ContainersNotInstalledTitle));
        assert_eq!(
            vm.tree.note,
            Some(i18n::t(Key::ContainersPanelNotInstalled))
        );
        assert!(vm.tree.rows.is_empty());
        assert_eq!(vm.inspector, None);
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
    fn before_any_engine_is_known_the_main_area_says_it_searches() {
        let state = ContainersState::default();

        let vm = build(&state);

        assert_eq!(vm.main, MainViewModel::Searching);
        assert!(vm.tree.rows.is_empty());
        assert_eq!(vm.inspector, None);
    }

    /// The labels of the tree rows, top to bottom.
    fn labels(state: &ContainersState) -> Vec<String> {
        EngineTreeViewModel::from_state(state)
            .rows
            .into_iter()
            .map(|row| row.label)
            .collect()
    }

    /// The labels of the highlighted tree rows with the engine of each.
    fn highlighted(state: &ContainersState) -> Vec<(String, EngineAddress)> {
        EngineTreeViewModel::from_state(state)
            .rows
            .into_iter()
            .filter(|row| row.is_selected)
            .map(|row| (row.label, row.place.engine().clone()))
            .collect()
    }

    fn press_root(state: &mut ContainersState, info: &EngineInfo) {
        let _ = reduce(
            state,
            ContainersIntent::ToggleEngine(info.address().clone()),
        );
    }

    /// The labels of an open engine: its root and its four lists.
    fn open_labels(root: &str) -> Vec<String> {
        [
            root,
            i18n::t(Key::ContainersContainers),
            i18n::t(Key::ContainersImages),
            i18n::t(Key::ContainersVolumes),
            i18n::t(Key::ContainersNetworks),
        ]
        .map(str::to_string)
        .to_vec()
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
    fn closing_the_shown_engine_keeps_its_table_and_highlights_no_row() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let before = build(&state);
        let rows_before = table(&before).rows.clone();
        let inspector_before = before.inspector.clone();

        press_root(&mut state, &docker());

        let vm = build(&state);
        assert_eq!(table(&vm).resource, ResourceKind::Containers);
        assert_eq!(table(&vm).rows, rows_before);
        assert_eq!(vm.inspector, inspector_before);
        assert_eq!(highlighted(&state), Vec::new());
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
                TreeRowViewModel {
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
    fn the_containers_table_has_a_compose_column_after_image_with_links() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));

        let vm = build(&state);
        let table = table(&vm);

        assert_eq!(
            table.columns[3],
            TableColumn::new(i18n::t(Key::ContainersFieldCompose), 2)
                .min_width(80)
                .sortable(true)
        );
        let projects: Vec<&TableCell> =
            table.rows.iter().map(|cells| &cells[3]).collect();
        assert!(projects.contains(&&TableCell::link("otty-fx")));
        assert!(projects.contains(&&TableCell::muted("")));
    }

    fn search_does(state: &mut ContainersState, action: TagSearchAction) {
        let _ = reduce(state, ContainersIntent::TagSearch(action));
    }

    fn type_text(state: &mut ContainersState, text: &str) {
        search_does(state, TagSearchAction::InputChanged(text.to_string()));
    }

    /// Adds the typed `key:text` as it is, as Enter does with no suggestion.
    fn add_tag(state: &mut ContainersState, text: &str) {
        type_text(state, text);
        search_does(state, TagSearchAction::Close);
        search_does(state, TagSearchAction::Apply);
    }

    /// The texts of the suggestions the table shows.
    fn suggested(state: &ContainersState) -> Vec<String> {
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
            .rows
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

            let shown: Vec<String> =
                crate::widgets::containers::table_search::keys(resource)
                    .iter()
                    .map(|key| key.shown().to_string())
                    .collect();
            assert_eq!(suggested(&state), shown, "{resource:?}");
        }
    }

    #[test]
    fn the_table_gives_the_keys_of_its_columns_to_the_tags_row() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));

        for resource in ResourceKind::ALL {
            open(&mut state, resource);

            let vm = build(&state);

            assert_eq!(
                table(&vm).keys,
                crate::widgets::containers::table_search::keys(resource),
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
            (ResourceKind::Images, "repository:busy", "busybox"),
            (ResourceKind::Images, "tag:1.3", "1.37"),
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
        partial.volumes = Err(ContainersError::new("no"));
        load(&mut state, &docker(), partial);
        let loading = found(&[EngineKind::Docker]);

        let vm = build(&state);
        let loading_vm = build(&loading);

        assert_eq!(vm.tree.rows[3].note, TreeNote::Failed);
        assert_eq!(loading_vm.tree.rows[1].note, TreeNote::Nothing);
        assert_eq!(loading_vm.tree.rows[2].note, TreeNote::Nothing);
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

        let page =
            Place::new(docker().address().clone(), ResourceKind::Containers);
        assert_eq!(table.sort, state.sort(&page));
        assert_eq!(table.sort.column(), 1);
        let running = i18n::t(Key::ContainersStateRunning);
        let first_status = &table.rows[0][1];
        assert_eq!(first_status, &TableCell::badge(running, StatusTone::Ok));
    }

    #[test]
    fn a_sort_press_reorders_the_rows_and_keeps_the_selected_item() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let _ = reduce(&mut state, ContainersIntent::SelectRow(2));
        let selected_name = |vm: &Vm<'_>| {
            let table = table(vm);
            table.rows[table.selected.expect("a selected row")][0].clone()
        };
        let before = selected_name(&build(&state));

        let _ = reduce(&mut state, ContainersIntent::SortBy(0));
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
        let _ = reduce(&mut state, ContainersIntent::SortBy(0));
        let vm = build(&state);
        let expected = table(&vm).rows[3][0].clone();

        let _ = reduce(&mut state, ContainersIntent::SelectRow(3));

        let vm = build(&state);
        let table = table(&vm);
        assert_eq!(table.selected, Some(3));
        assert_eq!(table.rows[3][0], expected);
    }

    #[test]
    fn the_cells_of_the_ports_column_follow_a_sort_by_it() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));

        let _ = reduce(&mut state, ContainersIntent::SortBy(PORTS_COLUMN));
        let vm = build(&state);
        let table = table(&vm);

        assert_eq!(table.sort.column(), PORTS_COLUMN);
        let ports: Vec<&TableCell> = table
            .rows
            .iter()
            .map(|cells| &cells[PORTS_COLUMN])
            .collect();
        let linked = |text: &str| TableCell::parts(vec![CellPart::link(text)]);
        let no_ports = TableCell::muted_mono(NO_VALUE);
        assert_eq!(
            ports,
            [
                &linked("18080:80/tcp"),
                &linked("18081:8080/tcp"),
                &no_ports,
                &no_ports,
                &no_ports,
                &no_ports,
            ]
        );
    }

    #[test]
    fn sorting_applies_inside_the_search() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        type_text(&mut state, "exit");
        let _ = reduce(&mut state, ContainersIntent::SortBy(0));
        let _ = reduce(&mut state, ContainersIntent::SortBy(0));

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
        let _ = reduce(&mut state, ContainersIntent::SelectRow(position));
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
        let expected = Some(i18n::t(Key::ContainersNothingMatches).to_string());

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
            Some(i18n::t(Key::ContainersLoading).to_string())
        );
        assert_eq!(
            table(&vm).empty_message,
            Some(i18n::t(Key::ContainersNoContainers).to_string())
        );
        assert_eq!(vm.inspector, None);
    }

    /// Docker with one image of two tags, its images open.
    fn two_tags_of_one_image() -> ContainersState {
        let mut state = found(&[EngineKind::Docker]);
        let mut loaded = lists(EngineKind::Docker);
        loaded.images = Ok(fixtures::image_rows(
            "sha256:app",
            &["app:1", "app:2"],
            &[],
            &[],
        ));
        load(&mut state, &docker(), loaded);
        open(&mut state, ResourceKind::Images);
        state
    }

    fn reload_images(state: &mut ContainersState, names: &[&str]) {
        let mut loaded = lists(EngineKind::Docker);
        loaded.images = Ok(fixtures::image_rows("sha256:app", names, &[], &[]));
        load(state, &docker(), loaded);
    }

    #[test]
    fn the_tree_counts_the_rows_of_the_images_table() {
        let state = two_tags_of_one_image();

        let vm = build(&state);

        assert_eq!(vm.tree.rows[2].note, TreeNote::Count(2));
        assert_eq!(table(&vm).rows.len(), 2);
    }

    #[test]
    fn only_the_pressed_tag_of_an_image_is_selected() {
        let mut state = two_tags_of_one_image();

        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        let vm = build(&state);
        assert_eq!(table(&vm).selected, Some(1));
        assert_eq!(table(&vm).rows[1][1], TableCell::muted_mono("2"));
    }

    #[test]
    fn the_inspector_names_the_selected_tag() {
        let mut state = two_tags_of_one_image();

        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        assert_eq!(
            build(&state).inspector.map(|inspector| inspector.title),
            Some(String::from("app:2"))
        );
    }

    #[test]
    fn a_sort_keeps_the_selected_tag_selected() {
        let mut state = two_tags_of_one_image();
        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        let _ = reduce(&mut state, ContainersIntent::SortBy(0));
        let vm = build(&state);

        assert_eq!(table(&vm).sort.column(), 0);
        let selected = table(&vm).selected.expect("a selected row");
        assert_eq!(table(&vm).rows[selected][1], TableCell::muted_mono("2"));
    }

    #[test]
    fn a_reload_keeps_the_same_tag_selected() {
        let mut state = two_tags_of_one_image();
        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        reload_images(&mut state, &["app:1", "app:2", "app:3"]);

        assert_eq!(
            build(&state).inspector.map(|inspector| inspector.title),
            Some(String::from("app:2"))
        );
    }

    #[test]
    fn a_reload_without_the_selected_tag_selects_as_for_any_removed_row() {
        let mut state = two_tags_of_one_image();
        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        reload_images(&mut state, &["app:1"]);

        assert_eq!(
            build(&state).inspector.map(|inspector| inspector.title),
            Some(String::from("app:1"))
        );
    }

    #[test]
    fn a_failed_list_shows_the_engine_message_and_no_rows() {
        let mut state = found(&[EngineKind::Docker]);
        let mut failed = lists(EngineKind::Docker);
        failed.containers = Err(ContainersError::new("engine said no"));
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

    #[test]
    fn every_list_has_a_create_button() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let mut enabled = Vec::new();

        for resource in ResourceKind::ALL {
            open(&mut state, resource);
            enabled.push(can_create(&state));
        }

        assert_eq!(enabled, vec![true, true, true, true]);
        assert!(!can_create(&ContainersState::default()));
    }

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
        let whole_list = StatusViewModel::from_state(&state).info;

        let _ = reduce(
            &mut state,
            ContainersIntent::TagSearch(TagSearchAction::InputChanged(
                String::from("no such name"),
            )),
        );

        assert_eq!(StatusViewModel::from_state(&state).info, whole_list);
    }

    #[test]
    fn while_searching_the_bar_says_it_looks_for_engines() {
        let state = ContainersState::default();

        let vm = StatusViewModel::from_state(&state);

        assert_eq!(vm.engine, None);
        assert_eq!(vm.info, "Looking for engines…");
    }

    #[test]
    fn while_the_run_form_is_open_the_bar_says_run_container() {
        let mut state = loaded(&[EngineKind::Docker]);
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);

        let vm = StatusViewModel::from_state(&state);

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

        let vm = StatusViewModel::from_state(&state);

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
            StatusViewModel::from_state(&state).engine.as_deref(),
            Some("Docker 29.9.1")
        );
    }

    #[test]
    fn the_bar_goes_back_to_the_open_list_when_the_run_form_closes() {
        let mut state = run_form_on_podman();

        let _ = reduce(&mut state, ContainersIntent::Run(RunIntent::Close));

        let vm = StatusViewModel::from_state(&state);
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

        let vm = StatusViewModel::from_state(&state);
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

        let vm = StatusViewModel::from_state(&state);
        assert!(state.run().is_open());
        assert_eq!(vm.engine, None);
        assert_eq!(vm.info, "no engine found");
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

        let vm = StatusViewModel::from_state(&state);

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

        let vm = StatusViewModel::from_state(&state);

        assert_eq!(vm.info, format!("2 images, {}", size_text(bytes)));
    }

    #[test]
    fn images_show_the_total_and_the_size() {
        let mut state = loaded(&[EngineKind::Docker]);
        open(&mut state, ResourceKind::Images);
        let resources = fixtures::resources(EngineKind::Docker);
        let count = resources.images().len();
        let bytes: u64 = resources.images().iter().map(Image::size).sum();

        let vm = StatusViewModel::from_state(&state);

        assert_eq!(vm.info, format!("{count} images, {}", size_text(bytes)));
    }

    #[test]
    fn volumes_and_networks_show_their_count() {
        let mut state = loaded(&[EngineKind::Docker]);
        let resources = fixtures::resources(EngineKind::Docker);

        open(&mut state, ResourceKind::Volumes);
        let volumes = StatusViewModel::from_state(&state).info;
        open(&mut state, ResourceKind::Networks);
        let networks = StatusViewModel::from_state(&state).info;

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

        let containers = StatusViewModel::from_state(&state).info;
        open(&mut state, ResourceKind::Images);
        let images = StatusViewModel::from_state(&state).info;
        open(&mut state, ResourceKind::Volumes);
        let volumes = StatusViewModel::from_state(&state).info;
        open(&mut state, ResourceKind::Networks);
        let networks = StatusViewModel::from_state(&state).info;

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

        let vm = StatusViewModel::from_state(&state);
        let mut ui = simulator(status_bar_view(
            &vm,
            0,
            "0.4.0",
            (),
            ThemeProps::new(themes.current()),
        ));

        ui.find("Docker 29.9.1").expect("the open engine");
        assert!(ui.find("Podman 5.8.8").is_err());
        assert!(!vm.info.contains("Podman"), "{}", vm.info);
    }

    #[test]
    fn a_list_that_loads_shows_no_number() {
        let state = found(&[EngineKind::Docker]);

        let vm = StatusViewModel::from_state(&state);

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

        let vm = StatusViewModel::from_state(&state);

        assert_eq!(vm.info, "failed");
    }

    #[test]
    fn without_an_engine_the_bar_gives_the_reason() {
        let state = found(&[]);

        let vm = StatusViewModel::from_state(&state);

        assert_eq!(vm.engine, None);
        assert_eq!(vm.info, "no engine found");
    }
}
