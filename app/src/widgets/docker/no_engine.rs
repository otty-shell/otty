//! What the Docker mode says when no engine answered: the case, its texts,
//! command and links (spec Decision 9).

use otty_docker::{CandidateSource, Diagnosis, Discovery};

use crate::i18n::{self, Key};

/// Page that explains how to install Docker.
const DOCKER_INSTALL_URL: &str =
    "https://docs.docker.com/get-started/get-docker/";
/// Page that explains how to install Podman.
const PODMAN_INSTALL_URL: &str = "https://podman.io/docs/installation";
/// Page that explains how to start the Docker service.
const DOCKER_START_URL: &str = "https://docs.docker.com/engine/daemon/start/";
/// Page that explains the Podman API service.
const PODMAN_SERVICE_URL: &str =
    "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html";
/// Page that explains the docker group.
const DOCKER_GROUP_URL: &str =
    "https://docs.docker.com/engine/install/linux-postinstall/";

/// Why no engine can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoEngineCase {
    /// Neither Docker nor Podman is installed.
    NotInstalled,
    /// Docker is installed but no Docker socket answers.
    DockerNotRunning,
    /// Podman is installed but its API service is off.
    PodmanApiOff,
    /// A socket exists but this user may not connect to it.
    NoPermission,
}

/// A command to run in a terminal, with the text around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandViewModel {
    pub(crate) label: &'static str,
    pub(crate) text: &'static str,
    pub(crate) hint: &'static str,
}

/// A button that opens a page in the browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkViewModel {
    pub(crate) label: &'static str,
    /// The site name shown next to the label.
    pub(crate) site: &'static str,
    pub(crate) url: &'static str,
}

/// The card of the main area and the short text of the left panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoEngineViewModel {
    pub(crate) case: NoEngineCase,
    pub(crate) title: &'static str,
    pub(crate) description: String,
    /// The short text of the left panel.
    pub(crate) panel_text: &'static str,
    /// The short text of the status bar.
    pub(crate) status_text: String,
    /// The command that fixes the case; none when nothing is installed.
    pub(crate) command: Option<CommandViewModel>,
    /// Install pages; only when nothing is installed.
    pub(crate) installs: Vec<LinkViewModel>,
    /// The page of the "How to fix" button.
    pub(crate) fix_url: Option<&'static str>,
    /// What OTTY looked for, shown under the card.
    pub(crate) looked_for: String,
}

/// Build the card for a finished search; `None` when an engine answered.
pub(crate) fn no_engine(discovery: &Discovery) -> Option<NoEngineViewModel> {
    let (case, socket) = match discovery.diagnosis() {
        Diagnosis::Engines(_) => return None,
        Diagnosis::NoPermission(candidate) => {
            (NoEngineCase::NoPermission, candidate.address().to_string())
        },
        Diagnosis::DockerNotRunning => {
            (NoEngineCase::DockerNotRunning, String::new())
        },
        Diagnosis::PodmanApiOff => (NoEngineCase::PodmanApiOff, String::new()),
        Diagnosis::NotInstalled => (NoEngineCase::NotInstalled, String::new()),
    };

    let places: Vec<String> = discovery
        .probes()
        .iter()
        .map(|probe| {
            let address = probe.candidate().address();
            match probe.candidate().source() {
                CandidateSource::DockerHost => {
                    format!("$DOCKER_HOST ({address})")
                },
                _ => address.to_string(),
            }
        })
        .collect();

    Some(view_model(case, &socket, &places))
}

