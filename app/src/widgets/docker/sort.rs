//! How the Docker tables are ordered: the kept choice per table, the
//! defaults, what a header press does, and the sorting of the shown rows
//! (spec Decision 19).

use std::cmp::Ordering;

use otty_docker::{Container, ContainerState, Image, Network, Volume};

use super::types::{ResourceKind, image_name};

// Column positions. Column 0 is the name in every table, so the name
// sort is the fallback arm of each `sort_*` function.

/// Column positions in the containers table.
const CONTAINER_STATUS: usize = 1;
const CONTAINER_IMAGE: usize = 2;
const CONTAINER_COMPOSE: usize = 3;
const CONTAINER_PORTS: usize = 4;
const CONTAINER_AGE: usize = 5;

/// Column positions in the images table.
const IMAGE_ID: usize = 1;
const IMAGE_SIZE: usize = 2;
const IMAGE_USED_BY: usize = 3;
const IMAGE_AGE: usize = 4;

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
    fn default_for(resource: ResourceKind) -> Self {
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

/// The kept sort of each table; it lives as long as OTTY runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TableSorts {
    containers: TableSort,
    images: TableSort,
    volumes: TableSort,
    networks: TableSort,
}

impl TableSorts {
    /// The sort of the `resource` table.
    pub(crate) fn get(&self, resource: ResourceKind) -> TableSort {
        match resource {
            ResourceKind::Containers => self.containers,
            ResourceKind::Images => self.images,
            ResourceKind::Volumes => self.volumes,
            ResourceKind::Networks => self.networks,
        }
    }

    /// These sorts after the title of `column` of the `resource` table was
    /// pressed.
    pub(crate) fn pressed(
        mut self,
        resource: ResourceKind,
        column: usize,
    ) -> Self {
        let pressed = self.get(resource).pressed(resource, column);
        match resource {
            ResourceKind::Containers => self.containers = pressed,
            ResourceKind::Images => self.images = pressed,
            ResourceKind::Volumes => self.volumes = pressed,
            ResourceKind::Networks => self.networks = pressed,
        }

        self
    }
}

impl Default for TableSorts {
    fn default() -> Self {
        Self {
            containers: TableSort::default_for(ResourceKind::Containers),
            images: TableSort::default_for(ResourceKind::Images),
            volumes: TableSort::default_for(ResourceKind::Volumes),
            networks: TableSort::default_for(ResourceKind::Networks),
        }
    }
}

/// Whether the column at `column` of the `resource` table can be sorted.
/// Only the Ports column cannot.
pub(crate) fn is_sortable(resource: ResourceKind, column: usize) -> bool {
    match resource {
        ResourceKind::Containers => {
            column <= CONTAINER_AGE && column != CONTAINER_PORTS
        },
        ResourceKind::Images => column <= IMAGE_AGE,
        ResourceKind::Volumes => column <= VOLUME_AGE,
        ResourceKind::Networks => column <= NETWORK_AGE,
    }
}

/// Order the shown containers by `sort`; ties are ordered by name.
pub(crate) fn sort_containers(rows: &mut [&Container], sort: TableSort) {
    rows.sort_by(|a, b| {
        let primary = match sort.column {
            CONTAINER_STATUS => {
                status_rank(a.state()).cmp(&status_rank(b.state()))
            },
            CONTAINER_IMAGE => a.image().cmp(b.image()),
            CONTAINER_COMPOSE => a.compose_project().cmp(&b.compose_project()),
            CONTAINER_AGE => a.created().cmp(&b.created()),
            _ => a.name().cmp(b.name()),
        };

        sort.order
            .apply(primary)
            .then_with(|| a.name().cmp(b.name()))
    });
}

/// Order the shown images by `sort`; ties are ordered by name.
pub(crate) fn sort_images(rows: &mut [&Image], sort: TableSort) {
    rows.sort_by(|a, b| {
        let primary = match sort.column {
            IMAGE_ID => a.id().cmp(b.id()),
            IMAGE_SIZE => a.size().cmp(&b.size()),
            IMAGE_USED_BY => a.containers().cmp(&b.containers()),
            IMAGE_AGE => a.created().cmp(&b.created()),
            _ => image_name(a).cmp(image_name(b)),
        };

        sort.order
            .apply(primary)
            .then_with(|| image_name(a).cmp(image_name(b)))
    });
}

/// Order the shown volumes by `sort`; ties are ordered by name.
pub(crate) fn sort_volumes(rows: &mut [&Volume], sort: TableSort) {
    rows.sort_by(|a, b| {
        let primary = match sort.column {
            VOLUME_DRIVER => a.driver().cmp(b.driver()),
            VOLUME_USED_BY => a.containers().cmp(b.containers()),
            VOLUME_AGE => a.created().cmp(&b.created()),
            _ => a.name().cmp(b.name()),
        };

        sort.order
            .apply(primary)
            .then_with(|| a.name().cmp(b.name()))
    });
}

