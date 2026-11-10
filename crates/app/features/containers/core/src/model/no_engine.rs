use otty_app_i18n::{self as i18n, Key};
use otty_containers::{CandidateSource, Discovery, EngineKind};

use super::super::no_engine::no_engine_case;
use super::super::state::{ContainersState, Search};
use super::super::types::NoEngineCase;

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
    pub(crate) commands: Vec<NoEngineCommandViewModel>,
    /// Install pages; only when nothing is installed.
    pub(crate) installs: Vec<NoEngineLinkViewModel>,
    /// The "How to fix" buttons, in the order of the commands.
    pub(crate) fixes: Vec<NoEngineFixViewModel>,
    /// What OTTY looked for, shown under the card.
    pub(crate) looked_for: String,
}

impl NoEngineViewModel {
    /// Build the card of a finished search; `None` while the search runs or
    /// when an engine answered.
    pub(in super::super) fn from_state(
        state: &ContainersState,
    ) -> Option<Self> {
        let Search::Done(discovery) = state.search() else {
            return None;
        };
        let (case, socket) = no_engine_case(discovery)?;

        let places = looked_for_places(discovery);

        Some(Self::for_case(case, &socket, &places))
    }

    /// Build the card of `case`. `socket` is the denied socket (used by the
    /// cases of a refused socket); `places` are the sockets OTTY tried.
    pub(in super::super) fn for_case(
        case: NoEngineCase,
        socket: &str,
        places: &[String],
    ) -> Self {
        let looked_for = if places.is_empty() {
            i18n::t(Key::ContainersLookedForCommands).to_string()
        } else {
            i18n::containers_looked_for(&places.join(", "))
        };

        match case {
            NoEngineCase::NotInstalled => NoEngineViewModel {
                case,
                title: i18n::t(Key::ContainersNotInstalledTitle),
                description: i18n::t(Key::ContainersNotInstalledText)
                    .to_string(),
                panel_text: i18n::t(Key::ContainersPanelNotInstalled),
                status_text: i18n::t(Key::ContainersStatusNoEngine).to_string(),
                commands: Vec::new(),
                installs: vec![
                    NoEngineLinkViewModel {
                        label: i18n::t(Key::ContainersInstallDocker),
                        site: "docs.docker.com",
                        url: DOCKER_INSTALL_URL,
                    },
                    NoEngineLinkViewModel {
                        label: i18n::t(Key::ContainersInstallPodman),
                        site: "podman.io",
                        url: PODMAN_INSTALL_URL,
                    },
                ],
                fixes: Vec::new(),
                looked_for,
            },
            NoEngineCase::DockerNotRunning => NoEngineViewModel {
                case,
                title: i18n::t(Key::ContainersDockerStoppedTitle),
                description: i18n::t(Key::ContainersDockerStoppedText)
                    .to_string(),
                panel_text: i18n::t(Key::ContainersPanelDockerStopped),
                status_text: i18n::t(Key::ContainersStatusDockerStopped)
                    .to_string(),
                commands: vec![docker_start_command(
                    Key::ContainersRunInTerminal,
                )],
                installs: Vec::new(),
                fixes: vec![how_to_fix(
                    Key::ContainersHowToFix,
                    DOCKER_START_URL,
                )],
                looked_for,
            },
            NoEngineCase::PodmanApiOff => NoEngineViewModel {
                case,
                title: i18n::t(Key::ContainersPodmanOffTitle),
                description: i18n::t(Key::ContainersPodmanOffText).to_string(),
                panel_text: i18n::t(Key::ContainersPanelPodmanOff),
                status_text: i18n::t(Key::ContainersStatusPodmanOff)
                    .to_string(),
                commands: vec![podman_socket_command(
                    Key::ContainersRunInTerminal,
                )],
                installs: Vec::new(),
                fixes: vec![how_to_fix(
                    Key::ContainersHowToFix,
                    PODMAN_SERVICE_URL,
                )],
                looked_for,
            },
            NoEngineCase::DockerAndPodmanOff => NoEngineViewModel {
                case,
                title: i18n::t(Key::ContainersBothOffTitle),
                description: i18n::t(Key::ContainersBothOffText).to_string(),
                panel_text: i18n::t(Key::ContainersPanelBothOff),
                status_text: i18n::t(Key::ContainersStatusBothOff).to_string(),
                commands: vec![
                    docker_start_command(Key::ContainersBothOffDockerLabel),
                    podman_socket_command(Key::ContainersBothOffPodmanLabel),
                ],
                installs: Vec::new(),
                fixes: vec![
                    how_to_fix(Key::ContainersHowToFixDocker, DOCKER_START_URL),
                    how_to_fix(
                        Key::ContainersHowToFixPodman,
                        PODMAN_SERVICE_URL,
                    ),
                ],
                looked_for,
            },
            NoEngineCase::NoPermission => NoEngineViewModel {
                case,
                title: i18n::t(Key::ContainersDockerNoAccessTitle),
                description: i18n::containers_docker_no_access_text(socket),
                panel_text: i18n::t(Key::ContainersPanelDockerNoAccess),
                status_text: i18n::containers_status_no_access(socket),
                commands: vec![docker_group_command()],
                installs: Vec::new(),
                fixes: vec![how_to_fix(
                    Key::ContainersHowToFix,
                    DOCKER_GROUP_URL,
                )],
                looked_for,
            },
            NoEngineCase::PodmanNoPermission => NoEngineViewModel {
                case,
                title: i18n::t(Key::ContainersPodmanNoAccessTitle),
                description: i18n::containers_podman_no_access_text(socket),
                panel_text: i18n::t(Key::ContainersPanelPodmanNoAccess),
                status_text: i18n::containers_status_no_access(socket),
                commands: vec![podman_own_socket_command()],
                installs: Vec::new(),
                fixes: vec![how_to_fix(
                    Key::ContainersHowToFix,
                    PODMAN_SERVICE_URL,
                )],
                looked_for,
            },
            NoEngineCase::NoPermissionAndOtherOff(EngineKind::Docker) => {
                NoEngineViewModel {
                    case,
                    title: i18n::t(Key::ContainersDockerNoAccessPodmanOffTitle),
                    description:
                        i18n::containers_docker_no_access_podman_off_text(
                            socket,
                        ),
                    panel_text: i18n::t(
                        Key::ContainersPanelDockerNoAccessPodmanOff,
                    ),
                    status_text:
                        i18n::containers_status_docker_no_access_podman_off(
                            socket,
                        ),
                    commands: vec![
                        docker_group_command(),
                        podman_socket_command(
                            Key::ContainersBothOffPodmanLabel,
                        ),
                    ],
                    installs: Vec::new(),
                    fixes: vec![
                        how_to_fix(
                            Key::ContainersHowToFixDocker,
                            DOCKER_GROUP_URL,
                        ),
                        how_to_fix(
                            Key::ContainersHowToFixPodman,
                            PODMAN_SERVICE_URL,
                        ),
                    ],
                    looked_for,
                }
            },
            NoEngineCase::NoPermissionAndOtherOff(EngineKind::Podman) => {
                NoEngineViewModel {
                    case,
                    title: i18n::t(Key::ContainersPodmanNoAccessDockerOffTitle),
                    description:
                        i18n::containers_podman_no_access_docker_off_text(
                            socket,
                        ),
                    panel_text: i18n::t(
                        Key::ContainersPanelPodmanNoAccessDockerOff,
                    ),
                    status_text:
                        i18n::containers_status_podman_no_access_docker_off(
                            socket,
                        ),
                    commands: vec![
                        podman_own_socket_command(),
                        docker_start_command(Key::ContainersBothOffDockerLabel),
                    ],
                    installs: Vec::new(),
                    fixes: vec![
                        how_to_fix(
                            Key::ContainersHowToFixPodman,
                            PODMAN_SERVICE_URL,
                        ),
                        how_to_fix(
                            Key::ContainersHowToFixDocker,
                            DOCKER_START_URL,
                        ),
                    ],
                    looked_for,
                }
            },
        }
    }
}

