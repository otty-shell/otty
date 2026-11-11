//! What the tag search of each Containers table knows (spec Decision 21): the keys
//! in column order and the text of each row's cells, and the kept search of
//! each page. The rules of the search live in the kit; this module only
//! supplies keys and values.

use std::collections::HashMap;

use otty_app_i18n::{self as i18n, Locale};
use otty_containers::{Container, Image, Network, Volume};
use otty_ui_kit::components::tag_search::{
    SearchRow, TagKey, TagSchema, TagSearch,
};

use super::format::{age_text, port_text, short_id, shown_ports, size_text};
use super::resource_rows::{columns_in, container_badge};
use super::sort::{
    TableSort, sort_containers, sort_images, sort_networks, sort_volumes,
};
use super::types::{
    Place, ResourceKind, image_name, image_repository, image_tag,
};

/// The internal name of the Compose project key; the Compose link adds a tag
/// with it.
pub(crate) const COMPOSE_KEY: &str = "compose";

/// The internal key names of the containers table, in column order.
const CONTAINER_KEYS: &[&str] =
    &["name", "status", "image", COMPOSE_KEY, "ports", "age"];
/// The internal key of the images table's Tag column, not to be confused with
/// the tags of the search itself.
const IMAGE_TAG_KEY: &str = "image-tag";
/// The internal key names of the images table, in column order.
const IMAGE_KEYS: &[&str] =
    &["repository", IMAGE_TAG_KEY, "id", "size", "used-by", "age"];
/// The internal key names of the volumes table, in column order.
const VOLUME_KEYS: &[&str] = &["name", "driver", "used-by", "age"];
/// The internal key names of the networks table, in column order.
const NETWORK_KEYS: &[&str] =
    &["name", "driver", "scope", "subnet", "containers", "age"];

/// The kept search of each page (spec Decision 21). It lives as long as OTTY
/// runs: apart from the engines, so a page keeps its search while its engine
/// is away.
#[derive(Debug, Default)]
pub(crate) struct TableSearches {
    pages: HashMap<Place, TagSearch>,
    /// What a page shows until someone searches in it.
    untouched: TagSearch,
}

impl TableSearches {
    /// The search of the page `place`.
    pub(crate) fn get(&self, place: &Place) -> &TagSearch {
        self.pages.get(place).unwrap_or(&self.untouched)
    }

    /// The search of the page `place`, to change it.
    pub(crate) fn get_mut(&mut self, place: &Place) -> &mut TagSearch {
        self.pages.entry(place.clone()).or_default()
    }
}

/// The keys of the `resource` table in column order, shown in the interface
/// language.
pub(crate) fn keys(resource: ResourceKind) -> Vec<TagKey> {
    keys_in(i18n::current_locale(), resource)
}

/// The keys of the `resource` table in column order, shown in `locale`. A
/// shown name comes from the title of its column, so the two cannot differ.
pub(super) fn keys_in(locale: Locale, resource: ResourceKind) -> Vec<TagKey> {
    let names = match resource {
        ResourceKind::Containers => CONTAINER_KEYS,
        ResourceKind::Images => IMAGE_KEYS,
        ResourceKind::Volumes => VOLUME_KEYS,
        ResourceKind::Networks => NETWORK_KEYS,
    };

    names
        .iter()
        .zip(columns_in(locale, resource))
        .map(|(name, column)| TagKey::new(name, column.title()))
        .collect()
}

/// A container as the search sees it: the texts of its cells, each port its
/// own value. `now` counts its age (Unix seconds).
pub(crate) fn container_row(container: &Container, now: i64) -> SearchRow {
    let (status, _) = container_badge(container.state(), container.exit_code());

    let mut row = SearchRow::new(container.name())
        .value("name", container.name())
        .value("status", status)
        .value("image", container.image());
    if let Some(project) = container.compose_project() {
        row = row.value(COMPOSE_KEY, project);
    }
    for port in shown_ports(container.ports()) {
        row = row.value("ports", port_text(port));
    }

    with_age(row, now, container.created())
}

/// An image as the search sees it: the texts of its cells. Typed text finds
/// the whole reference, so `app:1` works as it does in `docker images`.
pub(crate) fn image_row(image: &Image, now: i64) -> SearchRow {
    let row = SearchRow::new(image_name(image))
        .value("repository", image_repository(image))
        .value(IMAGE_TAG_KEY, image_tag(image))
        .value("id", short_id(image.id()))
        .value("size", size_text(image.size()))
        .value("used-by", format!("{}", image.containers()));

    with_age(row, now, Some(image.created()))
}

