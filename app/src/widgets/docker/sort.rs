//! How the Docker tables are ordered: the kept choice per page, the
//! defaults, what a header press does, and the sorting of the shown rows
//! (spec Decision 19).

use std::cmp::Ordering;
use std::collections::HashMap;

use otty_docker::{
    Container, ContainerState, Image, Network, Volume, compare_text,
};

use super::format::ports_text;
use super::types::{
    Place, ResourceKind, image_name, image_repository, image_tag,
};

// Column positions. Column 0 is the name in every table, so the name
// sort is the fallback arm of each `sort_*` function.

/// Column positions in the containers table.
const CONTAINER_STATUS: usize = 1;
const CONTAINER_IMAGE: usize = 2;
const CONTAINER_COMPOSE: usize = 3;
const CONTAINER_PORTS: usize = 4;
const CONTAINER_AGE: usize = 5;

/// Column positions in the images table.
const IMAGE_TAG: usize = 1;
const IMAGE_ID: usize = 2;
const IMAGE_SIZE: usize = 3;
const IMAGE_USED_BY: usize = 4;
const IMAGE_AGE: usize = 5;

/// Column positions in the volumes table.
const VOLUME_DRIVER: usize = 1;
const VOLUME_USED_BY: usize = 2;
const VOLUME_AGE: usize = 3;

/// Column positions in the networks table.
const NETWORK_DRIVER: usize = 1;
const NETWORK_SCOPE: usize = 2;
const NETWORK_SUBNET: usize = 3;
const NETWORK_CONTAINERS: usize = 4;
const NETWORK_AGE: usize = 5;

/// Which way a column is ordered. Ascending is A to Z, smallest first and
/// oldest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    /// The other way.
    fn reversed(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    /// `ordering` as this way sees it.
    fn apply(self, ordering: Ordering) -> Ordering {
        match self {
            Self::Ascending => ordering,
            Self::Descending => ordering.reverse(),
        }
    }
}

/// The column a table is ordered by and the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TableSort {
    column: usize,
    order: SortOrder,
}

impl TableSort {
    /// The position of the sorted column.
    pub(crate) fn column(&self) -> usize {
        self.column
    }

    /// The way the column is ordered.
    pub(crate) fn order(&self) -> SortOrder {
        self.order
    }

    /// The sort of a table nobody sorted yet: containers by status (running
    /// first), the other tables by name.
    pub(crate) fn default_for(resource: ResourceKind) -> Self {
        let column = match resource {
            ResourceKind::Containers => CONTAINER_STATUS,
            _ => 0,
        };

        Self {
            column,
            order: SortOrder::Ascending,
        }
    }

    /// The sort after the title of `column` was pressed: the same column
    /// turns around, a new column starts in its natural way.
    fn pressed(self, resource: ResourceKind, column: usize) -> Self {
        if !is_sortable(resource, column) {
            return self;
        }
        if column == self.column {
            return Self {
                column,
                order: self.order.reversed(),
            };
        }

        Self {
            column,
            order: natural_order(resource, column),
        }
    }
}

/// The kept sort of each page (spec Decision 21). It lives as long as OTTY
/// runs: apart from the engines, so a page keeps its sort while its engine is
/// away.
#[derive(Debug, Default)]
pub(crate) struct TableSorts {
    /// Only the pages someone sorted; the others have their default.
    pages: HashMap<Place, TableSort>,
}

impl TableSorts {
    /// The sort of the page `place`.
    pub(crate) fn get(&self, place: &Place) -> TableSort {
        match self.pages.get(place) {
            Some(sort) => *sort,
            None => TableSort::default_for(place.resource()),
        }
    }

    /// Take a press on the title of `column` of the page `place`.
    pub(crate) fn press(&mut self, place: &Place, column: usize) {
        let pressed = self.get(place).pressed(place.resource(), column);

        self.pages.insert(place.clone(), pressed);
    }
}

/// Whether the `resource` table has a column at `column`: every column can be
/// sorted.
pub(crate) fn is_sortable(resource: ResourceKind, column: usize) -> bool {
    match resource {
        ResourceKind::Containers => column <= CONTAINER_AGE,
        ResourceKind::Images => column <= IMAGE_AGE,
        ResourceKind::Volumes => column <= VOLUME_AGE,
        ResourceKind::Networks => column <= NETWORK_AGE,
    }
}

