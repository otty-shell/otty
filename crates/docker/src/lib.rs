//! Finds the local Docker and Podman engines and talks to them.
//!
//! This crate is the only place in OTTY that uses `bollard`. Both engines
//! serve the Docker Engine API, so one client covers them; whether an engine
//! is Docker or Podman is read from its `/version` answer.
//!
//! Start with [`discover`]: it tries the known sockets and returns a
//! [`Discovery`] whose [`Diagnosis`] lists the reached engines or says why
//! none was reached.

mod address;
mod candidates;
mod cli;
mod diagnosis;
mod discovery;
mod engine;
mod probe;
#[cfg(test)]
mod test_support;

pub use crate::address::EngineAddress;
pub use crate::candidates::{CandidateSource, SocketCandidate};
pub use crate::cli::CliPresence;
pub use crate::diagnosis::Diagnosis;
pub use crate::discovery::{Discovery, discover};
pub use crate::engine::{EngineInfo, EngineKind};
pub use crate::probe::{Probe, ProbeOutcome};
