//! A network as OTTY shows it, mapped from the engine's network list.

#[cfg(feature = "client")]
use bollard::models::Network as EngineNetwork;

#[cfg(feature = "client")]
use crate::container::{Container, ContainerNetwork};
#[cfg(feature = "client")]
use crate::text_order::compare_text;
#[cfg(feature = "client")]
use crate::timestamp::unix_seconds;

/// Networks every engine has and refuses to remove. Podman's API shows its
/// default network (`podman`) as `bridge`.
#[cfg(feature = "client")]
const BUILT_IN_NETWORKS: [&str; 3] = ["bridge", "host", "none"];

/// One network of an engine (spec Decision 10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Network {
    id: String,
    name: String,
    driver: String,
    scope: String,
    subnet: Option<String>,
    gateway: Option<String>,
    containers: Vec<String>,
    created: Option<i64>,
    built_in: bool,
}

impl Network {
    /// The full network id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The network name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The driver, for example `bridge`, `host` or `null`.
    pub fn driver(&self) -> &str {
        &self.driver
    }

    /// `local` for networks of this computer.
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// The first address range, for example `172.17.0.0/16`.
    pub fn subnet(&self) -> Option<&str> {
        self.subnet.as_deref()
    }

    /// The gateway of the first address range.
    pub fn gateway(&self) -> Option<&str> {
        self.gateway.as_deref()
    }

    /// Names of the containers (running or not) attached to it, sorted. The
    /// engine refuses to remove a network while this is not empty.
    pub fn containers(&self) -> &[String] {
        &self.containers
    }

    /// When the network was made, in Unix seconds. Podman sends the time of
    /// the request for its default network.
    pub fn created(&self) -> Option<i64> {
        self.created
    }

    /// `bridge`, `host` or `none`: made by the engine, cannot be removed.
    pub fn is_built_in(&self) -> bool {
        self.built_in
    }
}

#[cfg(feature = "client")]
impl Network {
    /// Map one entry of the engine's network list. Who is attached comes
    /// from `containers`: Docker's list leaves it empty.
    pub(crate) fn from_network(
        network: &EngineNetwork,
        containers: &[Container],
    ) -> Self {
        let id = network.id.clone().unwrap_or_default();
        let name = network.name.clone().unwrap_or_default();
        let address_range = network
            .ipam
            .as_ref()
            .and_then(|ipam| ipam.config.as_ref())
            .and_then(|ranges| ranges.first());

        let mut users: Vec<String> = containers
            .iter()
            .filter(|container| {
                container
                    .networks()
                    .iter()
                    .any(|entry| is_attached(entry, &id, &name))
            })
            .map(|container| container.name().to_owned())
            .collect();
        users.sort_by(|a, b| compare_text(a, b));

        Self {
            built_in: BUILT_IN_NETWORKS.contains(&name.as_str()),
            driver: network.driver.clone().unwrap_or_default(),
            scope: network.scope.clone().unwrap_or_default(),
            subnet: address_range.and_then(|range| range.subnet.clone()),
            gateway: address_range.and_then(|range| range.gateway.clone()),
            containers: users,
            created: network.created.as_deref().and_then(unix_seconds),
            id,
            name,
        }
    }
}

/// Whether a container's network entry points at this network: by id when
/// the engine sent one, by name otherwise.
#[cfg(feature = "client")]
fn is_attached(entry: &ContainerNetwork, id: &str, name: &str) -> bool {
    match entry.id() {
        Some(entry_id) => entry_id == id,
        None => entry.name() == name,
    }
}

#[cfg(all(test, feature = "client"))]
mod tests {
    use super::*;
    use crate::test_support::{
        DOCKER_CONTAINERS, PODMAN_CONTAINERS, containers,
    };

    const DOCKER_NETWORKS: &str =
        include_str!("../tests/fixtures/docker-networks.json");
    const PODMAN_NETWORKS: &str =
        include_str!("../tests/fixtures/podman-networks.json");

    fn networks(networks_json: &str, containers_json: &str) -> Vec<Network> {
        let answer: Vec<EngineNetwork> =
            serde_json::from_str(networks_json).expect("network list fixture");
        let containers = containers(containers_json);

        answer
            .iter()
            .map(|network| Network::from_network(network, &containers))
            .collect()
    }

