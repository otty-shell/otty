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
//!
//! # Features
//!
//! - `client` (on by default): the engine client and everything else that
//!   talks to an engine or to this computer, such as [`discover`],
//!   [`Engine`], [`host_folder_exists`] and [`wait_for`], and the mapping of
//!   the engine's answers into the data types. Without it the crate has only
//!   the data types, for crates that show engines but must not call one.
//! - `fixtures`: values mapped from captured engine answers, for tests. It
//!   turns on `client`.
#![cfg_attr(
    not(feature = "client"),
    doc = r#"
This build has no `client`, so there is no engine to call:

```compile_fail
use otty_containers::Engine;
```

```compile_fail
use otty_containers::discover;
```
"#
)]

mod address;
#[cfg(feature = "client")]
mod candidates;
#[cfg(feature = "client")]
mod cli;
mod cli_presence;
#[cfg(feature = "client")]
mod connection;
mod container;
#[cfg(feature = "client")]
mod debounce;
mod diagnosis;
mod discovery;
mod engine;
#[cfg(feature = "client")]
mod events;
#[cfg(feature = "fixtures")]
pub mod fixtures;
#[cfg(feature = "client")]
mod host_folder;
mod image;
mod image_defaults;
#[cfg(feature = "client")]
mod inspect;
#[cfg(feature = "client")]
mod log_answer;
mod logs;
mod network;
#[cfg(feature = "client")]
mod probe;
mod probe_outcome;
mod pull;
mod resources;
mod run;
#[cfg(feature = "client")]
mod selinux;
mod socket_candidate;
#[cfg(all(test, feature = "client"))]
mod test_support;
mod text_order;
#[cfg(feature = "client")]
mod timestamp;
mod volume;
#[cfg(feature = "client")]
mod watch;
mod watch_event;

pub use crate::address::EngineAddress;
pub use crate::cli_presence::CliPresence;
#[cfg(feature = "client")]
pub use crate::connection::{Engine, EngineError};
pub use crate::container::{
    COMPOSE_PROJECT_LABEL, Container, ContainerNetwork, ContainerState, Mount,
    MountKind, PortMapping,
};
pub use crate::diagnosis::Diagnosis;
pub use crate::discovery::Discovery;
#[cfg(feature = "client")]
pub use crate::discovery::discover;
pub use crate::engine::{EngineInfo, EngineKind};
#[cfg(feature = "client")]
pub use crate::host_folder::{host_folder_exists, wait_for};
pub use crate::image::Image;
pub use crate::image_defaults::ImageDefaults;
pub use crate::logs::{ContainerRun, LogLine, LogPosition, MAX_LOG_LINE_BYTES};
pub use crate::network::Network;
pub use crate::probe_outcome::{Probe, ProbeOutcome};
pub use crate::pull::PullProgress;
pub use crate::resources::Resources;
pub use crate::run::{
    MountSource, PortProtocol, PublishedPort, RestartPolicy, RunRequest,
    VolumeMount,
};
pub use crate::socket_candidate::{CandidateSource, SocketCandidate};
pub use crate::text_order::compare_text;
pub use crate::volume::Volume;
pub use crate::watch_event::{ListKind, WatchEvent};
