//! A volume as OTTY shows it, mapped from the engine's volume list.

use std::collections::BTreeMap;

#[cfg(feature = "client")]
use bollard::models::Volume as EngineVolume;

#[cfg(feature = "client")]
use crate::container::{COMPOSE_PROJECT_LABEL, Container, MountKind};
#[cfg(feature = "client")]
use crate::text_order::compare_text;
#[cfg(feature = "client")]
use crate::timestamp::unix_seconds;

/// The label Docker (23 and newer) puts on volumes it made for a
/// `-v /path` mount without a name.
#[cfg(feature = "client")]
const ANONYMOUS_LABEL: &str = "com.docker.volume.anonymous";
/// Engines name anonymous volumes with 64 hex digits.
#[cfg(feature = "client")]
const ANONYMOUS_NAME_LEN: usize = 64;

/// One volume of an engine (spec Decision 10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    name: String,
    driver: String,
    mountpoint: String,
    created: Option<i64>,
    labels: BTreeMap<String, String>,
    compose_project: Option<String>,
    containers: Vec<String>,
    anonymous: bool,
}

impl Volume {
    /// The volume name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The volume driver, usually `local`.
    pub fn driver(&self) -> &str {
        &self.driver
    }

    /// Where the engine keeps the data on its host.
    pub fn mountpoint(&self) -> &str {
        &self.mountpoint
    }

    /// When the volume was made, in Unix seconds.
    pub fn created(&self) -> Option<i64> {
        self.created
    }

    /// The volume's labels, sorted by key.
    pub fn labels(&self) -> &BTreeMap<String, String> {
        &self.labels
    }

    /// Names of the containers (running or not) that mount it, sorted. The
    /// engine refuses to remove a volume while this is not empty.
    pub fn containers(&self) -> &[String] {
        &self.containers
    }

    /// The Compose project it belongs to, from the
    /// `com.docker.compose.project` label; blank means none.
    pub fn compose_project(&self) -> Option<&str> {
        self.compose_project.as_deref()
    }

    /// Whether the engine made it for a mount without a name. Docker's prune
    /// removes only these unless asked for all unused volumes.
    pub fn is_anonymous(&self) -> bool {
        self.anonymous
    }
}

#[cfg(feature = "client")]
impl Volume {
    /// Map one entry of the engine's volume list. Who uses it comes from
    /// `containers`.
    pub(crate) fn from_volume(
        volume: &EngineVolume,
        containers: &[Container],
    ) -> Self {
        let labels: BTreeMap<String, String> = volume
            .labels
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let anonymous = labels.contains_key(ANONYMOUS_LABEL)
            || is_generated_name(&volume.name);
        let compose_project = labels
            .get(COMPOSE_PROJECT_LABEL)
            .map(|project| project.trim())
            .filter(|project| !project.is_empty())
            .map(String::from);

        let mut users: Vec<String> = containers
            .iter()
            .filter(|container| mounts_volume(container, &volume.name))
            .map(|container| container.name().to_owned())
            .collect();
        users.sort_by(|a, b| compare_text(a, b));

        Self {
            name: volume.name.clone(),
            driver: volume.driver.clone(),
            mountpoint: volume.mountpoint.clone(),
            created: volume.created_at.as_deref().and_then(unix_seconds),
            labels,
            compose_project,
            containers: users,
            anonymous,
        }
    }
}

#[cfg(feature = "client")]
fn mounts_volume(container: &Container, volume: &str) -> bool {
    container.mounts().iter().any(|mount| {
        mount.kind() == MountKind::Volume && mount.name() == Some(volume)
    })
}

/// Podman marks anonymous volumes only in its own API, so a name of 64 hex
/// digits (what both engines generate) counts too.
#[cfg(feature = "client")]
fn is_generated_name(name: &str) -> bool {
    name.len() == ANONYMOUS_NAME_LEN
        && name.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(all(test, feature = "client"))]
mod tests {
    use bollard::models::VolumeListResponse;

    use super::*;
    use crate::test_support::{
        DOCKER_CONTAINERS, PODMAN_CONTAINERS, containers,
    };

    const DOCKER_VOLUMES: &str =
        include_str!("../tests/fixtures/docker-volumes.json");
    const PODMAN_VOLUMES: &str =
        include_str!("../tests/fixtures/podman-volumes.json");

    fn volumes(volumes_json: &str, containers_json: &str) -> Vec<Volume> {
        let answer: VolumeListResponse =
            serde_json::from_str(volumes_json).expect("volume list fixture");
        let containers = containers(containers_json);

        answer
            .volumes
            .unwrap_or_default()
            .iter()
            .map(|volume| Volume::from_volume(volume, &containers))
            .collect()
    }

