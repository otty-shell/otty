//! Which places of an inspect answer name something OTTY can show (spec
//! Decision 20). A place is its path of JSON keys, with [`LIST_ITEM`] for an
//! item of a list; `links.rs` holds the rules for checking a value against
//! the engine's lists.

use otty_docker::{COMPOSE_PROJECT_LABEL, Container, Image, Network, Volume};
use serde_json::Value;

use super::links::{
    LinkTarget, compose_target, container_id_target, image_target,
    named_container_target, network_target, port_url, volume_target,
};
use super::types::ResourceKind;

/// The path key of an item of a list.
pub(super) const LIST_ITEM: &str = "[]";

/// What links in an inspect answer are checked against: the kind of the shown
/// item and the engine's lists.
pub(crate) struct InspectLinks<'a> {
    pub(crate) resource: ResourceKind,
    pub(crate) containers: &'a [Container],
    pub(crate) images: &'a [Image],
    pub(crate) volumes: &'a [Volume],
    pub(crate) networks: &'a [Network],
}

impl InspectLinks<'_> {
    /// Where the value at `path` leads, when it is text that names something
    /// listed. `parent` is the object or list that holds it.
    pub(super) fn value_target(
        &self,
        path: &[&str],
        value: &Value,
        parent: &Value,
    ) -> Option<LinkTarget> {
        let text = value.as_str()?;

        match self.resource {
            ResourceKind::Containers => {
                self.container_value_target(path, text, parent)
            },
            ResourceKind::Images => match path {
                ["Container"] => container_id_target(text, self.containers),
                _ => None,
            },
            ResourceKind::Volumes => match path {
                ["Labels", COMPOSE_PROJECT_LABEL] => {
                    compose_target(text, self.containers)
                },
                _ => None,
            },
            ResourceKind::Networks => match path {
                ["Containers", _, "Name"] => {
                    named_container_target(text, self.containers)
                },
                _ => None,
            },
        }
    }

    /// Where the key at the end of `path` leads, when the key itself names
    /// something listed: a network of a container. `value` is what the key
    /// holds.
    pub(super) fn key_target(
        &self,
        path: &[&str],
        value: &Value,
    ) -> Option<LinkTarget> {
        if self.resource != ResourceKind::Containers {
            return None;
        }

        match path {
            ["NetworkSettings", "Networks", name] => {
                // The engine sends an empty id for a container that never
                // ran.
                let id = value
                    .get("NetworkID")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty());

                network_target(id, name, self.networks)
            },
            _ => None,
        }
    }

    /// Where the `text` value at `path` of a container leads.
    fn container_value_target(
        &self,
        path: &[&str],
        text: &str,
        parent: &Value,
    ) -> Option<LinkTarget> {
        match path {
            ["Image"] | ["Config", "Image"] => image_target(text, self.images),
            ["HostConfig", "NetworkMode"] => {
                network_target(None, text, self.networks)
            },
            ["Mounts", _, "Name"] => volume_target(text, self.volumes),
            ["Config", "Labels", COMPOSE_PROJECT_LABEL] => {
                compose_target(text, self.containers)
            },
            ["NetworkSettings", "Ports", port, _, "HostPort"] => {
                published_port_target(port, text, parent)
            },
            _ => None,
        }
    }
}

#[cfg(test)]
impl InspectLinks<'static> {
    /// Nothing to link to, for a `resource`.
    pub(crate) fn without_lists(resource: ResourceKind) -> Self {
        Self {
            resource,
            containers: &[],
            images: &[],
            volumes: &[],
            networks: &[],
        }
    }
}