/// A command to run in a terminal, with the text around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoEngineCommandViewModel {
    pub(crate) label: &'static str,
    pub(crate) text: &'static str,
    pub(crate) hint: &'static str,
}

/// A button that opens a page in the browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoEngineLinkViewModel {
    pub(crate) label: &'static str,
    /// The site name shown next to the label.
    pub(crate) site: &'static str,
    pub(crate) url: &'static str,
}

/// A button that opens the page about the fix of a case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoEngineFixViewModel {
    pub(crate) label: &'static str,
    pub(crate) url: &'static str,
}

/// The sockets the engine search tried, as the "looked for" line names them.
fn looked_for_places(discovery: &Discovery) -> Vec<String> {
    discovery
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
        .collect()
}

/// The command that adds the user to the docker group, with its warning.
fn docker_group_command() -> NoEngineCommandViewModel {
    NoEngineCommandViewModel {
        label: i18n::t(Key::ContainersDockerNoAccessLabel),
        text: DOCKER_GROUP_COMMAND,
        hint: i18n::t(Key::ContainersDockerNoAccessHint),
    }
}

/// The command that turns on the user's own Podman socket, for a user a Podman
/// socket refused.
fn podman_own_socket_command() -> NoEngineCommandViewModel {
    NoEngineCommandViewModel {
        label: i18n::t(Key::ContainersPodmanNoAccessLabel),
        text: PODMAN_SOCKET_COMMAND,
        hint: i18n::t(Key::ContainersPodmanNoAccessHint),
    }
}

