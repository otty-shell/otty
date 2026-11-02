//! What to run: the settings of a new container (spec Decision 18) and how
//! they become the engine's create request.

use std::fmt;
use std::path::PathBuf;

use bollard::models::{
    ContainerCreateBody, HostConfig, PortBinding, PortMap,
    RestartPolicy as EngineRestartPolicy, RestartPolicyNameEnum,
};
use bollard::query_parameters::CreateContainerOptions;

/// The protocol of a published port.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PortProtocol {
    #[default]
    Tcp,
    Udp,
}

impl fmt::Display for PortProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tcp => f.write_str("tcp"),
            Self::Udp => f.write_str("udp"),
        }
    }
}

/// A container port reachable on this computer, `docker run -p`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedPort {
    host: Option<u16>,
    container: u16,
    protocol: PortProtocol,
}

impl PublishedPort {
    /// The port on this computer; `None` lets the engine pick a free one.
    pub fn host(&self) -> Option<u16> {
        self.host
    }

    /// The port the container listens on.
    pub fn container(&self) -> u16 {
        self.container
    }

    /// `tcp` or `udp`.
    pub fn protocol(&self) -> PortProtocol {
        self.protocol
    }

    /// Publish `container` on `host` (`None`: any free port) over
    /// `protocol`.
    pub fn new(
        host: Option<u16>,
        container: u16,
        protocol: PortProtocol,
    ) -> Self {
        Self {
            host,
            container,
            protocol,
        }
    }

    /// The engine's key of the container port, for example `80/tcp`.
    fn key(&self) -> String {
        format!("{}/{}", self.container, self.protocol)
    }
}

/// Where the data of a volume row comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MountSource {
    /// A folder on this computer (a bind mount); an absolute path.
    HostPath(PathBuf),
    /// A named volume of the engine; the engine creates it when missing.
    Volume(String),
}

/// A folder or volume inside the container, `docker run -v`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeMount {
    source: MountSource,
    target: String,
    is_read_only: bool,
    is_relabeled: bool,
}

impl VolumeMount {
    /// The folder on this computer or the volume name.
    pub fn source(&self) -> &MountSource {
        &self.source
    }

    /// The absolute path inside the container.
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Whether the container can only read it.
    pub fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// Whether the engine relabels the folder for SELinux (the `z` bind
    /// option). Only a folder on this computer is ever relabeled.
    pub fn is_relabeled(&self) -> bool {
        self.is_relabeled
    }

    /// Mount `source` at `target` inside the container, without relabeling.
    pub fn new(source: MountSource, target: &str, is_read_only: bool) -> Self {
        Self {
            source,
            target: target.to_string(),
            is_read_only,
            is_relabeled: false,
        }
    }

    /// Ask the engine to relabel the folder so that the container can read
    /// it under SELinux (`z`). It changes the labels of the folder's files
    /// on this computer. A named volume ignores it.
    pub fn with_relabel(mut self, is_relabeled: bool) -> Self {
        self.is_relabeled = is_relabeled;
        self
    }

    /// The engine's bind text: `source:target`, then the options `ro` and
    /// `z` joined by a comma when set, such as `source:target:ro,z`.
    /// `docker run -v` sends the same text for folders and named volumes.
    fn bind(&self) -> String {
        let (source, is_relabeled) = match &self.source {
            MountSource::HostPath(path) => {
                (path.display().to_string(), self.is_relabeled)
            },
            MountSource::Volume(name) => (name.clone(), false),
        };
        let target = &self.target;

        let mut options: Vec<&str> = Vec::new();
        if self.is_read_only {
            options.push("ro");
        }
        if is_relabeled {
            options.push("z");
        }

        if options.is_empty() {
            format!("{source}:{target}")
        } else {
            format!("{source}:{target}:{}", options.join(","))
        }
    }
}

/// When the engine starts the container again on its own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Never (`no`).
    #[default]
    No,
    /// When it exits with an error (`on-failure`).
    OnFailure,
    /// Always, until the user stops it (`unless-stopped`).
    UnlessStopped,
    /// Always, even after the user stopped it and the engine restarted
    /// (`always`).
    Always,
}

impl RestartPolicy {
    /// The engine's name of the policy.
    fn engine_name(self) -> RestartPolicyNameEnum {
        match self {
            Self::No => RestartPolicyNameEnum::NO,
            Self::OnFailure => RestartPolicyNameEnum::ON_FAILURE,
            Self::UnlessStopped => RestartPolicyNameEnum::UNLESS_STOPPED,
            Self::Always => RestartPolicyNameEnum::ALWAYS,
        }
    }
}

