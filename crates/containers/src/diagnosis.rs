//! What the engine search means for the user (spec Decision 9).

#[cfg(feature = "client")]
use crate::address::EngineAddress;
#[cfg(feature = "client")]
use crate::cli_presence::CliPresence;
use crate::engine::{EngineInfo, EngineKind};
#[cfg(feature = "client")]
use crate::probe_outcome::{Probe, ProbeOutcome};
#[cfg(feature = "client")]
use crate::socket_candidate::CandidateSource;
use crate::socket_candidate::SocketCandidate;

/// The answer to "can OTTY reach an engine, and if not, why".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnosis {
    /// At least one engine answered, in search order.
    Engines(Vec<EngineInfo>),
    /// A socket is there, but this user may not connect to it.
    NoPermission(SocketCandidate),
    /// A Podman socket is there, but this user may not connect to it. The
    /// docker group does not help here; the user's own socket does.
    ///
    /// ```
    /// # #[cfg(feature = "fixtures")]
    /// # {
    /// use otty_containers::{CandidateSource, Diagnosis, fixtures};
    ///
    /// let discovery = fixtures::denied_discovery(
    ///     fixtures::PODMAN_SOCKET,
    ///     CandidateSource::Podman,
    /// );
    ///
    /// assert!(matches!(
    ///     discovery.diagnosis(),
    ///     Diagnosis::PodmanNoPermission(_)
    /// ));
    /// # }
    /// ```
    PodmanNoPermission(SocketCandidate),
    /// One engine refused this user and the other is installed but off, each
    /// as the cases around it tell it. OTTY cannot know which one the user
    /// wants. When both refused, the first denied socket decides alone.
    ///
    /// ```
    /// # #[cfg(feature = "fixtures")]
    /// # {
    /// use otty_containers::{CandidateSource, Diagnosis, EngineKind, fixtures};
    ///
    /// let discovery = fixtures::denied_discovery_with_cli(
    ///     fixtures::DOCKER_SOCKET,
    ///     CandidateSource::Docker,
    ///     None,
    ///     Some("/usr/bin/podman"),
    /// );
    ///
    /// assert!(matches!(
    ///     discovery.diagnosis(),
    ///     Diagnosis::NoPermissionAndOtherOff {
    ///         denied_engine: EngineKind::Docker,
    ///         ..
    ///     }
    /// ));
    /// # }
    /// ```
    NoPermissionAndOtherOff {
        /// The socket that refused this user.
        socket: SocketCandidate,
        /// The engine that socket belongs to.
        denied_engine: EngineKind,
    },
    /// Docker is installed (Docker's `docker` command found, or a Docker
    /// socket that does not answer) but no Docker socket answers.
    DockerNotRunning,
    /// Podman is installed (`podman` found, or a `docker` command that is
    /// really Podman, or a Podman socket that does not answer) but its API
    /// service does not answer.
    PodmanApiOff,
    /// Docker and Podman are both installed, each as the two cases above tell
    /// it, and neither answers. OTTY cannot know which one the user wants.
    ///
    /// ```
    /// # #[cfg(feature = "fixtures")]
    /// # {
    /// use otty_containers::{Diagnosis, fixtures};
    ///
    /// let discovery = fixtures::discovery_with_cli(
    ///     &[],
    ///     Some("/usr/bin/docker"),
    ///     Some("/usr/bin/podman"),
    /// );
    ///
    /// assert_eq!(discovery.diagnosis(), &Diagnosis::DockerAndPodmanOff);
    /// # }
    /// ```
    DockerAndPodmanOff,
    /// No engine answered and neither `docker` nor `podman` was found.
    NotInstalled,
}

