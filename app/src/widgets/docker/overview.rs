//! The Overview tab of the inspector: the key facts of the selected row as
//! label and value pairs (spec Decision 12). Fields without a value are
//! left out.
//!
//! A value that names something OTTY can show is a link; `links.rs` holds
//! the rules. A value that names nothing in the engine's lists stays text.

use otty_docker::{
    Container, ContainerNetwork, Image, Mount, MountKind, Network, PortMapping,
    Volume,
};

use super::format::{
    NO_VALUE, date_time_text, port_text, short_id, shown_ports, size_text,
};
use super::links::{
    LinkTarget, compose_target, container_target, image_target,
    named_container_target, network_target, port_url, volume_target,
};
use crate::i18n::{self, Key};

/// Separator between the items of a list value.
const LIST_SEPARATOR: &str = ", ";

/// One line of the Overview: the translated label and the value.
pub(crate) type Field = (&'static str, Vec<Piece>);

/// A piece of an Overview value: text, or a link with where it leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Piece {
    Text(String),
    Link(String, LinkTarget),
}

impl Piece {
    /// The shown text.
    pub(crate) fn text(&self) -> &str {
        match self {
            Self::Text(text) | Self::Link(text, _) => text,
        }
    }
}

/// Image, ID, ports, mounts, networks, Compose project, command and
/// creation time of a container. `images`, `volumes` and `networks` are
/// the engine's lists the values link to.
pub(crate) fn container_fields(
    container: &Container,
    images: &[Image],
    volumes: &[Volume],
    networks: &[Network],
) -> Vec<Field> {
    let image = image_target(container.image_id(), images);
    let mounts = container
        .mounts()
        .iter()
        .map(|mount| mount_value(mount, volumes))
        .collect();
    let attached = container
        .networks()
        .iter()
        .map(|network| network_value(network, networks))
        .collect();
    // The container itself belongs to its project, so the tag always leaves
    // a row.
    let compose = container.compose_project().map(|project| {
        let target = LinkTarget::ComposeProject(project.to_string());
        vec![Piece::Link(project.to_string(), target)]
    });

    present(vec![
        (
            Key::DockerColumnImage,
            vec![piece(container.image(), image)],
        ),
        (Key::DockerColumnId, text(short_id(container.id()))),
        (Key::DockerColumnPorts, ports_value(container.ports())),
        (Key::DockerFieldMounts, joined(mounts)),
        (Key::DockerNetworks, joined(attached)),
        (Key::DockerFieldCompose, compose.unwrap_or_default()),
        (Key::DockerFieldCommand, text(container.command())),
        (Key::DockerFieldCreated, date_or_empty(container.created())),
    ])
}

/// ID, size, the containers that use it and creation time of an image.
/// `containers` is the engine's container list.
pub(crate) fn image_fields(
    image: &Image,
    containers: &[Container],
) -> Vec<Field> {
    let mut users: Vec<&Container> = containers
        .iter()
        .filter(|container| container.image_id() == image.id())
        .collect();
    users.sort_unstable_by_key(|container| container.name());
    let users = users
        .into_iter()
        .map(|container| vec![container_link(container)])
        .collect();

    present(vec![
        (Key::DockerColumnId, text(short_id(image.id()))),
        (Key::DockerColumnSize, text(&size_text(image.size()))),
        (Key::DockerColumnUsedBy, joined(users)),
        (
            Key::DockerFieldCreated,
            text(&date_time_text(image.created())),
        ),
    ])
}

/// Driver, mountpoint, Compose project, users and creation time of a
/// volume. `containers` is the engine's container list.
pub(crate) fn volume_fields(
    volume: &Volume,
    containers: &[Container],
) -> Vec<Field> {
    present(vec![
        (Key::DockerColumnDriver, text(volume.driver())),
        (Key::DockerFieldMountpoint, text(volume.mountpoint())),
        (
            Key::DockerFieldCompose,
            compose_value(volume.compose_project(), containers),
        ),
        (
            Key::DockerColumnUsedBy,
            container_names_value(volume.containers(), containers),
        ),
        (Key::DockerFieldCreated, date_or_empty(volume.created())),
    ])
}

