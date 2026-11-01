//! What the engine search means for the user (spec Decision 9).

use crate::candidates::{CandidateSource, SocketCandidate};
use crate::cli::CliPresence;
use crate::engine::EngineInfo;
use crate::probe::{Probe, ProbeOutcome};

/// The answer to "can OTTY reach an engine, and if not, why".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnosis {
    /// At least one engine answered, in search order.
    Engines(Vec<EngineInfo>),
    /// A socket is there, but this user may not connect to it.
    NoPermission(SocketCandidate),
    /// Docker is installed (`docker` on `PATH`, or a Docker socket that does
    /// not answer) but no Docker socket answers.
    DockerNotRunning,
    /// Podman is installed (`podman` on `PATH`, or a Podman socket that does
    /// not answer) but its API service does not answer.
    PodmanApiOff,
    /// No engine answered and neither `docker` nor `podman` is on `PATH`.
    NotInstalled,
}

/// Decide what the search results mean.
///
/// Reached engines win. Without them the first matching case wins, in this
/// order: no permission, Docker not running, Podman API off, not installed.
pub(crate) fn diagnose(probes: &[Probe], cli: CliPresence) -> Diagnosis {
    let engines: Vec<EngineInfo> = probes
        .iter()
        .filter_map(|probe| match probe.outcome() {
            ProbeOutcome::Reached(info) => Some(info.clone()),
            _ => None,
        })
        .collect();
    if !engines.is_empty() {
        return Diagnosis::Engines(engines);
    }

    let denied = probes
        .iter()
        .find(|probe| *probe.outcome() == ProbeOutcome::PermissionDenied);
    if let Some(probe) = denied {
        return Diagnosis::NoPermission(probe.candidate().clone());
    }

    if cli.docker() || has_silent_socket(probes, CandidateSource::Docker) {
        return Diagnosis::DockerNotRunning;
    }

    if cli.podman() || has_silent_socket(probes, CandidateSource::Podman) {
        return Diagnosis::PodmanApiOff;
    }

    Diagnosis::NotInstalled
}

