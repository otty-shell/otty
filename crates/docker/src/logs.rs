//! Following a container's logs for the inspector's Logs tab (spec
//! Decision 12).
//!
//! bollard splits the engine's answer into stdout and stderr frames (or
//! plain lines for a container with a terminal); they are kept in the order
//! they came and cut into lines here. bollard's 30 s request limit covers
//! only the wait for the answer to start, not the stream after it, so one
//! client serves both the lists and a long follow.

use bollard::container::LogOutput;
use bollard::query_parameters::LogsOptionsBuilder;
use futures_util::stream::{self, BoxStream, StreamExt, TryStreamExt};

use crate::connection::{Engine, EngineError};

impl Engine {
    /// The last `tail` lines of the container with `id` (stdout and stderr),
    /// then every new line until the container stops or the stream is
    /// dropped. An engine error ends the stream with that error.
    pub fn follow_logs(
        &self,
        id: &str,
        tail: usize,
    ) -> BoxStream<'static, Result<String, EngineError>> {
        let options = LogsOptionsBuilder::new()
            .follow(true)
            .stdout(true)
            .stderr(true)
            .tail(&tail.to_string())
            .build();

        self.client()
            .logs(id, Some(options))
            .map_err(EngineError::from)
            .map_ok(|output| stream::iter(output_lines(&output)).map(Ok))
            .try_flatten()
            .boxed()
    }
}

/// The lines of one piece of output, without their line breaks. Input the
/// container read (stdin) is not output and gives no lines.
fn output_lines(output: &LogOutput) -> Vec<String> {
    let message = match output {
        LogOutput::StdOut { message }
        | LogOutput::StdErr { message }
        | LogOutput::Console { message } => message,
        _ => return Vec::new(),
    };

    let text = String::from_utf8_lossy(message);
    if text.is_empty() {
        return Vec::new();
    }

    let text = text.strip_suffix('\n').unwrap_or(&text);
    text.split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use std::thread::JoinHandle;

    use bollard::models::SystemVersion;
    use futures_util::TryStreamExt;

    use super::*;
    use crate::address::EngineAddress;
    use crate::engine::EngineInfo;
    use crate::test_support::{TempDir, read_request_line, write_response};

    /// Stream kinds of the engine's multiplexed log format.
    const STDOUT: u8 = 1;
    const STDERR: u8 = 2;

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

    /// One frame of the multiplexed log format: kind, three zero bytes, the
    /// big-endian length, then the text.
    fn frame(kind: u8, text: &str) -> Vec<u8> {
        let length = u32::try_from(text.len()).expect("short text");
        let mut frame = vec![kind, 0, 0, 0];
        frame.extend_from_slice(&length.to_be_bytes());
        frame.extend_from_slice(text.as_bytes());
        frame
    }

    /// Answer one request with a log `body` and close the connection, as an
    /// engine does when the container stops; return the request line.
    fn serve_logs(
        listener: UnixListener,
        content_type: &'static str,
        body: Vec<u8>,
    ) -> JoinHandle<String> {
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let Some(line) = read_request_line(&mut stream) else {
                    continue;
                };
                let length = body.len();
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
                return line;
            }
            String::new()
        })
    }

    #[tokio::test]
    async fn stdout_and_stderr_lines_come_in_the_engine_order() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let mut body = frame(STDOUT, "starting\nlistening on :80\n");
        body.extend(frame(STDERR, "warning: no config\n"));
        body.extend(frame(STDOUT, "ready\r\n"));
        let server = serve_logs(
            listener,
            "application/vnd.docker.multiplexed-stream",
            body,
        );

        let lines: Vec<String> = engine_at(&path)
            .follow_logs("1f3a9c2e", 500)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(
            lines,
            [
                "starting",
                "listening on :80",
                "warning: no config",
                "ready"
            ]
        );
        let line = server.join().expect("server thread");
        assert!(line.contains("/containers/1f3a9c2e/logs?"), "{line}");
        for query in ["follow=true", "stdout=true", "stderr=true", "tail=500"] {
            assert!(line.contains(query), "{query} missing in {line}");
        }
    }

    #[tokio::test]
    async fn a_terminal_container_sends_plain_lines() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let body = b"$ ls\r\nbin  etc\r\nlast line without a break".to_vec();
        let server =
            serve_logs(listener, "application/vnd.docker.raw-stream", body);

        let lines: Vec<String> = engine_at(&path)
            .follow_logs("b93d6f0a", 10)
            .try_collect()
            .await
            .expect("log lines");

        assert_eq!(lines, ["$ ls", "bin  etc", "last line without a break"]);
        let line = server.join().expect("server thread");
        assert!(line.contains("tail=10"), "{line}");
    }

    #[tokio::test]
    async fn an_engine_error_ends_the_stream_with_its_message() {
        let dir = TempDir::new();
        let path = dir.path().join("engine.sock");
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server = std::thread::spawn(move || {
            if let Some(mut stream) = listener.incoming().flatten().next()
                && read_request_line(&mut stream).is_some()
            {
                write_response(
                    &mut stream,
                    "404 Not Found",
                    r#"{"message":"No such container: gone"}"#,
                );
            }
        });

        let result: Result<Vec<String>, EngineError> = engine_at(&path)
            .follow_logs("gone", 500)
            .try_collect()
            .await;

        server.join().expect("server thread");
        let Err(error) = result else {
            panic!("expected an error, got {result:?}");
        };
        assert!(error.to_string().contains("No such container"), "{error}");
    }
}