/// Driver, scope, subnet, gateway and the connected containers of a
/// network. `containers` is the engine's container list.
pub(crate) fn network_fields(
    network: &Network,
    containers: &[Container],
) -> Vec<Field> {
    present(vec![
        (Key::DockerColumnDriver, text(network.driver())),
        (Key::DockerColumnScope, text(network.scope())),
        (Key::DockerColumnSubnet, text_or_empty(network.subnet())),
        (Key::DockerFieldGateway, text_or_empty(network.gateway())),
        (
            Key::DockerContainers,
            container_names_value(network.containers(), containers),
        ),
    ])
}

/// The fields that have a value, with their labels translated.
fn present(fields: Vec<(Key, Vec<Piece>)>) -> Vec<Field> {
    fields
        .into_iter()
        .filter(|(_, value)| {
            let shown: String = value.iter().map(Piece::text).collect();
            !shown.is_empty() && shown != NO_VALUE
        })
        .map(|(key, value)| (i18n::t(key), value))
        .collect()
}

/// A value of one text piece.
fn text(value: &str) -> Vec<Piece> {
    vec![Piece::Text(value.to_string())]
}

/// A link to `target`, or text when there is no target.
fn piece(value: &str, target: Option<LinkTarget>) -> Piece {
    match target {
        Some(target) => Piece::Link(value.to_string(), target),
        None => Piece::Text(value.to_string()),
    }
}

/// The values of a list, one after another with ", " between them.
fn joined(values: Vec<Vec<Piece>>) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for (index, value) in values.into_iter().enumerate() {
        if index > 0 {
            pieces.push(Piece::Text(LIST_SEPARATOR.to_string()));
        }
        pieces.extend(value);
    }

    pieces
}

/// The ports as `docker ps` shows them, each published TCP port a link to
/// its page. The containers table builds its Ports cell from the same
/// pieces.
pub(crate) fn ports_value(ports: &[PortMapping]) -> Vec<Piece> {
    let shown = shown_ports(ports)
        .into_iter()
        .map(|port| {
            let url =
                port_url(port.host_ip(), port.host_port(), port.protocol());
            vec![piece(&port_text(port), url.map(LinkTarget::Url))]
        })
        .collect();

    joined(shown)
}

/// A mount as "source → destination": the volume name (a link when the
/// volume is listed) or the host path, or only the destination for other
/// kinds (tmpfs).
fn mount_value(mount: &Mount, volumes: &[Volume]) -> Vec<Piece> {
    let destination = mount.destination();
    let source = match mount.kind() {
        MountKind::Volume => {
            let name = mount.name().unwrap_or(mount.source());
            let volume =
                mount.name().and_then(|name| volume_target(name, volumes));
            piece(name, volume)
        },
        MountKind::Bind => Piece::Text(mount.source().to_string()),
        _ => Piece::Text(String::new()),
    };

    if source.text().is_empty() {
        return text(destination);
    }
    vec![source, Piece::Text(format!(" → {destination}"))]
}

/// The network a container is attached to, a link when it is listed: by
/// id, or by name when the engine sent no id.
fn network_value(
    attached: &ContainerNetwork,
    networks: &[Network],
) -> Vec<Piece> {
    let target = network_target(attached.id(), attached.name(), networks);

    vec![piece(attached.name(), target)]
}

/// A Compose project, a link that adds its `compose` tag when some container
/// belongs to it.
fn compose_value(
    project: Option<&str>,
    containers: &[Container],
) -> Vec<Piece> {
    let Some(project) = project else {
        return Vec::new();
    };

    vec![piece(project, compose_target(project, containers))]
}

/// Container names, each a link to the container when it is listed.
fn container_names_value(
    names: &[String],
    containers: &[Container],
) -> Vec<Piece> {
    let values = names
        .iter()
        .map(|name| vec![piece(name, named_container_target(name, containers))])
        .collect();

    joined(values)
}

/// The name of `container` as a link to it.
fn container_link(container: &Container) -> Piece {
    piece(container.name(), Some(container_target(container)))
}

fn text_or_empty(value: Option<&str>) -> Vec<Piece> {
    value.map(text).unwrap_or_default()
}

