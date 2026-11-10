//! Why no engine answered: the no-engine case of a finished search and the
//! socket it names (spec Decision 9).

use otty_containers::{Diagnosis, Discovery};

use super::types::NoEngineCase;

/// The no-engine case of a finished search, with the denied socket that the
/// cases of a refused socket name (empty for the others); `None` when an
/// engine answered.
pub(crate) fn no_engine_case(
    discovery: &Discovery,
) -> Option<(NoEngineCase, String)> {
    let found = match discovery.diagnosis() {
        Diagnosis::Engines(_) => return None,
        Diagnosis::NoPermission(candidate) => {
            (NoEngineCase::NoPermission, candidate.address().to_string())
        },
        Diagnosis::PodmanNoPermission(candidate) => (
            NoEngineCase::PodmanNoPermission,
            candidate.address().to_string(),
        ),
        Diagnosis::NoPermissionAndOtherOff {
            socket,
            denied_engine,
        } => (
            NoEngineCase::NoPermissionAndOtherOff(*denied_engine),
            socket.address().to_string(),
        ),
        Diagnosis::DockerNotRunning => {
            (NoEngineCase::DockerNotRunning, String::new())
        },
        Diagnosis::PodmanApiOff => (NoEngineCase::PodmanApiOff, String::new()),
        Diagnosis::DockerAndPodmanOff => {
            (NoEngineCase::DockerAndPodmanOff, String::new())
        },
        Diagnosis::NotInstalled => (NoEngineCase::NotInstalled, String::new()),
    };

    Some(found)
}

#[cfg(test)]
mod tests {
    use otty_containers::{CandidateSource, EngineKind, fixtures};

    use super::*;

    #[test]
    fn both_tools_installed_are_the_case_of_both_engines() {
        let discovery = fixtures::discovery_with_cli(
            &[],
            Some("/usr/bin/docker"),
            Some("/usr/bin/podman"),
        );

        let found = no_engine_case(&discovery);

        assert_eq!(
            found,
            Some((NoEngineCase::DockerAndPodmanOff, String::new()))
        );
    }

    #[test]
    fn a_denied_podman_socket_is_the_podman_case_with_its_path() {
        let socket = "/run/podman/podman.sock";
        let discovery =
            fixtures::denied_discovery(socket, CandidateSource::Podman);

        let (case, named) =
            no_engine_case(&discovery).expect("no engine answered");

        assert_eq!(case, NoEngineCase::PodmanNoPermission);
        assert!(named.contains(socket));
    }

    #[test]
    fn a_denied_docker_socket_is_the_docker_case_with_its_path() {
        let socket = "/var/run/docker.sock";
        let discovery =
            fixtures::denied_discovery(socket, CandidateSource::Docker);

        let (case, named) =
            no_engine_case(&discovery).expect("no engine answered");

        assert_eq!(case, NoEngineCase::NoPermission);
        assert!(named.contains(socket));
    }

    #[test]
    fn a_search_that_reached_an_engine_has_no_case() {
        let discovery = fixtures::discovery(&[EngineKind::Docker]);

        assert_eq!(no_engine_case(&discovery), None);
    }

    #[test]
    fn a_search_that_found_nothing_says_nothing_is_installed() {
        let discovery = fixtures::discovery(&[]);

        let (case, _) = no_engine_case(&discovery).expect("no engine answered");

        assert_eq!(case, NoEngineCase::NotInstalled);
    }
}