/// A volume as the search sees it: the texts of its cells, each container
/// that uses it its own value.
pub(crate) fn volume_row(volume: &Volume, now: i64) -> SearchRow {
    let mut row = SearchRow::new(volume.name())
        .value("name", volume.name())
        .value("driver", volume.driver());
    for container in volume.containers() {
        row = row.value("used-by", container.as_str());
    }

    with_age(row, now, volume.created())
}

/// A network as the search sees it: the texts of its cells.
pub(crate) fn network_row(network: &Network, now: i64) -> SearchRow {
    let mut row = SearchRow::new(network.name())
        .value("name", network.name())
        .value("driver", network.driver())
        .value("scope", network.scope());
    if let Some(subnet) = network.subnet() {
        row = row.value("subnet", subnet);
    }
    row = row.value("containers", format!("{}", network.containers().len()));

    with_age(row, now, network.created())
}

/// The schema of the containers table: every row in the table's order,
/// whatever the search filters out.
pub(crate) fn containers_schema(
    containers: &[Container],
    now: i64,
    sort: TableSort,
) -> TagSchema {
    let mut items: Vec<&Container> = containers.iter().collect();
    sort_containers(&mut items, sort);

    let rows = items.iter().map(|item| container_row(item, now)).collect();
    TagSchema::new(keys(ResourceKind::Containers), rows)
}

/// The schema of the images table, like [`containers_schema`].
pub(crate) fn images_schema(
    images: &[Image],
    now: i64,
    sort: TableSort,
) -> TagSchema {
    let mut items: Vec<&Image> = images.iter().collect();
    sort_images(&mut items, sort);

    let rows = items.iter().map(|item| image_row(item, now)).collect();
    TagSchema::new(keys(ResourceKind::Images), rows)
}

/// The schema of the volumes table, like [`containers_schema`].
pub(crate) fn volumes_schema(
    volumes: &[Volume],
    now: i64,
    sort: TableSort,
) -> TagSchema {
    let mut items: Vec<&Volume> = volumes.iter().collect();
    sort_volumes(&mut items, sort);

    let rows = items.iter().map(|item| volume_row(item, now)).collect();
    TagSchema::new(keys(ResourceKind::Volumes), rows)
}

/// The schema of the networks table, like [`containers_schema`].
pub(crate) fn networks_schema(
    networks: &[Network],
    now: i64,
    sort: TableSort,
) -> TagSchema {
    let mut items: Vec<&Network> = networks.iter().collect();
    sort_networks(&mut items, sort);

    let rows = items.iter().map(|item| network_row(item, now)).collect();
    TagSchema::new(keys(ResourceKind::Networks), rows)
}

/// Adds the age of `created` to `row`; an unknown time shows no value.
fn with_age(row: SearchRow, now: i64, created: Option<i64>) -> SearchRow {
    if created.is_none() {
        return row;
    }

    row.value("age", age_text(now, created))
}

#[cfg(test)]
mod tests {
    use otty_app_i18n::{Key, Locale, text_in};
    use otty_containers::{EngineKind, Resources, fixtures};
    use otty_ui_kit::components::data_table::{CellPart, TableCell};
    use otty_ui_kit::components::tag_search::{TagKey, TagSearchAction};

    use super::*;
    use crate::format::NO_VALUE;
    use crate::resource_rows::{
        columns, container_cells, image_cells, network_cells, volume_cells,
    };

    const NOW: i64 = 1_793_557_960;

    fn docker() -> Resources {
        fixtures::resources(EngineKind::Docker)
    }

    /// The text a cell shows.
    fn shown_text(cell: &TableCell) -> String {
        match cell {
            TableCell::Parts(parts) => parts
                .iter()
                .map(|part| match part {
                    CellPart::Text(text) | CellPart::Link(text) => {
                        text.as_str()
                    },
                })
                .collect(),
            TableCell::Plain(text)
            | TableCell::Mono(text)
            | TableCell::MutedMono(text)
            | TableCell::Muted(text)
            | TableCell::Number(text)
            | TableCell::Badge(text, _)
            | TableCell::Link(text) => text.clone(),
        }
    }

    /// Checks that each key has the shown text of its column; `—` and blank
    /// cells have no value.
    fn assert_values_are_the_shown_text(
        resource: ResourceKind,
        row: &SearchRow,
        cells: &[TableCell],
    ) {
        let keys = keys(resource);
        assert_eq!(keys.len(), cells.len());

        for (key, cell) in keys.iter().zip(cells) {
            let name = key.name();
            let shown = shown_text(cell);
            let values: Vec<&str> = row.values_of(name).collect();
            if shown.is_empty() || shown == NO_VALUE {
                assert!(values.is_empty(), "{name}: {values:?}");
            } else {
                assert_eq!(values.join(", "), shown, "{name}");
            }
        }
    }

