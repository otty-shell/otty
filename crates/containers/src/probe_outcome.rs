//! What happened when OTTY tried one place of the engine search.

use crate::engine::EngineInfo;
use crate::socket_candidate::SocketCandidate;

/// What happened when OTTY tried one candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// An engine answered `/version`.
    Reached(EngineInfo),
    /// There is no socket file at this path, or OTTY cannot see it.
    Missing,
    /// The socket is there, but connecting to it was refused for this user.
    PermissionDenied,
    /// Something is there but no engine answered in time; the text says why.
    NotAnswering(String),
}

/// One candidate and what happened when OTTY tried it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    candidate: SocketCandidate,
    outcome: ProbeOutcome,
}

impl Probe {
    /// The place that was tried.
    pub fn candidate(&self) -> &SocketCandidate {
        &self.candidate
    }

    /// What happened.
    pub fn outcome(&self) -> &ProbeOutcome {
        &self.outcome
    }

    /// Create a probe result.
    #[cfg(feature = "client")]
    pub(crate) fn new(
        candidate: SocketCandidate,
        outcome: ProbeOutcome,
    ) -> Self {
        Self { candidate, outcome }
    }
}
