//! What the Docker mode says when no engine answered: the case, its texts,
//! command and links (spec Decision 9).

use otty_docker::{CandidateSource, Diagnosis, Discovery, EngineKind};

use crate::i18n::{self, Key};

/// Page that explains how to install Docker.
const DOCKER_INSTALL_URL: &str =
    "https://docs.docker.com/get-started/get-docker/";
/// Page that explains how to install Podman.
const PODMAN_INSTALL_URL: &str = "https://podman.io/docs/installation";
/// Starts the Docker service.
const DOCKER_START_COMMAND: &str = "sudo systemctl start docker";
/// Page that explains how to start the Docker service.
const DOCKER_START_URL: &str = "https://docs.docker.com/engine/daemon/start/";
/// Page that explains the Podman API service.
const PODMAN_SERVICE_URL: &str =
    "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html";
/// Turns on the user's own Podman socket, which the engine search looks for:
/// the `podman.socket` user unit listens on
/// `$XDG_RUNTIME_DIR/podman/podman.sock`.
const PODMAN_SOCKET_COMMAND: &str =
    "systemctl --user enable --now podman.socket";
/// Adds the user to the docker group.
const DOCKER_GROUP_COMMAND: &str = "sudo usermod -aG docker $USER";
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
    /// Docker and Podman are both installed and neither answers.
    DockerAndPodmanOff,
    /// A socket exists but this user may not connect to it.
    NoPermission,
    /// A Podman socket exists but this user may not connect to it.
    PodmanNoPermission,
    /// The socket of this engine refused this user, and the other engine is
    /// installed but off.
    NoPermissionAndOtherOff(EngineKind),
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

/// A button that opens the page about the fix of a case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixViewModel {
    pub(crate) label: &'static str,
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
    /// The commands that fix the case: none when nothing is installed, one for
    /// each engine when the card speaks of both.
    pub(crate) commands: Vec<CommandViewModel>,
    /// Install pages; only when nothing is installed.
    pub(crate) installs: Vec<LinkViewModel>,
    /// The "How to fix" buttons, in the order of the commands.
    pub(crate) fixes: Vec<FixViewModel>,
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