/// Decide what the search results mean.
///
/// Reached engines win. Without them a denied socket comes first, with the
/// other engine when it is installed and off, then which of Docker and Podman
/// are installed: both, Docker only, Podman only or none.
#[cfg(feature = "client")]
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

    let has_docker =
        cli.docker() || has_silent_socket(probes, CandidateSource::Docker);
    let has_podman =
        cli.podman() || has_silent_socket(probes, CandidateSource::Podman);

    let denied: Vec<&SocketCandidate> = probes
        .iter()
        .filter(|probe| *probe.outcome() == ProbeOutcome::PermissionDenied)
        .map(Probe::candidate)
        .collect();
    if let Some(first) = denied.first() {
        let is_podman = is_podman_socket(first);
        let other_refused = denied
            .iter()
            .any(|candidate| is_podman_socket(candidate) != is_podman);
        let other_installed = if is_podman { has_docker } else { has_podman };
        let socket = (*first).clone();

        if other_installed && !other_refused {
            return Diagnosis::NoPermissionAndOtherOff {
                socket,
                denied_engine: if is_podman {
                    EngineKind::Podman
                } else {
                    EngineKind::Docker
                },
            };
        }
        if is_podman {
            return Diagnosis::PodmanNoPermission(socket);
        }
        return Diagnosis::NoPermission(socket);
    }

    match (has_docker, has_podman) {
        (true, true) => Diagnosis::DockerAndPodmanOff,
        (true, false) => Diagnosis::DockerNotRunning,
        (false, true) => Diagnosis::PodmanApiOff,
        (false, false) => Diagnosis::NotInstalled,
    }
}

/// The socket belongs to Podman. A socket that refuses the connection cannot
/// be asked who serves it, so `$DOCKER_HOST` is told by the socket's own file
/// name.
#[cfg(feature = "client")]
fn is_podman_socket(candidate: &SocketCandidate) -> bool {
    match candidate.source() {
        CandidateSource::Podman => true,
        CandidateSource::Docker => false,
        CandidateSource::DockerHost => match candidate.address() {
            EngineAddress::Unix(path) => path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().contains("podman")),
            EngineAddress::Tcp(_) => false,
        },
    }
}

/// A socket of this source is there but no engine answers on it, which
/// means the engine is installed but stopped.
#[cfg(feature = "client")]
fn has_silent_socket(probes: &[Probe], source: CandidateSource) -> bool {
    probes.iter().any(|probe| {
        probe.candidate().source() == source
            && matches!(probe.outcome(), ProbeOutcome::NotAnswering(_))
    })
}

#[cfg(all(test, feature = "client"))]
mod tests {
    use std::path::PathBuf;

    use bollard::models::SystemVersion;

    use super::*;

    /// Rootful Podman's socket, which a normal user may not open.
    const PODMAN_SYSTEM: &str = "/run/podman/podman.sock";

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
    fn podman_docker_user_without_answer_is_podman_api_off() {
        let docker_is_podman = true;
        let cli = CliPresence::from_lookup(true, docker_is_podman, true);

        let diagnosis = diagnose(&nothing_answers(), cli);

        assert_eq!(diagnosis, Diagnosis::PodmanApiOff);
    }

    #[test]
    fn podman_docker_wrapper_alone_is_podman_api_off() {
        let docker_is_podman = true;
        let cli = CliPresence::from_lookup(true, docker_is_podman, false);

        let diagnosis = diagnose(&nothing_answers(), cli);

        assert_eq!(diagnosis, Diagnosis::PodmanApiOff);
    }

    #[test]
    fn both_tools_without_answer_names_both_engines() {
        let diagnosis =
            diagnose(&nothing_answers(), CliPresence::new(true, true));

        assert_eq!(diagnosis, Diagnosis::DockerAndPodmanOff);
    }

    #[test]
    fn a_tool_and_the_stale_socket_of_the_other_engine_name_both_engines() {
        let docker_tool = vec![
            probe(docker_socket(), ProbeOutcome::Missing),
            probe(podman_socket(), refused()),
        ];
        let podman_tool = vec![
            probe(docker_socket(), refused()),
            probe(podman_socket(), ProbeOutcome::Missing),
        ];

        assert_eq!(
            diagnose(&docker_tool, CliPresence::new(true, false)),
            Diagnosis::DockerAndPodmanOff
        );
        assert_eq!(
            diagnose(&podman_tool, CliPresence::new(false, true)),
            Diagnosis::DockerAndPodmanOff
        );
    }

    #[test]
    fn stale_sockets_of_both_engines_name_both_engines() {
        let probes = vec![
            probe(docker_socket(), refused()),
            probe(podman_socket(), refused()),
        ];

        let diagnosis = diagnose(&probes, CliPresence::new(false, false));

        assert_eq!(diagnosis, Diagnosis::DockerAndPodmanOff);
    }

