//! What an image sets on its own (listening ports, command, entrypoint,
//! working directory), for the side panel of the Run container form (spec
//! Decision 18).

#[cfg(feature = "client")]
use bollard::models::ImageInspect;

#[cfg(feature = "client")]
use crate::connection::{Engine, EngineError};

/// The engine's answer for an image that is not on this computer.
#[cfg(feature = "client")]
const NOT_FOUND: u16 = 404;

/// What an image sets on its own; the Run form overrides it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageDefaults {
    ports: Vec<String>,
    command: Vec<String>,
    entrypoint: Vec<String>,
    workdir: Option<String>,
    size: Option<u64>,
}

impl ImageDefaults {
    /// The ports the image says it listens on, for example `80/tcp`, sorted.
    pub fn ports(&self) -> &[String] {
        &self.ports
    }

    /// The command the container runs unless the form sets one.
    pub fn command(&self) -> &[String] {
        &self.command
    }

    /// The program the command is passed to.
    pub fn entrypoint(&self) -> &[String] {
        &self.entrypoint
    }

    /// The working directory; `None` when the image sets none (`/`).
    pub fn workdir(&self) -> Option<&str> {
        self.workdir.as_deref()
    }

    /// The size of the image on this computer, in bytes.
    pub fn size(&self) -> Option<u64> {
        self.size
    }
}

#[cfg(feature = "client")]
impl ImageDefaults {
    /// Read the defaults from an image inspect answer.
    pub(crate) fn from_inspect(answer: &ImageInspect) -> Self {
        let size = answer.size.and_then(|size| u64::try_from(size).ok());
        let Some(config) = answer.config.as_ref() else {
            return Self {
                size,
                ..Self::default()
            };
        };

        let mut ports = config.exposed_ports.clone().unwrap_or_default();
        ports.sort();
        let workdir = config
            .working_dir
            .as_deref()
            .filter(|dir| !dir.is_empty())
            .map(str::to_string);

        Self {
            ports,
            command: config.cmd.clone().unwrap_or_default(),
            entrypoint: config.entrypoint.clone().unwrap_or_default(),
            workdir,
            size,
        }
    }
}

#[cfg(feature = "client")]
impl Engine {
    /// What the image `reference` (for example `nginx:1.27`) sets on its
    /// own; `None` when the image is not on this computer.
    pub async fn image_defaults(
        &self,
        reference: &str,
    ) -> Result<Option<ImageDefaults>, EngineError> {
        match self.client().inspect_image(reference).await {
            Ok(answer) => Ok(Some(ImageDefaults::from_inspect(&answer))),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: NOT_FOUND,
                ..
            }) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(all(test, feature = "client"))]
mod tests {
    use super::*;
    use crate::test_support::FakeEngine;

    const NGINX: &str =
        include_str!("../tests/fixtures/docker-image-inspect.json");

    fn nginx() -> ImageInspect {
        serde_json::from_str(NGINX).expect("image inspect fixture")
    }

    #[test]
    fn the_defaults_are_read_from_the_image_config() {
        let defaults = ImageDefaults::from_inspect(&nginx());

        assert_eq!(defaults.ports(), ["443/tcp", "80/tcp"]);
        assert_eq!(defaults.command(), ["nginx", "-g", "daemon off;"]);
        assert_eq!(defaults.entrypoint(), ["/docker-entrypoint.sh"]);
        assert_eq!(defaults.workdir(), None);
        assert_eq!(defaults.size(), Some(192_004_563));
    }

    #[test]
    fn an_image_without_a_config_sets_nothing() {
        let answer: ImageInspect =
            serde_json::from_str(r#"{"Id":"sha256:abc","WorkingDir":""}"#)
                .expect("inspect answer");

        let defaults = ImageDefaults::from_inspect(&answer);

        assert_eq!(defaults, ImageDefaults::default());
    }

    #[test]
    fn a_working_directory_is_kept() {
        let answer: ImageInspect = serde_json::from_str(
            r#"{"Config":{"WorkingDir":"/app"},"Size":10}"#,
        )
        .expect("inspect answer");

        let defaults = ImageDefaults::from_inspect(&answer);

        assert_eq!(defaults.workdir(), Some("/app"));
    }

    #[tokio::test]
    async fn an_image_on_this_computer_is_inspected_by_its_reference() {
        let fake = FakeEngine::start(vec![("200 OK", NGINX.to_string())]);
        let engine = Engine::connect(fake.info()).expect("engine");

        let defaults = engine
            .image_defaults("nginx:1.27")
            .await
            .expect("an answer");

        let defaults = defaults.expect("the image is here");
        assert_eq!(defaults.entrypoint(), ["/docker-entrypoint.sh"]);
        let requests = fake.requests();
        assert!(requests[0].line.starts_with("GET "), "{}", requests[0].line);
        assert!(
            requests[0].line.contains("/images/nginx:1.27/json"),
            "{}",
            requests[0].line
        );
    }

    #[tokio::test]
    async fn a_missing_image_is_none() {
        let fake = FakeEngine::start(vec![(
            "404 Not Found",
            String::from(r#"{"message":"No such image: nginx:9"}"#),
        )]);
        let engine = Engine::connect(fake.info()).expect("engine");

        let defaults = engine.image_defaults("nginx:9").await;

        assert!(matches!(defaults, Ok(None)), "{defaults:?}");
        fake.requests();
    }

    #[tokio::test]
    async fn another_engine_error_carries_its_message() {
        let fake = FakeEngine::start(vec![(
            "500 Internal Server Error",
            String::from(r#"{"message":"store is locked"}"#),
        )]);
        let engine = Engine::connect(fake.info()).expect("engine");

        let defaults = engine.image_defaults("nginx").await;

        let Err(error) = defaults else {
            panic!("expected an error");
        };
        assert_eq!(error.message(), "store is locked");
        fake.requests();
    }
}
