//! One full engine search: build the list, try every place, diagnose.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::future::join_all;

use crate::candidates::{DiscoveryEnv, drop_same_sockets, socket_candidates};
use crate::cli::CliPresence;
use crate::diagnosis::{Diagnosis, diagnose};
use crate::probe::{Probe, probe};

/// How long one candidate may take to connect and answer `/version`.
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// The result of one engine search: what was tried and what it means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    probes: Vec<Probe>,
    cli: CliPresence,
    diagnosis: Diagnosis,
}

impl Discovery {
    /// Every place that was tried, in search order, with its outcome. This
    /// is the "looked for" line under the no-engine cards.
    pub fn probes(&self) -> &[Probe] {
        &self.probes
    }

    /// Which engine command line tools are on `PATH`.
    pub fn cli(&self) -> CliPresence {
        self.cli
    }

    /// What the search means for the user.
    pub fn diagnosis(&self) -> &Diagnosis {
        &self.diagnosis
    }

    /// Diagnose the given search results.
    pub(crate) fn new(probes: Vec<Probe>, cli: CliPresence) -> Self {
        let diagnosis = diagnose(&probes, cli);

        Self {
            probes,
            cli,
            diagnosis,
        }
    }
}

/// Look for local Docker and Podman engines (spec Decisions 8 and 9).
///
/// Tries every candidate at once, each with a short time limit and no
/// retries, so the whole search takes about two seconds at most. Never
/// fails: an unreachable engine is part of the result.
///
/// Needs a tokio runtime.
///
/// ```no_run
/// # async fn run() {
/// use otty_docker::{Diagnosis, discover};
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

    let probes = join_all(
        candidates
            .into_iter()
            .map(|candidate| probe(candidate, PROBE_TIMEOUT)),
    )
    .await;
    let cli = CliPresence::from_path(std::env::var_os("PATH").as_deref());

    for probe in &probes {
        let address = probe.candidate().address();
        let outcome = probe.outcome();
        log::debug!("docker discovery: {address}: {outcome:?}");
    }

    Discovery::new(probes, cli)
}

/// Follow links so the same socket is tried once; a missing path stays as
/// it is.
fn resolve_socket_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
