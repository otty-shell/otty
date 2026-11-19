//! Reads one Codex session file, plain or compressed, into what its first
//! line tells about the session, and opens a session file for the sources
//! that read more of it.
//!
//! The first line of a session file is a `session_meta` record with the
//! session's id and folder, so nothing after it is read. A
//! compressed file is unpacked as a stream and only up to that line.

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use ruzstd::decoding::StreamingDecoder;
use serde::Deserialize;
use serde_json::Value;

/// The longest first line read, in bytes. A longer one is taken as broken.
const MAX_FIRST_LINE: u64 = 1024 * 1024;

/// The first line of a session file: only the fields a session list needs.
#[derive(Debug, Deserialize)]
struct FirstLine {
    #[serde(rename = "type")]
    kind: Option<String>,
    payload: Option<MetaPayload>,
}

/// The `payload` of a `session_meta` record.
#[derive(Debug, Deserialize)]
struct MetaPayload {
    id: Option<String>,
    cwd: Option<String>,
    source: Option<Value>,
    cli_version: Option<String>,
}

impl MetaPayload {
    /// Whether a helper agent started the session.
    ///
    /// A session the user started names its source as text, such as `cli`
    /// or `vscode`; the session of a helper agent has an object there.
    fn is_helper_session(&self) -> bool {
        matches!(self.source, Some(Value::Object(_)))
    }
}

/// What the first line of a session file tells about its session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionMeta {
    id: String,
    folder: PathBuf,
}

impl SessionMeta {
    /// The id `codex resume` takes.
    pub(super) fn id(&self) -> &str {
        &self.id
    }

    /// The project folder the session ran in.
    pub(super) fn folder(&self) -> &Path {
        &self.folder
    }
}

/// What the session file at `path` tells about its session.
///
/// `None` when the file can not be read, when its first line is not the
/// `session_meta` record with an id and a folder, or when a helper agent
/// started the session.
pub(super) fn read_session_meta(path: &Path) -> Option<SessionMeta> {
    let line = read_first_line(path).ok()?;

    let first_line = serde_json::from_slice::<FirstLine>(&line).ok()?;
    let payload = first_line.payload?;
    if first_line.kind.as_deref() != Some("session_meta")
        || payload.is_helper_session()
    {
        return None;
    }

    let id = payload.id.filter(|id| !id.is_empty())?;
    let folder = payload.cwd.filter(|cwd| !cwd.is_empty())?;

    Some(SessionMeta {
        id,
        folder: PathBuf::from(folder),
    })
}

/// The version of `codex` that started the session in the file at `path`,
/// such as "0.169.0", as its `session_meta` record names it. `None` when the
/// file can not be read or its first line names no version.
pub(super) fn read_cli_version(path: &Path) -> Option<String> {
    let line = read_first_line(path).ok()?;

    let first_line = serde_json::from_slice::<FirstLine>(&line).ok()?;
    if first_line.kind.as_deref() != Some("session_meta") {
        return None;
    }

    first_line
        .payload?
        .cli_version
        .filter(|version| !version.is_empty())
}

/// The content of the session file at `path`, unpacked as it is read when
/// the file is compressed.
pub(super) fn open_session_file(path: &Path) -> io::Result<Box<dyn Read>> {
    let file = File::open(path)?;

    if is_compressed(path) {
        let decoder = StreamingDecoder::new(BufReader::new(file))
            .map_err(io::Error::other)?;
        return Ok(Box::new(decoder));
    }

    Ok(Box::new(file))
}

/// Whether the file at `path` is compressed with zstd: its name ends with
/// `.zst`.
fn is_compressed(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "zst")
}

/// The first line of the file at `path`, unpacked when it is compressed.
fn read_first_line(path: &Path) -> io::Result<Vec<u8>> {
    first_line(open_session_file(path)?)
}

