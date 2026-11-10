use std::path::Path;

use otty_containers::{CliPresence, EngineKind};

use crate::types::OpenList;

/// What the actions read from the rest of the feature: the open list and the
/// command line tools the last finished search found.
#[derive(Debug)]
pub(crate) struct ActionsCtx<'a> {
    /// The list the main area shows; `None` without engines.
    pub(crate) open: Option<OpenList<'a>>,
    /// The command line tools the last finished search found; `None` while
    /// a search runs.
    pub(crate) cli: Option<&'a CliPresence>,
}

impl<'a> ActionsCtx<'a> {
    /// Return the full path of the command line tool of `kind`, when the
    /// last search found it (a `podman-docker` wrapper counts as Podman).
    pub(crate) fn cli_path(&self, kind: EngineKind) -> Option<&'a Path> {
        let cli = self.cli?;

        match kind {
            EngineKind::Docker => cli.docker_path(),
            EngineKind::Podman => cli.podman_path(),
        }
    }
}