/// Everything needed to create and start one container (spec Decision 18):
/// the image, an optional name and command, published ports, environment,
/// volumes, network, restart policy and auto remove. The container runs
/// detached, without a terminal.
///
/// ```
/// use otty_docker::{PortProtocol, PublishedPort, RestartPolicy, RunRequest};
///
/// let request = RunRequest::new("nginx:1.27")
///     .with_name("web")
///     .with_port(PublishedPort::new(Some(8081), 80, PortProtocol::Tcp))
///     .with_restart(RestartPolicy::UnlessStopped);
///
/// assert_eq!(request.image(), "nginx:1.27");
/// assert_eq!(request.name(), Some("web"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRequest {
    image: String,
    name: Option<String>,
    command: Vec<String>,
    ports: Vec<PublishedPort>,
    env: Vec<(String, String)>,
    volumes: Vec<VolumeMount>,
    network: Option<String>,
    restart: RestartPolicy,
    is_auto_remove: bool,
}

impl RunRequest {
    /// The image reference, for example `nginx:1.27`.
    pub fn image(&self) -> &str {
        &self.image
    }

    /// The container name; `None` lets the engine pick one.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The command that replaces the image's own; empty keeps it.
    pub fn command(&self) -> &[String] {
        &self.command
    }

    /// The published ports.
    pub fn ports(&self) -> &[PublishedPort] {
        &self.ports
    }

    /// The environment variables, name and value.
    pub fn env(&self) -> &[(String, String)] {
        &self.env
    }

    /// The folders and volumes inside the container.
    pub fn volumes(&self) -> &[VolumeMount] {
        &self.volumes
    }

    /// The network to connect to; `None` uses the engine's default.
    pub fn network(&self) -> Option<&str> {
        self.network.as_deref()
    }

    /// When the engine starts the container again.
    pub fn restart(&self) -> RestartPolicy {
        self.restart
    }

    /// Whether the engine removes the container when it stops.
    pub fn is_auto_remove(&self) -> bool {
        self.is_auto_remove
    }

    /// Run `image` with the engine's defaults for everything else.
    pub fn new(image: &str) -> Self {
        Self {
            image: image.to_string(),
            name: None,
            command: Vec::new(),
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            network: None,
            restart: RestartPolicy::No,
            is_auto_remove: false,
        }
    }

    /// Name the container.
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Replace the image's command.
    pub fn with_command(mut self, command: Vec<String>) -> Self {
        self.command = command;
        self
    }

    /// Publish one more port.
    pub fn with_port(mut self, port: PublishedPort) -> Self {
        self.ports.push(port);
        self
    }

    /// Set one more environment variable.
    pub fn with_env(mut self, name: &str, value: &str) -> Self {
        self.env.push((name.to_string(), value.to_string()));
        self
    }

    /// Mount one more folder or volume.
    pub fn with_volume(mut self, volume: VolumeMount) -> Self {
        self.volumes.push(volume);
        self
    }

    /// Connect the container to `network`.
    pub fn with_network(mut self, network: &str) -> Self {
        self.network = Some(network.to_string());
        self
    }

    /// Set the restart policy.
    pub fn with_restart(mut self, restart: RestartPolicy) -> Self {
        self.restart = restart;
        self
    }

    /// Remove the container when it stops.
    pub fn with_auto_remove(mut self, is_auto_remove: bool) -> Self {
        self.is_auto_remove = is_auto_remove;
        self
    }

    /// The query of the create request: the name, when one is set.
    pub(crate) fn create_options(&self) -> Option<CreateContainerOptions> {
        let name = self.name.clone()?;

        Some(CreateContainerOptions {
            name: Some(name),
            ..CreateContainerOptions::default()
        })
    }

    /// The body of the create request. A container that is removed when it
    /// stops gets the `no` restart policy, because the engine refuses auto
    /// remove together with any other policy.
    pub(crate) fn create_body(&self) -> ContainerCreateBody {
        let restart = if self.is_auto_remove {
            RestartPolicy::No
        } else {
            self.restart
        };

        let host_config = HostConfig {
            binds: non_empty(
                self.volumes.iter().map(VolumeMount::bind).collect(),
            ),
            port_bindings: (!self.ports.is_empty())
                .then(|| port_bindings(&self.ports)),
            network_mode: self.network.clone(),
            restart_policy: Some(EngineRestartPolicy {
                name: Some(restart.engine_name()),
                maximum_retry_count: None,
            }),
            auto_remove: Some(self.is_auto_remove),
            ..HostConfig::default()
        };

        ContainerCreateBody {
            image: Some(self.image.clone()),
            cmd: non_empty(self.command.clone()),
            env: non_empty(
                self.env
                    .iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect(),
            ),
            exposed_ports: non_empty(exposed_ports(&self.ports)),
            host_config: Some(host_config),
            ..ContainerCreateBody::default()
        }
    }
}