/// Build the card of `case`. `socket` is the denied socket (used by the cases
/// of a refused socket); `places` are the sockets OTTY tried.
pub(crate) fn view_model(
    case: NoEngineCase,
    socket: &str,
    places: &[String],
) -> NoEngineViewModel {
    let looked_for = if places.is_empty() {
        i18n::t(Key::DockerLookedForCommands).to_string()
    } else {
        i18n::docker_looked_for(&places.join(", "))
    };

    match case {
        NoEngineCase::NotInstalled => NoEngineViewModel {
            case,
            title: i18n::t(Key::DockerNotInstalledTitle),
            description: i18n::t(Key::DockerNotInstalledText).to_string(),
            panel_text: i18n::t(Key::DockerPanelNotInstalled),
            status_text: i18n::t(Key::DockerStatusNoEngine).to_string(),
            commands: Vec::new(),
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
            fixes: Vec::new(),
            looked_for,
        },
        NoEngineCase::DockerNotRunning => NoEngineViewModel {
            case,
            title: i18n::t(Key::DockerStoppedTitle),
            description: i18n::t(Key::DockerStoppedText).to_string(),
            panel_text: i18n::t(Key::DockerPanelStopped),
            status_text: i18n::t(Key::DockerStatusStopped).to_string(),
            commands: vec![docker_start_command(Key::DockerRunInTerminal)],
            installs: Vec::new(),
            fixes: vec![how_to_fix(Key::DockerHowToFix, DOCKER_START_URL)],
            looked_for,
        },
        NoEngineCase::PodmanApiOff => NoEngineViewModel {
            case,
            title: i18n::t(Key::DockerPodmanOffTitle),
            description: i18n::t(Key::DockerPodmanOffText).to_string(),
            panel_text: i18n::t(Key::DockerPanelPodmanOff),
            status_text: i18n::t(Key::DockerStatusPodmanOff).to_string(),
            commands: vec![podman_socket_command(Key::DockerRunInTerminal)],
            installs: Vec::new(),
            fixes: vec![how_to_fix(Key::DockerHowToFix, PODMAN_SERVICE_URL)],
            looked_for,
        },
        NoEngineCase::DockerAndPodmanOff => NoEngineViewModel {
            case,
            title: i18n::t(Key::DockerBothOffTitle),
            description: i18n::t(Key::DockerBothOffText).to_string(),
            panel_text: i18n::t(Key::DockerPanelBothOff),
            status_text: i18n::t(Key::DockerStatusBothOff).to_string(),
            commands: vec![
                docker_start_command(Key::DockerBothOffDockerLabel),
                podman_socket_command(Key::DockerBothOffPodmanLabel),
            ],
            installs: Vec::new(),
            fixes: vec![
                how_to_fix(Key::DockerHowToFixDocker, DOCKER_START_URL),
                how_to_fix(Key::DockerHowToFixPodman, PODMAN_SERVICE_URL),
            ],
            looked_for,
        },
        NoEngineCase::NoPermission => NoEngineViewModel {
            case,
            title: i18n::t(Key::DockerNoAccessTitle),
            description: i18n::docker_no_access_text(socket),
            panel_text: i18n::t(Key::DockerPanelNoAccess),
            status_text: i18n::docker_status_no_access(socket),
            commands: vec![docker_group_command()],
            installs: Vec::new(),
            fixes: vec![how_to_fix(Key::DockerHowToFix, DOCKER_GROUP_URL)],
            looked_for,
        },
        NoEngineCase::PodmanNoPermission => NoEngineViewModel {
            case,
            title: i18n::t(Key::DockerPodmanNoAccessTitle),
            description: i18n::docker_podman_no_access_text(socket),
            panel_text: i18n::t(Key::DockerPanelPodmanNoAccess),
            status_text: i18n::docker_status_no_access(socket),
            commands: vec![podman_own_socket_command()],
            installs: Vec::new(),
            fixes: vec![how_to_fix(Key::DockerHowToFix, PODMAN_SERVICE_URL)],
            looked_for,
        },
        NoEngineCase::NoPermissionAndOtherOff(EngineKind::Docker) => {
            NoEngineViewModel {
                case,
                title: i18n::t(Key::DockerNoAccessPodmanOffTitle),
                description: i18n::docker_no_access_podman_off_text(socket),
                panel_text: i18n::t(Key::DockerPanelNoAccessPodmanOff),
                status_text: i18n::docker_status_no_access_podman_off(socket),
                commands: vec![
                    docker_group_command(),
                    podman_socket_command(Key::DockerBothOffPodmanLabel),
                ],
                installs: Vec::new(),
                fixes: vec![
                    how_to_fix(Key::DockerHowToFixDocker, DOCKER_GROUP_URL),
                    how_to_fix(Key::DockerHowToFixPodman, PODMAN_SERVICE_URL),
                ],
                looked_for,
            }
        },
        NoEngineCase::NoPermissionAndOtherOff(EngineKind::Podman) => {
            NoEngineViewModel {
                case,
                title: i18n::t(Key::DockerPodmanNoAccessDockerOffTitle),
                description: i18n::docker_podman_no_access_docker_off_text(
                    socket,
                ),
                panel_text: i18n::t(Key::DockerPanelPodmanNoAccessDockerOff),
                status_text: i18n::docker_status_podman_no_access_docker_off(
                    socket,
                ),
                commands: vec![
                    podman_own_socket_command(),
                    docker_start_command(Key::DockerBothOffDockerLabel),
                ],
                installs: Vec::new(),
                fixes: vec![
                    how_to_fix(Key::DockerHowToFixPodman, PODMAN_SERVICE_URL),
                    how_to_fix(Key::DockerHowToFixDocker, DOCKER_START_URL),
                ],
                looked_for,
            }
        },
    }
}

