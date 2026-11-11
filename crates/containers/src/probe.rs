//! Tries one candidate: connect, ask `/version`, with a short time limit.

use std::io;
use std::time::Duration;

use tokio::net::{TcpStream, UnixStream};

use crate::address::EngineAddress;
use crate::connection::{BASELINE_API_VERSION, EngineError, open_client};
use crate::engine::EngineInfo;
use crate::probe_outcome::{Probe, ProbeOutcome};
use crate::socket_candidate::SocketCandidate;

/// bollard's own request limit in whole seconds; the probe's shorter limit
/// is applied around the whole exchange.
const CLIENT_TIMEOUT_SECS: u64 = 5;

/// Why a candidate did not give an engine answer.
#[derive(Debug, thiserror::Error)]
enum ProbeError {
    #[error("cannot connect: {0}")]
    Connect(#[source] io::Error),
    #[error(transparent)]
    Client(#[from] EngineError),
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

    // Docker rejects a URL version outside its range; the baseline is
    // inside every supported engine's range.
    let client =
        open_client(address, CLIENT_TIMEOUT_SECS, &BASELINE_API_VERSION)?;
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

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use std::path::{Path, PathBuf};
    use std::thread::JoinHandle;

    use super::*;
    use crate::engine::EngineKind;
    use crate::socket_candidate::CandidateSource;
    use crate::test_support::{TempDir, read_request_line, write_response};

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
        if read_request_line(&mut stream).is_some() {
            write_response(&mut stream, status, body);
        }
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
