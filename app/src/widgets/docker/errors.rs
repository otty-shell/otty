use otty_docker::EngineError;
use thiserror::Error;

/// Why something the Docker widget asked for failed. It holds the text to
/// show, because the intents that carry it must stay `Clone + PartialEq`,
/// which the engine's own error is not.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub(crate) struct DockerError(String);

#[cfg(test)]
impl DockerError {
    /// An error showing `text`.
    pub(crate) fn new(text: &str) -> Self {
        Self(text.to_string())
    }
}

impl From<EngineError> for DockerError {
    /// The engine's own words when it refused a request, otherwise the whole
    /// error text.
    fn from(err: EngineError) -> Self {
        Self(err.message())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn it_shows_its_text_as_it_is() {
        let error = DockerError::new("port is already allocated");

        assert_eq!(error.to_string(), "port is already allocated");
    }

    #[test]
    fn an_engine_error_becomes_the_text_of_its_message() {
        let engine_error = EngineError::PathNotUtf8(PathBuf::from("/run/x"));
        let message = engine_error.message();

        let error = DockerError::from(engine_error);

        assert_eq!(error.to_string(), message);
    }
}
