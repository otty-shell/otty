//! What the tag search of each Docker table knows (spec Decision 21): the keys
//! in column order and the text of each row's cells. The rules of the search
//! live in the kit; this module only supplies keys and values.

use otty_docker::{Container, Image, Network, Volume};
use otty_ui_kit::components::tag_search::{SearchRow, TagSchema, TagSearch};

use super::format::{age_text, port_text, short_id, shown_ports, size_text};
use super::resource_rows::container_badge;
use super::sort::{
    TableSort, sort_containers, sort_images, sort_networks, sort_volumes,
};
use super::types::{ResourceKind, image_name};

/// The key of the Compose project; the Compose link adds a tag with it.
pub(crate) const COMPOSE_KEY: &str = "compose";

/// The keys of the containers table, in column order.
const CONTAINER_KEYS: &[&str] =
    &["name", "status", "image", COMPOSE_KEY, "ports", "age"];
/// The keys of the images table, in column order.
const IMAGE_KEYS: &[&str] = &["repository", "id", "size", "used-by", "age"];
/// The keys of the volumes table, in column order.
const VOLUME_KEYS: &[&str] = &["name", "driver", "used-by", "age"];
/// The keys of the networks table, in column order.
const NETWORK_KEYS: &[&str] =
    &["name", "driver", "scope", "subnet", "containers", "age"];

/// The kept search of each table: its tags live as long as OTTY runs.
#[derive(Debug, Default)]
pub(crate) struct TableSearches {
    containers: TagSearch,
    images: TagSearch,
    volumes: TagSearch,
    networks: TagSearch,
}

impl TableSearches {
    /// The search of the `resource` table.
    pub(crate) fn get(&self, resource: ResourceKind) -> &TagSearch {
        match resource {
            ResourceKind::Containers => &self.containers,
            ResourceKind::Images => &self.images,
            ResourceKind::Volumes => &self.volumes,
            ResourceKind::Networks => &self.networks,
        }
    }

    /// The search of the `resource` table, to change it.
    pub(crate) fn get_mut(&mut self, resource: ResourceKind) -> &mut TagSearch {
        match resource {
            ResourceKind::Containers => &mut self.containers,
            ResourceKind::Images => &mut self.images,
            ResourceKind::Volumes => &mut self.volumes,
            ResourceKind::Networks => &mut self.networks,
        }
    }
}

/// The keys of the `resource` table, in column order.
pub(crate) fn keys(resource: ResourceKind) -> &'static [&'static str] {
    match resource {
        ResourceKind::Containers => CONTAINER_KEYS,
        ResourceKind::Images => IMAGE_KEYS,
        ResourceKind::Volumes => VOLUME_KEYS,
        ResourceKind::Networks => NETWORK_KEYS,
    }
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

/// An image as the search sees it: the texts of its cells.
pub(crate) fn image_row(image: &Image, now: i64) -> SearchRow {
    let row = SearchRow::new(image_name(image))
        .value("repository", image_name(image))
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
    TagSchema::new(CONTAINER_KEYS, rows)
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
    TagSchema::new(IMAGE_KEYS, rows)
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
    TagSchema::new(VOLUME_KEYS, rows)
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
    TagSchema::new(NETWORK_KEYS, rows)
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
    use otty_docker::{EngineKind, Resources, fixtures};
    use otty_ui_kit::components::data_table::{CellPart, TableCell};
    use otty_ui_kit::components::tag_search::TagSearchAction;

    use super::*;
    use crate::widgets::docker::format::NO_VALUE;
    use crate::widgets::docker::resource_rows::{
        columns, container_cells, image_cells, network_cells, volume_cells,
    };
    use crate::widgets::docker::sort::TableSorts;

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
            let shown = shown_text(cell);
            let values: Vec<&str> = row.values_of(key).collect();
            if shown.is_empty() || shown == NO_VALUE {
                assert!(values.is_empty(), "{key}: {values:?}");
            } else {
                assert_eq!(values.join(", "), shown, "{key}");
            }
        }
    }

    #[test]
    fn each_table_has_its_keys_in_column_order() {
        assert_eq!(
            keys(ResourceKind::Containers),
            ["name", "status", "image", "compose", "ports", "age"]
        );
        assert_eq!(
            keys(ResourceKind::Images),
            ["repository", "id", "size", "used-by", "age"]
        );
        assert_eq!(
            keys(ResourceKind::Volumes),
            ["name", "driver", "used-by", "age"]
        );
        assert_eq!(
            keys(ResourceKind::Networks),
            ["name", "driver", "scope", "subnet", "containers", "age"]
        );
    }

    #[test]
    fn each_table_has_one_key_per_column() {
        for resource in ResourceKind::ALL {
            assert_eq!(keys(resource).len(), columns(resource).len());
        }
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
        let sort = TableSorts::default().get(ResourceKind::Containers);

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