/// Build the card of `case`. `socket` is the denied socket (used by
/// `NoPermission`); `places` are the sockets OTTY tried.
pub(crate) fn view_model(
    case: NoEngineCase,
    socket: &str,
    places: &[String],
) -> NoEngineViewModel {
    let looked_for = i18n::docker_looked_for(&places.join(", "));

    match case {
        NoEngineCase::NotInstalled => NoEngineViewModel {
            case,
            title: i18n::t(Key::DockerNotInstalledTitle),
            description: i18n::t(Key::DockerNotInstalledText).to_string(),
            panel_text: i18n::t(Key::DockerPanelNotInstalled),
            status_text: i18n::t(Key::DockerStatusNoEngine).to_string(),
            command: None,
            installs: vec![
                LinkViewModel {
                    label: i18n::t(Key::DockerInstallDocker),
                    site: "docs.docker.com",
                    url: DOCKER_INSTALL_URL,
                },
                LinkViewModel {
                    label: i18n::t(Key::DockerInstallPodman),
                    site: "podman.io",
                    url: PODMAN_INSTALL_URL,
                },
            ],
            fix_url: None,
            looked_for,
        },
        NoEngineCase::DockerNotRunning => NoEngineViewModel {
            case,
            title: i18n::t(Key::DockerStoppedTitle),
            description: i18n::t(Key::DockerStoppedText).to_string(),
            panel_text: i18n::t(Key::DockerPanelStopped),
            status_text: i18n::t(Key::DockerStatusStopped).to_string(),
            command: Some(CommandViewModel {
                label: i18n::t(Key::DockerRunInTerminal),
                text: "sudo systemctl start docker",
                hint: i18n::t(Key::DockerStoppedHint),
            }),
            installs: Vec::new(),
            fix_url: Some(DOCKER_START_URL),
            looked_for,
        },
        NoEngineCase::PodmanApiOff => NoEngineViewModel {
            case,
            title: i18n::t(Key::DockerPodmanOffTitle),
            description: i18n::t(Key::DockerPodmanOffText).to_string(),
            panel_text: i18n::t(Key::DockerPanelPodmanOff),
            status_text: i18n::t(Key::DockerStatusPodmanOff).to_string(),
            command: Some(CommandViewModel {
                label: i18n::t(Key::DockerRunInTerminal),
                text: "systemctl --user enable --now podman.socket",
                hint: i18n::t(Key::DockerPodmanOffHint),
            }),
            installs: Vec::new(),
            fix_url: Some(PODMAN_SERVICE_URL),
            looked_for,
        },
        NoEngineCase::NoPermission => NoEngineViewModel {
            case,
            title: i18n::t(Key::DockerNoAccessTitle),
            description: i18n::docker_no_access_text(socket),
            panel_text: i18n::t(Key::DockerPanelNoAccess),
            status_text: i18n::docker_status_no_access(socket),
            command: Some(CommandViewModel {
                label: i18n::t(Key::DockerNoAccessLabel),
                text: "sudo usermod -aG docker $USER",
                hint: i18n::t(Key::DockerNoAccessHint),
            }),
            installs: Vec::new(),
            fix_url: Some(DOCKER_GROUP_URL),
            looked_for,
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;

    fn places() -> Vec<String> {
        vec![
            String::from("/var/run/docker.sock"),
            String::from("/run/podman/podman.sock"),
        ]
    }

    #[test]
    fn nothing_installed_offers_both_install_pages_and_no_command() {
        let vm = view_model(NoEngineCase::NotInstalled, "", &places());

        assert_eq!(vm.title, i18n::t(Key::DockerNotInstalledTitle));
        assert_eq!(vm.description, i18n::t(Key::DockerNotInstalledText));
        assert_eq!(vm.panel_text, i18n::t(Key::DockerPanelNotInstalled));
        assert_eq!(vm.command, None);
        assert_eq!(vm.fix_url, None);
        assert_eq!(
            vm.installs,
            vec![
                LinkViewModel {
                    label: i18n::t(Key::DockerInstallDocker),
                    site: "docs.docker.com",
                    url: "https://docs.docker.com/get-started/get-docker/",
                },
                LinkViewModel {
                    label: i18n::t(Key::DockerInstallPodman),
                    site: "podman.io",
                    url: "https://podman.io/docs/installation",
                },
            ]
        );
    }

    #[test]
    fn docker_not_running_gives_the_start_command_and_its_page() {
        let vm = view_model(NoEngineCase::DockerNotRunning, "", &places());

        assert_eq!(vm.title, i18n::t(Key::DockerStoppedTitle));
        assert_eq!(vm.description, i18n::t(Key::DockerStoppedText));
        assert_eq!(vm.panel_text, i18n::t(Key::DockerPanelStopped));
        assert_eq!(
            vm.command,
            Some(CommandViewModel {
                label: i18n::t(Key::DockerRunInTerminal),
                text: "sudo systemctl start docker",
                hint: i18n::t(Key::DockerStoppedHint),
            })
        );
        assert_eq!(
            vm.fix_url,
            Some("https://docs.docker.com/engine/daemon/start/")
        );
        assert!(vm.installs.is_empty());
    }

    #[test]
    fn podman_api_off_gives_the_socket_command_and_its_page() {
        let vm = view_model(NoEngineCase::PodmanApiOff, "", &places());

        assert_eq!(vm.title, i18n::t(Key::DockerPodmanOffTitle));
        assert_eq!(vm.description, i18n::t(Key::DockerPodmanOffText));
        assert_eq!(vm.panel_text, i18n::t(Key::DockerPanelPodmanOff));
        assert_eq!(
            vm.command,
            Some(CommandViewModel {
                label: i18n::t(Key::DockerRunInTerminal),
                text: "systemctl --user enable --now podman.socket",
                hint: i18n::t(Key::DockerPodmanOffHint),
            })
        );
        assert_eq!(
            vm.fix_url,
            Some(
                "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html"
            )
        );
    }

    #[test]
    fn no_permission_names_the_socket_and_gives_the_group_command() {
        let vm = view_model(
            NoEngineCase::NoPermission,
            "/var/run/docker.sock",
            &places(),
        );

        assert_eq!(vm.title, i18n::t(Key::DockerNoAccessTitle));
        assert_eq!(
            vm.description,
            i18n::docker_no_access_text("/var/run/docker.sock")
        );
        assert!(vm.description.contains("/var/run/docker.sock"));
        assert_eq!(vm.panel_text, i18n::t(Key::DockerPanelNoAccess));
        assert_eq!(
            vm.command,
            Some(CommandViewModel {
                label: i18n::t(Key::DockerNoAccessLabel),
                text: "sudo usermod -aG docker $USER",
                hint: i18n::t(Key::DockerNoAccessHint),
            })
        );
        assert_eq!(
            vm.fix_url,
            Some("https://docs.docker.com/engine/install/linux-postinstall/")
        );
    }

    #[test]
    fn the_looked_for_line_lists_every_place_and_the_command_search() {
        let vm = view_model(NoEngineCase::NotInstalled, "", &places());

        assert_eq!(
            vm.looked_for,
            i18n::docker_looked_for(
                "/var/run/docker.sock, /run/podman/podman.sock"
            )
        );
        assert!(vm.looked_for.contains("/run/podman/podman.sock"));
    }

    #[test]
    fn each_case_has_a_short_status_bar_text() {
        let socket = "/var/run/docker.sock";

        let texts = [
            (NoEngineCase::NotInstalled, "no engine found"),
            (NoEngineCase::DockerNotRunning, "docker is not running"),
            (NoEngineCase::PodmanApiOff, "podman socket is off"),
            (
                NoEngineCase::NoPermission,
                "permission denied on /var/run/docker.sock",
            ),
        ];

        for (case, text) in texts {
            let vm = view_model(case, socket, &places());

            assert_eq!(vm.status_text, text);
        }
    }

    #[test]
    fn a_search_that_reached_an_engine_has_no_card() {
        let discovery = fixtures::discovery(&[EngineKind::Docker]);

        assert_eq!(no_engine(&discovery), None);
    }

    #[test]
    fn a_search_that_found_nothing_says_nothing_is_installed() {
        let discovery = fixtures::discovery(&[]);

        let vm = no_engine(&discovery).expect("a card is shown");

        assert_eq!(vm.case, NoEngineCase::NotInstalled);
    }
}