    fn named<'a>(volumes: &'a [Volume], name: &str) -> &'a Volume {
        volumes
            .iter()
            .find(|volume| volume.name() == name)
            .unwrap_or_else(|| panic!("no volume {name}"))
    }

    fn anonymous(volumes: &[Volume]) -> &Volume {
        volumes
            .iter()
            .find(|volume| volume.name() != "otty-fx-data")
            .expect("an anonymous volume")
    }

    #[test]
    fn docker_named_volume_is_mapped_with_its_users() {
        let volumes = volumes(DOCKER_VOLUMES, DOCKER_CONTAINERS);

        let data = named(&volumes, "otty-fx-data");

        assert_eq!(data.driver(), "local");
        assert_eq!(
            data.mountpoint(),
            "/var/lib/docker/volumes/otty-fx-data/_data"
        );
        assert_eq!(data.created(), Some(1_793_557_836));
        assert_eq!(
            data.labels().get("otty.fixture").map(String::as_str),
            Some("1")
        );
        assert_eq!(data.containers(), ["otty-fx-running"]);
        assert!(!data.is_anonymous());
    }

    #[test]
    fn podman_named_volume_is_mapped_with_its_users() {
        let volumes = volumes(PODMAN_VOLUMES, PODMAN_CONTAINERS);

        let data = named(&volumes, "otty-fx-data");

        assert_eq!(data.driver(), "local");
        assert_eq!(data.created(), Some(1_793_557_894));
        assert_eq!(data.containers(), ["otty-fx-running"]);
        assert!(!data.is_anonymous());
    }

    #[test]
    fn docker_anonymous_volume_is_marked_by_its_label() {
        let volumes = volumes(DOCKER_VOLUMES, DOCKER_CONTAINERS);

        let scratch = anonymous(&volumes);

        assert!(scratch.labels().contains_key(ANONYMOUS_LABEL));
        assert!(scratch.is_anonymous());
        assert_eq!(scratch.containers(), ["otty-fx-exit3"]);
    }

    #[test]
    fn podman_anonymous_volume_is_marked_by_its_name() {
        let volumes = volumes(PODMAN_VOLUMES, PODMAN_CONTAINERS);

        let scratch = anonymous(&volumes);

        assert!(scratch.labels().is_empty());
        assert!(scratch.is_anonymous());
        assert_eq!(scratch.containers(), ["otty-fx-exit3"]);
    }

    #[test]
    fn unused_volume_has_no_users() {
        let volume: EngineVolume = serde_json::from_str(
            r#"{"Name":"cache","Driver":"local","Mountpoint":"/v/cache",
                "Labels":null,"Scope":"local","Options":null}"#,
        )
        .expect("volume");
        let containers = containers(DOCKER_CONTAINERS);

        let volume = Volume::from_volume(&volume, &containers);

        assert!(volume.containers().is_empty());
        assert_eq!(volume.created(), None);
        assert!(!volume.is_anonymous());
    }

    #[test]
    fn users_are_sorted_and_bind_mounts_with_the_same_name_do_not_count() {
        let summaries: Vec<bollard::models::ContainerSummary> = serde_json::from_str(
            r#"[
                {"Names":["/zeta"],"Mounts":[{"Type":"volume","Name":"shared"}]},
                {"Names":["/alpha"],"Mounts":[{"Type":"volume","Name":"shared"}]},
                {"Names":["/bind"],"Mounts":[{"Type":"bind","Name":"shared"}]}
            ]"#,
        )
        .expect("container list");
        let containers: Vec<Container> =
            summaries.iter().map(Container::from_summary).collect();
        let volume: EngineVolume = serde_json::from_str(
            r#"{"Name":"shared","Driver":"local","Mountpoint":"","Labels":{},"Scope":"local","Options":{}}"#,
        )
        .expect("volume");

        let volume = Volume::from_volume(&volume, &containers);

        assert_eq!(volume.containers(), ["alpha", "zeta"]);
    }

    #[test]
    fn users_are_listed_a_to_z_whatever_the_case() {
        let summaries: Vec<bollard::models::ContainerSummary> = serde_json::from_str(
            r#"[
                {"Names":["/Zed"],"Mounts":[{"Type":"volume","Name":"shared"}]},
                {"Names":["/beta"],"Mounts":[{"Type":"volume","Name":"shared"}]},
                {"Names":["/Alpha"],"Mounts":[{"Type":"volume","Name":"shared"}]}
            ]"#,
        )
        .expect("container list");
        let containers: Vec<Container> =
            summaries.iter().map(Container::from_summary).collect();
        let volume: EngineVolume = serde_json::from_str(
            r#"{"Name":"shared","Driver":"local","Mountpoint":"","Labels":{},"Scope":"local","Options":{}}"#,
        )
        .expect("volume");

        let volume = Volume::from_volume(&volume, &containers);

        assert_eq!(volume.containers(), ["Alpha", "beta", "Zed"]);
    }

    #[test]
    fn hex_like_names_of_other_lengths_are_not_anonymous() {
        let volume: EngineVolume = serde_json::from_str(
            r#"{"Name":"deadbeef","Driver":"local","Mountpoint":"","Labels":{},"Scope":"local","Options":{}}"#,
        )
        .expect("volume");

        assert!(!Volume::from_volume(&volume, &[]).is_anonymous());
    }

    #[test]
    fn a_compose_volume_names_its_project() {
        let compose: EngineVolume = serde_json::from_str(
            r#"{"Name":"site_data","Driver":"local","Mountpoint":"","Scope":"local","Options":{},
                "Labels":{"com.docker.compose.project":"site","com.docker.compose.volume":"data"}}"#,
        )
        .expect("volume");
        let blank: EngineVolume = serde_json::from_str(
            r#"{"Name":"odd","Driver":"local","Mountpoint":"","Scope":"local","Options":{},
                "Labels":{"com.docker.compose.project":" "}}"#,
        )
        .expect("volume");

        assert_eq!(
            Volume::from_volume(&compose, &[]).compose_project(),
            Some("site")
        );
        assert_eq!(Volume::from_volume(&blank, &[]).compose_project(), None);
        let volumes = volumes(DOCKER_VOLUMES, DOCKER_CONTAINERS);
        assert_eq!(named(&volumes, "otty-fx-data").compose_project(), None);
    }
}
