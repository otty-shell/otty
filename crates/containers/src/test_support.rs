//! Test-only helpers shared by the file system, socket and mapping tests.

use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;

use bollard::models::{ContainerSummary, SystemVersion};

use crate::address::EngineAddress;
use crate::container::Container;
use crate::engine::EngineInfo;

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

/// One request the fake engine got: its request line, for example
/// `POST /containers/abc/stop HTTP/1.1`, and its body.
pub(crate) struct Request {
    pub(crate) line: String,
    pub(crate) body: String,
}

/// A fake engine on a unix socket in a fresh temp dir. It answers each
/// request with the next of its answers, in order, and records what it got.
pub(crate) struct FakeEngine {
    _dir: TempDir,
    info: EngineInfo,
    server: JoinHandle<Vec<Request>>,
}

impl FakeEngine {
    /// The engine info that points at the fake socket.
    pub(crate) fn info(&self) -> &EngineInfo {
        &self.info
    }

    /// Start a fake Docker engine (API 1.44) that sends `answers`, each an
    /// HTTP status such as `204 No Content` and a body.
    pub(crate) fn start(answers: Vec<(&'static str, String)>) -> Self {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let answer: SystemVersion =
            serde_json::from_str(r#"{"Version":"1.0","ApiVersion":"1.44"}"#)
                .expect("version answer");
        let info = EngineInfo::from_version(&answer, EngineAddress::Unix(path));

        let server = std::thread::spawn(move || {
            let mut requests = Vec::new();
            let mut incoming = listener.incoming().flatten();
            for (status, body) in answers {
                let Some(mut stream) = incoming.next() else {
                    break;
                };
                let Some(request) = read_request(&mut stream) else {
                    continue;
                };
                write_response(&mut stream, status, &body);
                requests.push(request);
            }
            requests
        });

        Self {
            _dir: dir,
            info,
            server,
        }
    }

    /// Wait until every answer was sent and return the requests in order.
    pub(crate) fn requests(self) -> Vec<Request> {
        self.server.join().expect("fake engine thread")
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

/// Read one HTTP request with its body (by `Content-Length`). `None` when
/// the peer closed the connection without a request.
pub(crate) fn read_request<S: Read>(stream: &mut S) -> Option<Request> {
    let mut request = Vec::new();
    let mut buffer = [0u8; 1024];
    let head_end = loop {
        if let Some(end) =
            request.windows(4).position(|window| window == b"\r\n\r\n")
        {
            break end + 4;
        }
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return None,
            Ok(read) => request.extend_from_slice(&buffer[..read]),
        }
    };

    let head = String::from_utf8_lossy(&request[..head_end]).to_string();
    let length = head
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    while request.len() < head_end + length {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => request.extend_from_slice(&buffer[..read]),
        }
    }

    let body = String::from_utf8_lossy(&request[head_end..]).to_string();
    Some(Request {
        line: head.lines().next().unwrap_or_default().to_string(),
        body,
    })
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

/// Start a streamed answer, as the engine does for `/events`: no length, the
/// body runs until the connection closes. Send the lines with
/// [`write_stream_line`].
pub(crate) fn write_stream_head<S: Write>(stream: &mut S) {
    let head = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n";
    let _ = stream.write_all(head.as_bytes());
}

/// Send one JSON line of a streamed answer.
pub(crate) fn write_stream_line<S: Write>(stream: &mut S, line: &str) {
    let _ = stream.write_all(format!("{line}\n").as_bytes());
    let _ = stream.flush();
}
