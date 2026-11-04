//! How each resource is shown: table columns and cells (spec Decision 10)
//! and the status line of the inspector.

use otty_docker::{
    Container, ContainerState, Image, Network, PortMapping, Volume,
};
use otty_ui_kit::components::data_table::{
    CellPart, ColumnAlign, TableCell, TableColumn,
};
use otty_ui_kit::components::status_badge::StatusTone;

use super::format::{NO_VALUE, age_text, ports_text, short_id, size_text};
use super::links::LinkTarget;
use super::overview::{Piece, ports_value};
use super::sort::is_sortable;
use super::types::{ResourceKind, image_repository, image_tag};
use crate::i18n::{self, Key, Locale};

/// Position of the Ports column of the containers table; its cell holds
/// the port links.
pub(crate) const PORTS_COLUMN: usize = 4;

/// The table columns of `resource`, with their width shares and minimum
/// widths. Number columns are right-aligned. A minimum keeps the title and a
/// typical value readable (side padding included); on a narrow window the
/// table scrolls sideways instead of going below the minimums.
pub(crate) fn columns(resource: ResourceKind) -> Vec<TableColumn> {
    columns_in(i18n::current_locale(), resource)
}

/// The table columns of `resource`, with the titles in `locale`.
pub(super) fn columns_in(
    locale: Locale,
    resource: ResourceKind,
) -> Vec<TableColumn> {
    let column = |key: Key, share: u16, min: u16| {
        TableColumn::new(i18n::text_in(locale, key), share).min_width(min)
    };
    let number =
        |key: Key, min: u16| column(key, 1, min).align(ColumnAlign::End);

    let columns = match resource {
        ResourceKind::Containers => vec![
            column(Key::DockerColumnName, 3, 110),
            // The dot and "Exited (3)".
            column(Key::DockerColumnStatus, 2, 104),
            column(Key::DockerColumnImage, 3, 96),
            column(Key::DockerFieldCompose, 2, 80),
            // One published port such as `18080:8080/tcp`.
            column(Key::DockerColumnPorts, 2, 130),
            column(Key::DockerColumnAge, 1, 52),
        ],
        ResourceKind::Images => vec![
            column(Key::DockerColumnRepository, 4, 140),
            // `latest`, or `<none>` for an image without a tag.
            column(Key::DockerColumnTag, 2, 72),
            // A short ID of 12 characters.
            column(Key::DockerColumnId, 2, 118),
            number(Key::DockerColumnSize, 76),
            number(Key::DockerColumnUsedBy, 76),
            column(Key::DockerColumnAge, 1, 52),
        ],
        ResourceKind::Volumes => vec![
            column(Key::DockerColumnName, 4, 140),
            column(Key::DockerColumnDriver, 1, 76),
            column(Key::DockerColumnUsedBy, 3, 110),
            column(Key::DockerColumnAge, 1, 52),
        ],
        ResourceKind::Networks => vec![
            column(Key::DockerColumnName, 3, 120),
            column(Key::DockerColumnDriver, 1, 76),
            column(Key::DockerColumnScope, 1, 72),
            // A subnet such as `172.17.0.0/16`.
            column(Key::DockerColumnSubnet, 2, 124),
            number(Key::DockerContainers, 100),
            column(Key::DockerColumnAge, 1, 52),
        ],
    };

    columns
        .into_iter()
        .enumerate()
        .map(|(index, column)| column.sortable(is_sortable(resource, index)))
        .collect()
}

/// The cells of a container row: name, status badge, image, Compose project
/// link (blank without a project), ports, age.
pub(crate) fn container_cells(
    container: &Container,
    now: i64,
) -> Vec<TableCell> {
    let (word, tone) =
        container_badge(container.state(), container.exit_code());

    let project = match container.compose_project() {
        Some(project) => TableCell::link(project),
        None => TableCell::muted(""),
    };

    vec![
        TableCell::mono(container.name()),
        TableCell::badge(word, tone),
        TableCell::muted_mono(container.image()),
        project,
        ports_cell(container.ports()),
        TableCell::muted(age_text(now, container.created())),
    ]
}

