//! A container as OTTY shows it, mapped from the engine's container list.

#[cfg(feature = "client")]
use bollard::models::{
    ContainerSummary, ContainerSummaryStateEnum, PortSummaryTypeEnum,
};

#[cfg(feature = "client")]
use crate::text_order::compare_text;

/// The label Docker Compose, `podman compose` and `podman-compose` put on
/// every service container; its value is the project name.
pub const COMPOSE_PROJECT_LABEL: &str = "com.docker.compose.project";

/// What a container is doing now, as the engine reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    /// Made but never started.
    Created,
    /// Its main process runs.
    Running,
    /// Frozen by Pause.
    Paused,
    /// The engine is starting it again after it stopped.
    Restarting,
    /// The engine is removing it.
    Removing,
    /// Stopped; see [`Container::exit_code`].
    Exited,
    /// The engine failed to stop or remove it.
    Dead,
    /// The engine is stopping it.
    Stopping,
    /// The engine sent no state.
    Unknown,
}

/// A published or exposed port of a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortMapping {
    host_ip: Option<String>,
    host_port: Option<u16>,
    container_port: u16,
    protocol: String,
}

impl PortMapping {
    /// The host address the port is published on, for example `0.0.0.0` or
    /// `::`. Docker lists one mapping per address family.
    pub fn host_ip(&self) -> Option<&str> {
        self.host_ip.as_deref()
    }

    /// The port on the host; `None` when the port is only exposed.
    pub fn host_port(&self) -> Option<u16> {
        self.host_port
    }

    /// The port inside the container.
    pub fn container_port(&self) -> u16 {
        self.container_port
    }

    /// `tcp`, `udp` or `sctp`.
    pub fn protocol(&self) -> &str {
        &self.protocol
    }
}

/// What kind of storage a mount is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountKind {
    /// A volume managed by the engine; [`Mount::name`] is its name.
    Volume,
    /// A host path.
    Bind,
    /// Anything else (`tmpfs`, `image`, ...).
    Other,
}

/// A volume or host path mounted into a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    kind: MountKind,
    name: Option<String>,
    source: String,
    destination: String,
    read_only: bool,
}

impl Mount {
    /// Volume, bind or other.
    pub fn kind(&self) -> MountKind {
        self.kind
    }

    /// The volume name, for volume mounts.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The host path (for volumes, where the engine keeps the data; can be
    /// empty).
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The path inside the container.
    pub fn destination(&self) -> &str {
        &self.destination
    }

    /// Whether the container can only read it.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }
}

/// A network a container is attached to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerNetwork {
    name: String,
    id: Option<String>,
    ip_address: Option<String>,
}

impl ContainerNetwork {
    /// The network name, for example `bridge`. Rootless Podman shows
    /// `pasta` or `slirp4netns` here, which are not listed networks.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The network id; `None` when the engine did not send it (Docker, for a
    /// container that never ran).
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// The container's IPv4 address on this network, while it runs.
    pub fn ip_address(&self) -> Option<&str> {
        self.ip_address.as_deref()
    }
}

/// One container of an engine (spec Decision 10).
///
/// Times are Unix seconds; the app formats them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    id: String,
    name: String,
    state: ContainerState,
    status: String,
    exit_code: Option<i64>,
    image: String,
    image_id: String,
    ports: Vec<PortMapping>,
    created: Option<i64>,
    compose_project: Option<String>,
    mounts: Vec<Mount>,
    networks: Vec<ContainerNetwork>,
    command: String,
}

impl Container {
    /// The full container id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The container name without the leading `/`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// What the container is doing now.
    pub fn state(&self) -> ContainerState {
        self.state
    }

    /// The engine's own status text, for example `Up 3 minutes` or
    /// `Exited (1) 2 hours ago`.
    pub fn status(&self) -> &str {
        &self.status
    }

    /// The exit code of an exited container, read from its status text.
    pub fn exit_code(&self) -> Option<i64> {
        self.exit_code
    }

    /// The image as the container was created from it, for example
    /// `busybox:1.37` (Docker) or `docker.io/library/busybox:1.37` (Podman).
    pub fn image(&self) -> &str {
        &self.image
    }

