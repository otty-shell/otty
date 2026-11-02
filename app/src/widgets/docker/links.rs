//! Where a value that names something OTTY can show leads (spec Decision
//! 20): a published TCP port opens in the browser, an image, network, volume
//! or container opens its list with that item selected, and a Compose project
//! opens the compose filter. A value that names nothing in the engine's lists
//! has no target and stays text. The Overview and the Inspect tree share
//! these rules.

use otty_docker::{Container, Image, Network, Volume};

use super::types::ResourceKind;

/// Host addresses that mean "every address of this computer" or its
/// loopback: a port published on them opens at `localhost`.
const LOCAL_HOSTS: [&str; 3] = ["0.0.0.0", "::", "127.0.0.1"];
/// The protocol a browser can open.
const BROWSER_PROTOCOL: &str = "tcp";

/// Where a link leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkTarget {
    /// A page opened in the browser: a published port.
    Url(String),
    /// The item with this id (container, image or network id, or volume
    /// name) in this list of the open engine.
    Item(ResourceKind, String),
    /// The containers of this Compose project.
    ComposeProject(String),
}

/// The page a published port opens in the browser: `localhost` for a port
/// published on every address (`0.0.0.0`, `::`), on `127.0.0.1` or on no
/// stated address, else the address it is bound to. `None` for a port
/// that is only exposed or is not TCP.
pub(crate) fn port_url(
    host_ip: Option<&str>,
    host_port: Option<u16>,
    protocol: &str,
) -> Option<String> {
    let port = host_port?;
    if protocol != BROWSER_PROTOCOL {
        return None;
    }

    let host = match host_ip {
        Some(ip) if !ip.is_empty() && !LOCAL_HOSTS.contains(&ip) => {
            // An IPv6 address goes in brackets in a URL.
            if ip.contains(':') {
                format!("[{ip}]")
            } else {
                ip.to_string()
            }
        },
        _ => String::from("localhost"),
    };

    Some(format!("http://{host}:{port}"))
}

/// The listed image that `reference`, an image id or a tag, names.
pub(crate) fn image_target(
    reference: &str,
    images: &[Image],
) -> Option<LinkTarget> {
    images
        .iter()
        .find(|image| {
            image.id() == reference
                || image.tags().iter().any(|tag| tag == reference)
        })
        .map(|image| LinkTarget::Item(ResourceKind::Images, image.id().into()))
}

/// The listed volume called `name`.
pub(crate) fn volume_target(
    name: &str,
    volumes: &[Volume],
) -> Option<LinkTarget> {
    volumes
        .iter()
        .find(|volume| volume.name() == name)
        .map(|volume| {
            LinkTarget::Item(ResourceKind::Volumes, volume.name().into())
        })
}

/// The listed network a container is attached to: by `id`, or by `name` when
/// the engine sent no id.
pub(crate) fn network_target(
    id: Option<&str>,
    name: &str,
    networks: &[Network],
) -> Option<LinkTarget> {
    networks
        .iter()
        .find(|network| match id {
            Some(id) => network.id() == id,
            None => network.name() == name,
        })
        .map(|network| {
            LinkTarget::Item(ResourceKind::Networks, network.id().into())
        })
}

/// The link to `container`.
pub(crate) fn container_target(container: &Container) -> LinkTarget {
    LinkTarget::Item(ResourceKind::Containers, container.id().into())
}

/// The listed container called `name`.
pub(crate) fn named_container_target(
    name: &str,
    containers: &[Container],
) -> Option<LinkTarget> {
    containers
        .iter()
        .find(|container| container.name() == name)
        .map(container_target)
}

/// The listed container with this `id`.
pub(crate) fn container_id_target(
    id: &str,
    containers: &[Container],
) -> Option<LinkTarget> {
    containers
        .iter()
        .find(|container| container.id() == id)
        .map(container_target)
}

