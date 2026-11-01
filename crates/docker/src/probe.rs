//! Tries one candidate: connect, ask `/version`, with a short time limit.

use std::io;
use std::time::Duration;

use bollard::{ClientVersion, Docker};
use tokio::net::{TcpStream, UnixStream};

use crate::address::EngineAddress;
use crate::candidates::SocketCandidate;
use crate::engine::EngineInfo;

/// The API version used to ask `/version`. Docker rejects requests for a
/// newer API than it speaks and for one older than its minimum; 1.40
/// (Docker 19.03) is accepted by every Docker from 19.03 to 29 and by
/// Podman, which accepts any version. Later calls use the version the
/// engine reports.
const DISCOVERY_API_VERSION: ClientVersion = ClientVersion {
    major_version: 1,
    minor_version: 40,
};
/// bollard's own request limit in whole seconds; the probe's shorter limit
/// is applied around the whole exchange.
const CLIENT_TIMEOUT_SECS: u64 = 5;

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
    pub(crate) fn new(
        candidate: SocketCandidate,
        outcome: ProbeOutcome,
    ) -> Self {
        Self { candidate, outcome }
    }
}

/// Why a candidate did not give an engine answer.
#[derive(Debug, thiserror::Error)]
enum ProbeError {
    #[error("cannot connect: {0}")]
    Connect(#[source] io::Error),
    #[error("socket path is not valid UTF-8")]
    PathNotUtf8,
    #[error(transparent)]
    Engine(#[from] bollard::errors::Error),
}

/// Try one candidate once, giving up after `timeout`. No retries.
pub(crate) async fn probe(
    candidate: SocketCandidate,
    timeout: Duration,
) -> Probe {
    let outcome = probe_outcome(candidate.address(), timeout).await;

    Probe::new(candidate, outcome)
}

async fn probe_outcome(
    address: &EngineAddress,
    timeout: Duration,
) -> ProbeOutcome {
    if let EngineAddress::Unix(path) = address
        && std::fs::metadata(path).is_err()
    {
        return ProbeOutcome::Missing;
    }

    let answer = tokio::time::timeout(timeout, ask_version(address)).await;

    match answer {
        Ok(Ok(info)) => ProbeOutcome::Reached(info),
        Ok(Err(ProbeError::Connect(err)))
            if err.kind() == io::ErrorKind::PermissionDenied =>
        {
            ProbeOutcome::PermissionDenied
        },
        Ok(Err(err)) => ProbeOutcome::NotAnswering(err.to_string()),
        Err(_) => {
            ProbeOutcome::NotAnswering(format!("no answer within {timeout:?}"))
        },
    }
}

/// Connect, then ask the engine who it is.
async fn ask_version(
    address: &EngineAddress,
) -> Result<EngineInfo, ProbeError> {
    check_connect(address).await.map_err(ProbeError::Connect)?;

    let client = connect(address)?;
    let answer = client.version().await?;

    Ok(EngineInfo::from_version(&answer, address.clone()))
}

/// Open and close a plain connection first: its error says clearly whether
/// access was denied, which bollard's wrapped errors hide.
async fn check_connect(address: &EngineAddress) -> io::Result<()> {
    match address {
        EngineAddress::Unix(path) => UnixStream::connect(path).await.map(drop),
        EngineAddress::Tcp(host) => {
            TcpStream::connect(host.as_str()).await.map(drop)
        },
    }
}

fn connect(address: &EngineAddress) -> Result<Docker, ProbeError> {
    match address {
        EngineAddress::Unix(path) => {
            let path = path.to_str().ok_or(ProbeError::PathNotUtf8)?;
            let client = Docker::connect_with_unix(
                path,
                CLIENT_TIMEOUT_SECS,
                &DISCOVERY_API_VERSION,
            )?;
            Ok(client)
        },
        EngineAddress::Tcp(host) => {
            let client = Docker::connect_with_http(
                host,
                CLIENT_TIMEOUT_SECS,
                &DISCOVERY_API_VERSION,
            )?;
            Ok(client)
        },
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use std::path::{Path, PathBuf};
    use std::thread::JoinHandle;

    use super::*;
    use crate::candidates::CandidateSource;
    use crate::engine::EngineKind;
    use crate::test_support::TempDir;

    const DOCKER_VERSION: &str =
        include_str!("../tests/fixtures/docker-version.json");
    const PODMAN_VERSION: &str =
        include_str!("../tests/fixtures/podman-version.json");
    const TIMEOUT: Duration = Duration::from_secs(5);

    fn unix_candidate(path: &Path) -> SocketCandidate {
        SocketCandidate::new(
            EngineAddress::Unix(path.to_path_buf()),
            CandidateSource::Docker,
        )
    }

    fn socket_path(dir: &TempDir) -> PathBuf {
        dir.path().join("engine.sock")
    }

    /// Answer every request with `status` and `body`; a connection closed
    /// without a request (the connect check) is skipped.
    fn answer<S: Read + Write>(mut stream: S, status: &str, body: &str) {
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(read) => request.extend_from_slice(&buffer[..read]),
            }
        }

        let length = body.len();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}"
        );
        let _ = stream.write_all(response.as_bytes());
    }

    fn serve_unix(
        listener: UnixListener,
        status: &'static str,
        body: &'static str,
    ) -> JoinHandle<()> {
        std::thread::spawn(move || {
            for stream in listener.incoming().take(2).flatten() {
                answer(stream, status, body);
            }
        })
    }

    #[tokio::test]
    async fn docker_engine_on_a_unix_socket_is_reached() {
        let dir = TempDir::new();
        let path = socket_path(&dir);
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve_unix(listener, "200 OK", DOCKER_VERSION);

        let probe = probe(unix_candidate(&path), TIMEOUT).await;

        let ProbeOutcome::Reached(info) = probe.outcome() else {
            panic!("expected an engine, got {:?}", probe.outcome());
        };
        assert_eq!(info.kind(), EngineKind::Docker);
        assert_eq!(info.version(), Some("29.9.1"));
        assert_eq!(info.address(), &EngineAddress::Unix(path));
        assert_eq!(probe.candidate().source(), CandidateSource::Docker);
        server.join().expect("server thread");
    }

    #[tokio::test]
    async fn podman_engine_on_a_unix_socket_is_reached() {
        let dir = TempDir::new();
        let path = socket_path(&dir);
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve_unix(listener, "200 OK", PODMAN_VERSION);

        let probe = probe(unix_candidate(&path), TIMEOUT).await;

        let ProbeOutcome::Reached(info) = probe.outcome() else {
            panic!("expected an engine, got {:?}", probe.outcome());
        };
        assert_eq!(info.kind(), EngineKind::Podman);
        assert_eq!(info.version(), Some("5.8.8"));
        server.join().expect("server thread");
    }

    #[tokio::test]
    async fn engine_on_tcp_is_reached() {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("bind tcp");
        let port = listener.local_addr().expect("tcp address").port();
        let server = std::thread::spawn(move || {
            for stream in listener.incoming().take(2).flatten() {
                answer(stream, "200 OK", DOCKER_VERSION);
            }
        });
        let candidate = SocketCandidate::new(
            EngineAddress::Tcp(format!("127.0.0.1:{port}")),
            CandidateSource::DockerHost,
        );

        let probe = probe(candidate, TIMEOUT).await;

        let ProbeOutcome::Reached(info) = probe.outcome() else {
            panic!("expected an engine, got {:?}", probe.outcome());
        };
        assert_eq!(info.kind(), EngineKind::Docker);
        server.join().expect("server thread");
    }

    #[tokio::test]
    async fn missing_socket_file_is_missing() {
        let dir = TempDir::new();

        let probe = probe(unix_candidate(&socket_path(&dir)), TIMEOUT).await;

        assert_eq!(probe.outcome(), &ProbeOutcome::Missing);
    }

    #[tokio::test]
    async fn socket_file_without_a_listener_is_not_answering() {
        let dir = TempDir::new();
        let path = socket_path(&dir);
        drop(UnixListener::bind(&path).expect("bind test socket"));

        let probe = probe(unix_candidate(&path), TIMEOUT).await;

        assert!(
            matches!(probe.outcome(), ProbeOutcome::NotAnswering(_)),
            "got {:?}",
            probe.outcome()
        );
    }

    #[tokio::test]
    async fn socket_without_access_is_permission_denied() {
        let dir = TempDir::new();
        let path = socket_path(&dir);
        let _listener = UnixListener::bind(&path).expect("bind test socket");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
            .expect("lock test socket");
        if std::os::unix::net::UnixStream::connect(&path).is_ok() {
            // Running as root: file modes do not stop root.
            return;
        }

        let probe = probe(unix_candidate(&path), TIMEOUT).await;

        assert_eq!(probe.outcome(), &ProbeOutcome::PermissionDenied);
    }

    #[tokio::test]
    async fn silent_listener_times_out() {
        let dir = TempDir::new();
        let path = socket_path(&dir);
        let _listener = UnixListener::bind(&path).expect("bind test socket");

        let probe =
            probe(unix_candidate(&path), Duration::from_millis(200)).await;

        assert_eq!(
            probe.outcome(),
            &ProbeOutcome::NotAnswering(String::from("no answer within 200ms"))
        );
    }

    #[tokio::test]
    async fn engine_error_answer_is_not_answering_with_its_message() {
        let dir = TempDir::new();
        let path = socket_path(&dir);
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = serve_unix(
            listener,
            "500 Internal Server Error",
            r#"{"message":"engine is starting"}"#,
        );

        let probe = probe(unix_candidate(&path), TIMEOUT).await;

        let ProbeOutcome::NotAnswering(reason) = probe.outcome() else {
            panic!("expected no answer, got {:?}", probe.outcome());
        };
        assert!(reason.contains("engine is starting"), "reason: {reason}");
        server.join().expect("server thread");
    }
}