/// The command that starts Docker, with its macOS note under the label `label`.
fn docker_start_command(label: Key) -> NoEngineCommandViewModel {
    NoEngineCommandViewModel {
        label: i18n::t(label),
        text: DOCKER_START_COMMAND,
        hint: i18n::t(Key::ContainersDockerStoppedHint),
    }
}

/// The command that turns on the Podman socket, with its macOS note under the
/// label `label`.
fn podman_socket_command(label: Key) -> NoEngineCommandViewModel {
    NoEngineCommandViewModel {
        label: i18n::t(label),
        text: PODMAN_SOCKET_COMMAND,
        hint: i18n::t(Key::ContainersPodmanOffHint),
    }
}

fn how_to_fix(label: Key, url: &'static str) -> NoEngineFixViewModel {
    NoEngineFixViewModel {
        label: i18n::t(label),
        url,
    }
}

#[cfg(test)]
mod tests {
    use otty_app_i18n::{self as i18n, Key};
    use otty_containers::{CandidateSource, Discovery, EngineKind, fixtures};

    use super::*;
    use crate::event::ContainersIntent;
    use crate::reducer::reduce;
    use crate::state::ContainersState;
    use crate::types::NoEngineCase;

    /// The no-engine card of a state whose search found `discovery`.
    fn card(discovery: Discovery) -> Option<NoEngineViewModel> {
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ =
            reduce(&mut state, ContainersIntent::DiscoveryFinished(discovery));

        NoEngineViewModel::from_state(&state)
    }

    fn places() -> Vec<String> {
        vec![
            String::from("/var/run/docker.sock"),
            String::from("/run/podman/podman.sock"),
        ]
    }

    fn one_fix(url: &'static str) -> NoEngineFixViewModel {
        NoEngineFixViewModel {
            label: i18n::t(Key::ContainersHowToFix),
            url,
        }
    }

    #[test]
    fn nothing_installed_offers_both_install_pages_and_no_command() {
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::NotInstalled,
            "",
            &places(),
        );

