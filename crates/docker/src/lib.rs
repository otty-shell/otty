//! Finds the local Docker and Podman engines and talks to them.
//!
//! This crate is the only place in OTTY that uses `bollard`. Both engines
//! serve the Docker Engine API, so one client covers them; whether an engine
//! is Docker or Podman is read from its `/version` answer.
//!
//! Start with [`discover`]: it tries the known sockets and returns a
//! [`Discovery`] whose [`Diagnosis`] lists the reached engines or says why
//! none was reached. Then [`Engine::connect`] to a reached engine and load
//! its [`Container`]s, [`Image`]s, [`Volume`]s and [`Network`]s;
//! [`Engine::watch`] says when one of those lists changed.

mod address;
mod candidates;
mod cli;
mod connection;
mod container;
mod debounce;
mod diagnosis;
mod discovery;
mod engine;
mod events;
#[cfg(feature = "fixtures")]
pub mod fixtures;
mod image;
mod image_defaults;
mod inspect;
mod logs;
mod network;
mod probe;
mod pull;
mod run;
mod selinux;
#[cfg(test)]
mod test_support;
mod timestamp;
mod volume;
mod watch;

pub use crate::address::EngineAddress;
pub use crate::candidates::{CandidateSource, SocketCandidate};
pub use crate::cli::CliPresence;
pub use crate::connection::{Engine, EngineError, Resources};
pub use crate::container::{
    Container, ContainerNetwork, ContainerState, Mount, MountKind, PortMapping,
};
pub use crate::diagnosis::Diagnosis;
pub use crate::discovery::{Discovery, discover};
pub use crate::engine::{EngineInfo, EngineKind};
pub use crate::events::ListKind;
pub use crate::image::Image;
pub use crate::image_defaults::ImageDefaults;
pub use crate::network::Network;
pub use crate::probe::{Probe, ProbeOutcome};
pub use crate::pull::PullProgress;
pub use crate::run::{
    MountSource, PortProtocol, PublishedPort, RestartPolicy, RunRequest,
    VolumeMount,
};
pub use crate::selinux::is_selinux_enabled;
pub use crate::volume::Volume;
pub use crate::watch::WatchEvent;
