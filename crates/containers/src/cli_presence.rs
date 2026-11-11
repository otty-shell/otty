//! Which engine command line tools are installed, as the search found them.

use std::path::{Path, PathBuf};

/// Which engine command line tools are installed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliPresence {
    docker: bool,
    podman: bool,
    docker_path: Option<PathBuf>,
    podman_path: Option<PathBuf>,
}

impl CliPresence {
    /// Docker's `docker` command is installed. A `docker` command that is
    /// really Podman does not count here.
    pub fn docker(&self) -> bool {
        self.docker
    }

    /// `podman` is installed, or a `docker` command that is really Podman.
    pub fn podman(&self) -> bool {
        self.podman
    }

    /// The full path of the `docker` command, to run it when the app's own
    /// `PATH` is short. `None` when it is not installed or is really Podman.
    pub fn docker_path(&self) -> Option<&Path> {
        self.docker_path.as_deref()
    }

    /// The full path of the command that runs Podman: `podman`, or else a
    /// `docker` command that is really Podman.
    pub fn podman_path(&self) -> Option<&Path> {
        self.podman_path.as_deref()
    }

    /// Combine what the lookup found: a `docker` command that is really
    /// Podman counts as `podman`, not as `docker`.
    #[cfg(feature = "client")]
    pub(crate) fn from_lookup(
        docker_found: bool,
        docker_is_podman: bool,
        podman_found: bool,
    ) -> Self {
        let podman_docker = docker_found && docker_is_podman;

        Self {
            docker: docker_found && !docker_is_podman,
            podman: podman_found || podman_docker,
            docker_path: None,
            podman_path: None,
        }
    }

    /// Add the full paths of the commands.
    #[cfg(feature = "client")]
    pub(crate) fn with_paths(
        mut self,
        docker_path: Option<PathBuf>,
        podman_path: Option<PathBuf>,
    ) -> Self {
        self.docker_path = docker_path;
        self.podman_path = podman_path;
        self
    }
}

#[cfg(all(feature = "client", any(test, feature = "fixtures")))]
impl CliPresence {
    /// Create a presence from known answers.
    pub(crate) fn new(docker: bool, podman: bool) -> Self {
        Self {
            docker,
            podman,
            docker_path: None,
            podman_path: None,
        }
    }
}
