//! Container actions on an engine: start, stop, restart, pause, unpause,
//! remove and prune stopped containers (spec Decision 11).

use bollard::query_parameters::{
    PruneContainersOptions, RemoveContainerOptions,
};

use super::{Engine, EngineError};

impl Engine {
    /// Start a created or stopped container.
    pub async fn start_container(&self, id: &str) -> Result<(), EngineError> {
        self.client.start_container(id, None).await?;

        Ok(())
    }

    /// Stop a container: the engine sends the stop signal, then kills it
    /// after the container's own timeout.
    pub async fn stop_container(&self, id: &str) -> Result<(), EngineError> {
        self.client.stop_container(id, None).await?;

        Ok(())
    }

    /// Stop and start a container again.
    pub async fn restart_container(&self, id: &str) -> Result<(), EngineError> {
        self.client.restart_container(id, None).await?;

        Ok(())
    }

    /// Freeze every process of a running container.
    pub async fn pause_container(&self, id: &str) -> Result<(), EngineError> {
        self.client.pause_container(id).await?;

        Ok(())
    }

    /// Let a paused container run again.
    pub async fn unpause_container(&self, id: &str) -> Result<(), EngineError> {
        self.client.unpause_container(id).await?;

        Ok(())
    }

    /// Remove a container. The engine refuses a running one unless `force`
    /// is set; then it kills the container first. Volumes stay.
    pub async fn remove_container(
        &self,
        id: &str,
        force: bool,
    ) -> Result<(), EngineError> {
        let options = RemoveContainerOptions {
            force,
            ..RemoveContainerOptions::default()
        };

        self.client.remove_container(id, Some(options)).await?;

        Ok(())
    }

    /// Remove every stopped container (created, exited or dead).
    pub async fn prune_containers(&self) -> Result<(), EngineError> {
        self.client
            .prune_containers(None::<PruneContainersOptions>)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::connection::{Engine, EngineError};
    use crate::test_support::FakeEngine;

    const NO_CONTENT: &str = "204 No Content";
    const ID: &str = "4f1c2b";

    fn engine(fake: &FakeEngine) -> Engine {
        Engine::connect(fake.info()).expect("engine")
    }

    fn no_content() -> Vec<(&'static str, String)> {
        vec![(NO_CONTENT, String::new())]
    }

    /// The method and the path of a request line, without the query.
    fn method_and_path(line: &str) -> (String, String) {
        let mut parts = line.split(' ');
        let method = parts.next().unwrap_or_default().to_string();
        let target = parts.next().unwrap_or_default();
        let path = target.split('?').next().unwrap_or_default().to_string();

        (method, path)
    }

    #[tokio::test]
    async fn start_stop_restart_pause_and_unpause_post_to_the_container() {
        let cases = [
            ("start", ID),
            ("stop", ID),
            ("restart", ID),
            ("pause", ID),
            ("unpause", ID),
        ];

        for (action, id) in cases {
            let fake = FakeEngine::start(no_content());
            let engine = engine(&fake);

            let result = match action {
                "start" => engine.start_container(id).await,
                "stop" => engine.stop_container(id).await,
                "restart" => engine.restart_container(id).await,
                "pause" => engine.pause_container(id).await,
                _ => engine.unpause_container(id).await,
            };

            assert!(result.is_ok(), "{action}: {result:?}");
            let requests = fake.requests();
            assert_eq!(requests.len(), 1);
            let (method, path) = method_and_path(&requests[0].line);
            assert_eq!(method, "POST");
            assert!(
                path.ends_with(&format!("/containers/{id}/{action}")),
                "{action}: {path}"
            );
        }
    }

    #[tokio::test]
    async fn remove_deletes_the_container_and_forces_only_when_asked() {
        for force in [false, true] {
            let fake = FakeEngine::start(no_content());
            let engine = engine(&fake);

            engine
                .remove_container(ID, force)
                .await
                .expect("container removed");

            let requests = fake.requests();
            let (method, path) = method_and_path(&requests[0].line);
            assert_eq!(method, "DELETE");
            assert!(path.ends_with(&format!("/containers/{ID}")), "{path}");
            assert_eq!(
                requests[0].line.contains("force=true"),
                force,
                "{}",
                requests[0].line
            );
        }
    }

    #[tokio::test]
    async fn prune_posts_to_container_prune_without_filters() {
        let fake = FakeEngine::start(vec![(
            "200 OK",
            String::from(
                r#"{"ContainersDeleted":["a","b"],"SpaceReclaimed":12}"#,
            ),
        )]);
        let engine = engine(&fake);

        engine.prune_containers().await.expect("pruned");

        let requests = fake.requests();
        let (method, path) = method_and_path(&requests[0].line);
        assert_eq!(method, "POST");
        assert!(path.ends_with("/containers/prune"), "{path}");
        assert!(
            !requests[0].line.contains("filters"),
            "{}",
            requests[0].line
        );
        assert!(requests[0].body.is_empty(), "{}", requests[0].body);
    }

    #[tokio::test]
    async fn a_refused_action_carries_the_engines_own_message() {
        let fake = FakeEngine::start(vec![(
            "409 Conflict",
            String::from(
                r#"{"message":"cannot stop container: it is restarting"}"#,
            ),
        )]);
        let engine = engine(&fake);

        let result = engine.stop_container(ID).await;

        let Err(error @ EngineError::Request(_)) = result else {
            panic!("expected a request error, got {result:?}");
        };
        assert_eq!(error.message(), "cannot stop container: it is restarting");
        fake.requests();
    }
}