/// The page of the `host_port` published under `port` ("8080/tcp"); `binding`
/// is the object that holds it, with its `HostIp`.
fn published_port_target(
    port: &str,
    host_port: &str,
    binding: &Value,
) -> Option<LinkTarget> {
    let (_, protocol) = port.split_once('/')?;
    let host_port = host_port.parse().ok();
    let host_ip = binding.get("HostIp").and_then(Value::as_str);

    port_url(host_ip, host_port, protocol).map(LinkTarget::Url)
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, Resources, fixtures};
    use otty_ui_kit::components::data_tree::DataRow;
    use otty_ui_kit::components::key_value_list::ValuePart;
    use serde_json::json;

    use super::*;
    use crate::widgets::docker::inspect_tree::{InspectTree, inspect_tree};

    fn docker() -> Resources {
        fixtures::resources(EngineKind::Docker)
    }

    fn links_of(
        resource: ResourceKind,
        resources: &Resources,
    ) -> InspectLinks<'_> {
        InspectLinks {
            resource,
            containers: resources.containers(),
            images: resources.images(),
            volumes: resources.volumes(),
            networks: resources.networks(),
        }
    }

    /// The tree of `value`, shown as an item of `resource`, against the
    /// fixture lists.
    fn tree(resource: ResourceKind, value: &Value) -> InspectTree {
        let resources = docker();

        inspect_tree(value, false, &links_of(resource, &resources))
            .expect("an object")
    }

    fn section<'a>(
        tree: &'a InspectTree,
        title: &str,
    ) -> &'a [DataRow<LinkTarget>] {
        &tree
            .sections
            .iter()
            .find(|section| section.title == title)
            .expect("the section")
            .rows
    }

    fn linked(
        depth: u16,
        key: &str,
        value: &str,
        target: LinkTarget,
    ) -> DataRow<LinkTarget> {
        DataRow::with_parts(depth, key, [ValuePart::link(value, target)])
    }

    fn item(resource: ResourceKind, id: &str) -> LinkTarget {
        LinkTarget::Item(resource, id.to_string())
    }

    fn url(address: &str) -> LinkTarget {
        LinkTarget::Url(address.to_string())
    }

    fn busybox_id(resources: &Resources) -> String {
        resources
            .images()
            .iter()
            .find(|image| image.tags() == ["busybox:1.37"])
            .map(|image| image.id().to_string())
            .expect("busybox")
    }

    fn container_id(resources: &Resources, name: &str) -> String {
        resources
            .containers()
            .iter()
            .find(|container| container.name() == name)
            .map(|container| container.id().to_string())
            .expect("the container")
    }

    fn network_id(resources: &Resources, name: &str) -> String {
        resources
            .networks()
            .iter()
            .find(|network| network.name() == name)
            .map(|network| network.id().to_string())
            .expect("the network")
    }

    #[test]
    fn a_container_links_its_image_id_when_it_is_listed() {
        let resources = docker();
        let busybox = busybox_id(&resources);
        let listed = json!({ "Image": busybox });
        let gone = json!({ "Image": "sha256:gone" });

        let listed = tree(ResourceKind::Containers, &listed);
        let gone = tree(ResourceKind::Containers, &gone);

        assert_eq!(
            listed.general,
            vec![linked(
                0,
                "Image",
                &busybox,
                item(ResourceKind::Images, &busybox)
            )]
        );
        assert_eq!(gone.general, vec![DataRow::new(0, "Image", "sha256:gone")]);
    }

    #[test]
    fn a_container_links_its_config_image_by_tag_or_id_when_it_is_listed() {
        let resources = docker();
        let busybox = busybox_id(&resources);
        let by_tag = json!({ "Config": { "Image": "busybox:1.37" } });
        let by_id = json!({ "Config": { "Image": busybox } });
        let gone = json!({ "Config": { "Image": "busybox:9.99" } });

        let by_tag = tree(ResourceKind::Containers, &by_tag);
        let by_id = tree(ResourceKind::Containers, &by_id);
        let gone = tree(ResourceKind::Containers, &gone);

        let target = item(ResourceKind::Images, &busybox);
        assert_eq!(
            section(&by_tag, "Config"),
            [linked(0, "Image", "busybox:1.37", target.clone())]
        );
        assert_eq!(
            section(&by_id, "Config"),
            [linked(0, "Image", &busybox, target)]
        );
        assert_eq!(
            section(&gone, "Config"),
            [DataRow::new(0, "Image", "busybox:9.99")]
        );
    }

    #[test]
    fn each_network_key_of_a_container_links_by_its_network_id() {
        let resources = docker();
        let net = network_id(&resources, "otty-fx-net");
        let value = json!({
            "NetworkSettings": {
                "Networks": {
                    "otty-fx-net": { "NetworkID": net, "IPPrefixLen": 20 },
                    "stale-name": { "NetworkID": "gone", "IPPrefixLen": 16 },
                },
            },
        });

        let tree = tree(ResourceKind::Containers, &value);

        assert_eq!(
            section(&tree, "NetworkSettings"),
            [
                DataRow::heading(0, "Networks"),
                DataRow::heading_link(
                    1,
                    "otty-fx-net",
                    item(ResourceKind::Networks, &net)
                ),
                DataRow::new(2, "IPPrefixLen", "20"),
                DataRow::new(2, "NetworkID", &net),
                DataRow::heading(1, "stale-name"),
                DataRow::new(2, "IPPrefixLen", "16"),
                DataRow::new(2, "NetworkID", "gone"),
            ]
        );
    }

    #[test]
    fn a_network_key_without_an_id_links_by_its_name() {
        let resources = docker();
        let bridge = network_id(&resources, "bridge");
        let value = json!({
            "NetworkSettings": {
                "Networks": {
                    "bridge": { "NetworkID": "", "IPPrefixLen": 0 },
                    "pasta": { "IPPrefixLen": 0 },
                },
            },
        });

        let tree = tree(ResourceKind::Containers, &value);

        assert_eq!(
            section(&tree, "NetworkSettings"),
            [
                DataRow::heading(0, "Networks"),
                DataRow::heading_link(
                    1,
                    "bridge",
                    item(ResourceKind::Networks, &bridge)
                ),
                DataRow::new(2, "IPPrefixLen", "0"),
                DataRow::heading(1, "pasta"),
                DataRow::new(2, "IPPrefixLen", "0"),
            ]
        );
    }

    #[test]
    fn a_container_links_its_network_mode_when_it_names_a_listed_network() {
        let resources = docker();
        let net = network_id(&resources, "otty-fx-net");
        let listed = json!({ "HostConfig": { "NetworkMode": "otty-fx-net" } });
        let other = json!({ "HostConfig": { "NetworkMode": "container:web" } });

        let listed = tree(ResourceKind::Containers, &listed);
        let other = tree(ResourceKind::Containers, &other);

        assert_eq!(
            section(&listed, "HostConfig"),
            [linked(
                0,
                "NetworkMode",
                "otty-fx-net",
                item(ResourceKind::Networks, &net)
            )]
        );
        assert_eq!(
            section(&other, "HostConfig"),
            [DataRow::new(0, "NetworkMode", "container:web")]
        );
    }

    #[test]
    fn each_mount_name_links_to_its_volume_when_it_is_listed() {
        let value = json!({
            "Mounts": [
                { "Name": "otty-fx-data" },
                { "Name": "gone" },
                { "Source": "otty-fx-data" },
            ],
        });

        let tree = tree(ResourceKind::Containers, &value);

        assert_eq!(
            section(&tree, "Mounts"),
            [
                DataRow::heading(0, "1"),
                linked(
                    1,
                    "Name",
                    "otty-fx-data",
                    item(ResourceKind::Volumes, "otty-fx-data")
                ),
                DataRow::heading(0, "2"),
                DataRow::new(1, "Name", "gone"),
                DataRow::heading(0, "3"),
                DataRow::new(1, "Source", "otty-fx-data"),
            ]
        );
    }

    #[test]
    fn the_compose_label_of_a_container_links_while_the_project_has_containers()
    {
        let listed = json!({
            "Config": {
                "Labels": {
                    "com.docker.compose.project": "otty-fx",
                    "com.docker.compose.service": "otty-fx",
                },
            },
        });
        let gone = json!({
            "Config": {
                "Labels": { "com.docker.compose.project": "otty-old" },
            },
        });

        let listed = tree(ResourceKind::Containers, &listed);
        let gone = tree(ResourceKind::Containers, &gone);

        assert_eq!(
            section(&listed, "Config"),
            [
                DataRow::heading(0, "Labels"),
                linked(
                    1,
                    "com.docker.compose.project",
                    "otty-fx",
                    LinkTarget::ComposeProject(String::from("otty-fx"))
                ),
                DataRow::new(1, "com.docker.compose.service", "otty-fx"),
            ]
        );
        assert_eq!(
            section(&gone, "Config"),
            [
                DataRow::heading(0, "Labels"),
                DataRow::new(1, "com.docker.compose.project", "otty-old"),
            ]
        );
    }

    #[test]
    fn each_published_tcp_port_links_by_the_rule_of_the_overview() {
        let value = json!({
            "NetworkSettings": {
                "Ports": {
                    "8080/tcp": [
                        { "HostIp": "0.0.0.0", "HostPort": "18081" },
                        { "HostIp": "192.168.1.20", "HostPort": "18082" },
                        { "HostIp": "fd00::5", "HostPort": "18083" },
                    ],
                    "9000/tcp": null,
                },
            },
        });

        let tree = tree(ResourceKind::Containers, &value);

        assert_eq!(
            section(&tree, "NetworkSettings"),
            [
                DataRow::heading(0, "Ports"),
                DataRow::heading(1, "8080/tcp"),
                DataRow::heading(2, "1"),
                DataRow::new(3, "HostIp", "0.0.0.0"),
                linked(3, "HostPort", "18081", url("http://localhost:18081")),
                DataRow::heading(2, "2"),
                DataRow::new(3, "HostIp", "192.168.1.20"),
                linked(
                    3,
                    "HostPort",
                    "18082",
                    url("http://192.168.1.20:18082")
                ),
                DataRow::heading(2, "3"),
                DataRow::new(3, "HostIp", "fd00::5"),
                linked(3, "HostPort", "18083", url("http://[fd00::5]:18083")),
            ]
        );
    }

    #[test]
    fn a_port_that_a_browser_cannot_open_stays_text() {
        let value = json!({
            "NetworkSettings": {
                "Ports": {
                    "53/udp": [{ "HostPort": "5353" }],
                    "80": [{ "HostPort": "8080" }],
                    "81/tcp": [{ "HostPort": "not a port" }],
                },
            },
        });

        let tree = tree(ResourceKind::Containers, &value);

        assert_eq!(
            section(&tree, "NetworkSettings"),
            [
                DataRow::heading(0, "Ports"),
                DataRow::heading(1, "53/udp"),
                DataRow::heading(2, "1"),
                DataRow::new(3, "HostPort", "5353"),
                DataRow::heading(1, "80"),
                DataRow::heading(2, "1"),
                DataRow::new(3, "HostPort", "8080"),
                DataRow::heading(1, "81/tcp"),
                DataRow::heading(2, "1"),
                DataRow::new(3, "HostPort", "not a port"),
            ]
        );
    }

    #[test]
    fn the_id_tags_and_config_image_of_an_image_stay_text() {
        let resources = docker();
        let busybox = busybox_id(&resources);
        let value = json!({
            "Id": busybox,
            "RepoTags": ["busybox:1.37"],
            "Config": { "Image": busybox },
        });

        let tree = tree(ResourceKind::Images, &value);

        assert_eq!(
            tree.general,
            vec![
                DataRow::new(0, "Id", &busybox),
                DataRow::new(0, "RepoTags", "busybox:1.37"),
            ]
        );
        assert_eq!(
            section(&tree, "Config"),
            [DataRow::new(0, "Image", &busybox)]
        );
    }

    #[test]
    fn the_container_an_image_names_links_when_it_is_listed() {
        let resources = docker();
        let web = container_id(&resources, "otty-fx-web-1");
        let listed = json!({ "Container": web });
        let gone = json!({ "Container": "build-step" });

        let listed = tree(ResourceKind::Images, &listed);
        let gone = tree(ResourceKind::Images, &gone);

        assert_eq!(
            listed.general,
            vec![linked(
                0,
                "Container",
                &web,
                item(ResourceKind::Containers, &web)
            )]
        );
        assert_eq!(
            gone.general,
            vec![DataRow::new(0, "Container", "build-step")]
        );
    }

    #[test]
    fn the_compose_label_of_a_volume_links_while_the_project_has_containers() {
        let listed = json!({
            "Labels": { "com.docker.compose.project": "otty-fx" },
        });
        let gone = json!({
            "Labels": { "com.docker.compose.project": "otty-old" },
        });

        let listed = tree(ResourceKind::Volumes, &listed);
        let gone = tree(ResourceKind::Volumes, &gone);

        assert_eq!(
            section(&listed, "Labels"),
            [linked(
                0,
                "com.docker.compose.project",
                "otty-fx",
                LinkTarget::ComposeProject(String::from("otty-fx"))
            )]
        );
        assert_eq!(
            section(&gone, "Labels"),
            [DataRow::new(0, "com.docker.compose.project", "otty-old")]
        );
    }

    #[test]
    fn each_container_of_a_network_links_by_its_name_when_it_is_listed() {
        let resources = docker();
        let web = container_id(&resources, "otty-fx-web-1");
        let value = json!({
            "Containers": {
                "abc": { "Name": "otty-fx-web-1" },
                "def": { "Name": "gone" },
            },
        });

        let tree = tree(ResourceKind::Networks, &value);

        assert_eq!(
            section(&tree, "Containers"),
            [
                DataRow::heading(0, "abc"),
                linked(
                    1,
                    "Name",
                    "otty-fx-web-1",
                    item(ResourceKind::Containers, &web)
                ),
                DataRow::heading(0, "def"),
                DataRow::new(1, "Name", "gone"),
            ]
        );
    }

    #[test]
    fn a_path_links_only_for_the_kind_of_item_it_belongs_to() {
        let resources = docker();
        let busybox = busybox_id(&resources);
        let value = json!({
            "Image": busybox,
            "Containers": { "abc": { "Name": "otty-fx-web-1" } },
            "Labels": { "com.docker.compose.project": "otty-fx" },
        });

        let container = tree(ResourceKind::Containers, &value);
        let volume = tree(ResourceKind::Volumes, &value);
        let network = tree(ResourceKind::Networks, &value);

        assert_eq!(
            section(&container, "Containers")[1],
            DataRow::new(1, "Name", "otty-fx-web-1")
        );
        assert_eq!(
            section(&container, "Labels")[0],
            DataRow::new(0, "com.docker.compose.project", "otty-fx")
        );
        assert_eq!(volume.general[0], DataRow::new(0, "Image", &busybox));
        assert_eq!(network.general[0], DataRow::new(0, "Image", &busybox));
        assert_eq!(
            section(&network, "Labels")[0],
            DataRow::new(0, "com.docker.compose.project", "otty-fx")
        );
    }

    #[test]
    fn the_same_key_elsewhere_in_the_answer_stays_text() {
        let resources = docker();
        let busybox = busybox_id(&resources);
        let value = json!({
            "State": { "Image": busybox, "Name": "otty-fx-data" },
            "Config": { "Nested": { "Image": busybox } },
        });

        let tree = tree(ResourceKind::Containers, &value);

        assert_eq!(
            section(&tree, "State"),
            [
                DataRow::new(0, "Image", &busybox),
                DataRow::new(0, "Name", "otty-fx-data"),
            ]
        );
        assert_eq!(
            section(&tree, "Config"),
            [
                DataRow::heading(0, "Nested"),
                DataRow::new(1, "Image", &busybox),
            ]
        );
    }

    #[test]
    fn a_value_that_is_not_text_stays_text() {
        let value = json!({
            "Image": 5,
            "Mounts": [{ "Name": ["otty-fx-data"] }],
        });

        let tree = tree(ResourceKind::Containers, &value);

        assert_eq!(tree.general, vec![DataRow::new(0, "Image", "5")]);
        assert_eq!(
            section(&tree, "Mounts"),
            [
                DataRow::heading(0, "1"),
                DataRow::new(1, "Name", "otty-fx-data"),
            ]
        );
    }

    #[test]
    fn show_empty_leaves_empty_values_as_text() {
        let resources = docker();
        let value = json!({
            "Image": "",
            "Mounts": [{ "Name": "" }],
            "HostConfig": { "NetworkMode": null },
        });

        let tree = inspect_tree(
            &value,
            true,
            &links_of(ResourceKind::Containers, &resources),
        )
        .expect("an object");

        assert_eq!(tree.general, vec![DataRow::new(0, "Image", "\"\"")]);
        assert_eq!(
            section(&tree, "Mounts"),
            [DataRow::heading(0, "1"), DataRow::new(1, "Name", "\"\""),]
        );
        assert_eq!(
            section(&tree, "HostConfig"),
            [DataRow::new(0, "NetworkMode", "null")]
        );
    }

    #[test]
    fn with_empty_lists_every_value_stays_text() {
        let resources = docker();
        let busybox = busybox_id(&resources);
        let value = json!({
            "Image": busybox,
            "Mounts": [{ "Name": "otty-fx-data" }],
        });

        let tree = inspect_tree(
            &value,
            false,
            &InspectLinks::without_lists(ResourceKind::Containers),
        )
        .expect("an object");

        assert_eq!(tree.general, vec![DataRow::new(0, "Image", &busybox)]);
        assert_eq!(
            section(&tree, "Mounts"),
            [
                DataRow::heading(0, "1"),
                DataRow::new(1, "Name", "otty-fx-data"),
            ]
        );
    }
}
