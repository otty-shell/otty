//! One place of the engine search order and the step that listed it.

use crate::address::EngineAddress;

/// Which step of the search order produced a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateSource {
    /// The `DOCKER_HOST` environment variable.
    DockerHost,
    /// A well-known Docker socket.
    Docker,
    /// A well-known Podman socket.
    Podman,
}

/// One place where a local engine may listen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketCandidate {
    address: EngineAddress,
    source: CandidateSource,
}

impl SocketCandidate {
    /// Where to connect.
    pub fn address(&self) -> &EngineAddress {
        &self.address
    }

    /// Which step of the search order found this place.
    pub fn source(&self) -> CandidateSource {
        self.source
    }

    /// Create a candidate.
    #[cfg(feature = "client")]
    pub(crate) fn new(address: EngineAddress, source: CandidateSource) -> Self {
        Self { address, source }
    }
}