/// The command that adds the user to the docker group, with its warning.
fn docker_group_command() -> CommandViewModel {
    CommandViewModel {
        label: i18n::t(Key::DockerNoAccessLabel),
        text: DOCKER_GROUP_COMMAND,
        hint: i18n::t(Key::DockerNoAccessHint),
    }
}

/// The command that turns on the user's own Podman socket, for a user a Podman
/// socket refused.
fn podman_own_socket_command() -> CommandViewModel {
    CommandViewModel {
        label: i18n::t(Key::DockerPodmanNoAccessLabel),
        text: PODMAN_SOCKET_COMMAND,
        hint: i18n::t(Key::DockerPodmanNoAccessHint),
    }
}

/// The command that starts Docker, with its macOS note under the label `label`.
fn docker_start_command(label: Key) -> CommandViewModel {
    CommandViewModel {
        label: i18n::t(label),
        text: DOCKER_START_COMMAND,
        hint: i18n::t(Key::DockerStoppedHint),
    }
}

/// The command that turns on the Podman socket, with its macOS note under the
/// label `label`.
fn podman_socket_command(label: Key) -> CommandViewModel {
    CommandViewModel {
        label: i18n::t(label),
        text: PODMAN_SOCKET_COMMAND,
        hint: i18n::t(Key::DockerPodmanOffHint),
    }
}

fn how_to_fix(label: Key, url: &'static str) -> FixViewModel {
    FixViewModel {
        label: i18n::t(label),
        url,
    }
}

#[cfg(test)]
mod tests {
    use otty_docker::{CandidateSource, EngineKind, fixtures};

    use super::*;

    fn places() -> Vec<String> {
        vec![
            String::from("/var/run/docker.sock"),
            String::from("/run/podman/podman.sock"),
        ]
    }

    fn one_fix(url: &'static str) -> FixViewModel {
        FixViewModel {
            label: i18n::t(Key::DockerHowToFix),
            url,
        }
    }