/// The compose filter of `project`, when some container belongs to it.
pub(crate) fn compose_target(
    project: &str,
    containers: &[Container],
) -> Option<LinkTarget> {
    containers
        .iter()
        .any(|container| container.compose_project() == Some(project))
        .then(|| LinkTarget::ComposeProject(project.to_string()))
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, Resources, fixtures};

    use super::*;

    fn docker() -> Resources {
        fixtures::resources(EngineKind::Docker)
    }

    fn item(resource: ResourceKind, id: &str) -> LinkTarget {
        LinkTarget::Item(resource, id.to_string())
    }

    #[test]
    fn a_port_on_every_address_or_the_loopback_opens_on_localhost() {
        for host_ip in [None, Some("0.0.0.0"), Some("::"), Some("127.0.0.1")] {
            assert_eq!(
                port_url(host_ip, Some(8080), "tcp"),
                Some(String::from("http://localhost:8080")),
                "{host_ip:?}"
            );
        }
    }

    #[test]
    fn a_port_bound_to_one_address_opens_on_that_address() {
        assert_eq!(
            port_url(Some("192.168.1.20"), Some(8080), "tcp"),
            Some(String::from("http://192.168.1.20:8080"))
        );
        assert_eq!(
            port_url(Some("fd00::5"), Some(8443), "tcp"),
            Some(String::from("http://[fd00::5]:8443"))
        );
    }

    #[test]
    fn an_exposed_or_udp_port_opens_nothing() {
        assert_eq!(port_url(Some("0.0.0.0"), None, "tcp"), None);
        assert_eq!(port_url(None, None, "tcp"), None);
        assert_eq!(port_url(Some("0.0.0.0"), Some(53), "udp"), None);
        assert_eq!(port_url(Some("0.0.0.0"), Some(9000), "sctp"), None);
    }

    #[test]
    fn an_image_is_named_by_its_id_or_by_one_of_its_tags() {
        let resources = docker();
        let busybox = resources
            .images()
            .iter()
            .find(|image| image.tags() == ["busybox:1.37"])
            .expect("busybox");
        let target = Some(item(ResourceKind::Images, busybox.id()));

        assert_eq!(image_target(busybox.id(), resources.images()), target);
        assert_eq!(image_target("busybox:1.37", resources.images()), target);
        assert_eq!(image_target("busybox:9.99", resources.images()), None);
        assert_eq!(image_target("busybox:1.37", &[]), None);
    }

    #[test]
    fn a_volume_is_named_by_its_name() {
        let resources = docker();

        assert_eq!(
            volume_target("otty-fx-data", resources.volumes()),
            Some(item(ResourceKind::Volumes, "otty-fx-data"))
        );
        assert_eq!(volume_target("gone", resources.volumes()), None);
    }

    #[test]
    fn a_network_is_named_by_its_id_or_without_an_id_by_its_name() {
        let resources = docker();
        let bridge = resources
            .networks()
            .iter()
            .find(|network| network.name() == "bridge")
            .expect("bridge network");
        let target = Some(item(ResourceKind::Networks, bridge.id()));

        assert_eq!(
            network_target(Some(bridge.id()), "any", resources.networks()),
            target
        );
        assert_eq!(
            network_target(None, "bridge", resources.networks()),
            target
        );
        assert_eq!(
            network_target(Some("gone"), "bridge", resources.networks()),
            None
        );
        assert_eq!(network_target(None, "pasta", resources.networks()), None);
    }

    #[test]
    fn a_container_is_named_by_its_name_or_by_its_id() {
        let resources = docker();
        let web = resources
            .containers()
            .iter()
            .find(|container| container.name() == "otty-fx-web-1")
            .expect("web container");
        let target = Some(item(ResourceKind::Containers, web.id()));

        assert_eq!(
            named_container_target("otty-fx-web-1", resources.containers()),
            target
        );
        assert_eq!(
            container_id_target(web.id(), resources.containers()),
            target
        );
        assert_eq!(
            named_container_target("gone", resources.containers()),
            None
        );
        assert_eq!(container_id_target("gone", resources.containers()), None);
    }

    #[test]
    fn a_compose_project_has_a_target_only_while_it_has_containers() {
        let resources = docker();

        assert_eq!(
            compose_target("otty-fx", resources.containers()),
            Some(LinkTarget::ComposeProject(String::from("otty-fx")))
        );
        assert_eq!(compose_target("otty-old", resources.containers()), None);
    }
}
