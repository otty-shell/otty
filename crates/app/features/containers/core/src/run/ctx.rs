use std::path::Path;

use otty_containers::{EngineAddress, EngineInfo, Network};

/// What the Run form reads from the rest of the feature: the reached engines
/// with their networks and the facts of the last engine search.
#[derive(Debug)]
pub(crate) struct RunCtx<'a> {
    /// The reached engines, in search order.
    pub(crate) engines: Vec<RunEngine<'a>>,
    /// Whether SELinux is enabled, from the last finished search.
    pub(crate) is_selinux_enabled: bool,
    /// The home folder, from the last finished search; `~` stands for it.
    pub(crate) home_folder: Option<&'a Path>,
}

impl<'a> RunCtx<'a> {
    /// Return the reached engine listening at `address`.
    pub(crate) fn engine(
        &self,
        address: &EngineAddress,
    ) -> Option<&RunEngine<'a>> {
        self.engines
            .iter()
            .find(|engine| engine.info.address() == address)
    }
}

/// One reached engine as the Run form sees it.
#[derive(Debug)]
pub(crate) struct RunEngine<'a> {
    /// What the engine said about itself.
    pub(crate) info: &'a EngineInfo,
    /// The engine's loaded networks; empty while they load or failed.
    pub(crate) networks: &'a [Network],
}
