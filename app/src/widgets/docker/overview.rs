//! The Overview tab of the inspector: the key facts of the selected row as
//! label and value pairs (spec Decision 12). Fields without a value are
//! left out.

use otty_docker::{
    Container, ContainerNetwork, Image, Mount, MountKind, Network, Volume,
};

use super::format::{
    NO_VALUE, date_time_text, ports_text, short_id, size_text,
};
use crate::i18n::{self, Key};

/// One line of the Overview: the translated label and the value.
pub(crate) type Field = (&'static str, String);

/// Image, ID, ports, mounts, networks, Compose project, command and
/// creation time of a container.
pub(crate) fn container_fields(container: &Container) -> Vec<Field> {
    let mounts: Vec<String> =
        container.mounts().iter().map(mount_text).collect();
    let networks: Vec<&str> = container
        .networks()
        .iter()
        .map(ContainerNetwork::name)
        .collect();

    present(vec![
        (Key::DockerColumnImage, container.image().to_string()),
        (Key::DockerColumnId, short_id(container.id()).to_string()),
        (Key::DockerColumnPorts, ports_text(container.ports())),
        (Key::DockerFieldMounts, mounts.join(", ")),
        (Key::DockerNetworks, networks.join(", ")),
        (
            Key::DockerFieldCompose,
            text_or_empty(container.compose_project()),
        ),
        (Key::DockerFieldCommand, container.command().to_string()),
        (Key::DockerFieldCreated, date_or_empty(container.created())),
    ])
}

/// ID, size, the containers that use it and creation time of an image.
/// `containers` is the engine's container list.
pub(crate) fn image_fields(
    image: &Image,
    containers: &[Container],
) -> Vec<Field> {
    let mut users: Vec<&str> = containers
        .iter()
        .filter(|container| container.image_id() == image.id())
        .map(Container::name)
        .collect();
    users.sort_unstable();

    present(vec![
        (Key::DockerColumnId, short_id(image.id()).to_string()),
        (Key::DockerColumnSize, size_text(image.size())),
        (Key::DockerColumnUsedBy, users.join(", ")),
        (Key::DockerFieldCreated, date_time_text(image.created())),
    ])
}

/// Driver, mountpoint, Compose project, users and creation time of a
/// volume.
pub(crate) fn volume_fields(volume: &Volume) -> Vec<Field> {
    present(vec![
        (Key::DockerColumnDriver, volume.driver().to_string()),
        (Key::DockerFieldMountpoint, volume.mountpoint().to_string()),
        (
            Key::DockerFieldCompose,
            text_or_empty(volume.compose_project()),
        ),
        (Key::DockerColumnUsedBy, volume.containers().join(", ")),
        (Key::DockerFieldCreated, date_or_empty(volume.created())),
    ])
}

/// Driver, scope, subnet, gateway and the connected containers of a
/// network.
pub(crate) fn network_fields(network: &Network) -> Vec<Field> {
    present(vec![
        (Key::DockerColumnDriver, network.driver().to_string()),
        (Key::DockerColumnScope, network.scope().to_string()),
        (Key::DockerColumnSubnet, text_or_empty(network.subnet())),
        (Key::DockerFieldGateway, text_or_empty(network.gateway())),
        (Key::DockerContainers, network.containers().join(", ")),
    ])
}

/// The fields that have a value, with their labels translated.
fn present(fields: Vec<(Key, String)>) -> Vec<Field> {
    fields
        .into_iter()
        .filter(|(_, value)| !value.is_empty() && value != NO_VALUE)
        .map(|(key, value)| (i18n::t(key), value))
        .collect()
}

/// A mount as "source → destination": the volume name or the host path,
/// or only the destination for other kinds (tmpfs).
fn mount_text(mount: &Mount) -> String {
    let source = match mount.kind() {
        MountKind::Volume => mount.name().unwrap_or(mount.source()),
        MountKind::Bind => mount.source(),
        _ => "",
    };
    let destination = mount.destination();

    if source.is_empty() {
        return destination.to_string();
    }
    format!("{source} → {destination}")
}

fn text_or_empty(value: Option<&str>) -> String {
    value.unwrap_or_default().to_string()
}

fn date_or_empty(seconds: Option<i64>) -> String {
    seconds.map(date_time_text).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, Resources, fixtures};

    use super::*;
    use crate::i18n::{self, Key};

    fn docker() -> Resources {
        fixtures::resources(EngineKind::Docker)
    }

    fn container<'a>(resources: &'a Resources, name: &str) -> &'a Container {
        resources
            .containers()
            .iter()
            .find(|container| container.name() == name)
            .unwrap_or_else(|| panic!("no container {name}"))
    }

    fn field(key: Key, value: &str) -> Field {
        (i18n::t(key), value.to_string())
    }

    #[test]
    fn a_compose_container_shows_every_field_it_has() {
        let resources = docker();

        let fields = container_fields(container(&resources, "otty-fx-web-1"));

        assert_eq!(
            fields,
            vec![
                field(Key::DockerColumnImage, "busybox:1.37"),
                field(Key::DockerColumnId, "850b6438a531"),
                field(Key::DockerColumnPorts, "18081→8080/tcp"),
                field(Key::DockerNetworks, "otty-fx-net"),
                field(Key::DockerFieldCompose, "otty-fx"),
                field(Key::DockerFieldCommand, "sleep 3600"),
                field(Key::DockerFieldCreated, "2026-11-01 18:30 UTC"),
            ]
        );
    }

    #[test]
    fn a_container_with_a_volume_shows_its_mounts() {
        let resources = docker();

        let fields = container_fields(container(&resources, "otty-fx-running"));

        assert!(
            fields.contains(&field(
                Key::DockerFieldMounts,
                "otty-fx-data → /data"
            )),
            "{fields:?}"
        );
        assert!(
            !fields
                .iter()
                .any(|(label, _)| *label == i18n::t(Key::DockerFieldCompose))
        );
    }

    #[test]
    fn a_container_without_ports_mounts_or_project_leaves_them_out() {
        let resources = docker();

        let fields = container_fields(container(&resources, "otty-fx-exit0"));

        let labels: Vec<&str> =
            fields.iter().map(|(label, _)| *label).collect();
        assert_eq!(
            labels,
            vec![
                i18n::t(Key::DockerColumnImage),
                i18n::t(Key::DockerColumnId),
                i18n::t(Key::DockerNetworks),
                i18n::t(Key::DockerFieldCommand),
                i18n::t(Key::DockerFieldCreated),
            ]
        );
    }

    #[test]
    fn an_image_shows_its_size_and_the_containers_that_use_it() {
        let resources = docker();
        let busybox = resources
            .images()
            .iter()
            .find(|image| image.tags() == ["busybox:1.37"])
            .expect("busybox");

        let fields = image_fields(busybox, resources.containers());

        assert_eq!(
            fields,
            vec![
                field(Key::DockerColumnId, "a7dbc319b143"),
                field(Key::DockerColumnSize, "6.69 MB"),
                field(
                    Key::DockerColumnUsedBy,
                    "otty-fx-created, otty-fx-exit0, otty-fx-exit3, \
                     otty-fx-paused, otty-fx-running, otty-fx-web-1"
                ),
                field(Key::DockerFieldCreated, "2024-09-26 21:31 UTC"),
            ]
        );
    }

    #[test]
    fn an_unused_image_leaves_out_used_by() {
        let resources = docker();
        let dangling = resources
            .images()
            .iter()
            .find(|image| image.is_dangling())
            .expect("dangling image");

        let fields = image_fields(dangling, resources.containers());

        let labels: Vec<&str> =
            fields.iter().map(|(label, _)| *label).collect();
        assert_eq!(
            labels,
            vec![
                i18n::t(Key::DockerColumnId),
                i18n::t(Key::DockerColumnSize),
                i18n::t(Key::DockerFieldCreated),
            ]
        );
    }

    #[test]
    fn a_volume_shows_its_driver_mountpoint_and_users() {
        let resources = docker();
        let data = resources
            .volumes()
            .iter()
            .find(|volume| volume.name() == "otty-fx-data")
            .expect("named volume");

        let fields = volume_fields(data);

        assert_eq!(
            fields,
            vec![
                field(Key::DockerColumnDriver, "local"),
                field(
                    Key::DockerFieldMountpoint,
                    "/var/lib/docker/volumes/otty-fx-data/_data"
                ),
                field(Key::DockerColumnUsedBy, "otty-fx-running"),
                field(Key::DockerFieldCreated, "2026-11-01 18:30 UTC"),
            ]
        );
    }

    #[test]
    fn a_network_shows_its_addresses_and_containers() {
        let resources = docker();
        let net = resources
            .networks()
            .iter()
            .find(|network| network.name() == "otty-fx-net")
            .expect("user network");

        let fields = network_fields(net);

        assert_eq!(
            fields,
            vec![
                field(Key::DockerColumnDriver, "bridge"),
                field(Key::DockerColumnScope, "local"),
                field(Key::DockerColumnSubnet, "192.168.160.0/20"),
                field(Key::DockerFieldGateway, "192.168.160.1"),
                field(Key::DockerContainers, "otty-fx-running, otty-fx-web-1"),
            ]
        );
    }

    #[test]
    fn a_network_without_addresses_or_containers_leaves_them_out() {
        let resources = docker();
        let none = resources
            .networks()
            .iter()
            .find(|network| network.name() == "none")
            .expect("none network");

        let fields = network_fields(none);

        assert_eq!(
            fields,
            vec![
                field(Key::DockerColumnDriver, "null"),
                field(Key::DockerColumnScope, "local"),
            ]
        );
    }
}