/// The container ports to expose, each once, in the order typed.
fn exposed_ports(ports: &[PublishedPort]) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for port in ports {
        let key = port.key();
        if !keys.contains(&key) {
            keys.push(key);
        }
    }

    keys
}

/// The host side of each published port, grouped by container port: one
/// container port can be published on several host ports.
fn port_bindings(ports: &[PublishedPort]) -> PortMap {
    let mut map = PortMap::new();
    for port in ports {
        let binding = PortBinding {
            host_ip: None,
            host_port: port.host.map(|host| host.to_string()),
        };
        map.entry(port.key())
            .or_insert_with(|| Some(Vec::new()))
            .get_or_insert_with(Vec::new)
            .push(binding);
    }

    map
}

/// `None` for an empty list, so the engine keeps its default.
fn non_empty(items: Vec<String>) -> Option<Vec<String>> {
    (!items.is_empty()).then_some(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_json(request: &RunRequest) -> serde_json::Value {
        serde_json::to_value(request.create_body()).expect("body as json")
    }

    #[test]
    fn a_bare_request_sends_only_the_image_and_the_defaults() {
        let request = RunRequest::new("nginx:1.27");

        let body = body_json(&request);

        assert_eq!(body["Image"], "nginx:1.27");
        assert!(body["Cmd"].is_null(), "{body}");
        assert!(body["Env"].is_null(), "{body}");
        assert!(body["ExposedPorts"].is_null(), "{body}");
        assert!(body["HostConfig"]["Binds"].is_null(), "{body}");
        assert!(body["HostConfig"]["PortBindings"].is_null(), "{body}");
        assert!(body["HostConfig"]["NetworkMode"].is_null(), "{body}");
        assert_eq!(body["HostConfig"]["RestartPolicy"]["Name"], "no");
        assert_eq!(body["HostConfig"]["AutoRemove"], false);
        assert!(request.create_options().is_none());
    }

    #[test]
    fn the_name_goes_into_the_query() {
        let request = RunRequest::new("nginx").with_name("otty-fx-web");

        let options = request.create_options().expect("options");

        assert_eq!(options.name.as_deref(), Some("otty-fx-web"));
    }

    #[test]
    fn ports_are_exposed_and_published_with_their_protocol() {
        let request = RunRequest::new("nginx")
            .with_port(PublishedPort::new(Some(8081), 80, PortProtocol::Tcp))
            .with_port(PublishedPort::new(Some(5353), 53, PortProtocol::Udp))
            .with_port(PublishedPort::new(None, 443, PortProtocol::Tcp));

        let body = body_json(&request);

        let exposed = body["ExposedPorts"].as_object().expect("exposed ports");
        assert_eq!(exposed.len(), 3);
        assert!(exposed.contains_key("80/tcp"));
        assert!(exposed.contains_key("53/udp"));
        assert!(exposed.contains_key("443/tcp"));
        let bindings = &body["HostConfig"]["PortBindings"];
        assert_eq!(bindings["80/tcp"][0]["HostPort"], "8081");
        assert_eq!(bindings["53/udp"][0]["HostPort"], "5353");
        assert!(bindings["443/tcp"][0]["HostPort"].is_null(), "{bindings}");
    }

    #[test]
    fn one_container_port_can_be_published_on_two_host_ports() {
        let request = RunRequest::new("nginx")
            .with_port(PublishedPort::new(Some(8081), 80, PortProtocol::Tcp))
            .with_port(PublishedPort::new(Some(8082), 80, PortProtocol::Tcp));

        let body = body_json(&request);

        let bindings = body["HostConfig"]["PortBindings"]["80/tcp"]
            .as_array()
            .expect("two bindings");
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[1]["HostPort"], "8082");
        assert_eq!(
            body["ExposedPorts"].as_object().map(|map| map.len()),
            Some(1)
        );
    }

    #[test]
    fn environment_variables_are_name_equals_value() {
        let request = RunRequest::new("nginx")
            .with_env("NGINX_ENTRYPOINT_QUIET_LOGS", "1")
            .with_env("GREETING", "a=b c");

        let body = body_json(&request);

        assert_eq!(
            body["Env"],
            serde_json::json!([
                "NGINX_ENTRYPOINT_QUIET_LOGS=1",
                "GREETING=a=b c"
            ])
        );
    }

    #[test]
    fn folders_and_named_volumes_are_bound_and_read_only_is_kept() {
        let request = RunRequest::new("nginx")
            .with_volume(VolumeMount::new(
                MountSource::HostPath(PathBuf::from("/srv/site")),
                "/usr/share/nginx/html",
                true,
            ))
            .with_volume(VolumeMount::new(
                MountSource::Volume(String::from("otty-fx-cache")),
                "/cache",
                false,
            ));

        let body = body_json(&request);

        assert_eq!(
            body["HostConfig"]["Binds"],
            serde_json::json!([
                "/srv/site:/usr/share/nginx/html:ro",
                "otty-fx-cache:/cache"
            ])
        );
    }

    #[test]
    fn the_bind_text_of_a_folder_has_ro_and_z_for_each_combination() {
        let folder = |is_read_only: bool, is_relabeled: bool| {
            VolumeMount::new(
                MountSource::HostPath(PathBuf::from("/srv/site")),
                "/site",
                is_read_only,
            )
            .with_relabel(is_relabeled)
            .bind()
        };

        assert_eq!(folder(false, false), "/srv/site:/site");
        assert_eq!(folder(true, false), "/srv/site:/site:ro");
        assert_eq!(folder(false, true), "/srv/site:/site:z");
        assert_eq!(folder(true, true), "/srv/site:/site:ro,z");
    }

    #[test]
    fn a_named_volume_never_gets_the_relabel_option() {
        let volume = |is_read_only: bool| {
            VolumeMount::new(
                MountSource::Volume(String::from("otty-fx-cache")),
                "/cache",
                is_read_only,
            )
            .with_relabel(true)
            .bind()
        };

        assert_eq!(volume(false), "otty-fx-cache:/cache");
        assert_eq!(volume(true), "otty-fx-cache:/cache:ro");
    }

    #[test]
    fn a_relabeled_folder_reaches_the_create_body() {
        let request = RunRequest::new("nginx").with_volume(
            VolumeMount::new(
                MountSource::HostPath(PathBuf::from("/srv/site")),
                "/site",
                false,
            )
            .with_relabel(true),
        );

        let body = body_json(&request);

        assert_eq!(
            body["HostConfig"]["Binds"],
            serde_json::json!(["/srv/site:/site:z"])
        );
        assert!(request.volumes()[0].is_relabeled());
    }

    #[test]
    fn the_network_is_the_network_mode() {
        let request = RunRequest::new("nginx").with_network("otty-fx-net");

        let body = body_json(&request);

        assert_eq!(body["HostConfig"]["NetworkMode"], "otty-fx-net");
    }

    #[test]
    fn each_restart_policy_has_its_engine_name() {
        let cases = [
            (RestartPolicy::No, "no"),
            (RestartPolicy::OnFailure, "on-failure"),
            (RestartPolicy::UnlessStopped, "unless-stopped"),
            (RestartPolicy::Always, "always"),
        ];

        for (policy, name) in cases {
            let body =
                body_json(&RunRequest::new("nginx").with_restart(policy));

            assert_eq!(body["HostConfig"]["RestartPolicy"]["Name"], name);
        }
    }

    #[test]
    fn auto_remove_forces_the_no_restart_policy() {
        let request = RunRequest::new("nginx")
            .with_restart(RestartPolicy::Always)
            .with_auto_remove(true);

        let body = body_json(&request);

        assert_eq!(body["HostConfig"]["AutoRemove"], true);
        assert_eq!(body["HostConfig"]["RestartPolicy"]["Name"], "no");
    }

    #[test]
    fn a_command_replaces_the_images_own() {
        let request = RunRequest::new("busybox").with_command(vec![
            String::from("sh"),
            String::from("-c"),
            String::from("echo hi"),
        ]);

        let body = body_json(&request);

        assert_eq!(body["Cmd"], serde_json::json!(["sh", "-c", "echo hi"]));
    }

    #[test]
    fn protocols_read_as_the_engine_writes_them() {
        assert_eq!(PortProtocol::Tcp.to_string(), "tcp");
        assert_eq!(PortProtocol::Udp.to_string(), "udp");
    }
}