/// The Ports cell: the ports as `docker ps` shows them, each published TCP
/// port a link (the same rule as the Overview). Without any link it is
/// plain muted text.
pub(crate) fn ports_cell(ports: &[PortMapping]) -> TableCell {
    let pieces = ports_value(ports);
    if !pieces.iter().any(|piece| matches!(piece, Piece::Link(..))) {
        return TableCell::muted_mono(ports_text(ports));
    }

    let mut parts: Vec<CellPart> = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Link(text, _) => parts.push(CellPart::link(text)),
            // Neighboring texts (a separator and an unlinked port) are one
            // part.
            Piece::Text(text) => match parts.last_mut() {
                Some(CellPart::Text(previous)) => previous.push_str(&text),
                _ => parts.push(CellPart::text(text)),
            },
        }
    }

    TableCell::parts(parts)
}

/// The pages the links of [`ports_cell`] open, in the order of the links.
pub(crate) fn port_urls(ports: &[PortMapping]) -> Vec<String> {
    ports_value(ports)
        .into_iter()
        .filter_map(|piece| match piece {
            Piece::Link(_, LinkTarget::Url(url)) => Some(url),
            _ => None,
        })
        .collect()
}

/// The cells of an image row: repository, tag, short ID, size, how many
/// containers use it, age.
pub(crate) fn image_cells(image: &Image, now: i64) -> Vec<TableCell> {
    vec![
        TableCell::mono(image_repository(image)),
        TableCell::muted_mono(image_tag(image)),
        TableCell::muted_mono(short_id(image.id())),
        TableCell::number(size_text(image.size())),
        TableCell::number(format!("{}", image.containers())),
        TableCell::muted(age_text(now, Some(image.created()))),
    ]
}

/// The cells of a volume row: name, driver, the containers that use it,
/// age.
pub(crate) fn volume_cells(volume: &Volume, now: i64) -> Vec<TableCell> {
    vec![
        TableCell::mono(volume.name()),
        TableCell::muted_mono(volume.driver()),
        TableCell::muted_mono(names_or_no_value(volume.containers())),
        TableCell::muted(age_text(now, volume.created())),
    ]
}

/// The cells of a network row: name, driver, scope, subnet, how many
/// containers are connected, age.
pub(crate) fn network_cells(network: &Network, now: i64) -> Vec<TableCell> {
    vec![
        TableCell::mono(network.name()),
        TableCell::muted_mono(network.driver()),
        TableCell::muted(network.scope()),
        TableCell::muted_mono(network.subnet().unwrap_or(NO_VALUE)),
        TableCell::number(format!("{}", network.containers().len())),
        TableCell::muted(age_text(now, network.created())),
    ]
}

/// The badge word and tone of a container state: running → ok, paused and
/// restarting → warn, exited with a non-zero code and dead → error,
/// created, exited with 0 and the rest → neutral.
pub(crate) fn container_badge(
    state: ContainerState,
    exit_code: Option<i64>,
) -> (String, StatusTone) {
    let word = |key: Key| i18n::t(key).to_string();

    match (state, exit_code) {
        (ContainerState::Running, _) => {
            (word(Key::DockerStateRunning), StatusTone::Ok)
        },
        (ContainerState::Paused, _) => {
            (word(Key::DockerStatePaused), StatusTone::Warn)
        },
        (ContainerState::Restarting, _) => {
            (word(Key::DockerStateRestarting), StatusTone::Warn)
        },
        (ContainerState::Exited, Some(0)) => {
            (i18n::docker_exited(0), StatusTone::Neutral)
        },
        (ContainerState::Exited, Some(code)) => {
            (i18n::docker_exited(code), StatusTone::Error)
        },
        (ContainerState::Exited, None) => {
            (word(Key::DockerStateExited), StatusTone::Neutral)
        },
        (ContainerState::Dead, _) => {
            (word(Key::DockerStateDead), StatusTone::Error)
        },
        (ContainerState::Created, _) => {
            (word(Key::DockerStateCreated), StatusTone::Neutral)
        },
        (ContainerState::Removing, _) => {
            (word(Key::DockerStateRemoving), StatusTone::Neutral)
        },
        (ContainerState::Stopping, _) => {
            (word(Key::DockerStateStopping), StatusTone::Neutral)
        },
        _ => (word(Key::DockerStateUnknown), StatusTone::Neutral),
    }
}