/// Order the shown networks by `sort`; ties are ordered by name.
pub(crate) fn sort_networks(rows: &mut [&Network], sort: TableSort) {
    rows.sort_by(|a, b| {
        let primary = match sort.column {
            NETWORK_DRIVER => a.driver().cmp(b.driver()),
            NETWORK_SCOPE => a.scope().cmp(b.scope()),
            NETWORK_SUBNET => a.subnet().cmp(&b.subnet()),
            NETWORK_CONTAINERS => {
                a.containers().len().cmp(&b.containers().len())
            },
            NETWORK_AGE => a.created().cmp(&b.created()),
            _ => a.name().cmp(b.name()),
        };

        sort.order
            .apply(primary)
            .then_with(|| a.name().cmp(b.name()))
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
            sorts.get(ResourceKind::Containers),
            sort_of(CONTAINER_STATUS, SortOrder::Ascending)
        );
        for resource in [
            ResourceKind::Images,
            ResourceKind::Volumes,
            ResourceKind::Networks,
        ] {
            assert_eq!(
                sorts.get(resource),
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
            TableSorts::default().get(ResourceKind::Containers),
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

            check(IMAGE_NAME, &|i| image_name(i).to_string());
            check(IMAGE_ID, &|i| i.id().to_string());
            check(IMAGE_SIZE, &|i| format!("{:020}", i.size()));
            check(IMAGE_USED_BY, &|i| format!("{:020}", i.containers()));
            check(IMAGE_AGE, &|i| format!("{:020}", i.created()));
        }
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
        let natural = |resource, column| {
            TableSorts::default()
                .pressed(resource, column)
                .get(resource)
        };

        // Text A to Z.
        assert_eq!(
            natural(ResourceKind::Containers, CONTAINER_NAME),
            sort_of(CONTAINER_NAME, SortOrder::Ascending)
        );
        assert_eq!(
            natural(ResourceKind::Networks, NETWORK_SUBNET),
            sort_of(NETWORK_SUBNET, SortOrder::Ascending)
        );
        // Ages: newest first.
        assert_eq!(
            natural(ResourceKind::Containers, CONTAINER_AGE),
            sort_of(CONTAINER_AGE, SortOrder::Descending)
        );
        assert_eq!(
            natural(ResourceKind::Volumes, VOLUME_AGE),
            sort_of(VOLUME_AGE, SortOrder::Descending)
        );
        // Sizes and counts: largest first.
        assert_eq!(
            natural(ResourceKind::Images, IMAGE_SIZE),
            sort_of(IMAGE_SIZE, SortOrder::Descending)
        );
        assert_eq!(
            natural(ResourceKind::Images, IMAGE_USED_BY),
            sort_of(IMAGE_USED_BY, SortOrder::Descending)
        );
        assert_eq!(
            natural(ResourceKind::Networks, NETWORK_CONTAINERS),
            sort_of(NETWORK_CONTAINERS, SortOrder::Descending)
        );
        // Status: the state order, running first.
        let by_name = TableSorts::default()
            .pressed(ResourceKind::Containers, CONTAINER_NAME)
            .pressed(ResourceKind::Containers, CONTAINER_STATUS);
        assert_eq!(
            by_name.get(ResourceKind::Containers),
            sort_of(CONTAINER_STATUS, SortOrder::Ascending)
        );
    }

    #[test]
    fn pressing_the_same_column_again_reverses_it() {
        let once =
            TableSorts::default().pressed(ResourceKind::Images, IMAGE_SIZE);

        let twice = once.pressed(ResourceKind::Images, IMAGE_SIZE);
        let thrice = twice.pressed(ResourceKind::Images, IMAGE_SIZE);

        assert_eq!(
            twice.get(ResourceKind::Images),
            sort_of(IMAGE_SIZE, SortOrder::Ascending)
        );
        assert_eq!(
            thrice.get(ResourceKind::Images),
            once.get(ResourceKind::Images)
        );
    }

    #[test]
    fn pressing_the_default_column_reverses_the_default() {
        let sorts = TableSorts::default()
            .pressed(ResourceKind::Containers, CONTAINER_STATUS);

        assert_eq!(
            sorts.get(ResourceKind::Containers),
            sort_of(CONTAINER_STATUS, SortOrder::Descending)
        );
    }

    #[test]
    fn each_table_keeps_its_own_sort() {
        let sorts =
            TableSorts::default().pressed(ResourceKind::Images, IMAGE_SIZE);

        assert_eq!(
            sorts.get(ResourceKind::Containers),
            TableSorts::default().get(ResourceKind::Containers)
        );
        assert_eq!(
            sorts.get(ResourceKind::Volumes),
            TableSorts::default().get(ResourceKind::Volumes)
        );
    }

    #[test]
    fn only_the_ports_column_cannot_be_sorted() {
        assert!(!is_sortable(ResourceKind::Containers, 4));
        assert!(is_sortable(ResourceKind::Containers, CONTAINER_AGE));
        assert!(is_sortable(ResourceKind::Images, IMAGE_AGE));
        assert!(!is_sortable(ResourceKind::Images, 9));

        let unchanged =
            TableSorts::default().pressed(ResourceKind::Containers, 4);
        assert_eq!(unchanged, TableSorts::default());
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
