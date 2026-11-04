//! What OTTY knows about a reached engine, read from its `/version` answer.

use bollard::models::SystemVersion;

use crate::address::EngineAddress;

/// The component name Podman puts in its `/version` answer.
const PODMAN_COMPONENT: &str = "Podman Engine";
/// The component name Docker puts in its `/version` answer.
const DOCKER_COMPONENT: &str = "Engine";

/// Which engine answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineKind {
    /// Docker Engine or Docker Desktop.
    Docker,
    /// Podman, through its Docker-compatible API service.
    Podman,
}

/// A local engine that answered `/version`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EngineInfo {
    kind: EngineKind,
    version: Option<String>,
    api_version: Option<String>,
    address: EngineAddress,
}

impl EngineInfo {
    /// Docker or Podman.
    pub fn kind(&self) -> EngineKind {
        self.kind
    }

    /// The engine's own version, for example `29.0.1` or `5.8.0`.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// The newest Docker Engine API version the engine speaks, for example
    /// `1.47`.
    pub fn api_version(&self) -> Option<&str> {
        self.api_version.as_deref()
    }

    /// Where the engine listens.
    pub fn address(&self) -> &EngineAddress {
        &self.address
    }

    /// Read the engine kind and versions from a `/version` answer.
    ///
    /// A "Podman Engine" component means Podman; anything else is Docker.
    pub(crate) fn from_version(
        answer: &SystemVersion,
        address: EngineAddress,
    ) -> Self {
        let components = answer.components.as_deref().unwrap_or_default();
        let component_version = |name: &str| {
            components
                .iter()
                .find(|component| component.name == name)
                .map(|component| component.version.as_str())
        };

        let podman_version = component_version(PODMAN_COMPONENT);
        let (kind, component) = match podman_version {
            Some(version) => (EngineKind::Podman, Some(version)),
            None => (EngineKind::Docker, component_version(DOCKER_COMPONENT)),
        };
        let version =
            non_blank(answer.version.as_deref()).or(non_blank(component));

        Self {
            kind,
            version: version.map(String::from),
            api_version: non_blank(answer.api_version.as_deref())
                .map(String::from),
            address,
        }
    }
}

/// Treat empty or blank text as missing.
fn non_blank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const DOCKER_VERSION: &str =
        include_str!("../tests/fixtures/docker-version.json");
    const PODMAN_VERSION: &str =
        include_str!("../tests/fixtures/podman-version.json");

    fn address() -> EngineAddress {
        EngineAddress::Unix(PathBuf::from("/var/run/docker.sock"))
    }

    fn parse(json: &str) -> SystemVersion {
        serde_json::from_str(json).expect("fixture is a /version answer")
    }

    #[test]
    fn docker_answer_is_docker() {
        let info = EngineInfo::from_version(&parse(DOCKER_VERSION), address());

        assert_eq!(info.kind(), EngineKind::Docker);
        assert_eq!(info.version(), Some("29.9.1"));
        assert_eq!(info.api_version(), Some("1.56"));
        assert_eq!(info.address(), &address());
    }

    #[test]
    fn podman_answer_is_podman() {
        let info = EngineInfo::from_version(&parse(PODMAN_VERSION), address());

        assert_eq!(info.kind(), EngineKind::Podman);
        assert_eq!(info.version(), Some("5.8.8"));
        assert_eq!(info.api_version(), Some("1.44"));
    }

    #[test]
    fn podman_without_top_level_version_uses_its_component() {
        let answer = parse(
            r#"{"Components":[{"Name":"Podman Engine","Version":"4.9.4"}]}"#,
        );

        let info = EngineInfo::from_version(&answer, address());

        assert_eq!(info.kind(), EngineKind::Podman);
        assert_eq!(info.version(), Some("4.9.4"));
        assert_eq!(info.api_version(), None);
    }

    #[test]
    fn docker_without_top_level_version_uses_its_engine_component() {
        let answer = parse(
            r#"{"Components":[{"Name":"containerd","Version":"v2.0.0"},{"Name":"Engine","Version":"27.3.1"}]}"#,
        );

        let info = EngineInfo::from_version(&answer, address());

        assert_eq!(info.kind(), EngineKind::Docker);
        assert_eq!(info.version(), Some("27.3.1"));
    }

    #[test]
    fn empty_answer_is_docker_without_version() {
        let info = EngineInfo::from_version(&parse("{}"), address());

        assert_eq!(info.kind(), EngineKind::Docker);
        assert_eq!(info.version(), None);
    }

    #[test]
    fn blank_version_counts_as_missing() {
        let answer = parse(r#"{"Version":"","ApiVersion":" "}"#);

        let info = EngineInfo::from_version(&answer, address());

        assert_eq!(info.version(), None);
        assert_eq!(info.api_version(), None);
    }
}
