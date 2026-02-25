mod errors;
mod event;
mod model;
mod services;
mod state;

/// Capability token that allows mutable explorer state access through app state.
pub(crate) struct ExplorerWritePermit(());

pub(crate) use event::{ExplorerDeps, ExplorerEvent, explorer_reducer};
pub(crate) use model::FileNode;
pub(crate) use state::ExplorerState;