    fn shown_names(locale: Locale, resource: ResourceKind) -> Vec<String> {
        keys_in(locale, resource)
            .iter()
            .map(|key| key.shown().to_string())
            .collect()
    }

    #[test]
    fn each_table_keeps_its_internal_names_in_column_order_in_every_locale() {
        let names = |locale, resource| -> Vec<&'static str> {
            keys_in(locale, resource).iter().map(TagKey::name).collect()
        };

        for locale in Locale::ALL {
            assert_eq!(
                names(locale, ResourceKind::Containers),
                ["name", "status", "image", "compose", "ports", "age"]
            );
            assert_eq!(
                names(locale, ResourceKind::Images),
                ["repository", "image-tag", "id", "size", "used-by", "age"]
            );
            assert_eq!(
                names(locale, ResourceKind::Volumes),
                ["name", "driver", "used-by", "age"]
            );
            assert_eq!(
                names(locale, ResourceKind::Networks),
                ["name", "driver", "scope", "subnet", "containers", "age"]
            );
        }
    }

    #[test]
    fn each_table_has_one_key_per_column() {
        for resource in ResourceKind::ALL {
            assert_eq!(keys(resource).len(), columns(resource).len());
        }
    }

    #[test]
    fn in_every_locale_and_table_the_shown_names_can_be_typed() {
        for locale in Locale::ALL {
            for resource in ResourceKind::ALL {
                let tag = locale.tag();
                let shown = shown_names(locale, resource);

                for name in &shown {
                    assert!(
                        !name.trim().is_empty(),
                        "{tag} {resource:?}: blank"
                    );
                    assert!(
                        !name.contains([':', '：']),
                        "{tag} {resource:?}: {name} has a colon"
                    );
                }

                let mut unique: Vec<String> =
                    shown.iter().map(|name| name.to_uppercase()).collect();
                unique.sort_unstable();
                unique.dedup();
                assert_eq!(
                    unique.len(),
                    shown.len(),
                    "{tag} {resource:?} repeats a name: {shown:?}"
                );
            }
        }
    }

    #[test]
    fn the_shown_names_are_the_column_headers_up_to_the_colon() {
        let cases: [(Locale, ResourceKind, &[&str]); 7] = [
            (
                Locale::En,
                ResourceKind::Containers,
                &["Name", "Status", "Image", "Compose", "Ports", "Age"],
            ),
            (
                Locale::En,
                ResourceKind::Images,
                &["Repository", "Tag", "ID", "Size", "Used by", "Age"],
            ),
            (
                Locale::Ru,
                ResourceKind::Containers,
                &["Имя", "Состояние", "Образ", "Compose", "Порты", "Возраст"],
            ),
            (
                Locale::Ru,
                ResourceKind::Images,
                &[
                    "Репозиторий",
                    "Тег",
                    "ID",
                    "Размер",
                    "Используется",
                    "Возраст",
                ],
            ),
            (
                Locale::Ru,
                ResourceKind::Networks,
                &[
                    "Имя",
                    "Драйвер",
                    "Область",
                    "Подсеть",
                    "Контейнеры",
                    "Возраст",
                ],
            ),
            (
                Locale::De,
                ResourceKind::Volumes,
                &["Name", "Treiber", "Genutzt von", "Alter"],
            ),
            (
                Locale::ZhCn,
                ResourceKind::Images,
                &["仓库", "标签", "ID", "大小", "使用者", "时间"],
            ),
        ];

        for (locale, resource, expected) in cases {
            assert_eq!(
                shown_names(locale, resource),
                expected,
                "{} {resource:?}",
                locale.tag()
            );
        }
    }

    #[test]
    fn the_keys_follow_the_interface_language() {
        for resource in ResourceKind::ALL {
            assert_eq!(
                keys(resource),
                keys_in(otty_app_i18n::current_locale(), resource)
            );
        }
    }

    /// A Russian containers table with one running container, its status as
    /// the Russian cell shows it.
    fn russian_containers() -> TagSchema {
        TagSchema::new(
            keys_in(Locale::Ru, ResourceKind::Containers),
            vec![SearchRow::new("web-1").value(
                "status",
                text_in(Locale::Ru, Key::ContainersStateRunning),
            )],
        )
    }

    #[test]
    fn in_russian_tab_completes_the_header_then_the_text_of_the_cell() {
        let schema = russian_containers();
        let mut search = TagSearch::default();

        let _ = search.perform(
            TagSearchAction::InputChanged(String::from("Сост")),
            &schema,
        );
        let _ = search.perform(TagSearchAction::Complete, &schema);
        assert_eq!(search.input(), "Состояние:");

        let _ = search.perform(
            TagSearchAction::InputChanged(String::from("Состояние:раб")),
            &schema,
        );
        let _ = search.perform(TagSearchAction::Complete, &schema);

        assert_eq!(search.input(), "");
        assert_eq!(search.tags()[0].key(), "status");
        assert_eq!(search.tags()[0].label(schema.keys()), "Состояние:Работает");
    }

    #[test]
    fn in_russian_an_english_key_is_a_name_filter() {
        let schema = russian_containers();
        let mut search = TagSearch::default();

        let _ = search.perform(
            TagSearchAction::InputChanged(String::from("status:x")),
            &schema,
        );

        assert_eq!(search.name_text(schema.keys()), "status:x");
    }

    #[test]
    fn container_values_are_the_text_shown_in_the_cells() {
        for container in docker().containers() {
            assert_values_are_the_shown_text(
                ResourceKind::Containers,
                &container_row(container, NOW),
                &container_cells(container, NOW),
            );
        }
    }

    #[test]
    fn image_values_are_the_text_shown_in_the_cells() {
        for image in docker().images() {
            assert_values_are_the_shown_text(
                ResourceKind::Images,
                &image_row(image, NOW),
                &image_cells(image, NOW),
            );
        }
    }

    #[test]
    fn volume_values_are_the_text_shown_in_the_cells() {
        for volume in docker().volumes() {
            assert_values_are_the_shown_text(
                ResourceKind::Volumes,
                &volume_row(volume, NOW),
                &volume_cells(volume, NOW),
            );
        }
    }

    #[test]
    fn network_values_are_the_text_shown_in_the_cells() {
        for network in docker().networks() {
            assert_values_are_the_shown_text(
                ResourceKind::Networks,
                &network_row(network, NOW),
                &network_cells(network, NOW),
            );
        }
    }

    #[test]
    fn rows_are_named_like_the_name_filter_of_each_table() {
        let resources = docker();

        assert_eq!(
            container_row(&resources.containers()[0], NOW).name(),
            resources.containers()[0].name()
        );
        assert_eq!(
            image_row(&resources.images()[2], NOW).name(),
            "busybox:1.37"
        );
        assert_eq!(
            volume_row(&resources.volumes()[1], NOW).name(),
            "otty-fx-data"
        );
        assert_eq!(network_row(&resources.networks()[0], NOW).name(), "bridge");
    }

    #[test]
    fn an_image_row_is_named_by_its_reference_and_keeps_its_parts_apart() {
        let rows = fixtures::image_rows("sha256:app", &["reg/app:1"], &[], &[]);

        let row = image_row(&rows[0], NOW);

        assert_eq!(row.name(), "reg/app:1");
        assert_eq!(
            row.values_of("repository").collect::<Vec<_>>(),
            vec!["reg/app"]
        );
        assert_eq!(row.values_of("image-tag").collect::<Vec<_>>(), vec!["1"]);
    }

    #[test]
    fn each_port_of_a_container_is_its_own_value() {
        let container = fixtures::container_with_ports(&[
            (Some("0.0.0.0"), Some(8080), 80, "tcp"),
            (Some("::"), Some(8080), 80, "tcp"),
            (None, None, 443, "tcp"),
        ]);

        let row = container_row(&container, NOW);

        let ports: Vec<&str> = row.values_of("ports").collect();
        assert_eq!(ports, vec!["8080:80/tcp", "443/tcp"]);
    }

    #[test]
    fn each_container_of_a_volume_is_its_own_value() {
        let resources = docker();
        let used = &resources.volumes()[1];

        let row = volume_row(used, NOW);

        let users: Vec<&str> = row.values_of("used-by").collect();
        let expected: Vec<&str> =
            used.containers().iter().map(String::as_str).collect();
        assert_eq!(users, expected);
    }

    #[test]
    fn the_schema_lists_the_rows_in_the_order_of_the_table() {
        let resources = docker();
        let sort = TableSort::default_for(ResourceKind::Containers);

        let schema = containers_schema(resources.containers(), NOW, sort);
        let mut search = TagSearch::default();
        let _ = search.perform(
            TagSearchAction::InputChanged(String::from("otty-fx")),
            &schema,
        );

        let names: Vec<String> = search
            .suggestions(&schema)
            .iter()
            .map(|found| found.text().to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                "otty-fx", // the typed text comes first
                "otty-fx-running",
                "otty-fx-web-1",
                "otty-fx-paused",
                "otty-fx-created",
                "otty-fx-exit0",
                "otty-fx-exit3",
            ]
        );
    }
}
