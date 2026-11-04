//! One full engine search: build the list, try every place, diagnose.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::future::{join, join_all};

use crate::candidates::{DiscoveryEnv, drop_same_sockets, socket_candidates};
use crate::cli::{CliPresence, command_dirs};
use crate::diagnosis::{Diagnosis, diagnose};
use crate::probe::{Probe, probe};
use crate::selinux;

/// How long one candidate may take to connect and answer `/version`.
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
/// How long `docker --version` may take (the `podman-docker` check). It runs
/// alongside the probes, so it adds no time to the search.
const CLI_VERSION_TIMEOUT: Duration = Duration::from_millis(1500);

/// The result of one engine search: what was tried and what it means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    probes: Vec<Probe>,
    cli: CliPresence,
    diagnosis: Diagnosis,
    is_selinux_enabled: bool,
    home_folder: Option<PathBuf>,
}

impl Discovery {
    /// Every place that was tried, in search order, with its outcome. This
    /// is the "looked for" line under the no-engine cards.
    pub fn probes(&self) -> &[Probe] {
        &self.probes
    }

    /// Which engine command line tools are installed.
    pub fn cli(&self) -> &CliPresence {
        &self.cli
    }

    /// What the search means for the user.
    pub fn diagnosis(&self) -> &Diagnosis {
        &self.diagnosis
    }

    /// Whether SELinux is enabled on this computer, which decides if a
    /// folder mounted into a container can need the `z` option.
    pub fn is_selinux_enabled(&self) -> bool {
        self.is_selinux_enabled
    }

    /// The home folder of this user, which `~` in a folder path stands for.
    /// `None` when `$HOME` is not set or is empty.
    ///
    /// ```
    /// # #[cfg(feature = "fixtures")]
    /// # {
    /// use std::path::Path;
    ///
    /// use otty_containers::{EngineKind, fixtures};
    ///
    /// let known = fixtures::discovery(&[EngineKind::Docker]);
    /// let unknown = fixtures::discovery_without_home(&[EngineKind::Docker]);
    ///
    /// assert_eq!(known.home_folder(), Some(Path::new(fixtures::HOME_FOLDER)));
    /// assert_eq!(unknown.home_folder(), None);
    /// # }
    /// ```
    pub fn home_folder(&self) -> Option<&Path> {
        self.home_folder.as_deref()
    }

    /// Diagnose the given search results.
    pub(crate) fn new(
        probes: Vec<Probe>,
        cli: CliPresence,
        is_selinux_enabled: bool,
        home_folder: Option<PathBuf>,
    ) -> Self {
        let diagnosis = diagnose(&probes, cli.clone());

        Self {
            probes,
            cli,
            diagnosis,
            is_selinux_enabled,
            home_folder,
        }
    }
}

/// Look for local Docker and Podman engines (spec Decisions 8 and 9).
///
/// Tries every candidate at once, each with a short time limit and no
/// retries, while it looks for the `docker` and `podman` commands (and asks
/// a found `docker` for its version, to spot Podman's `podman-docker`), so
/// the whole search takes about two seconds at most. Also reads whether
/// SELinux is enabled and where the home folder is. Never fails: an unreachable
/// engine is part of the result.
///
/// Needs a tokio runtime.
///
/// ```no_run
/// # async fn run() {
/// use otty_containers::{Diagnosis, discover};
///
/// let discovery = discover().await;
/// if let Diagnosis::Engines(engines) = discovery.diagnosis() {
///     for engine in engines {
///         println!(
///             "{:?} {:?} at {}",
///             engine.kind(),
///             engine.version(),
///             engine.address()
///         );
///     }
/// }
/// # }
/// ```
pub async fn discover() -> Discovery {
    let env = DiscoveryEnv::from_process();
    let candidates =
        drop_same_sockets(socket_candidates(&env), resolve_socket_path);
    let command_dirs =
        command_dirs(std::env::var_os("PATH").as_deref(), env.home.as_deref());

    let probes = join_all(
        candidates
            .into_iter()
            .map(|candidate| probe(candidate, PROBE_TIMEOUT)),
    );
    let cli = CliPresence::find(&command_dirs, CLI_VERSION_TIMEOUT);
    let (probes, cli) = join(probes, cli).await;

    for probe in &probes {
        let address = probe.candidate().address();
        let outcome = probe.outcome();
        log::debug!("docker discovery: {address}: {outcome:?}");
    }

    Discovery::new(probes, cli, selinux::is_selinux_enabled(), env.home)
}

/// Follow links so the same socket is tried once; a missing path stays as
/// it is.
fn resolve_socket_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn discovery(is_selinux_enabled: bool, home: Option<PathBuf>) -> Discovery {
        Discovery::new(
            Vec::new(),
            CliPresence::new(false, false),
            is_selinux_enabled,
            home,
        )
    }

    #[test]
    fn a_discovery_carries_the_selinux_fact() {
        assert!(discovery(true, None).is_selinux_enabled());
        assert!(!discovery(false, None).is_selinux_enabled());
    }

    #[test]
    fn a_discovery_carries_the_home_folder_when_it_is_known() {
        let known = discovery(false, Some(PathBuf::from("/home/ann")));
        let unknown = discovery(false, None);

        assert_eq!(known.home_folder(), Some(Path::new("/home/ann")));
        assert_eq!(unknown.home_folder(), None);
    }
}