    fn named<'a>(networks: &'a [Network], name: &str) -> &'a Network {
        networks
            .iter()
            .find(|network| network.name() == name)
            .unwrap_or_else(|| panic!("no network {name}"))
    }

    #[test]
    fn docker_default_bridge_is_built_in_with_its_containers() {
        let networks = networks(DOCKER_NETWORKS, DOCKER_CONTAINERS);

        let bridge = named(&networks, "bridge");

        assert_eq!(
            bridge.id(),
            "223d4f086cec4a90d7d423471bffac84e170ffd60775609002aba53ce2484a1f"
        );
        assert_eq!(bridge.driver(), "bridge");
        assert_eq!(bridge.scope(), "local");
        assert_eq!(bridge.subnet(), Some("172.17.0.0/16"));
        assert_eq!(bridge.gateway(), Some("172.17.0.1"));
        assert_eq!(bridge.created(), Some(1_793_530_261));
        assert!(bridge.is_built_in());
        // `otty-fx-created` never ran, so Docker sent no network id for it:
        // it is matched by name.
        assert_eq!(
            bridge.containers(),
            [
                "otty-fx-created",
                "otty-fx-exit0",
                "otty-fx-exit3",
                "otty-fx-paused"
            ]
        );
    }

    #[test]
    fn docker_host_and_none_are_built_in_without_subnet() {
        let networks = networks(DOCKER_NETWORKS, DOCKER_CONTAINERS);

        for name in ["host", "none"] {
            let network = named(&networks, name);

            assert!(network.is_built_in(), "{name}");
            assert_eq!(network.subnet(), None, "{name}");
            assert_eq!(network.gateway(), None, "{name}");
            assert!(network.containers().is_empty(), "{name}");
        }
        assert_eq!(named(&networks, "none").driver(), "null");
        assert_eq!(named(&networks, "none").created(), Some(1_768_778_728));
    }

    #[test]
    fn user_network_is_not_built_in_and_lists_its_containers() {
        for (networks_json, containers_json) in [
            (DOCKER_NETWORKS, DOCKER_CONTAINERS),
            (PODMAN_NETWORKS, PODMAN_CONTAINERS),
        ] {
            let networks = networks(networks_json, containers_json);

            let own = named(&networks, "otty-fx-net");

            assert!(!own.is_built_in());
            assert!(own.subnet().is_some());
            assert_eq!(own.containers(), ["otty-fx-running", "otty-fx-web-1"]);
        }
    }

    #[test]
    fn podman_default_network_is_shown_as_bridge() {
        let networks = networks(PODMAN_NETWORKS, PODMAN_CONTAINERS);

        let bridge = named(&networks, "bridge");

        assert!(bridge.is_built_in());
        assert_eq!(bridge.subnet(), Some("10.88.0.0/16"));
        assert_eq!(bridge.gateway(), Some("10.88.0.1"));
        // Rootless containers use `pasta`, not this network.
        assert!(bridge.containers().is_empty());
    }

    #[test]
    fn podman_user_network_matches_the_engines_own_container_map() {
        // Podman fills `Containers` in its network list (bollard's model
        // drops it); the names computed from the container list agree.
        let raw: serde_json::Value = serde_json::from_str(PODMAN_NETWORKS)
            .expect("network list fixture");
        let mut engine_names: Vec<&str> = raw
            .as_array()
            .into_iter()
            .flatten()
            .filter(|network| network["Name"] == "otty-fx-net")
            .filter_map(|network| network["Containers"].as_object())
            .flat_map(|map| map.values())
            .filter_map(|container| container["Name"].as_str())
            .collect();
        engine_names.sort_unstable();

        let networks = networks(PODMAN_NETWORKS, PODMAN_CONTAINERS);

        assert_eq!(named(&networks, "otty-fx-net").containers(), engine_names);
    }

    #[test]
    fn users_are_listed_a_to_z_whatever_the_case() {
        let summaries: Vec<bollard::models::ContainerSummary> = serde_json::from_str(
            r#"[
                {"Names":["/Zed"],"NetworkSettings":{"Networks":{"shared":{"NetworkID":"net"}}}},
                {"Names":["/beta"],"NetworkSettings":{"Networks":{"shared":{"NetworkID":"net"}}}},
                {"Names":["/Alpha"],"NetworkSettings":{"Networks":{"shared":{"NetworkID":"net"}}}}
            ]"#,
        )
        .expect("container list");
        let containers: Vec<Container> =
            summaries.iter().map(Container::from_summary).collect();
        let network: EngineNetwork =
            serde_json::from_str(r#"{"Id":"net","Name":"shared"}"#)
                .expect("network");

        let network = Network::from_network(&network, &containers);

        assert_eq!(network.containers(), ["Alpha", "beta", "Zed"]);
    }

    #[test]
    fn empty_network_gives_blank_values() {
        let network: EngineNetwork =
            serde_json::from_str("{}").expect("network");

        let network = Network::from_network(&network, &[]);

        assert_eq!(network.id(), "");
        assert_eq!(network.name(), "");
        assert_eq!(network.driver(), "");
        assert_eq!(network.created(), None);
        assert!(!network.is_built_in());
    }
}
