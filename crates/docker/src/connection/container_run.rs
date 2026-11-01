//! Running a new container: create it from a [`RunRequest`], then start it
//! (spec Decision 18).

use super::{Engine, EngineError};
use crate::run::RunRequest;

impl Engine {
    /// Create a container from `request` and start it in the background.
    /// Returns the new container's id. When the start fails, the created
    /// container stays (as with `docker run`) and the error says why.
    pub async fn run_container(
        &self,
        request: &RunRequest,
    ) -> Result<String, EngineError> {
        let created = self
            .client
            .create_container(request.create_options(), request.create_body())
            .await?;
        self.start_container(&created.id).await?;

        Ok(created.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::{PortProtocol, PublishedPort};
    use crate::test_support::FakeEngine;

    const CREATED: &str = r#"{"Id":"4f1c2b9a8e7d","Warnings":[]}"#;

    #[tokio::test]
    async fn a_run_creates_the_container_and_then_starts_it() {
        let fake = FakeEngine::start(vec![
            ("201 Created", CREATED.to_string()),
            ("204 No Content", String::new()),
        ]);
        let engine = Engine::connect(fake.info()).expect("engine");
        let request = RunRequest::new("nginx:1.27")
            .with_name("otty-fx-web")
            .with_port(PublishedPort::new(Some(8081), 80, PortProtocol::Tcp));

        let id = engine.run_container(&request).await.expect("started");

        assert_eq!(id, "4f1c2b9a8e7d");
        let requests = fake.requests();
        assert_eq!(requests.len(), 2);
        assert!(
            requests[0].line.starts_with("POST ")
                && requests[0].line.contains("/containers/create")
                && requests[0].line.contains("name=otty-fx-web"),
            "{}",
            requests[0].line
        );
        let body: serde_json::Value =
            serde_json::from_str(&requests[0].body).expect("json body");
        assert_eq!(body["Image"], "nginx:1.27");
        assert_eq!(
            body["HostConfig"]["PortBindings"]["80/tcp"][0]["HostPort"],
            "8081"
        );
        assert!(
            requests[1].line.starts_with("POST ")
                && requests[1].line.contains("/containers/4f1c2b9a8e7d/start"),
            "{}",
            requests[1].line
        );
    }

    #[tokio::test]
    async fn a_refused_create_starts_nothing_and_carries_the_message() {
        let fake = FakeEngine::start(vec![(
            "409 Conflict",
            String::from(
                r#"{"message":"Conflict. The container name \"/web\" is already in use"}"#,
            ),
        )]);
        let engine = Engine::connect(fake.info()).expect("engine");

        let result = engine
            .run_container(&RunRequest::new("nginx").with_name("web"))
            .await;

        let Err(error) = result else {
            panic!("expected an error");
        };
        assert!(
            error.message().contains("already in use"),
            "{}",
            error.message()
        );
        assert_eq!(fake.requests().len(), 1);
    }

    #[tokio::test]
    async fn a_refused_start_carries_the_engines_message() {
        let fake = FakeEngine::start(vec![
            ("201 Created", CREATED.to_string()),
            (
                "500 Internal Server Error",
                String::from(
                    r#"{"message":"Bind for 0.0.0.0:8081 failed: port is already allocated"}"#,
                ),
            ),
        ]);
        let engine = Engine::connect(fake.info()).expect("engine");

        let result = engine.run_container(&RunRequest::new("nginx")).await;

        let Err(error) = result else {
            panic!("expected an error");
        };
        assert_eq!(
            error.message(),
            "Bind for 0.0.0.0:8081 failed: port is already allocated"
        );
        assert_eq!(fake.requests().len(), 2);
    }
}