fn date_or_empty(seconds: Option<i64>) -> Vec<Piece> {
    seconds
        .map(|seconds| text(&date_time_text(seconds)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, Resources, fixtures};

    use super::*;
    use crate::i18n::{self, Key};
    use crate::widgets::docker::types::ResourceKind;

    fn docker() -> Resources {
        fixtures::resources(EngineKind::Docker)
    }

    fn podman() -> Resources {
        fixtures::resources(EngineKind::Podman)
    }

    fn container<'a>(resources: &'a Resources, name: &str) -> &'a Container {
        resources
            .containers()
            .iter()
            .find(|container| container.name() == name)
            .unwrap_or_else(|| panic!("no container {name}"))
    }

    fn fields_of(resources: &Resources, name: &str) -> Vec<Field> {
        container_fields(
            container(resources, name),
            resources.images(),
            resources.volumes(),
            resources.networks(),
        )
    }

    /// The fields as label and shown text, for checks of the text only.
    fn texts(fields: &[Field]) -> Vec<(&'static str, String)> {
        fields
            .iter()
            .map(|(label, value)| {
                (*label, value.iter().map(Piece::text).collect())
            })
            .collect()
    }

    fn field(key: Key, value: &str) -> (&'static str, String) {
        (i18n::t(key), value.to_string())
    }

    /// The value of the field labeled by `key`.
    fn value_of(fields: &[Field], key: Key) -> Vec<Piece> {
        fields
            .iter()
            .find(|(label, _)| *label == i18n::t(key))
            .map(|(_, value)| value.clone())
            .unwrap_or_else(|| panic!("no field {}", i18n::t(key)))
    }

    fn link(text: &str, target: LinkTarget) -> Piece {
        Piece::Link(text.to_string(), target)
    }

    fn plain(text: &str) -> Piece {
        Piece::Text(text.to_string())
    }

    fn item(resource: ResourceKind, id: &str) -> LinkTarget {
        LinkTarget::Item(resource, id.to_string())
    }

    #[test]
    fn a_compose_container_shows_every_field_it_has() {
        let resources = docker();

        let fields = fields_of(&resources, "otty-fx-web-1");

        assert_eq!(
            texts(&fields),
            vec![
                field(Key::DockerColumnImage, "busybox:1.37"),
                field(Key::DockerColumnId, "850b6438a531"),
                field(Key::DockerColumnPorts, "18081:8080/tcp"),
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

        let fields = fields_of(&resources, "otty-fx-running");

        assert!(
            texts(&fields).contains(&field(
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

        let fields = fields_of(&resources, "otty-fx-exit0");

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
    fn a_container_links_its_image_port_network_and_project() {
        let resources = docker();
        let web = container(&resources, "otty-fx-web-1");
        let network_id = web.networks()[0].id().expect("network id");

        let fields = fields_of(&resources, "otty-fx-web-1");

        assert_eq!(
            value_of(&fields, Key::DockerColumnImage),
            vec![link(
                "busybox:1.37",
                item(ResourceKind::Images, web.image_id())
            )]
        );
        assert_eq!(
            value_of(&fields, Key::DockerColumnPorts),
            vec![link(
                "18081:8080/tcp",
                LinkTarget::Url(String::from("http://localhost:18081"))
            )]
        );
        assert_eq!(
            value_of(&fields, Key::DockerNetworks),
            vec![link(
                "otty-fx-net",
                item(ResourceKind::Networks, network_id)
            )]
        );
        assert_eq!(
            value_of(&fields, Key::DockerFieldCompose),
            vec![link(
                "otty-fx",
                LinkTarget::ComposeProject(String::from("otty-fx"))
            )]
        );
        assert_eq!(
            value_of(&fields, Key::DockerColumnId),
            vec![plain("850b6438a531")]
        );
    }

    #[test]
    fn a_named_volume_in_the_mounts_links_to_the_volume() {
        let resources = docker();

        let fields = fields_of(&resources, "otty-fx-running");

        assert_eq!(
            value_of(&fields, Key::DockerFieldMounts),
            vec![
                link(
                    "otty-fx-data",
                    item(ResourceKind::Volumes, "otty-fx-data")
                ),
                plain(" → /data"),
            ]
        );
    }

    #[test]
    fn a_network_without_id_links_by_its_name() {
        let resources = docker();
        let bridge = resources
            .networks()
            .iter()
            .find(|network| network.name() == "bridge")
            .expect("bridge network");

        let fields = fields_of(&resources, "otty-fx-created");

        assert_eq!(
            value_of(&fields, Key::DockerNetworks),
            vec![link("bridge", item(ResourceKind::Networks, bridge.id()))]
        );
    }

    #[test]
    fn values_that_name_nothing_listed_stay_text() {
        let resources = docker();
        let running = container(&resources, "otty-fx-running");

        let fields = container_fields(running, &[], &[], &[]);

        assert_eq!(
            value_of(&fields, Key::DockerColumnImage),
            vec![plain("busybox:1.37")]
        );
        assert_eq!(
            value_of(&fields, Key::DockerNetworks),
            vec![plain("otty-fx-net")]
        );
        assert_eq!(
            value_of(&fields, Key::DockerFieldMounts),
            vec![plain("otty-fx-data"), plain(" → /data")]
        );
    }

    #[test]
    fn a_podman_network_that_is_not_listed_stays_text() {
        let resources = podman();

        let fields = fields_of(&resources, "otty-fx-exit0");

        assert_eq!(
            value_of(&fields, Key::DockerNetworks),
            vec![plain("pasta")]
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
            texts(&fields),
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
    fn an_image_links_each_container_that_uses_it() {
        let resources = docker();
        let busybox = resources
            .images()
            .iter()
            .find(|image| image.tags() == ["busybox:1.37"])
            .expect("busybox");
        let created = container(&resources, "otty-fx-created");
        let exit0 = container(&resources, "otty-fx-exit0");

        let fields = image_fields(busybox, resources.containers());

        let used_by = value_of(&fields, Key::DockerColumnUsedBy);
        assert_eq!(
            used_by[..3],
            [
                link(
                    "otty-fx-created",
                    item(ResourceKind::Containers, created.id())
                ),
                plain(", "),
                link(
                    "otty-fx-exit0",
                    item(ResourceKind::Containers, exit0.id())
                ),
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

        let fields = volume_fields(data, resources.containers());

        assert_eq!(
            texts(&fields),
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
    fn a_volume_links_its_users_and_keeps_unlisted_names_as_text() {
        let resources = docker();
        let data = resources
            .volumes()
            .iter()
            .find(|volume| volume.name() == "otty-fx-data")
            .expect("named volume");
        let running = container(&resources, "otty-fx-running");

        let listed = volume_fields(data, resources.containers());
        let unlisted = volume_fields(data, &[]);

        assert_eq!(
            value_of(&listed, Key::DockerColumnUsedBy),
            vec![link(
                "otty-fx-running",
                item(ResourceKind::Containers, running.id())
            )]
        );
        assert_eq!(
            value_of(&unlisted, Key::DockerColumnUsedBy),
            vec![plain("otty-fx-running")]
        );
    }

    #[test]
    fn a_compose_project_links_to_the_filter_only_while_it_has_containers() {
        let resources = docker();

        let listed = compose_value(Some("otty-fx"), resources.containers());
        let gone = compose_value(Some("otty-old"), resources.containers());
        let none = compose_value(None, resources.containers());

        assert_eq!(
            listed,
            vec![link(
                "otty-fx",
                LinkTarget::ComposeProject(String::from("otty-fx"))
            )]
        );
        assert_eq!(gone, vec![plain("otty-old")]);
        assert!(none.is_empty());
    }

    #[test]
    fn a_network_shows_its_addresses_and_containers() {
        let resources = docker();
        let net = resources
            .networks()
            .iter()
            .find(|network| network.name() == "otty-fx-net")
            .expect("user network");

        let fields = network_fields(net, resources.containers());

        assert_eq!(
            texts(&fields),
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
    fn a_network_links_its_containers() {
        let resources = docker();
        let net = resources
            .networks()
            .iter()
            .find(|network| network.name() == "otty-fx-net")
            .expect("user network");
        let running = container(&resources, "otty-fx-running");
        let web = container(&resources, "otty-fx-web-1");

        let fields = network_fields(net, resources.containers());

        assert_eq!(
            value_of(&fields, Key::DockerContainers),
            vec![
                link(
                    "otty-fx-running",
                    item(ResourceKind::Containers, running.id())
                ),
                plain(", "),
                link("otty-fx-web-1", item(ResourceKind::Containers, web.id())),
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

        let fields = network_fields(none, resources.containers());

        assert_eq!(
            texts(&fields),
            vec![
                field(Key::DockerColumnDriver, "null"),
                field(Key::DockerColumnScope, "local"),
            ]
        );
    }
}