/// The inspector status line of a container: the engine's status text
/// ("Up 2 hours", "Exited (1) 2 hours ago") in the badge tone.
pub(crate) fn container_status(container: &Container) -> (String, StatusTone) {
    let (_, tone) = container_badge(container.state(), container.exit_code());

    (container.status().to_string(), tone)
}

/// The inspector status line of an image: how many containers use it, or
/// that it is dangling.
pub(crate) fn image_status(image: &Image) -> (String, StatusTone) {
    match image.containers() {
        0 if image.is_dangling() => (
            i18n::t(Key::DockerImageDangling).to_string(),
            StatusTone::Warn,
        ),
        0 => (i18n::t(Key::DockerNotUsed).to_string(), StatusTone::Neutral),
        1 => (
            i18n::t(Key::DockerUsedByOneContainer).to_string(),
            StatusTone::Neutral,
        ),
        count => (
            i18n::with_count(Key::TplDockerUsedByContainers, count as u64),
            StatusTone::Neutral,
        ),
    }
}

/// The inspector status line of a volume: the containers that use it.
pub(crate) fn volume_status(volume: &Volume) -> (String, StatusTone) {
    if volume.containers().is_empty() {
        return (i18n::t(Key::DockerNotUsed).to_string(), StatusTone::Warn);
    }

    let names = volume.containers().join(", ");
    (i18n::docker_used_by(&names), StatusTone::Neutral)
}

/// The inspector status line of a network: built in, or how many
/// containers are connected.
pub(crate) fn network_status(network: &Network) -> (String, StatusTone) {
    if network.is_built_in() {
        return (
            i18n::t(Key::DockerBuiltInNetwork).to_string(),
            StatusTone::Neutral,
        );
    }

    let text = match network.containers().len() {
        1 => i18n::t(Key::DockerOneContainerConnected).to_string(),
        count => {
            i18n::with_count(Key::TplDockerContainersConnected, count as u64)
        },
    };
    (text, StatusTone::Neutral)
}