/// The bytes of `reader` up to the end of its first line, at most
/// [`MAX_FIRST_LINE`] of them.
fn first_line(reader: impl Read) -> io::Result<Vec<u8>> {
    let mut line = Vec::new();

    BufReader::new(reader.take(MAX_FIRST_LINE)).read_until(b'\n', &mut line)?;

    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TempDir, write_compressed, write_file};

    const ID: &str = "019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b";
    const META: &str = r#"{"timestamp":"2026-11-16T10:00:00.000Z","type":"session_meta","payload":{"id":"019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b","timestamp":"2026-11-16T10:00:00.000Z","cwd":"/home/user/otty","originator":"codex_cli_rs","cli_version":"0.163.0","source":"cli","git":{"commit_hash":"0123456789abcdef","branch":"main","repository_url":"https://example.com/otty.git"}}}"#;
    const MESSAGE: &str = r#"{"timestamp":"2026-11-16T10:00:05.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Fix the tabs"}]}}"#;

    fn expected() -> SessionMeta {
        SessionMeta {
            id: String::from(ID),
            folder: PathBuf::from("/home/user/otty"),
        }
    }

    fn meta_of_plain(lines: &[&str]) -> Option<SessionMeta> {
        let temp = TempDir::new();
        let path = temp.path().join(format!("rollout-{ID}.jsonl"));
        write_file(&path, &lines.join("\n"));

        read_session_meta(&path)
    }

    fn meta_with_payload(payload: &str) -> Option<SessionMeta> {
        let line = format!(r#"{{"type":"session_meta","payload":{payload}}}"#);

        meta_of_plain(&[&line])
    }

    #[test]
    fn a_plain_file_tells_its_id_and_folder() {
        let meta = meta_of_plain(&[META, MESSAGE, "{\"broken"]);

        assert_eq!(meta, Some(expected()));
    }

    #[test]
    fn a_compressed_file_tells_its_id_and_folder() {
        let temp = TempDir::new();
        let path = temp.path().join(format!("rollout-{ID}.jsonl.zst"));
        let mut content = format!("{META}\n");
        for _ in 0..2000 {
            content.push_str(MESSAGE);
            content.push('\n');
        }
        write_compressed(&path, &content);

        assert_eq!(read_session_meta(&path), Some(expected()));
    }

    #[test]
    fn a_helper_agents_session_tells_nothing() {
        let meta = meta_with_payload(
            r#"{"id":"a1","cwd":"/home/user/otty","source":{"subagent":{"thread_spawn":{"parent_thread_id":"p1","depth":1}}}}"#,
        );

        assert_eq!(meta, None);
    }

    #[test]
    fn a_session_with_no_source_or_git_is_the_users() {
        let meta = meta_with_payload(r#"{"id":"a1","cwd":"/home/user/otty"}"#);
        let with_null_git = meta_with_payload(
            r#"{"id":"a1","cwd":"/home/user/otty","source":"exec","git":null}"#,
        );

        let expected = SessionMeta {
            id: String::from("a1"),
            folder: PathBuf::from("/home/user/otty"),
        };
        assert_eq!(meta, Some(expected.clone()));
        assert_eq!(with_null_git, Some(expected));
    }

    #[test]
    fn a_first_line_that_is_not_a_readable_session_meta_tells_nothing() {
        let cases = [
            String::new(),
            String::from("{\"type\":\"session_meta\",\"payload\":"),
            String::from("not json"),
            String::from(MESSAGE),
            META.replace("session_meta", "turn_context"),
            META.replace(ID, ""),
            META.replace("\"cwd\":\"/home/user/otty\",", ""),
            META.replace("\"/home/user/otty\"", "42"),
        ];

        for line in cases {
            assert_eq!(meta_of_plain(&[&line, META]), None, "{line}");
        }
    }

    #[test]
    fn a_broken_or_missing_compressed_file_tells_nothing() {
        let temp = TempDir::new();
        let broken = temp.path().join("rollout-broken.jsonl.zst");
        write_file(&broken, META);
        let missing = temp.path().join("rollout-missing.jsonl.zst");

        assert_eq!(read_session_meta(&broken), None);
        assert_eq!(read_session_meta(&missing), None);
    }

    #[test]
    fn a_first_line_longer_than_the_limit_is_not_read_whole() {
        let padding = "x".repeat(MAX_FIRST_LINE as usize);
        let line = META.replace("\"cli\"", &format!("\"{padding}\""));

        assert_eq!(meta_of_plain(&[&line]), None);
    }
}