    #[test]
    fn a_docker_command_that_is_really_podman_does_not_name_docker() {
        let docker_is_podman = true;
        let cli = CliPresence::from_lookup(true, docker_is_podman, true);

        let diagnosis = diagnose(&nothing_answers(), cli);

        assert_eq!(diagnosis, Diagnosis::PodmanApiOff);
    }

    /// The card for `socket`, denied, with the other engine off.
    fn denied_and_other_off(
        socket: SocketCandidate,
        denied_engine: EngineKind,
    ) -> Diagnosis {
        Diagnosis::NoPermissionAndOtherOff {
            socket,
            denied_engine,
        }
    }

    #[test]
    fn a_denied_docker_socket_with_podman_off_names_both_engines() {
        let tool = vec![
            probe(docker_socket(), ProbeOutcome::PermissionDenied),
            probe(podman_socket(), ProbeOutcome::Missing),
        ];
        let stale_socket = vec![
            probe(docker_socket(), ProbeOutcome::PermissionDenied),
            probe(podman_socket(), refused()),
        ];
        let expected =
            denied_and_other_off(docker_socket(), EngineKind::Docker);

        assert_eq!(diagnose(&tool, CliPresence::new(true, true)), expected);
        assert_eq!(diagnose(&tool, CliPresence::new(false, true)), expected);
        assert_eq!(
            diagnose(&stale_socket, CliPresence::new(false, false)),
            expected
        );
    }

    #[test]
    fn a_denied_docker_socket_without_podman_keeps_the_docker_card() {
        let probes = vec![
            probe(docker_socket(), ProbeOutcome::PermissionDenied),
            probe(podman_socket(), ProbeOutcome::Missing),
        ];

        let diagnosis = diagnose(&probes, CliPresence::new(true, false));

        assert_eq!(diagnosis, Diagnosis::NoPermission(docker_socket()));
    }

    #[test]
    fn a_denied_podman_socket_with_docker_off_names_both_engines() {
        let rootful = candidate(PODMAN_SYSTEM, CandidateSource::Podman);
        let tool = vec![
            probe(docker_socket(), ProbeOutcome::Missing),
            probe(rootful.clone(), ProbeOutcome::PermissionDenied),
        ];
        let stale_socket = vec![
            probe(docker_socket(), refused()),
            probe(rootful.clone(), ProbeOutcome::PermissionDenied),
        ];
        let expected = denied_and_other_off(rootful, EngineKind::Podman);

        assert_eq!(diagnose(&tool, CliPresence::new(true, true)), expected);
        assert_eq!(diagnose(&tool, CliPresence::new(true, false)), expected);
        assert_eq!(
            diagnose(&stale_socket, CliPresence::new(false, false)),
            expected
        );
    }

    #[test]
    fn a_denied_podman_socket_with_a_fake_docker_command_does_not_name_docker()
    {
        let rootful = candidate(PODMAN_SYSTEM, CandidateSource::Podman);
        let probes = vec![
            probe(docker_socket(), ProbeOutcome::Missing),
            probe(rootful.clone(), ProbeOutcome::PermissionDenied),
        ];
        let docker_is_podman = true;

        for podman in [true, false] {
            let cli = CliPresence::from_lookup(true, docker_is_podman, podman);

            let diagnosis = diagnose(&probes, cli);

            assert_eq!(
                diagnosis,
                Diagnosis::PodmanNoPermission(rootful.clone())
            );
        }
    }

    #[test]
    fn a_docker_command_that_is_really_podman_counts_as_podman_off() {
        let probes = vec![
            probe(docker_socket(), ProbeOutcome::PermissionDenied),
            probe(podman_socket(), ProbeOutcome::Missing),
        ];
        let docker_is_podman = true;
        let cli = CliPresence::from_lookup(true, docker_is_podman, false);

        let diagnosis = diagnose(&probes, cli);

        assert_eq!(
            diagnosis,
            denied_and_other_off(docker_socket(), EngineKind::Docker)
        );
    }