    #[test]
    fn nothing_installed_offers_both_install_pages_and_no_command() {
        let vm = view_model(NoEngineCase::NotInstalled, "", &places());

        assert_eq!(vm.title, i18n::t(Key::DockerNotInstalledTitle));
        assert_eq!(vm.description, i18n::t(Key::DockerNotInstalledText));
        assert_eq!(vm.panel_text, i18n::t(Key::DockerPanelNotInstalled));
        assert!(vm.commands.is_empty());
        assert!(vm.fixes.is_empty());
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
            vm.commands,
            vec![CommandViewModel {
                label: i18n::t(Key::DockerRunInTerminal),
                text: "sudo systemctl start docker",
                hint: i18n::t(Key::DockerStoppedHint),
            }]
        );
        assert_eq!(
            vm.fixes,
            vec![one_fix("https://docs.docker.com/engine/daemon/start/")]
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
            vm.commands,
            vec![CommandViewModel {
                label: i18n::t(Key::DockerRunInTerminal),
                text: "systemctl --user enable --now podman.socket",
                hint: i18n::t(Key::DockerPodmanOffHint),
            }]
        );
        assert_eq!(
            vm.fixes,
            vec![one_fix(
                "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html"
            )]
        );
    }

    #[test]
    fn both_engines_off_gives_each_command_with_its_macos_note() {
        let vm = view_model(NoEngineCase::DockerAndPodmanOff, "", &places());

        assert_eq!(vm.title, i18n::t(Key::DockerBothOffTitle));
        assert_eq!(vm.description, i18n::t(Key::DockerBothOffText));
        assert_eq!(vm.panel_text, i18n::t(Key::DockerPanelBothOff));
        assert_eq!(vm.status_text, i18n::t(Key::DockerStatusBothOff));
        assert_eq!(
            vm.commands,
            vec![
                CommandViewModel {
                    label: i18n::t(Key::DockerBothOffDockerLabel),
                    text: "sudo systemctl start docker",
                    hint: i18n::t(Key::DockerStoppedHint),
                },
                CommandViewModel {
                    label: i18n::t(Key::DockerBothOffPodmanLabel),
                    text: "systemctl --user enable --now podman.socket",
                    hint: i18n::t(Key::DockerPodmanOffHint),
                },
            ]
        );
        assert!(vm.installs.is_empty());
    }

    #[test]
    fn both_engines_off_has_a_page_for_each_engine() {
        let vm = view_model(NoEngineCase::DockerAndPodmanOff, "", &places());

        assert_eq!(
            vm.fixes,
            vec![
                FixViewModel {
                    label: i18n::t(Key::DockerHowToFixDocker),
                    url: "https://docs.docker.com/engine/daemon/start/",
                },
                FixViewModel {
                    label: i18n::t(Key::DockerHowToFixPodman),
                    url: "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html",
                },
            ]
        );
    }

    /// Which commands the card has for what is installed, as a search finds it.
    fn commands_of(
        docker: Option<&str>,
        podman: Option<&str>,
    ) -> Vec<&'static str> {
        let discovery = fixtures::discovery_with_cli(&[], docker, podman);

        no_engine(&discovery)
            .expect("a card is shown")
            .commands
            .iter()
            .map(|command| command.text)
            .collect()
    }

    #[test]
    fn the_card_has_the_commands_of_what_is_installed() {
        let docker = "sudo systemctl start docker";
        let podman = "systemctl --user enable --now podman.socket";
        let found_docker = Some("/usr/bin/docker");
        let found_podman = Some("/usr/bin/podman");

        assert_eq!(commands_of(found_docker, found_podman), [docker, podman]);
        assert_eq!(commands_of(found_docker, None), [docker]);
        assert_eq!(commands_of(None, found_podman), [podman]);
        assert!(commands_of(None, None).is_empty());
    }

    #[test]
    fn both_tools_installed_show_the_card_of_both_engines() {
        let discovery = fixtures::discovery_with_cli(
            &[],
            Some("/usr/bin/docker"),
            Some("/usr/bin/podman"),
        );

        let vm = no_engine(&discovery).expect("a card is shown");

        assert_eq!(vm.case, NoEngineCase::DockerAndPodmanOff);
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
            vm.commands,
            vec![CommandViewModel {
                label: i18n::t(Key::DockerNoAccessLabel),
                text: "sudo usermod -aG docker $USER",
                hint: i18n::t(Key::DockerNoAccessHint),
            }]
        );
        assert_eq!(
            vm.fixes,
            vec![one_fix(
                "https://docs.docker.com/engine/install/linux-postinstall/"
            )]
        );
    }

    #[test]
    fn podman_no_permission_names_the_socket_and_starts_the_users_own() {
        let socket = "/run/podman/podman.sock";

        let vm =
            view_model(NoEngineCase::PodmanNoPermission, socket, &places());

        assert_eq!(vm.title, i18n::t(Key::DockerPodmanNoAccessTitle));
        assert_eq!(vm.description, i18n::docker_podman_no_access_text(socket));
        assert!(vm.description.contains(socket));
        assert_eq!(vm.panel_text, i18n::t(Key::DockerPanelPodmanNoAccess));
        assert_eq!(vm.status_text, i18n::docker_status_no_access(socket));
        assert_eq!(
            vm.commands,
            vec![CommandViewModel {
                label: i18n::t(Key::DockerPodmanNoAccessLabel),
                text: "systemctl --user enable --now podman.socket",
                hint: i18n::t(Key::DockerPodmanNoAccessHint),
            }]
        );
        assert_eq!(
            vm.fixes,
            vec![one_fix(
                "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html"
            )]
        );
    }

    #[test]
    fn a_denied_podman_socket_shows_the_podman_card_with_its_path() {
        let socket = "/run/podman/podman.sock";
        let discovery =
            fixtures::denied_discovery(socket, CandidateSource::Podman);

        let vm = no_engine(&discovery).expect("a card is shown");

        assert_eq!(vm.case, NoEngineCase::PodmanNoPermission);
        assert!(vm.description.contains(socket));
    }

    #[test]
    fn a_denied_docker_socket_shows_the_docker_card_with_its_path() {
        let socket = "/var/run/docker.sock";
        let discovery =
            fixtures::denied_discovery(socket, CandidateSource::Docker);

        let vm = no_engine(&discovery).expect("a card is shown");

        assert_eq!(vm.case, NoEngineCase::NoPermission);
        assert!(vm.description.contains(socket));
    }

    const DOCKER_SOCKET: &str = "/var/run/docker.sock";
    const PODMAN_SOCKET: &str = "/run/podman/podman.sock";
    const PODMAN_COMMAND: &str = "systemctl --user enable --now podman.socket";
    const PODMAN_PAGE: &str = "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html";

    #[test]
    fn a_denied_docker_socket_with_podman_off_has_both_commands_and_pages() {
        let vm = view_model(
            NoEngineCase::NoPermissionAndOtherOff(EngineKind::Docker),
            DOCKER_SOCKET,
            &places(),
        );

        assert_eq!(vm.title, i18n::t(Key::DockerNoAccessPodmanOffTitle));
        assert_eq!(
            vm.description,
            i18n::docker_no_access_podman_off_text(DOCKER_SOCKET)
        );
        assert!(vm.description.contains(DOCKER_SOCKET));
        assert_eq!(vm.panel_text, i18n::t(Key::DockerPanelNoAccessPodmanOff));
        assert_eq!(
            vm.status_text,
            i18n::docker_status_no_access_podman_off(DOCKER_SOCKET)
        );
        assert_eq!(
            vm.commands,
            vec![
                CommandViewModel {
                    label: i18n::t(Key::DockerNoAccessLabel),
                    text: DOCKER_GROUP_COMMAND,
                    hint: i18n::t(Key::DockerNoAccessHint),
                },
                CommandViewModel {
                    label: i18n::t(Key::DockerBothOffPodmanLabel),
                    text: PODMAN_COMMAND,
                    hint: i18n::t(Key::DockerPodmanOffHint),
                },
            ]
        );
        assert_eq!(
            vm.fixes,
            vec![
                FixViewModel {
                    label: i18n::t(Key::DockerHowToFixDocker),
                    url: "https://docs.docker.com/engine/install/linux-postinstall/",
                },
                FixViewModel {
                    label: i18n::t(Key::DockerHowToFixPodman),
                    url: PODMAN_PAGE,
                },
            ]
        );
        assert!(vm.installs.is_empty());
    }

    #[test]
    fn a_denied_podman_socket_with_docker_off_has_both_commands_and_pages() {
        let vm = view_model(
            NoEngineCase::NoPermissionAndOtherOff(EngineKind::Podman),
            PODMAN_SOCKET,
            &places(),
        );

        assert_eq!(vm.title, i18n::t(Key::DockerPodmanNoAccessDockerOffTitle));
        assert_eq!(
            vm.description,
            i18n::docker_podman_no_access_docker_off_text(PODMAN_SOCKET)
        );
        assert!(vm.description.contains(PODMAN_SOCKET));
        assert_eq!(
            vm.panel_text,
            i18n::t(Key::DockerPanelPodmanNoAccessDockerOff)
        );
        assert_eq!(
            vm.status_text,
            i18n::docker_status_podman_no_access_docker_off(PODMAN_SOCKET)
        );
        assert_eq!(
            vm.commands,
            vec![
                CommandViewModel {
                    label: i18n::t(Key::DockerPodmanNoAccessLabel),
                    text: PODMAN_COMMAND,
                    hint: i18n::t(Key::DockerPodmanNoAccessHint),
                },
                CommandViewModel {
                    label: i18n::t(Key::DockerBothOffDockerLabel),
                    text: DOCKER_START_COMMAND,
                    hint: i18n::t(Key::DockerStoppedHint),
                },
            ]
        );
        assert_eq!(
            vm.fixes,
            vec![
                FixViewModel {
                    label: i18n::t(Key::DockerHowToFixPodman),
                    url: PODMAN_PAGE,
                },
                FixViewModel {
                    label: i18n::t(Key::DockerHowToFixDocker),
                    url: "https://docs.docker.com/engine/daemon/start/",
                },
            ]
        );
        assert!(!vm.commands[0].text.contains("usermod"));
    }

    /// The case and the commands of a search whose only socket is denied.
    fn denied_card(
        socket: &str,
        source: CandidateSource,
        docker: Option<&str>,
        podman: Option<&str>,
    ) -> (NoEngineCase, Vec<&'static str>) {
        let discovery =
            fixtures::denied_discovery_with_cli(socket, source, docker, podman);

        let vm = no_engine(&discovery).expect("a card is shown");
        assert!(vm.description.contains(socket));

        (
            vm.case,
            vm.commands.iter().map(|command| command.text).collect(),
        )
    }

    #[test]
    fn a_denied_socket_offers_the_other_engine_only_when_it_is_installed() {
        let docker_tool = Some("/usr/bin/docker");
        let podman_tool = Some("/usr/bin/podman");
        let docker = CandidateSource::Docker;
        let podman = CandidateSource::Podman;

        assert_eq!(
            denied_card(DOCKER_SOCKET, docker, None, podman_tool),
            (
                NoEngineCase::NoPermissionAndOtherOff(EngineKind::Docker),
                vec![DOCKER_GROUP_COMMAND, PODMAN_COMMAND]
            )
        );
        assert_eq!(
            denied_card(DOCKER_SOCKET, docker, docker_tool, None),
            (NoEngineCase::NoPermission, vec![DOCKER_GROUP_COMMAND])
        );
        assert_eq!(
            denied_card(PODMAN_SOCKET, podman, docker_tool, None),
            (
                NoEngineCase::NoPermissionAndOtherOff(EngineKind::Podman),
                vec![PODMAN_COMMAND, DOCKER_START_COMMAND]
            )
        );
        assert_eq!(
            denied_card(PODMAN_SOCKET, podman, None, podman_tool),
            (NoEngineCase::PodmanNoPermission, vec![PODMAN_COMMAND])
        );
    }

    #[test]
    fn the_podman_card_never_advises_the_docker_group() {
        let vm = view_model(
            NoEngineCase::PodmanNoPermission,
            "/run/podman/podman.sock",
            &places(),
        );

        let command = &vm.commands[0];

        assert!(!command.text.contains("usermod"));
        assert!(!command.text.contains("docker"));
        assert!(!vm.description.contains("docker"));
        assert!(!command.label.contains("docker"));
        assert!(!command.hint.contains("docker"));
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
    fn the_looked_for_line_has_no_empty_place_list_without_probes() {
        let vm = view_model(NoEngineCase::NotInstalled, "", &[]);

        assert_eq!(vm.looked_for, i18n::t(Key::DockerLookedForCommands));
        assert!(!vm.looked_for.contains("for ,"));
    }

    #[test]
    fn each_case_has_a_short_status_bar_text() {
        let socket = "/var/run/docker.sock";

        let texts = [
            (NoEngineCase::NotInstalled, "no engine found"),
            (NoEngineCase::DockerNotRunning, "docker is not running"),
            (NoEngineCase::PodmanApiOff, "podman socket is off"),
            (
                NoEngineCase::DockerAndPodmanOff,
                "docker and podman are not answering",
            ),
            (
                NoEngineCase::NoPermission,
                "permission denied on /var/run/docker.sock",
            ),
            (
                NoEngineCase::PodmanNoPermission,
                "permission denied on /var/run/docker.sock",
            ),
            (
                NoEngineCase::NoPermissionAndOtherOff(EngineKind::Docker),
                "permission denied on /var/run/docker.sock, podman socket is off",
            ),
            (
                NoEngineCase::NoPermissionAndOtherOff(EngineKind::Podman),
                "permission denied on /var/run/docker.sock, docker is not running",
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
