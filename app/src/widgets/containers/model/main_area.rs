use otty_containers::{Container, Image, Network, Volume};
use otty_ui_kit::components::data_table::{TableCell, TableColumn};
use otty_ui_kit::components::tag_search::{Suggestion, TagKey, TagSearch};

use super::super::child::actions::{PruneViewModel, container_actions};
use super::super::resource_rows::{
    columns, container_cells, image_cells, network_cells, volume_cells,
};
use super::super::sort::TableSort;
use super::super::state::{ContainersState, ListLoad, ResourceList};
use super::super::table_search::keys;
use super::super::types::{
    Place, ResourceKind, shown_containers, shown_images, shown_networks,
    shown_volumes,
};
use super::no_engine::NoEngineViewModel;
use crate::i18n::{self, Key};

/// What the main area of Containers mode shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MainAreaViewModel<'a> {
    /// The engine search runs and no engine is known yet.
    Searching,
    /// The search found no engine; the card says why and what to do.
    NoEngine(NoEngineViewModel),
    /// The open list of an engine.
    Table(TableViewModel<'a>),
}

impl<'a> MainAreaViewModel<'a> {
    /// Build what the main area shows; ages are counted up to `now` (Unix
    /// seconds).
    pub(in super::super) fn from_state(
        state: &'a ContainersState,
        now: i64,
    ) -> Self {
        let (Some(place), Some(engine)) = (state.place(), state.open_engine())
        else {
            return NoEngineViewModel::from_state(state).map_or(
                MainAreaViewModel::Searching,
                MainAreaViewModel::NoEngine,
            );
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
        MainAreaViewModel::Table(TableViewModel {
            prune: PruneViewModel::from_state(
                state.actions(),
                &state.actions_ctx(),
            ),
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
    /// Whether the create button opens a dialog.
    pub(crate) can_create: bool,
}

/// What every table of the main area needs besides its list.
struct Shown<'a> {
    page: &'a Place,
    resource: ResourceKind,
    search: &'a TagSearch,
    suggestions: Vec<Suggestion>,
    sort: TableSort,
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
        can_create: false,
    }
}

#[cfg(test)]
mod tests {
    use otty_containers::{EngineKind, fixtures};
    use otty_ui_kit::components::data_table::{
        CellPart, TableCell, TableColumn,
    };
    use otty_ui_kit::components::status_badge::StatusTone;
    use otty_ui_kit::components::tag_search::TagSearchAction;

    use super::super::test_support::{
        NOW, Vm, add_tag, build, docker, found, lists, load, open, podman,
        search_does, table, two_tags_of_one_image, type_text,
    };
    use super::*;
    use crate::i18n::{self, Key};
    use crate::widgets::containers::event::ContainersIntent;
    use crate::widgets::containers::format::NO_VALUE;
    use crate::widgets::containers::reducer::reduce;
    use crate::widgets::containers::resource_rows::{self, PORTS_COLUMN};
    use crate::widgets::containers::state::ContainersState;
    use crate::widgets::containers::types::{LoadedLists, Place, ResourceKind};

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
        let MainAreaViewModel::Table(table) =
            MainAreaViewModel::from_state(state, NOW)
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
    fn only_the_pressed_tag_of_an_image_is_selected() {
        let mut state = two_tags_of_one_image();

        let _ = reduce(&mut state, ContainersIntent::SelectRow(1));

        let vm = build(&state);
        assert_eq!(table(&vm).selected, Some(1));
        assert_eq!(table(&vm).rows[1][1], TableCell::muted_mono("2"));
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
}