        assert_eq!(vm.title, i18n::t(Key::ContainersNotInstalledTitle));
        assert_eq!(vm.description, i18n::t(Key::ContainersNotInstalledText));
        assert_eq!(vm.panel_text, i18n::t(Key::ContainersPanelNotInstalled));
        assert!(vm.commands.is_empty());
        assert!(vm.fixes.is_empty());
        assert_eq!(
            vm.installs,
            vec![
                NoEngineLinkViewModel {
                    label: i18n::t(Key::ContainersInstallDocker),
                    site: "docs.docker.com",
                    url: "https://docs.docker.com/get-started/get-docker/",
                },
                NoEngineLinkViewModel {
                    label: i18n::t(Key::ContainersInstallPodman),
                    site: "podman.io",
                    url: "https://podman.io/docs/installation",
                },
            ]
        );
    }

    #[test]
    fn docker_not_running_gives_the_start_command_and_its_page() {
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::DockerNotRunning,
            "",
            &places(),
        );

        assert_eq!(vm.title, i18n::t(Key::ContainersDockerStoppedTitle));
        assert_eq!(vm.description, i18n::t(Key::ContainersDockerStoppedText));
        assert_eq!(vm.panel_text, i18n::t(Key::ContainersPanelDockerStopped));
        assert_eq!(
            vm.commands,
            vec![NoEngineCommandViewModel {
                label: i18n::t(Key::ContainersRunInTerminal),
                text: "sudo systemctl start docker",
                hint: i18n::t(Key::ContainersDockerStoppedHint),
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
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::PodmanApiOff,
            "",
            &places(),
        );

        assert_eq!(vm.title, i18n::t(Key::ContainersPodmanOffTitle));
        assert_eq!(vm.description, i18n::t(Key::ContainersPodmanOffText));
        assert_eq!(vm.panel_text, i18n::t(Key::ContainersPanelPodmanOff));
        assert_eq!(
            vm.commands,
            vec![NoEngineCommandViewModel {
                label: i18n::t(Key::ContainersRunInTerminal),
                text: "systemctl --user enable --now podman.socket",
                hint: i18n::t(Key::ContainersPodmanOffHint),
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
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::DockerAndPodmanOff,
            "",
            &places(),
        );

        assert_eq!(vm.title, i18n::t(Key::ContainersBothOffTitle));
        assert_eq!(vm.description, i18n::t(Key::ContainersBothOffText));
        assert_eq!(vm.panel_text, i18n::t(Key::ContainersPanelBothOff));
        assert_eq!(vm.status_text, i18n::t(Key::ContainersStatusBothOff));
        assert_eq!(
            vm.commands,
            vec![
                NoEngineCommandViewModel {
                    label: i18n::t(Key::ContainersBothOffDockerLabel),
                    text: "sudo systemctl start docker",
                    hint: i18n::t(Key::ContainersDockerStoppedHint),
                },
                NoEngineCommandViewModel {
                    label: i18n::t(Key::ContainersBothOffPodmanLabel),
                    text: "systemctl --user enable --now podman.socket",
                    hint: i18n::t(Key::ContainersPodmanOffHint),
                },
            ]
        );
        assert!(vm.installs.is_empty());
    }

    #[test]
    fn both_engines_off_has_a_page_for_each_engine() {
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::DockerAndPodmanOff,
            "",
            &places(),
        );

        assert_eq!(
            vm.fixes,
            vec![
                NoEngineFixViewModel {
                    label: i18n::t(Key::ContainersHowToFixDocker),
                    url: "https://docs.docker.com/engine/daemon/start/",
                },
                NoEngineFixViewModel {
                    label: i18n::t(Key::ContainersHowToFixPodman),
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

        card(discovery)
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
    fn no_permission_names_the_socket_and_gives_the_group_command() {
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::NoPermission,
            "/var/run/docker.sock",
            &places(),
        );

        assert_eq!(vm.title, i18n::t(Key::ContainersDockerNoAccessTitle));
        assert_eq!(
            vm.description,
            i18n::containers_docker_no_access_text("/var/run/docker.sock")
        );
        assert!(vm.description.contains("/var/run/docker.sock"));
        assert_eq!(vm.panel_text, i18n::t(Key::ContainersPanelDockerNoAccess));
        assert_eq!(
            vm.commands,
            vec![NoEngineCommandViewModel {
                label: i18n::t(Key::ContainersDockerNoAccessLabel),
                text: "sudo usermod -aG docker $USER",
                hint: i18n::t(Key::ContainersDockerNoAccessHint),
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

        let vm = NoEngineViewModel::for_case(
            NoEngineCase::PodmanNoPermission,
            socket,
            &places(),
        );

        assert_eq!(vm.title, i18n::t(Key::ContainersPodmanNoAccessTitle));
        assert_eq!(
            vm.description,
            i18n::containers_podman_no_access_text(socket)
        );
        assert!(vm.description.contains(socket));
        assert_eq!(vm.panel_text, i18n::t(Key::ContainersPanelPodmanNoAccess));
        assert_eq!(vm.status_text, i18n::containers_status_no_access(socket));
        assert_eq!(
            vm.commands,
            vec![NoEngineCommandViewModel {
                label: i18n::t(Key::ContainersPodmanNoAccessLabel),
                text: "systemctl --user enable --now podman.socket",
                hint: i18n::t(Key::ContainersPodmanNoAccessHint),
            }]
        );
        assert_eq!(
            vm.fixes,
            vec![one_fix(
                "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html"
            )]
        );
    }

    const DOCKER_SOCKET: &str = "/var/run/docker.sock";
    const PODMAN_SOCKET: &str = "/run/podman/podman.sock";
    const PODMAN_COMMAND: &str = "systemctl --user enable --now podman.socket";
    const PODMAN_PAGE: &str = "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html";

    #[test]
    fn a_denied_docker_socket_with_podman_off_has_both_commands_and_pages() {
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::NoPermissionAndOtherOff(EngineKind::Docker),
            DOCKER_SOCKET,
            &places(),
        );

        assert_eq!(
            vm.title,
            i18n::t(Key::ContainersDockerNoAccessPodmanOffTitle)
        );
        assert_eq!(
            vm.description,
            i18n::containers_docker_no_access_podman_off_text(DOCKER_SOCKET)
        );
        assert!(vm.description.contains(DOCKER_SOCKET));
        assert_eq!(
            vm.panel_text,
            i18n::t(Key::ContainersPanelDockerNoAccessPodmanOff)
        );
        assert_eq!(
            vm.status_text,
            i18n::containers_status_docker_no_access_podman_off(DOCKER_SOCKET)
        );
        assert_eq!(
            vm.commands,
            vec![
                NoEngineCommandViewModel {
                    label: i18n::t(Key::ContainersDockerNoAccessLabel),
                    text: DOCKER_GROUP_COMMAND,
                    hint: i18n::t(Key::ContainersDockerNoAccessHint),
                },
                NoEngineCommandViewModel {
                    label: i18n::t(Key::ContainersBothOffPodmanLabel),
                    text: PODMAN_COMMAND,
                    hint: i18n::t(Key::ContainersPodmanOffHint),
                },
            ]
        );
        assert_eq!(
            vm.fixes,
            vec![
                NoEngineFixViewModel {
                    label: i18n::t(Key::ContainersHowToFixDocker),
                    url: "https://docs.docker.com/engine/install/linux-postinstall/",
                },
                NoEngineFixViewModel {
                    label: i18n::t(Key::ContainersHowToFixPodman),
                    url: PODMAN_PAGE,
                },
            ]
        );
        assert!(vm.installs.is_empty());
    }

    #[test]
    fn a_denied_podman_socket_with_docker_off_has_both_commands_and_pages() {
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::NoPermissionAndOtherOff(EngineKind::Podman),
            PODMAN_SOCKET,
            &places(),
        );

        assert_eq!(
            vm.title,
            i18n::t(Key::ContainersPodmanNoAccessDockerOffTitle)
        );
        assert_eq!(
            vm.description,
            i18n::containers_podman_no_access_docker_off_text(PODMAN_SOCKET)
        );
        assert!(vm.description.contains(PODMAN_SOCKET));
        assert_eq!(
            vm.panel_text,
            i18n::t(Key::ContainersPanelPodmanNoAccessDockerOff)
        );
        assert_eq!(
            vm.status_text,
            i18n::containers_status_podman_no_access_docker_off(PODMAN_SOCKET)
        );
        assert_eq!(
            vm.commands,
            vec![
                NoEngineCommandViewModel {
                    label: i18n::t(Key::ContainersPodmanNoAccessLabel),
                    text: PODMAN_COMMAND,
                    hint: i18n::t(Key::ContainersPodmanNoAccessHint),
                },
                NoEngineCommandViewModel {
                    label: i18n::t(Key::ContainersBothOffDockerLabel),
                    text: DOCKER_START_COMMAND,
                    hint: i18n::t(Key::ContainersDockerStoppedHint),
                },
            ]
        );
        assert_eq!(
            vm.fixes,
            vec![
                NoEngineFixViewModel {
                    label: i18n::t(Key::ContainersHowToFixPodman),
                    url: PODMAN_PAGE,
                },
                NoEngineFixViewModel {
                    label: i18n::t(Key::ContainersHowToFixDocker),
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

        let vm = card(discovery).expect("a card is shown");
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
        let vm = NoEngineViewModel::for_case(
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
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::NotInstalled,
            "",
            &places(),
        );

        assert_eq!(
            vm.looked_for,
            i18n::containers_looked_for(
                "/var/run/docker.sock, /run/podman/podman.sock"
            )
        );
        assert!(vm.looked_for.contains("/run/podman/podman.sock"));
    }

    #[test]
    fn the_looked_for_line_has_no_empty_place_list_without_probes() {
        let vm =
            NoEngineViewModel::for_case(NoEngineCase::NotInstalled, "", &[]);

        assert_eq!(vm.looked_for, i18n::t(Key::ContainersLookedForCommands));
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
            let vm = NoEngineViewModel::for_case(case, socket, &places());

            assert_eq!(vm.status_text, text);
        }
    }
}