/// A socket of this source is there but no engine answers on it, which
/// means the engine is installed but stopped.
fn has_silent_socket(probes: &[Probe], source: CandidateSource) -> bool {
    probes.iter().any(|probe| {
        probe.candidate().source() == source
            && matches!(probe.outcome(), ProbeOutcome::NotAnswering(_))
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use bollard::models::SystemVersion;

    use super::*;
    use crate::address::EngineAddress;

    fn candidate(path: &str, source: CandidateSource) -> SocketCandidate {
        SocketCandidate::new(EngineAddress::Unix(PathBuf::from(path)), source)
    }

    fn docker_socket() -> SocketCandidate {
        candidate("/var/run/docker.sock", CandidateSource::Docker)
    }

    fn podman_socket() -> SocketCandidate {
        candidate("/run/user/1000/podman/podman.sock", CandidateSource::Podman)
    }

    fn engine(candidate: &SocketCandidate) -> EngineInfo {
        EngineInfo::from_version(
            &SystemVersion::default(),
            candidate.address().clone(),
        )
    }

    fn probe(candidate: SocketCandidate, outcome: ProbeOutcome) -> Probe {
        Probe::new(candidate, outcome)
    }

    fn nothing_answers() -> Vec<Probe> {
        vec![
            probe(docker_socket(), ProbeOutcome::Missing),
            probe(podman_socket(), ProbeOutcome::Missing),
        ]
    }

    fn refused() -> ProbeOutcome {
        ProbeOutcome::NotAnswering(String::from("connection refused"))
    }

    #[test]
    fn reached_engines_are_listed_in_search_order() {
        let probes = vec![
            probe(
                docker_socket(),
                ProbeOutcome::Reached(engine(&docker_socket())),
            ),
            probe(
                podman_socket(),
                ProbeOutcome::Reached(engine(&podman_socket())),
            ),
        ];

        let diagnosis = diagnose(&probes, CliPresence::new(true, true));

        assert_eq!(
            diagnosis,
            Diagnosis::Engines(vec![
                engine(&docker_socket()),
                engine(&podman_socket())
            ])
        );
    }

    #[test]
    fn one_reached_engine_wins_over_other_failures() {
        let probes = vec![
            probe(docker_socket(), ProbeOutcome::PermissionDenied),
            probe(
                podman_socket(),
                ProbeOutcome::Reached(engine(&podman_socket())),
            ),
        ];

        let diagnosis = diagnose(&probes, CliPresence::new(true, true));

        assert_eq!(
            diagnosis,
            Diagnosis::Engines(vec![engine(&podman_socket())])
        );
    }

    #[test]
    fn nothing_on_path_and_no_socket_is_not_installed() {
        let diagnosis =
            diagnose(&nothing_answers(), CliPresence::new(false, false));

        assert_eq!(diagnosis, Diagnosis::NotInstalled);
    }

    #[test]
    fn no_candidates_and_no_tools_is_not_installed() {
        let diagnosis = diagnose(&[], CliPresence::default());

        assert_eq!(diagnosis, Diagnosis::NotInstalled);
    }

    #[test]
    fn docker_on_path_without_answer_is_docker_not_running() {
        let diagnosis =
            diagnose(&nothing_answers(), CliPresence::new(true, false));

        assert_eq!(diagnosis, Diagnosis::DockerNotRunning);
    }

    #[test]
    fn stale_docker_socket_is_docker_not_running_without_cli() {
        let probes = vec![
            probe(docker_socket(), refused()),
            probe(podman_socket(), ProbeOutcome::Missing),
        ];

        let diagnosis = diagnose(&probes, CliPresence::new(false, false));

        assert_eq!(diagnosis, Diagnosis::DockerNotRunning);
    }

    #[test]
    fn podman_on_path_without_answer_is_podman_api_off() {
        let diagnosis =
            diagnose(&nothing_answers(), CliPresence::new(false, true));

        assert_eq!(diagnosis, Diagnosis::PodmanApiOff);
    }

    #[test]
    fn stale_podman_socket_is_podman_api_off_without_cli() {
        let probes = vec![
            probe(docker_socket(), ProbeOutcome::Missing),
            probe(podman_socket(), refused()),
        ];

        let diagnosis = diagnose(&probes, CliPresence::new(false, false));

        assert_eq!(diagnosis, Diagnosis::PodmanApiOff);
    }

    #[test]
    fn both_tools_without_answer_reports_docker_first() {
        let diagnosis =
            diagnose(&nothing_answers(), CliPresence::new(true, true));

        assert_eq!(diagnosis, Diagnosis::DockerNotRunning);
    }

    #[test]
    fn denied_socket_is_no_permission_before_not_running() {
        let probes = vec![
            probe(docker_socket(), ProbeOutcome::PermissionDenied),
            probe(podman_socket(), refused()),
        ];

        let diagnosis = diagnose(&probes, CliPresence::new(true, true));

        assert_eq!(diagnosis, Diagnosis::NoPermission(docker_socket()));
    }

    #[test]
    fn denied_docker_host_is_no_permission() {
        let host = candidate("/srv/docker.sock", CandidateSource::DockerHost);
        let probes = vec![
            probe(host.clone(), ProbeOutcome::PermissionDenied),
            probe(docker_socket(), ProbeOutcome::Missing),
        ];

        let diagnosis = diagnose(&probes, CliPresence::new(false, false));

        assert_eq!(diagnosis, Diagnosis::NoPermission(host));
    }

    #[test]
    fn silent_docker_host_alone_does_not_mean_docker_is_installed() {
        let host = candidate("/srv/docker.sock", CandidateSource::DockerHost);
        let probes = vec![probe(host, refused())];

        let diagnosis = diagnose(&probes, CliPresence::new(false, false));

        assert_eq!(diagnosis, Diagnosis::NotInstalled);
    }
}
