//! The inspect calls: everything the engine knows about one container,
//! image, volume or network, as pretty-printed JSON for the inspector's
//! Inspect tab (spec Decision 12).
//!
//! The answer is read into bollard's model and written out again, so fields
//! bollard does not know are left out. Each call makes one request.

use bollard::query_parameters::{
    InspectContainerOptions, InspectNetworkOptions,
};

use crate::connection::{Engine, EngineError};

impl Engine {
    /// `docker container inspect` of the container with `id`, pretty
    /// printed.
    pub async fn inspect_container(
        &self,
        id: &str,
    ) -> Result<String, EngineError> {
        let answer = self
            .client()
            .inspect_container(id, None::<InspectContainerOptions>)
            .await?;

        pretty(serde_json::to_string_pretty(&answer))
    }

    /// `docker image inspect` of the image with `id`, pretty printed.
    pub async fn inspect_image(&self, id: &str) -> Result<String, EngineError> {
        let answer = self.client().inspect_image(id).await?;

        pretty(serde_json::to_string_pretty(&answer))
    }

    /// `docker volume inspect` of the volume called `name`, pretty printed.
    pub async fn inspect_volume(
        &self,
        name: &str,
    ) -> Result<String, EngineError> {
        let answer = self.client().inspect_volume(name).await?;

        pretty(serde_json::to_string_pretty(&answer))
    }

    /// `docker network inspect` of the network with `id`, pretty printed.
    pub async fn inspect_network(
        &self,
        id: &str,
    ) -> Result<String, EngineError> {
        let answer = self
            .client()
            .inspect_network(id, None::<InspectNetworkOptions>)
            .await?;

        pretty(serde_json::to_string_pretty(&answer))
    }
}

/// Writing a model bollard just read cannot really fail; if it does, it is
/// reported like an answer OTTY cannot read.
fn pretty(
    written: Result<String, serde_json::Error>,
) -> Result<String, EngineError> {
    written.map_err(|err| EngineError::Request(err.into()))
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use std::thread::JoinHandle;

    use bollard::models::SystemVersion;

    use super::*;
    use crate::address::EngineAddress;
    use crate::engine::EngineInfo;
    use crate::test_support::{TempDir, read_request_line, write_response};

    fn engine_at(path: &Path) -> Engine {
        let answer: SystemVersion =
            serde_json::from_str(r#"{"Version":"1.0","ApiVersion":"1.44"}"#)
                .expect("version answer");
        let info = EngineInfo::from_version(
            &answer,
            EngineAddress::Unix(path.to_path_buf()),
        );

        Engine::connect(&info).expect("engine")
    }

    /// Answer one request with `status` and `body`; return its request line.
    fn serve_one(
        listener: UnixListener,
        status: &'static str,
        body: &'static str,
    ) -> JoinHandle<String> {
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                if let Some(line) = read_request_line(&mut stream) {
                    write_response(&mut stream, status, body);
                    return line;
                }
            }
            String::new()
        })
    }

    /// The path of a request line, without its query.
    fn path_of(line: &str) -> &str {
        let target = line.split(' ').nth(1).unwrap_or_default();

        target.split('?').next().unwrap_or_default()
    }

    #[tokio::test]
    async fn a_container_is_inspected_as_pretty_json() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve_one(
            listener,
            "200 OK",
            r#"{"Id":"1f3a9c2e","Name":"/web","State":{"Status":"running"}}"#,
        );

        let json = engine_at(&path)
            .inspect_container("1f3a9c2e")
            .await
            .expect("inspect");

        let line = server.join().expect("server thread");
        assert!(line.starts_with("GET "), "{line}");
        assert_eq!(path_of(&line), "/containers/1f3a9c2e/json");
        assert!(json.contains("\n  \"Id\": \"1f3a9c2e\""), "{json}");
        assert!(json.contains("\n    \"Status\": \"running\""), "{json}");
    }

    #[tokio::test]
    async fn an_image_is_inspected_as_pretty_json() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve_one(
            listener,
            "200 OK",
            r#"{"Id":"sha256:3b25","RepoTags":["busybox:1.37"]}"#,
        );

        let json = engine_at(&path)
            .inspect_image("sha256:3b25")
            .await
            .expect("inspect");

        let line = server.join().expect("server thread");
        assert_eq!(path_of(&line), "/images/sha256:3b25/json");
        assert!(
            json.contains("\n  \"RepoTags\": [\n    \"busybox:1.37\"\n  ]")
        );
    }

    #[tokio::test]
    async fn a_volume_is_inspected_as_pretty_json() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve_one(
            listener,
            "200 OK",
            r#"{"Name":"otty-fx-data","Driver":"local","Mountpoint":"/var/lib/docker/volumes/otty-fx-data/_data","Labels":{},"Scope":"local","Options":{}}"#,
        );

        let json = engine_at(&path)
            .inspect_volume("otty-fx-data")
            .await
            .expect("inspect");

        let line = server.join().expect("server thread");
        assert_eq!(path_of(&line), "/volumes/otty-fx-data");
        assert!(json.contains("\n  \"Driver\": \"local\""), "{json}");
    }

    #[tokio::test]
    async fn a_network_is_inspected_as_pretty_json() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve_one(
            listener,
            "200 OK",
            r#"{"Name":"otty-fx-net","Id":"9c1e","Driver":"bridge","Scope":"local"}"#,
        );

        let json = engine_at(&path)
            .inspect_network("9c1e")
            .await
            .expect("inspect");

        let line = server.join().expect("server thread");
        assert_eq!(path_of(&line), "/networks/9c1e");
        assert!(json.contains("\n  \"Name\": \"otty-fx-net\""), "{json}");
    }

    #[tokio::test]
    async fn a_missing_item_is_a_request_error_with_the_engine_message() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve_one(
            listener,
            "404 Not Found",
            r#"{"message":"No such container: gone"}"#,
        );

        let result = engine_at(&path).inspect_container("gone").await;

        server.join().expect("server thread");
        let Err(error @ EngineError::Request(_)) = result else {
            panic!("expected a request error, got {result:?}");
        };
        assert!(error.to_string().contains("No such container"), "{error}");
    }
}