    /// The id of the image, matching [`crate::Image::id`].
    pub fn image_id(&self) -> &str {
        &self.image_id
    }

    /// Published and exposed ports, in the engine's order.
    pub fn ports(&self) -> &[PortMapping] {
        &self.ports
    }

    /// When the container was made, in Unix seconds.
    pub fn created(&self) -> Option<i64> {
        self.created
    }

    /// The compose project it belongs to, from the
    /// `com.docker.compose.project` label.
    pub fn compose_project(&self) -> Option<&str> {
        self.compose_project.as_deref()
    }

    /// Volumes and host paths mounted into it.
    pub fn mounts(&self) -> &[Mount] {
        &self.mounts
    }

    /// Networks it is attached to, by name.
    pub fn networks(&self) -> &[ContainerNetwork] {
        &self.networks
    }

    /// The command it runs, as the engine shows it.
    pub fn command(&self) -> &str {
        &self.command
    }
}

#[cfg(feature = "client")]
impl Container {
    /// Map one entry of the engine's container list.
    pub(crate) fn from_summary(summary: &ContainerSummary) -> Self {
        let name = summary
            .names
            .iter()
            .flatten()
            .next()
            .map(|name| name.trim_start_matches('/'))
            .unwrap_or_default();
        let state = summary.state.map_or(ContainerState::Unknown, map_state);
        let status = summary.status.clone().unwrap_or_default();
        let exit_code = match state {
            ContainerState::Exited => exit_code_from_status(&status),
            _ => None,
        };
        let compose_project = summary
            .labels
            .as_ref()
            .and_then(|labels| labels.get(COMPOSE_PROJECT_LABEL))
            .map(|project| project.trim())
            .filter(|project| !project.is_empty())
            .map(String::from);

        Self {
            id: summary.id.clone().unwrap_or_default(),
            name: name.to_owned(),
            state,
            status,
            exit_code,
            image: summary.image.clone().unwrap_or_default(),
            image_id: summary.image_id.clone().unwrap_or_default(),
            ports: map_ports(summary),
            created: summary.created,
            compose_project,
            mounts: map_mounts(summary),
            networks: map_networks(summary),
            command: summary.command.clone().unwrap_or_default(),
        }
    }
}

#[cfg(feature = "client")]
fn map_state(state: ContainerSummaryStateEnum) -> ContainerState {
    match state {
        ContainerSummaryStateEnum::CREATED => ContainerState::Created,
        ContainerSummaryStateEnum::RUNNING => ContainerState::Running,
        ContainerSummaryStateEnum::PAUSED => ContainerState::Paused,
        ContainerSummaryStateEnum::RESTARTING => ContainerState::Restarting,
        ContainerSummaryStateEnum::REMOVING => ContainerState::Removing,
        ContainerSummaryStateEnum::EXITED => ContainerState::Exited,
        ContainerSummaryStateEnum::DEAD => ContainerState::Dead,
        ContainerSummaryStateEnum::STOPPING => ContainerState::Stopping,
        _ => ContainerState::Unknown,
    }
}

#[cfg(feature = "client")]
fn map_ports(summary: &ContainerSummary) -> Vec<PortMapping> {
    summary
        .ports
        .iter()
        .flatten()
        .map(|port| PortMapping {
            host_ip: port.ip.clone().filter(|ip| !ip.is_empty()),
            host_port: port.public_port,
            container_port: port.private_port,
            protocol: port
                .typ
                .filter(|protocol| *protocol != PortSummaryTypeEnum::EMPTY)
                .map_or_else(
                    || String::from("tcp"),
                    |protocol| protocol.to_string(),
                ),
        })
        .collect()
}

#[cfg(feature = "client")]
fn map_mounts(summary: &ContainerSummary) -> Vec<Mount> {
    summary
        .mounts
        .iter()
        .flatten()
        .map(|mount| Mount {
            kind: match mount.typ.as_deref() {
                Some("volume") => MountKind::Volume,
                Some("bind") => MountKind::Bind,
                _ => MountKind::Other,
            },
            name: mount.name.clone().filter(|name| !name.is_empty()),
            source: mount.source.clone().unwrap_or_default(),
            destination: mount.destination.clone().unwrap_or_default(),
            read_only: !mount.rw.unwrap_or(true),
        })
        .collect()
}