/// Order the shown containers by `sort`; ties are ordered by name.
pub(crate) fn sort_containers(rows: &mut [&Container], sort: TableSort) {
    if sort.column == CONTAINER_PORTS {
        sort_containers_by_ports(rows, sort.order);
        return;
    }

    rows.sort_by(|a, b| {
        let primary = match sort.column {
            CONTAINER_STATUS => {
                status_rank(a.state()).cmp(&status_rank(b.state()))
            },
            CONTAINER_IMAGE => compare_text(a.image(), b.image()),
            CONTAINER_COMPOSE => {
                compare_optional_text(a.compose_project(), b.compose_project())
            },
            CONTAINER_AGE => a.created().cmp(&b.created()),
            _ => compare_text(a.name(), b.name()),
        };

        sort.order
            .apply(primary)
            .then_with(|| compare_text(a.name(), b.name()))
    });
}

/// Order the shown images by `sort`; ties are ordered by name, which puts the
/// tags of one repository in order.
pub(crate) fn sort_images(rows: &mut [&Image], sort: TableSort) {
    rows.sort_by(|a, b| {
        let primary = match sort.column {
            IMAGE_TAG => compare_text(image_tag(a), image_tag(b)),
            IMAGE_ID => compare_text(a.id(), b.id()),
            IMAGE_SIZE => a.size().cmp(&b.size()),
            IMAGE_USED_BY => a.containers().cmp(&b.containers()),
            IMAGE_AGE => a.created().cmp(&b.created()),
            _ => compare_text(image_repository(a), image_repository(b)),
        };

        sort.order
            .apply(primary)
            .then_with(|| compare_text(image_name(a), image_name(b)))
    });
}

/// Order the shown volumes by `sort`; ties are ordered by name.
pub(crate) fn sort_volumes(rows: &mut [&Volume], sort: TableSort) {
    rows.sort_by(|a, b| {
        let primary = match sort.column {
            VOLUME_DRIVER => compare_text(a.driver(), b.driver()),
            VOLUME_USED_BY => {
                compare_text_lists(a.containers(), b.containers())
            },
            VOLUME_AGE => a.created().cmp(&b.created()),
            _ => compare_text(a.name(), b.name()),
        };

        sort.order
            .apply(primary)
            .then_with(|| compare_text(a.name(), b.name()))
    });
}

/// Order the shown networks by `sort`; ties are ordered by name.
pub(crate) fn sort_networks(rows: &mut [&Network], sort: TableSort) {
    rows.sort_by(|a, b| {
        let primary = match sort.column {
            NETWORK_DRIVER => compare_text(a.driver(), b.driver()),
            NETWORK_SCOPE => compare_text(a.scope(), b.scope()),
            NETWORK_SUBNET => compare_optional_text(a.subnet(), b.subnet()),
            NETWORK_CONTAINERS => {
                a.containers().len().cmp(&b.containers().len())
            },
            NETWORK_AGE => a.created().cmp(&b.created()),
            _ => compare_text(a.name(), b.name()),
        };

        sort.order
            .apply(primary)
            .then_with(|| compare_text(a.name(), b.name()))
    });
}

/// The way a column is ordered the first time it is pressed: text A to Z
/// and status by the state order; ages newest first; sizes and counts
/// largest first.
fn natural_order(resource: ResourceKind, column: usize) -> SortOrder {
    let is_newest_or_largest = match resource {
        ResourceKind::Containers => column == CONTAINER_AGE,
        ResourceKind::Images => {
            matches!(column, IMAGE_SIZE | IMAGE_USED_BY | IMAGE_AGE)
        },
        ResourceKind::Volumes => column == VOLUME_AGE,
        ResourceKind::Networks => {
            matches!(column, NETWORK_CONTAINERS | NETWORK_AGE)
        },
    };

    if is_newest_or_largest {
        SortOrder::Descending
    } else {
        SortOrder::Ascending
    }
}

/// Order the shown containers by the text of their Ports cell, which is built
/// once for each row before the sort; ties are ordered by name.
fn sort_containers_by_ports(rows: &mut [&Container], order: SortOrder) {
    let mut keyed: Vec<(String, &Container)> = rows
        .iter()
        .map(|container| (ports_text(container.ports()), *container))
        .collect();

    keyed.sort_by(|(a_ports, a), (b_ports, b)| {
        order
            .apply(compare_text(a_ports, b_ports))
            .then_with(|| compare_text(a.name(), b.name()))
    });

    for (row, (_, container)) in rows.iter_mut().zip(keyed) {
        *row = container;
    }
}

/// Compare two texts where a row may have none: no text comes first.
fn compare_optional_text(a: Option<&str>, b: Option<&str>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => compare_text(a, b),
        _ => a.is_some().cmp(&b.is_some()),
    }
}