    #[test]
    fn two_denied_sockets_of_one_engine_still_offer_the_other_engine() {
        let host = candidate("/srv/docker.sock", CandidateSource::DockerHost);
        let probes = vec![
            probe(host.clone(), ProbeOutcome::PermissionDenied),
            probe(docker_socket(), ProbeOutcome::PermissionDenied),
            probe(podman_socket(), ProbeOutcome::Missing),
        ];

        let diagnosis = diagnose(&probes, CliPresence::new(true, true));

        assert_eq!(diagnosis, denied_and_other_off(host, EngineKind::Docker));
    }

    #[test]
    fn a_denied_docker_host_with_podman_off_names_both_engines() {
        let host = candidate("/srv/docker.sock", CandidateSource::DockerHost);
        let probes = vec![probe(host.clone(), ProbeOutcome::PermissionDenied)];

        let diagnosis = diagnose(&probes, CliPresence::new(false, true));

        assert_eq!(diagnosis, denied_and_other_off(host, EngineKind::Docker));
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
    fn denied_podman_socket_is_podman_no_permission() {
        let rootful = candidate(PODMAN_SYSTEM, CandidateSource::Podman);
        let probes = vec![
            probe(docker_socket(), ProbeOutcome::Missing),
            probe(podman_socket(), ProbeOutcome::Missing),
            probe(rootful.clone(), ProbeOutcome::PermissionDenied),
        ];

        let diagnosis = diagnose(&probes, CliPresence::new(false, true));

        assert_eq!(diagnosis, Diagnosis::PodmanNoPermission(rootful));
    }

    #[test]
    fn denied_docker_host_at_a_podman_socket_is_podman_no_permission() {
        let host = candidate(PODMAN_SYSTEM, CandidateSource::DockerHost);
        let probes = vec![probe(host.clone(), ProbeOutcome::PermissionDenied)];

        let diagnosis = diagnose(&probes, CliPresence::new(false, true));

        assert_eq!(diagnosis, Diagnosis::PodmanNoPermission(host));
    }

    #[test]
    fn denied_docker_host_in_a_podman_folder_stays_no_permission() {
        // Only the socket's own name counts: this one is called docker.
        let host = candidate(
            "/srv/podman-old/docker.sock",
            CandidateSource::DockerHost,
        );
        let probes = vec![probe(host.clone(), ProbeOutcome::PermissionDenied)];

        let diagnosis = diagnose(&probes, CliPresence::new(false, false));

        assert_eq!(diagnosis, Diagnosis::NoPermission(host));
    }

    #[test]
    fn denied_tcp_docker_host_stays_no_permission() {
        let host = SocketCandidate::new(
            EngineAddress::Tcp(String::from("podman.local:2375")),
            CandidateSource::DockerHost,
        );
        let probes = vec![probe(host.clone(), ProbeOutcome::PermissionDenied)];

        let diagnosis = diagnose(&probes, CliPresence::new(false, false));

        assert_eq!(diagnosis, Diagnosis::NoPermission(host));
    }

    #[test]
    fn first_denied_socket_decides_between_docker_and_podman() {
        let rootful = candidate(PODMAN_SYSTEM, CandidateSource::Podman);
        let probes = vec![
            probe(docker_socket(), ProbeOutcome::PermissionDenied),
            probe(rootful, ProbeOutcome::PermissionDenied),
        ];

        let diagnosis = diagnose(&probes, CliPresence::new(true, true));

        assert_eq!(diagnosis, Diagnosis::NoPermission(docker_socket()));
    }

    #[test]
    fn when_both_engines_refuse_the_first_denied_socket_decides_alone() {
        let rootful = candidate(PODMAN_SYSTEM, CandidateSource::Podman);
        let podman_first = vec![
            probe(rootful.clone(), ProbeOutcome::PermissionDenied),
            probe(docker_socket(), ProbeOutcome::PermissionDenied),
        ];

        let diagnosis = diagnose(&podman_first, CliPresence::new(true, true));

        assert_eq!(diagnosis, Diagnosis::PodmanNoPermission(rootful));
    }

    #[test]
    fn silent_docker_host_alone_does_not_mean_docker_is_installed() {
        let host = candidate("/srv/docker.sock", CandidateSource::DockerHost);
        let probes = vec![probe(host, refused())];

        let diagnosis = diagnose(&probes, CliPresence::new(false, false));

        assert_eq!(diagnosis, Diagnosis::NotInstalled);
    }
}
