//! Test-only helpers shared by the file system, socket and mapping tests.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use bollard::models::ContainerSummary;

use crate::container::Container;

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

/// `/containers/json?all=1` captured from Docker 29.9.1.
pub(crate) const DOCKER_CONTAINERS: &str =
    include_str!("../tests/fixtures/docker-containers.json");
/// `/containers/json?all=1` captured from Podman 5.8.8.
pub(crate) const PODMAN_CONTAINERS: &str =
    include_str!("../tests/fixtures/podman-containers.json");

/// A fresh directory under the system temp dir, removed on drop.
///
/// The path stays short because unix socket paths are limited to about 100
/// bytes.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn new() -> Self {
        let number = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let name = format!("otty-dk-{}-{number}", std::process::id());
        let path = std::env::temp_dir().join(name);

        std::fs::create_dir_all(&path).expect("create test dir");

        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Map a captured container list fixture.
pub(crate) fn containers(json: &str) -> Vec<Container> {
    let summaries: Vec<ContainerSummary> =
        serde_json::from_str(json).expect("fixture is a container list");

    summaries.iter().map(Container::from_summary).collect()
}

/// Read one HTTP request and return its request line, for example
/// `GET /v1.44/volumes HTTP/1.1`. `None` when the peer closed the connection
/// without a request (the connect check).
pub(crate) fn read_request_line<S: Read>(stream: &mut S) -> Option<String> {
    let mut request = Vec::new();
    let mut buffer = [0u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return None,
            Ok(read) => request.extend_from_slice(&buffer[..read]),
        }
    }

    let text = String::from_utf8_lossy(&request);
    text.lines().next().map(String::from)
}

/// Answer an HTTP request with a JSON body and close the connection.
pub(crate) fn write_response<S: Write>(
    stream: &mut S,
    status: &str,
    body: &str,
) {
    let length = body.len();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}"
    );
    let _ = stream.write_all(response.as_bytes());
}