/// Compare two lists of texts, text by text; when one list starts with the
/// other, the shorter one comes first.
fn compare_text_lists(a: &[String], b: &[String]) -> Ordering {
    let first_difference = a
        .iter()
        .zip(b)
        .map(|(a, b)| compare_text(a, b))
        .find(|ordering| ordering.is_ne());

    first_difference.unwrap_or_else(|| a.len().cmp(&b.len()))
}

/// Where a container state comes in the status order: running, restarting,
/// paused, created, exited, dead, then the rest.
fn status_rank(state: ContainerState) -> usize {
    match state {
        ContainerState::Running => 0,
        ContainerState::Restarting => 1,
        ContainerState::Paused => 2,
        ContainerState::Created => 3,
        ContainerState::Exited => 4,
        ContainerState::Dead => 5,
        _ => 6,
    }
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;

    const CONTAINER_NAME: usize = 0;
    const IMAGE_NAME: usize = 0;
    const VOLUME_NAME: usize = 0;
    const NETWORK_NAME: usize = 0;

    fn docker() -> otty_docker::Resources {
        fixtures::resources(EngineKind::Docker)
    }

    /// Check that `rows` follow `key` in `order`, and that rows with an
    /// equal key follow the name ascending.
    fn assert_ordered<T, K: Ord + std::fmt::Debug>(
        rows: &[&T],
        order: SortOrder,
        key: impl Fn(&T) -> K,
        name: impl Fn(&T) -> String,
    ) {
        for pair in rows.windows(2) {
            let (a, b) = (key(pair[0]), key(pair[1]));
            match order {
                SortOrder::Ascending => assert!(a <= b, "{a:?} before {b:?}"),
                SortOrder::Descending => {
                    assert!(a >= b, "{a:?} before {b:?}")
                },
            }
            if a == b {
                assert!(name(pair[0]) <= name(pair[1]));
            }
        }
    }

    fn sort_of(column: usize, order: SortOrder) -> TableSort {
        TableSort { column, order }
    }

    /// The `resource` page of the fixture Docker engine.
    fn page(resource: ResourceKind) -> Place {
        let docker = fixtures::engine(EngineKind::Docker);

        Place::new(docker.address().clone(), resource)
    }

    /// The sort of a `resource` table nobody sorted, after a press on `column`.
    fn pressed(resource: ResourceKind, column: usize) -> TableSort {
        TableSort::default_for(resource).pressed(resource, column)
    }

    fn both() -> [SortOrder; 2] {
        [SortOrder::Ascending, SortOrder::Descending]
    }

    fn rank(container: &Container) -> usize {
        match container.state() {
            ContainerState::Running => 0,
            ContainerState::Restarting => 1,
            ContainerState::Paused => 2,
            ContainerState::Created => 3,
            ContainerState::Exited => 4,
            ContainerState::Dead => 5,
            _ => 6,
        }
    }

    #[test]
    fn the_default_sort_is_status_for_containers_and_name_for_the_rest() {
        let sorts = TableSorts::default();

        assert_eq!(
            sorts.get(&page(ResourceKind::Containers)),
            sort_of(CONTAINER_STATUS, SortOrder::Ascending)
        );
        for resource in [
            ResourceKind::Images,
            ResourceKind::Volumes,
            ResourceKind::Networks,
        ] {
            assert_eq!(
                sorts.get(&page(resource)),
                sort_of(0, SortOrder::Ascending),
                "{resource:?}"
            );
        }
    }

    #[test]
    fn the_default_container_order_is_running_first_then_name() {
        let resources = docker();
        let mut rows: Vec<&Container> = resources.containers().iter().collect();

        sort_containers(
            &mut rows,
            TableSort::default_for(ResourceKind::Containers),
        );

        assert_eq!(rows[0].state(), ContainerState::Running);
        assert_ordered(&rows, SortOrder::Ascending, rank, |c| {
            c.name().to_string()
        });
    }

    #[test]
    fn containers_sort_by_every_sortable_column_in_both_directions() {
        let resources = docker();

        for order in both() {
            let check = |column: usize, key: &dyn Fn(&Container) -> String| {
                let mut rows: Vec<&Container> =
                    resources.containers().iter().collect();
                sort_containers(&mut rows, sort_of(column, order));
                assert_ordered(&rows, order, key, |c| c.name().to_string());
            };

            check(CONTAINER_NAME, &|c| c.name().to_string());
            check(CONTAINER_IMAGE, &|c| c.image().to_string());
            check(CONTAINER_COMPOSE, &|c| {
                c.compose_project().unwrap_or("").to_string()
            });
            check(CONTAINER_AGE, &|c| {
                format!("{:020}", c.created().unwrap_or(0))
            });
            check(CONTAINER_STATUS, &|c| rank(c).to_string());
            check(CONTAINER_PORTS, &|c| ports_text(c.ports()));
        }
    }

    #[test]
    fn container_status_in_descending_order_puts_running_last() {
        let resources = docker();
        let mut rows: Vec<&Container> = resources.containers().iter().collect();

        sort_containers(
            &mut rows,
            sort_of(CONTAINER_STATUS, SortOrder::Descending),
        );

        assert_ne!(rows[0].state(), ContainerState::Running);
        assert_eq!(
            rows.last().map(|c| c.state()),
            Some(ContainerState::Running)
        );
    }

    #[test]
    fn images_sort_by_every_sortable_column_in_both_directions() {
        let resources = docker();

        for order in both() {
            let check = |column: usize, key: &dyn Fn(&Image) -> String| {
                let mut rows: Vec<&Image> = resources.images().iter().collect();
                sort_images(&mut rows, sort_of(column, order));
                assert_ordered(&rows, order, key, |i| {
                    image_name(i).to_string()
                });
            };

            check(IMAGE_NAME, &|i| image_repository(i).to_string());
            check(IMAGE_TAG, &|i| image_tag(i).to_string());
            check(IMAGE_ID, &|i| i.id().to_string());
            check(IMAGE_SIZE, &|i| format!("{:020}", i.size()));
            check(IMAGE_USED_BY, &|i| format!("{:020}", i.containers()));
            check(IMAGE_AGE, &|i| format!("{:020}", i.created()));
        }
    }

    fn image_order(names: &[&str], sort: TableSort) -> Vec<String> {
        let images = fixtures::image_rows("sha256:app", names, &[], &[]);
        let mut rows: Vec<&Image> = images.iter().collect();

        sort_images(&mut rows, sort);

        rows.iter().map(|row| image_name(row).to_string()).collect()
    }

    #[test]
    fn images_of_one_repository_are_ordered_by_tag_by_default() {
        let sort = TableSort::default_for(ResourceKind::Images);

        assert_eq!(
            image_order(&["b:1", "a:2", "a:1"], sort),
            vec!["a:1", "a:2", "b:1"]
        );
    }

    #[test]
    fn the_repository_column_orders_by_the_repository_alone() {
        let sort = sort_of(IMAGE_NAME, SortOrder::Ascending);

        // `a-b:1` sorts before `a:1` as text, but `a` comes before `a-b`.
        assert_eq!(image_order(&["a-b:1", "a:1"], sort), vec!["a:1", "a-b:1"]);
    }

    #[test]
    fn the_tag_column_orders_by_tag_then_by_name() {
        let sort = sort_of(IMAGE_TAG, SortOrder::Ascending);

        assert_eq!(
            image_order(&["b:1", "a:2", "a:1"], sort),
            vec!["a:1", "b:1", "a:2"]
        );
    }

    #[test]
    fn a_new_tag_column_starts_a_to_z() {
        assert_eq!(
            pressed(ResourceKind::Images, IMAGE_TAG),
            sort_of(IMAGE_TAG, SortOrder::Ascending)
        );
    }

    #[test]
    fn volumes_sort_by_every_sortable_column_in_both_directions() {
        let resources = docker();

        for order in both() {
            let check = |column: usize, key: &dyn Fn(&Volume) -> String| {
                let mut rows: Vec<&Volume> =
                    resources.volumes().iter().collect();
                sort_volumes(&mut rows, sort_of(column, order));
                assert_ordered(&rows, order, key, |v| v.name().to_string());
            };

            check(VOLUME_NAME, &|v| v.name().to_string());
            check(VOLUME_DRIVER, &|v| v.driver().to_string());
            check(VOLUME_USED_BY, &|v| v.containers().join(", "));
            check(VOLUME_AGE, &|v| format!("{:020}", v.created().unwrap_or(0)));
        }
    }

    #[test]
    fn networks_sort_by_every_sortable_column_in_both_directions() {
        let resources = docker();

        for order in both() {
            let check = |column: usize, key: &dyn Fn(&Network) -> String| {
                let mut rows: Vec<&Network> =
                    resources.networks().iter().collect();
                sort_networks(&mut rows, sort_of(column, order));
                assert_ordered(&rows, order, key, |n| n.name().to_string());
            };

            check(NETWORK_NAME, &|n| n.name().to_string());
            check(NETWORK_DRIVER, &|n| n.driver().to_string());
            check(NETWORK_SCOPE, &|n| n.scope().to_string());
            check(NETWORK_SUBNET, &|n| n.subnet().unwrap_or("").to_string());
            check(NETWORK_CONTAINERS, &|n| {
                format!("{:020}", n.containers().len())
            });
            check(NETWORK_AGE, &|n| {
                format!("{:020}", n.created().unwrap_or(0))
            });
        }
    }

    #[test]
    fn a_new_column_starts_in_its_natural_direction() {
        // Text A to Z.
        assert_eq!(
            pressed(ResourceKind::Containers, CONTAINER_NAME),
            sort_of(CONTAINER_NAME, SortOrder::Ascending)
        );
        assert_eq!(
            pressed(ResourceKind::Networks, NETWORK_SUBNET),
            sort_of(NETWORK_SUBNET, SortOrder::Ascending)
        );
        assert_eq!(
            pressed(ResourceKind::Containers, CONTAINER_PORTS),
            sort_of(CONTAINER_PORTS, SortOrder::Ascending)
        );
        // Ages: newest first.
        assert_eq!(
            pressed(ResourceKind::Containers, CONTAINER_AGE),
            sort_of(CONTAINER_AGE, SortOrder::Descending)
        );
        assert_eq!(
            pressed(ResourceKind::Volumes, VOLUME_AGE),
            sort_of(VOLUME_AGE, SortOrder::Descending)
        );
        // Sizes and counts: largest first.
        assert_eq!(
            pressed(ResourceKind::Images, IMAGE_SIZE),
            sort_of(IMAGE_SIZE, SortOrder::Descending)
        );
        assert_eq!(
            pressed(ResourceKind::Images, IMAGE_USED_BY),
            sort_of(IMAGE_USED_BY, SortOrder::Descending)
        );
        assert_eq!(
            pressed(ResourceKind::Networks, NETWORK_CONTAINERS),
            sort_of(NETWORK_CONTAINERS, SortOrder::Descending)
        );
        // Status: the state order, running first.
        let by_status = pressed(ResourceKind::Containers, CONTAINER_NAME)
            .pressed(ResourceKind::Containers, CONTAINER_STATUS);
        assert_eq!(by_status, sort_of(CONTAINER_STATUS, SortOrder::Ascending));
    }

    #[test]
    fn pressing_the_same_column_again_reverses_it() {
        let resource = ResourceKind::Images;
        let once = pressed(resource, IMAGE_SIZE);

        let twice = once.pressed(resource, IMAGE_SIZE);
        let thrice = twice.pressed(resource, IMAGE_SIZE);

        assert_eq!(twice, sort_of(IMAGE_SIZE, SortOrder::Ascending));
        assert_eq!(thrice, once);
    }

    #[test]
    fn pressing_the_default_column_reverses_the_default() {
        assert_eq!(
            pressed(ResourceKind::Containers, CONTAINER_STATUS),
            sort_of(CONTAINER_STATUS, SortOrder::Descending)
        );
    }

    #[test]
    fn each_table_keeps_its_own_sort() {
        let mut sorts = TableSorts::default();

        sorts.press(&page(ResourceKind::Images), IMAGE_SIZE);

        for resource in [ResourceKind::Containers, ResourceKind::Volumes] {
            assert_eq!(
                sorts.get(&page(resource)),
                TableSort::default_for(resource),
                "{resource:?}"
            );
        }
    }

    #[test]
    fn every_column_can_be_sorted_and_a_position_past_the_last_cannot() {
        for column in 0..=CONTAINER_AGE {
            assert!(is_sortable(ResourceKind::Containers, column), "{column}");
        }
        assert!(!is_sortable(ResourceKind::Containers, CONTAINER_AGE + 1));
        assert!(is_sortable(ResourceKind::Images, IMAGE_AGE));
        assert!(!is_sortable(ResourceKind::Images, 9));

        let mut sorts = TableSorts::default();
        sorts.press(&page(ResourceKind::Containers), CONTAINER_AGE + 1);
        assert_eq!(
            sorts.get(&page(ResourceKind::Containers)),
            TableSort::default_for(ResourceKind::Containers)
        );
    }

    /// One container: its name and its ports, as the fixture takes them.
    type PortRow<'a> =
        (&'a str, &'a [(Option<&'a str>, Option<u16>, u16, &'a str)]);

    /// The names of `rows`, as a sort of the Ports column in `order` puts
    /// them.
    fn ports_order(rows: &[PortRow], order: SortOrder) -> Vec<String> {
        let containers: Vec<Container> = rows
            .iter()
            .map(|(name, ports)| {
                fixtures::container_named_with_ports(name, ports)
            })
            .collect();
        let mut rows: Vec<&Container> = containers.iter().collect();

        sort_containers(&mut rows, sort_of(CONTAINER_PORTS, order));

        rows.iter().map(|row| row.name().to_string()).collect()
    }

    #[test]
    fn the_ports_column_is_text_so_10000_comes_before_80() {
        let rows: [PortRow; 3] = [
            ("eighty", &[(Some("0.0.0.0"), Some(80), 80, "tcp")]),
            ("big", &[(Some("0.0.0.0"), Some(10000), 80, "tcp")]),
            ("exposed", &[(None, None, 443, "tcp")]),
        ];

        assert_eq!(
            ports_order(&rows, SortOrder::Ascending),
            ["big", "exposed", "eighty"]
        );
        assert_eq!(
            ports_order(&rows, SortOrder::Descending),
            ["eighty", "exposed", "big"]
        );
    }

    #[test]
    fn a_container_without_ports_sorts_by_the_dash_its_cell_shows() {
        // The cell shows the dash, not nothing, and the dash comes after the
        // digits, so these rows come last ascending and first descending.
        let rows: [PortRow; 3] = [
            ("none", &[]),
            ("web", &[(Some("0.0.0.0"), Some(8080), 80, "tcp")]),
            ("cache", &[(None, None, 6379, "tcp")]),
        ];

        assert_eq!(
            ports_order(&rows, SortOrder::Ascending),
            ["cache", "web", "none"]
        );
        assert_eq!(
            ports_order(&rows, SortOrder::Descending),
            ["none", "web", "cache"]
        );
    }

    #[test]
    fn several_ports_compare_as_the_joined_text_of_the_cell() {
        // "8080:80/tcp, 5432:5432/tcp" starts with "8080:80/tcp", so it comes
        // right after it; the lower second port does not move it forward. The
        // names are in the other order, so only the text can explain this.
        let rows: [PortRow; 3] = [
            (
                "a-two",
                &[
                    (Some("0.0.0.0"), Some(8080), 80, "tcp"),
                    (Some("0.0.0.0"), Some(5432), 5432, "tcp"),
                ],
            ),
            ("b-one", &[(Some("0.0.0.0"), Some(8080), 80, "tcp")]),
            ("c-low", &[(Some("0.0.0.0"), Some(5432), 5432, "tcp")]),
        ];

        assert_eq!(
            ports_order(&rows, SortOrder::Ascending),
            ["c-low", "b-one", "a-two"]
        );
    }

    #[test]
    fn rows_whose_ports_read_the_same_follow_the_name_in_both_directions() {
        // Docker lists a port once per address family and the cell shows it
        // once; the address is not in the cell either. All three read
        // "8080:80/tcp".
        let rows: [PortRow; 3] = [
            (
                "zeta",
                &[
                    (Some("0.0.0.0"), Some(8080), 80, "tcp"),
                    (Some("::"), Some(8080), 80, "tcp"),
                ],
            ),
            ("Beta", &[(Some("0.0.0.0"), Some(8080), 80, "tcp")]),
            ("alpha", &[(Some("127.0.0.1"), Some(8080), 80, "tcp")]),
        ];

        for order in both() {
            assert_eq!(
                ports_order(&rows, order),
                ["alpha", "Beta", "zeta"],
                "{order:?}"
            );
        }
    }

    /// Rows the sort cannot tell apart keep the order they came in.
    #[test]
    fn rows_with_the_same_ports_and_name_keep_their_order() {
        let ports: &[(Option<&str>, Option<u16>, u16, &str)] =
            &[(Some("0.0.0.0"), Some(80), 80, "tcp")];
        let containers: Vec<Container> = (0..4)
            .map(|_| fixtures::container_named_with_ports("same", ports))
            .collect();

        for order in both() {
            let mut rows: Vec<&Container> = containers.iter().collect();
            sort_containers(&mut rows, sort_of(CONTAINER_PORTS, order));

            for (row, container) in rows.iter().zip(&containers) {
                assert!(std::ptr::eq(*row, container), "{order:?}");
            }
        }
    }

    /// The names of containers called `names`, as `sort` orders them.
    fn container_order(names: &[&str], sort: TableSort) -> Vec<String> {
        let containers: Vec<Container> = names
            .iter()
            .map(|name| {
                fixtures::container_of_image(name, "running", "app", "sha256:a")
            })
            .collect();
        let mut rows: Vec<&Container> = containers.iter().collect();

        sort_containers(&mut rows, sort);

        rows.iter().map(|row| row.name().to_string()).collect()
    }

    #[test]
    fn no_text_comes_first_and_text_ignores_case() {
        let (none, zebra, apple) = (None, Some("Zebra"), Some("apple"));

        assert_eq!(compare_optional_text(none, apple), Ordering::Less);
        assert_eq!(compare_optional_text(zebra, none), Ordering::Greater);
        assert_eq!(compare_optional_text(none, none), Ordering::Equal);
        assert_eq!(compare_optional_text(apple, zebra), Ordering::Less);
    }

    #[test]
    fn lists_of_text_compare_text_by_text_ignoring_case() {
        let list = |names: &[&str]| -> Vec<String> {
            names.iter().map(|name| name.to_string()).collect()
        };

        assert_eq!(
            compare_text_lists(&list(&["apple"]), &list(&["Banana"])),
            Ordering::Less
        );
        assert_eq!(
            compare_text_lists(&list(&["a", "Zed"]), &list(&["a", "bee"])),
            Ordering::Greater
        );
        assert_eq!(
            compare_text_lists(&list(&["a"]), &list(&["a", "b"])),
            Ordering::Less
        );
        assert_eq!(compare_text_lists(&[], &[]), Ordering::Equal);
    }

    #[test]
    fn the_name_column_of_containers_ignores_case_in_both_directions() {
        let names = ["zeta", "Alpha", "beta", "Gamma"];

        assert_eq!(
            container_order(
                &names,
                sort_of(CONTAINER_NAME, SortOrder::Ascending)
            ),
            ["Alpha", "beta", "Gamma", "zeta"]
        );
        assert_eq!(
            container_order(
                &names,
                sort_of(CONTAINER_NAME, SortOrder::Descending)
            ),
            ["zeta", "Gamma", "beta", "Alpha"]
        );
    }

    #[test]
    fn rows_with_one_status_follow_the_name_ignoring_case() {
        let sort = TableSort::default_for(ResourceKind::Containers);

        assert_eq!(
            container_order(&["beta", "Gamma", "alpha"], sort),
            ["alpha", "beta", "Gamma"]
        );
    }

    #[test]
    fn names_that_differ_only_in_case_stay_put_whatever_order_they_came_in() {
        let sort = sort_of(CONTAINER_NAME, SortOrder::Ascending);

        assert_eq!(container_order(&["web", "Web"], sort), ["Web", "web"]);
        assert_eq!(container_order(&["Web", "web"], sort), ["Web", "web"]);
    }

    #[test]
    fn the_image_column_of_containers_ignores_case() {
        let containers = [
            fixtures::container_of_image("a", "running", "Zeta:1", "sha256:a"),
            fixtures::container_of_image("b", "running", "beta:1", "sha256:a"),
        ];
        let mut rows: Vec<&Container> = containers.iter().collect();

        sort_containers(
            &mut rows,
            sort_of(CONTAINER_IMAGE, SortOrder::Ascending),
        );

        assert_eq!(rows[0].image(), "beta:1");
    }

    #[test]
    fn the_compose_column_ignores_case_and_rows_without_a_project_come_first() {
        let containers = [
            fixtures::container_in_project("a", "Zeta"),
            fixtures::container_of_image("b", "running", "app", "sha256:a"),
            fixtures::container_in_project("c", "beta"),
        ];
        let mut rows: Vec<&Container> = containers.iter().collect();

        sort_containers(
            &mut rows,
            sort_of(CONTAINER_COMPOSE, SortOrder::Ascending),
        );

        let names: Vec<&str> = rows.iter().map(|row| row.name()).collect();
        assert_eq!(names, ["b", "c", "a"]);
    }

    #[test]
    fn the_repository_column_ignores_case_and_so_do_its_ties() {
        let sort = sort_of(IMAGE_NAME, SortOrder::Ascending);

        assert_eq!(
            image_order(&["zeta:1", "Beta:1", "alpha:1"], sort),
            ["alpha:1", "Beta:1", "zeta:1"]
        );
        assert_eq!(
            image_order(&["app:b", "app:C", "app:a"], sort),
            ["app:a", "app:b", "app:C"]
        );
    }

    #[test]
    fn the_tag_column_ignores_case_with_none_after_digits_and_before_letters() {
        let sort = sort_of(IMAGE_TAG, SortOrder::Ascending);
        let digest = "app@sha256:d1";
        let mut images =
            fixtures::image_rows("sha256:a", &["a:Latest"], &[], &[]);
        images.extend(fixtures::image_rows("sha256:b", &["a:alpha"], &[], &[]));
        images.extend(fixtures::image_rows("sha256:c", &["a:10"], &[], &[]));
        images.extend(fixtures::image_rows("sha256:d", &[], &[digest], &[]));
        let mut rows: Vec<&Image> = images.iter().collect();

        sort_images(&mut rows, sort);

        let tags: Vec<&str> = rows.iter().map(|row| image_tag(row)).collect();
        assert_eq!(tags, ["10", "<none>", "alpha", "Latest"]);
    }

    #[test]
    fn the_id_column_ignores_case() {
        let mut images = fixtures::image_rows("sha256:B", &["a:1"], &[], &[]);
        images.extend(fixtures::image_rows("sha256:a", &["b:1"], &[], &[]));
        let mut rows: Vec<&Image> = images.iter().collect();

        sort_images(&mut rows, sort_of(IMAGE_ID, SortOrder::Ascending));

        assert_eq!(rows[0].id(), "sha256:a");
    }

    #[test]
    fn every_text_column_of_volumes_ignores_case() {
        let volumes = [
            fixtures::volume_named("Zeta", "local", &["alpha"]),
            fixtures::volume_named("beta", "Nfs", &["Bob"]),
        ];
        let order = |column: usize| -> Vec<String> {
            let mut rows: Vec<&Volume> = volumes.iter().collect();
            sort_volumes(&mut rows, sort_of(column, SortOrder::Ascending));

            rows.iter().map(|row| row.name().to_string()).collect()
        };

        assert_eq!(order(VOLUME_NAME), ["beta", "Zeta"]);
        assert_eq!(order(VOLUME_DRIVER), ["Zeta", "beta"]);
        assert_eq!(order(VOLUME_USED_BY), ["Zeta", "beta"]);
    }

    #[test]
    fn the_used_by_column_of_volumes_follows_the_names_as_the_cell_shows_them()
    {
        // The cell of `first` reads "alpha, Zed", so it comes before the
        // "beta" of `second`.
        let volumes = [
            fixtures::volume_named("second", "local", &["beta"]),
            fixtures::volume_named("first", "local", &["Zed", "alpha"]),
        ];
        let mut rows: Vec<&Volume> = volumes.iter().collect();

        sort_volumes(&mut rows, sort_of(VOLUME_USED_BY, SortOrder::Ascending));

        assert_eq!(rows[0].containers(), ["alpha", "Zed"]);
        assert_eq!(rows[1].containers(), ["beta"]);
    }

    #[test]
    fn every_text_column_of_networks_ignores_case() {
        let networks = [
            fixtures::network_named("Zeta", "bridge", "local", Some("a::/64")),
            fixtures::network_named("beta", "Overlay", "Swarm", Some("B::/64")),
        ];
        let order = |column: usize| -> Vec<String> {
            let mut rows: Vec<&Network> = networks.iter().collect();
            sort_networks(&mut rows, sort_of(column, SortOrder::Ascending));

            rows.iter().map(|row| row.name().to_string()).collect()
        };

        assert_eq!(order(NETWORK_NAME), ["beta", "Zeta"]);
        assert_eq!(order(NETWORK_DRIVER), ["Zeta", "beta"]);
        assert_eq!(order(NETWORK_SCOPE), ["Zeta", "beta"]);
        assert_eq!(order(NETWORK_SUBNET), ["Zeta", "beta"]);
    }

    #[test]
    fn each_page_keeps_its_own_sort() {
        let docker = page(ResourceKind::Containers);
        let podman = Place::new(
            fixtures::engine(EngineKind::Podman).address().clone(),
            ResourceKind::Containers,
        );
        let mut sorts = TableSorts::default();

        sorts.press(&docker, CONTAINER_NAME);

        assert_eq!(
            sorts.get(&docker),
            sort_of(CONTAINER_NAME, SortOrder::Ascending)
        );
        assert_eq!(
            sorts.get(&podman),
            TableSort::default_for(ResourceKind::Containers)
        );
    }

    #[test]
    fn ties_follow_the_name_whatever_the_direction() {
        let resources = docker();
        let mut rows: Vec<&Network> = resources.networks().iter().collect();

        sort_networks(&mut rows, sort_of(NETWORK_SCOPE, SortOrder::Descending));

        let local: Vec<&str> = rows
            .iter()
            .filter(|n| n.scope() == "local")
            .map(|n| n.name())
            .collect();
        let mut by_name = local.clone();
        by_name.sort_unstable();
        assert!(local.len() > 1);
        assert_eq!(local, by_name);
    }
}
