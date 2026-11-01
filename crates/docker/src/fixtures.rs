//! Values mapped from the engine answers captured in `tests/fixtures`
//! (Docker 29.9.1 and Podman 5.8.8), for the tests of crates that use this
//! one. The plain types have no public constructors, so tests outside this
//! crate get real values from here. Only built with the `fixtures` feature.
//!
//! # Example
//!
//! ```
//! use otty_docker::{EngineKind, fixtures};
//!
//! let engine = fixtures::engine(EngineKind::Docker);
//! let resources = fixtures::resources(EngineKind::Docker);
//!
//! assert_eq!(engine.version(), Some("29.9.1"));
//! assert!(!resources.containers().is_empty());
//! ```

use std::path::PathBuf;

use bollard::models::{
    ContainerSummary, ImageSummary, Network, SystemVersion, VolumeListResponse,
};

use crate::address::EngineAddress;
use crate::candidates::{CandidateSource, SocketCandidate};
use crate::cli::CliPresence;
use crate::connection::Resources;
use crate::discovery::Discovery;
use crate::engine::{EngineInfo, EngineKind};
use crate::probe::{Probe, ProbeOutcome};

/// The fixtures are part of the crate, so one that does not parse is a
/// build defect.
const BROKEN_FIXTURE: &str = "captured fixture should parse";

/// Where the fixture Docker engine listens.
pub const DOCKER_SOCKET: &str = "/var/run/docker.sock";
/// Where the fixture Podman engine listens.
pub const PODMAN_SOCKET: &str = "/run/user/1000/podman/podman.sock";

/// The engine that sent the captured `/version` answer of `kind`.
pub fn engine(kind: EngineKind) -> EngineInfo {
    let (answer, socket) = match kind {
        EngineKind::Docker => (
            include_str!("../tests/fixtures/docker-version.json"),
            DOCKER_SOCKET,
        ),
        EngineKind::Podman => (
            include_str!("../tests/fixtures/podman-version.json"),
            PODMAN_SOCKET,
        ),
    };

    let version: SystemVersion =
        serde_json::from_str(answer).expect(BROKEN_FIXTURE);

    EngineInfo::from_version(
        &version,
        EngineAddress::Unix(PathBuf::from(socket)),
    )
}

/// A search that reached the fixture engines of `kinds`, in that order.
/// With no kinds nothing answered and no command line tool is on `PATH`,
/// so the diagnosis is [`crate::Diagnosis::NotInstalled`].
pub fn discovery(kinds: &[EngineKind]) -> Discovery {
    let probes = kinds
        .iter()
        .map(|kind| {
            let info = engine(*kind);
            let source = match kind {
                EngineKind::Docker => CandidateSource::Docker,
                EngineKind::Podman => CandidateSource::Podman,
            };
            let candidate =
                SocketCandidate::new(info.address().clone(), source);

            Probe::new(candidate, ProbeOutcome::Reached(info))
        })
        .collect();

    Discovery::new(probes, CliPresence::new(false, false))
}

/// The four captured lists of `kind`, mapped as [`crate::Engine::resources`]
/// maps them.
pub fn resources(kind: EngineKind) -> Resources {
    let (containers, images, volumes, networks) = match kind {
        EngineKind::Docker => (
            include_str!("../tests/fixtures/docker-containers.json"),
            include_str!("../tests/fixtures/docker-images.json"),
            include_str!("../tests/fixtures/docker-volumes.json"),
            include_str!("../tests/fixtures/docker-networks.json"),
        ),
        EngineKind::Podman => (
            include_str!("../tests/fixtures/podman-containers.json"),
            include_str!("../tests/fixtures/podman-images.json"),
            include_str!("../tests/fixtures/podman-volumes.json"),
            include_str!("../tests/fixtures/podman-networks.json"),
        ),
    };

    let containers: Vec<ContainerSummary> =
        serde_json::from_str(containers).expect(BROKEN_FIXTURE);
    let images: Vec<ImageSummary> =
        serde_json::from_str(images).expect(BROKEN_FIXTURE);
    let volumes: VolumeListResponse =
        serde_json::from_str(volumes).expect(BROKEN_FIXTURE);
    let networks: Vec<Network> =
        serde_json::from_str(networks).expect(BROKEN_FIXTURE);

    Resources::from_answers(
        &containers,
        &images,
        volumes.volumes.as_deref().unwrap_or_default(),
        &networks,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnosis::Diagnosis;

    #[test]
    fn each_engine_kind_has_its_engine_and_lists() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            let engine = engine(kind);
            let resources = resources(kind);

            assert_eq!(engine.kind(), kind);
            assert_eq!(resources.containers().len(), 6);
            assert!(!resources.images().is_empty());
            assert!(!resources.volumes().is_empty());
            assert!(!resources.networks().is_empty());
        }
    }

    #[test]
    fn a_discovery_lists_the_engines_it_reached_or_says_none() {
        let both = discovery(&[EngineKind::Docker, EngineKind::Podman]);
        let none = discovery(&[]);

        assert_eq!(
            both.diagnosis(),
            &Diagnosis::Engines(vec![
                engine(EngineKind::Docker),
                engine(EngineKind::Podman)
            ])
        );
        assert_eq!(none.diagnosis(), &Diagnosis::NotInstalled);
    }
}