/// Container names joined with commas, or [`NO_VALUE`] for none.
fn names_or_no_value(names: &[String]) -> String {
    if names.is_empty() {
        return NO_VALUE.to_string();
    }

    names.join(", ")
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;

    /// A moment shortly after the fixtures were captured.
    const NOW: i64 = 1_793_557_960;

    fn titles(resource: ResourceKind) -> Vec<TableColumn> {
        columns(resource)
    }

    #[test]
    fn containers_have_name_status_image_compose_ports_and_age() {
        assert_eq!(
            titles(ResourceKind::Containers),
            vec![
                TableColumn::new(i18n::t(Key::DockerColumnName), 3)
                    .min_width(110)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnStatus), 2)
                    .min_width(104)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnImage), 3)
                    .min_width(96)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerFieldCompose), 2)
                    .min_width(80)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnPorts), 2)
                    .min_width(130)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnAge), 1)
                    .min_width(52)
                    .sortable(true),
            ]
        );
    }

    #[test]
    fn images_have_repository_tag_id_size_used_by_and_age_with_numbers_on_the_right()
     {
        assert_eq!(
            titles(ResourceKind::Images),
            vec![
                TableColumn::new(i18n::t(Key::DockerColumnRepository), 4)
                    .min_width(140)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnTag), 2)
                    .min_width(72)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnId), 2)
                    .min_width(118)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnSize), 1)
                    .min_width(76)
                    .sortable(true)
                    .align(ColumnAlign::End),
                TableColumn::new(i18n::t(Key::DockerColumnUsedBy), 1)
                    .min_width(76)
                    .sortable(true)
                    .align(ColumnAlign::End),
                TableColumn::new(i18n::t(Key::DockerColumnAge), 1)
                    .min_width(52)
                    .sortable(true),
            ]
        );
    }

    #[test]
    fn volumes_have_name_driver_used_by_and_age() {
        assert_eq!(
            titles(ResourceKind::Volumes),
            vec![
                TableColumn::new(i18n::t(Key::DockerColumnName), 4)
                    .min_width(140)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnDriver), 1)
                    .min_width(76)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnUsedBy), 3)
                    .min_width(110)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnAge), 1)
                    .min_width(52)
                    .sortable(true),
            ]
        );
    }

    #[test]
    fn networks_have_name_driver_scope_subnet_containers_and_age() {
        assert_eq!(
            titles(ResourceKind::Networks),
            vec![
                TableColumn::new(i18n::t(Key::DockerColumnName), 3)
                    .min_width(120)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnDriver), 1)
                    .min_width(76)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnScope), 1)
                    .min_width(72)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerColumnSubnet), 2)
                    .min_width(124)
                    .sortable(true),
                TableColumn::new(i18n::t(Key::DockerContainers), 1)
                    .min_width(100)
                    .sortable(true)
                    .align(ColumnAlign::End),
                TableColumn::new(i18n::t(Key::DockerColumnAge), 1)
                    .min_width(52)
                    .sortable(true),
            ]
        );
    }

    #[test]
    fn badge_tones_follow_the_container_state() {
        let tone = |state, code| container_badge(state, code).1;

        assert_eq!(tone(ContainerState::Running, None), StatusTone::Ok);
        assert_eq!(tone(ContainerState::Paused, None), StatusTone::Warn);
        assert_eq!(tone(ContainerState::Restarting, None), StatusTone::Warn);
        assert_eq!(tone(ContainerState::Exited, Some(3)), StatusTone::Error);
        assert_eq!(tone(ContainerState::Dead, None), StatusTone::Error);
        assert_eq!(tone(ContainerState::Exited, Some(0)), StatusTone::Neutral);
        assert_eq!(tone(ContainerState::Exited, None), StatusTone::Neutral);
        assert_eq!(tone(ContainerState::Created, None), StatusTone::Neutral);
        assert_eq!(tone(ContainerState::Removing, None), StatusTone::Neutral);
        assert_eq!(tone(ContainerState::Unknown, None), StatusTone::Neutral);
    }

    #[test]
    fn badge_words_name_the_state_and_the_exit_code() {
        let word = |state, code| container_badge(state, code).0;

        assert_eq!(
            word(ContainerState::Running, None),
            i18n::t(Key::DockerStateRunning)
        );
        assert_eq!(
            word(ContainerState::Exited, Some(3)),
            i18n::docker_exited(3)
        );
        assert_eq!(
            word(ContainerState::Exited, None),
            i18n::t(Key::DockerStateExited)
        );
    }

    #[test]
    fn a_container_row_shows_name_badge_image_ports_and_age() {
        let resources = fixtures::resources(EngineKind::Docker);
        let running = &resources.containers()[5];

        assert_eq!(
            container_cells(running, NOW),
            vec![
                TableCell::mono("otty-fx-running"),
                TableCell::badge(
                    i18n::t(Key::DockerStateRunning),
                    StatusTone::Ok
                ),
                TableCell::muted_mono("busybox:1.37"),
                TableCell::muted(""),
                TableCell::parts(vec![CellPart::link("18080:80/tcp")]),
                TableCell::muted(age_text(NOW, running.created())),
            ]
        );
    }

    /// The ports of a container as `(host address, host port, container
    /// port, protocol)`.
    type Ports<'a> = &'a [(Option<&'a str>, Option<u16>, u16, &'a str)];

    fn cell_of(ports: Ports) -> TableCell {
        let container = fixtures::container_with_ports(ports);

        ports_cell(container.ports())
    }

    fn urls_of(ports: Ports) -> Vec<String> {
        let container = fixtures::container_with_ports(ports);

        port_urls(container.ports())
    }

    #[test]
    fn a_published_tcp_port_is_a_link() {
        let ports: Ports = &[(Some("0.0.0.0"), Some(8080), 80, "tcp")];

        assert_eq!(
            cell_of(ports),
            TableCell::parts(vec![CellPart::link("8080:80/tcp")])
        );
        assert_eq!(urls_of(ports), vec!["http://localhost:8080"]);
    }

    #[test]
    fn a_port_on_both_address_families_is_one_link() {
        let ports: Ports = &[
            (Some("0.0.0.0"), Some(8080), 80, "tcp"),
            (Some("::"), Some(8080), 80, "tcp"),
        ];

        assert_eq!(
            cell_of(ports),
            TableCell::parts(vec![CellPart::link("8080:80/tcp")])
        );
        assert_eq!(urls_of(ports), vec!["http://localhost:8080"]);
    }

    #[test]
    fn udp_and_exposed_only_ports_stay_text_between_the_links() {
        let ports: Ports = &[
            (Some("0.0.0.0"), Some(8080), 80, "tcp"),
            (Some("0.0.0.0"), Some(5353), 53, "udp"),
            (None, None, 443, "tcp"),
            (Some("127.0.0.1"), Some(5432), 5432, "tcp"),
        ];

        assert_eq!(
            cell_of(ports),
            TableCell::parts(vec![
                CellPart::link("8080:80/tcp"),
                CellPart::text(", 5353:53/udp, 443/tcp, "),
                CellPart::link("5432:5432/tcp"),
            ])
        );
        assert_eq!(
            urls_of(ports),
            vec!["http://localhost:8080", "http://localhost:5432"]
        );
    }

    #[test]
    fn the_text_the_ports_column_sorts_by_is_the_text_of_the_linked_cell() {
        let ports: Ports = &[
            (Some("0.0.0.0"), Some(8080), 80, "tcp"),
            (Some("::"), Some(8080), 80, "tcp"),
            (Some("0.0.0.0"), Some(5353), 53, "udp"),
            (None, None, 443, "tcp"),
        ];
        let container = fixtures::container_with_ports(ports);

        let cell: String = ports_value(container.ports())
            .iter()
            .map(Piece::text)
            .collect();

        assert_eq!(cell, "8080:80/tcp, 5353:53/udp, 443/tcp");
        assert_eq!(ports_text(container.ports()), cell);
    }

    #[test]
    fn a_port_bound_to_one_address_links_to_that_address() {
        let ports: Ports = &[(Some("192.168.1.20"), Some(8080), 80, "tcp")];

        assert_eq!(urls_of(ports), vec!["http://192.168.1.20:8080"]);
    }

    #[test]
    fn ports_without_a_link_are_one_muted_text() {
        let ports: Ports = &[
            (Some("0.0.0.0"), Some(5353), 53, "udp"),
            (None, None, 443, "tcp"),
        ];

        assert_eq!(
            cell_of(ports),
            TableCell::muted_mono("5353:53/udp, 443/tcp")
        );
        assert!(urls_of(ports).is_empty());
    }

    #[test]
    fn a_container_without_ports_shows_no_value_and_no_links() {
        assert_eq!(cell_of(&[]), TableCell::muted_mono(NO_VALUE));
        assert!(urls_of(&[]).is_empty());
    }

    #[test]
    fn a_compose_container_shows_its_project_as_a_link() {
        let resources = fixtures::resources(EngineKind::Docker);
        let web = resources
            .containers()
            .iter()
            .find(|container| container.compose_project().is_some())
            .expect("fixture has a compose container");

        assert_eq!(container_cells(web, NOW)[3], TableCell::link("otty-fx"));
    }

    #[test]
    fn an_exited_container_with_an_error_code_has_an_error_badge() {
        let resources = fixtures::resources(EngineKind::Docker);
        let failed = &resources.containers()[2];

        assert_eq!(failed.name(), "otty-fx-exit3");
        assert_eq!(
            container_cells(failed, NOW)[1],
            TableCell::badge(i18n::docker_exited(3), StatusTone::Error)
        );
    }

    #[test]
    fn an_image_row_shows_repository_tag_short_id_size_users_and_age() {
        let resources = fixtures::resources(EngineKind::Docker);
        let busybox = &resources.images()[2];

        assert_eq!(
            image_cells(busybox, NOW),
            vec![
                TableCell::mono("busybox"),
                TableCell::muted_mono("1.37"),
                TableCell::muted_mono(short_id(busybox.id())),
                TableCell::number("6.69 MB"),
                TableCell::number(busybox.containers().to_string()),
                TableCell::muted(age_text(NOW, Some(busybox.created()))),
            ]
        );
    }

    #[test]
    fn a_row_without_a_name_shows_none_in_both_columns() {
        let dangling = fixtures::image_rows("sha256:app", &[], &[], &[]);

        let cells = image_cells(&dangling[0], NOW);

        assert_eq!(cells[0], TableCell::mono("<none>"));
        assert_eq!(cells[1], TableCell::muted_mono("<none>"));
    }

    #[test]
    fn a_row_known_only_by_its_digest_shows_the_repository_and_no_tag() {
        let row = fixtures::image_rows(
            "sha256:app",
            &[],
            &["reg/app@sha256:aa"],
            &[],
        );

        let cells = image_cells(&row[0], NOW);

        assert_eq!(cells[0], TableCell::mono("reg/app"));
        assert_eq!(cells[1], TableCell::muted_mono("<none>"));
    }

    #[test]
    fn the_rows_of_one_image_show_the_same_id_size_and_users() {
        let user =
            fixtures::container_of_image("web", "exited", "a:1", "sha256:app");
        let rows =
            fixtures::image_rows("sha256:app", &["a:1", "a:2"], &[], &[user]);

        let first = image_cells(&rows[0], NOW);
        let second = image_cells(&rows[1], NOW);

        assert_eq!(first[1], TableCell::muted_mono("1"));
        assert_eq!(second[1], TableCell::muted_mono("2"));
        assert_eq!(first[2..], second[2..]);
        assert_eq!(first[4], TableCell::number("1"));
    }

    #[test]
    fn a_volume_row_lists_its_users_or_no_value() {
        let resources = fixtures::resources(EngineKind::Docker);
        let anonymous = &resources.volumes()[0];
        let named = &resources.volumes()[1];

        assert_eq!(named.name(), "otty-fx-data");
        assert_eq!(
            volume_cells(named, NOW),
            vec![
                TableCell::mono("otty-fx-data"),
                TableCell::muted_mono("local"),
                TableCell::muted_mono(named.containers().join(", ")),
                TableCell::muted(age_text(NOW, named.created())),
            ]
        );
        assert!(!named.containers().is_empty());
        if anonymous.containers().is_empty() {
            assert_eq!(
                volume_cells(anonymous, NOW)[2],
                TableCell::muted_mono(NO_VALUE)
            );
        }
    }

    #[test]
    fn a_network_row_shows_driver_scope_subnet_and_container_count() {
        let resources = fixtures::resources(EngineKind::Docker);
        let bridge = &resources.networks()[0];
        let none = &resources.networks()[1];

        assert_eq!(
            network_cells(bridge, NOW),
            vec![
                TableCell::mono("bridge"),
                TableCell::muted_mono("bridge"),
                TableCell::muted("local"),
                TableCell::muted_mono("172.17.0.0/16"),
                TableCell::number(bridge.containers().len().to_string()),
                TableCell::muted(age_text(NOW, bridge.created())),
            ]
        );
        assert_eq!(
            network_cells(none, NOW)[3],
            TableCell::muted_mono(NO_VALUE)
        );
    }

    #[test]
    fn the_container_status_line_is_the_engine_text_in_the_badge_tone() {
        let resources = fixtures::resources(EngineKind::Docker);
        let paused = &resources.containers()[4];

        assert_eq!(
            container_status(paused),
            (String::from("Up 37 seconds (Paused)"), StatusTone::Warn)
        );
    }

    #[test]
    fn image_status_says_who_uses_it_or_that_it_dangles() {
        let resources = fixtures::resources(EngineKind::Docker);
        let dangling = &resources.images()[0];
        let busybox = &resources.images()[2];

        assert!(dangling.is_dangling());
        assert_eq!(
            image_status(dangling),
            (
                i18n::t(Key::DockerImageDangling).to_string(),
                StatusTone::Warn
            )
        );
        assert_eq!(
            image_status(busybox),
            (
                i18n::with_count(
                    Key::TplDockerUsedByContainers,
                    busybox.containers() as u64
                ),
                StatusTone::Neutral
            )
        );
    }

    #[test]
    fn volume_status_names_its_users() {
        let resources = fixtures::resources(EngineKind::Docker);
        let named = &resources.volumes()[1];

        assert_eq!(
            volume_status(named),
            (
                i18n::docker_used_by(&named.containers().join(", ")),
                StatusTone::Neutral
            )
        );
    }

    #[test]
    fn network_status_says_built_in_or_how_many_are_connected() {
        let resources = fixtures::resources(EngineKind::Docker);
        let bridge = &resources.networks()[0];
        let user = &resources.networks()[2];

        assert_eq!(
            network_status(bridge),
            (
                i18n::t(Key::DockerBuiltInNetwork).to_string(),
                StatusTone::Neutral
            )
        );
        assert_eq!(user.name(), "otty-fx-net");
        assert_eq!(
            network_status(user).0,
            i18n::with_count(
                Key::TplDockerContainersConnected,
                user.containers().len() as u64
            )
        );
    }
}