#[cfg(feature = "client")]
fn map_networks(summary: &ContainerSummary) -> Vec<ContainerNetwork> {
    let endpoints = summary
        .network_settings
        .as_ref()
        .and_then(|settings| settings.networks.as_ref());

    let mut networks: Vec<ContainerNetwork> = endpoints
        .into_iter()
        .flatten()
        .map(|(name, endpoint)| ContainerNetwork {
            name: name.clone(),
            id: endpoint.network_id.clone().filter(|id| !id.is_empty()),
            ip_address: endpoint
                .ip_address
                .clone()
                .filter(|address| !address.is_empty()),
        })
        .collect();
    networks.sort_by(|left, right| compare_text(&left.name, &right.name));

    networks
}

/// Read the exit code from a status text such as `Exited (137) 2 days ago`.
#[cfg(feature = "client")]
fn exit_code_from_status(status: &str) -> Option<i64> {
    let (code, _) = status.strip_prefix("Exited (")?.split_once(')')?;

    code.parse().ok()
}

#[cfg(all(test, feature = "client"))]
mod tests {
    use super::*;
    use crate::test_support::{
        DOCKER_CONTAINERS, PODMAN_CONTAINERS, containers,
    };

    fn named<'a>(containers: &'a [Container], name: &str) -> &'a Container {
        containers
            .iter()
            .find(|container| container.name() == name)
            .unwrap_or_else(|| panic!("no container {name}"))
    }

    fn summary(json: &str) -> ContainerSummary {
        serde_json::from_str(json).expect("container summary")
    }

    #[test]
    fn docker_running_container_is_mapped() {
        let containers = containers(DOCKER_CONTAINERS);

        let running = named(&containers, "otty-fx-running");

        assert_eq!(
            running.id(),
            "3bae86f2fff630944ef9e0306e969614d4a2cbc89ff9da972d29915382ace25c"
        );
        assert_eq!(running.state(), ContainerState::Running);
        assert!(running.status().starts_with("Up "), "{}", running.status());
        assert_eq!(running.exit_code(), None);
        assert_eq!(running.image(), "busybox:1.37");
        assert_eq!(
            running.image_id(),
            "sha256:a7dbc319b1438115a207e11930dc4bb3e643dcb3ec0bdd694bb36cd8fad289ec"
        );
        assert_eq!(running.created(), Some(1_793_557_836));
        assert_eq!(running.command(), "sleep 3600");
        assert_eq!(running.compose_project(), None);
    }

    #[test]
    fn docker_lists_each_port_once_per_address_family() {
        let containers = containers(DOCKER_CONTAINERS);

        let ports = named(&containers, "otty-fx-running").ports();

        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].host_ip(), Some("0.0.0.0"));
        assert_eq!(ports[0].host_port(), Some(18080));
        assert_eq!(ports[0].container_port(), 80);
        assert_eq!(ports[0].protocol(), "tcp");
        assert_eq!(ports[1].host_ip(), Some("::"));
    }

    #[test]
    fn docker_named_volume_mount_is_mapped() {
        let containers = containers(DOCKER_CONTAINERS);

        let mounts = named(&containers, "otty-fx-running").mounts();

        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].kind(), MountKind::Volume);
        assert_eq!(mounts[0].name(), Some("otty-fx-data"));
        assert_eq!(
            mounts[0].source(),
            "/var/lib/docker/volumes/otty-fx-data/_data"
        );
        assert_eq!(mounts[0].destination(), "/data");
        assert!(!mounts[0].is_read_only());
    }

    #[test]
    fn docker_networks_carry_name_id_and_address() {
        let containers = containers(DOCKER_CONTAINERS);

        let networks = named(&containers, "otty-fx-running").networks();

        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].name(), "otty-fx-net");
        assert_eq!(
            networks[0].id(),
            Some(
                "9b5579d1a73566f77bb264f1e2736a6b5d645dc49324250df2dcd496ae346af8"
            )
        );
        assert!(networks[0].ip_address().is_some());
    }

    #[test]
    fn docker_container_that_never_ran_has_a_network_without_id() {
        let containers = containers(DOCKER_CONTAINERS);

        let created = named(&containers, "otty-fx-created");

        assert_eq!(created.state(), ContainerState::Created);
        assert_eq!(created.exit_code(), None);
        assert_eq!(created.networks()[0].name(), "bridge");
        assert_eq!(created.networks()[0].id(), None);
        assert_eq!(created.networks()[0].ip_address(), None);
    }

    #[test]
    fn exited_containers_carry_their_exit_code() {
        for json in [DOCKER_CONTAINERS, PODMAN_CONTAINERS] {
            let containers = containers(json);

            let ok = named(&containers, "otty-fx-exit0");
            let failed = named(&containers, "otty-fx-exit3");

            assert_eq!(ok.state(), ContainerState::Exited);
            assert_eq!(ok.exit_code(), Some(0));
            assert_eq!(failed.state(), ContainerState::Exited);
            assert_eq!(failed.exit_code(), Some(3));
        }
    }

    #[test]
    fn anonymous_volume_mount_is_a_volume_with_a_generated_name() {
        for json in [DOCKER_CONTAINERS, PODMAN_CONTAINERS] {
            let containers = containers(json);

            let mounts = named(&containers, "otty-fx-exit3").mounts();

            assert_eq!(mounts.len(), 1);
            assert_eq!(mounts[0].kind(), MountKind::Volume);
            assert_eq!(mounts[0].name().map(str::len), Some(64));
            assert_eq!(mounts[0].destination(), "/scratch");
        }
    }

    #[test]
    fn paused_containers_are_paused() {
        for json in [DOCKER_CONTAINERS, PODMAN_CONTAINERS] {
            let containers = containers(json);

            let paused = named(&containers, "otty-fx-paused");

            assert_eq!(paused.state(), ContainerState::Paused);
            assert_eq!(paused.exit_code(), None);
        }
    }

    #[test]
    fn compose_project_comes_from_the_compose_label() {
        for json in [DOCKER_CONTAINERS, PODMAN_CONTAINERS] {
            let containers = containers(json);

            let web = named(&containers, "otty-fx-web-1");

            assert_eq!(web.compose_project(), Some("otty-fx"));
            assert_eq!(web.state(), ContainerState::Running);
        }
    }

    #[test]
    fn podman_running_container_is_mapped() {
        let containers = containers(PODMAN_CONTAINERS);

        let running = named(&containers, "otty-fx-running");

        assert_eq!(running.state(), ContainerState::Running);
        assert_eq!(running.image(), "docker.io/library/busybox:1.37");
        assert_eq!(
            running.image_id(),
            "sha256:30ecbe1509090b38ef3260f3609d64d8261230464eaa4d235901db496e9647fb"
        );
        assert_eq!(running.ports().len(), 1);
        assert_eq!(running.ports()[0].host_port(), Some(18080));
        assert_eq!(running.mounts()[0].name(), Some("otty-fx-data"));
        assert_eq!(running.networks()[0].name(), "otty-fx-net");
        assert_eq!(running.created(), Some(1_793_557_926));
    }

    #[test]
    fn podman_rootless_default_network_is_pasta() {
        let containers = containers(PODMAN_CONTAINERS);

        let networks = named(&containers, "otty-fx-exit0").networks();

        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].name(), "pasta");
    }

    #[test]
    fn podman_paused_container_has_no_networks() {
        let containers = containers(PODMAN_CONTAINERS);

        let paused = named(&containers, "otty-fx-paused");

        assert!(paused.networks().is_empty());
        assert!(paused.mounts().is_empty());
    }

    #[test]
    fn networks_are_sorted_by_name() {
        let container = Container::from_summary(&summary(
            r#"{"NetworkSettings":{"Networks":{"zeta":{"NetworkID":"z"},"alpha":{"NetworkID":"a"}}}}"#,
        ));

        let names: Vec<&str> = container
            .networks()
            .iter()
            .map(ContainerNetwork::name)
            .collect();

        assert_eq!(names, ["alpha", "zeta"]);
    }

    #[test]
    fn networks_are_listed_a_to_z_whatever_the_case() {
        let container = Container::from_summary(&summary(
            r#"{"NetworkSettings":{"Networks":{"Zed":{"NetworkID":"z"},"beta":{"NetworkID":"b"},"Alpha":{"NetworkID":"a"}}}}"#,
        ));

        let names: Vec<&str> = container
            .networks()
            .iter()
            .map(ContainerNetwork::name)
            .collect();

        assert_eq!(names, ["Alpha", "beta", "Zed"]);
    }

    #[test]
    fn empty_summary_gives_blank_unknown_container() {
        let container = Container::from_summary(&summary("{}"));

        assert_eq!(container.id(), "");
        assert_eq!(container.name(), "");
        assert_eq!(container.state(), ContainerState::Unknown);
        assert_eq!(container.status(), "");
        assert_eq!(container.created(), None);
        assert!(container.ports().is_empty());
    }

    #[test]
    fn every_engine_state_is_mapped() {
        let cases = [
            ("created", ContainerState::Created),
            ("running", ContainerState::Running),
            ("paused", ContainerState::Paused),
            ("restarting", ContainerState::Restarting),
            ("removing", ContainerState::Removing),
            ("exited", ContainerState::Exited),
            ("dead", ContainerState::Dead),
            ("stopping", ContainerState::Stopping),
            ("", ContainerState::Unknown),
        ];

        for (state, expected) in cases {
            let json = format!(r#"{{"State":"{state}"}}"#);

            let container = Container::from_summary(&summary(&json));

            assert_eq!(container.state(), expected, "{state}");
        }
    }

    #[test]
    fn mount_kinds_and_read_only_are_mapped() {
        let container = Container::from_summary(&summary(
            r#"{"Mounts":[
                {"Type":"bind","Source":"/srv","Destination":"/srv","RW":false},
                {"Type":"tmpfs","Destination":"/run"},
                {"Destination":"/x"}
            ]}"#,
        ));

        let mounts = container.mounts();

        assert_eq!(mounts[0].kind(), MountKind::Bind);
        assert!(mounts[0].is_read_only());
        assert_eq!(mounts[0].name(), None);
        assert_eq!(mounts[1].kind(), MountKind::Other);
        assert!(!mounts[1].is_read_only());
        assert_eq!(mounts[2].kind(), MountKind::Other);
        assert_eq!(mounts[2].source(), "");
    }

    #[test]
    fn exposed_port_without_host_side_is_kept() {
        let container = Container::from_summary(&summary(
            r#"{"Ports":[{"PrivatePort":53,"Type":"udp"},{"PrivatePort":9000,"Type":""}]}"#,
        ));

        let ports = container.ports();

        assert_eq!(ports[0].host_ip(), None);
        assert_eq!(ports[0].host_port(), None);
        assert_eq!(ports[0].protocol(), "udp");
        assert_eq!(ports[1].protocol(), "tcp");
    }

    #[test]
    fn blank_compose_label_is_no_project() {
        let container = Container::from_summary(&summary(
            r#"{"Labels":{"com.docker.compose.project":" "}}"#,
        ));

        assert_eq!(container.compose_project(), None);
    }

    #[test]
    fn exit_code_is_read_from_exited_status_only() {
        assert_eq!(exit_code_from_status("Exited (137) 2 days ago"), Some(137));
        assert_eq!(
            exit_code_from_status("Exited (0) Less than a second ago"),
            Some(0)
        );
        assert_eq!(exit_code_from_status("Exited (-1) 1 hour ago"), Some(-1));
        assert_eq!(exit_code_from_status("Up 3 seconds (Paused)"), None);
        assert_eq!(exit_code_from_status("Created"), None);
        assert_eq!(exit_code_from_status("Exited (x) 1 hour ago"), None);
        assert_eq!(exit_code_from_status("Exited"), None);
    }
}
