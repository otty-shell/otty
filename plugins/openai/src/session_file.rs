//! The first line of a Codex session file, plain or compressed: a
//! `session_meta` record with the session's id, its folder and the
//! version of `codex` that started it. Nothing after it is read; the host
//! decodes a compressed file only up to that line.

use otty_plugin_sdk::files;
use serde::Deserialize;
use serde_json::Value;

/// How many bytes one read of the first line asks for. The host reads on
/// until the line ends.
const FIRST_LINE_STEP: u32 = 16 * 1024;

/// The longest first line read, in bytes. A longer one is taken as broken.
const MAX_FIRST_LINE: usize = 1024 * 1024;

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
pub(crate) struct SessionMeta {
    id: String,
    folder: String,
}

impl SessionMeta {
    /// The id `codex resume` takes.
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    /// The project folder the session ran in.
    pub(crate) fn folder(&self) -> &str {
        &self.folder
    }
}

/// What the session file at `path` tells about its session.
///
/// `None` when the file can not be read, when its first line is not the
/// `session_meta` record with an id and a folder, or when a helper agent
/// started the session.
pub(crate) fn read_session_meta(path: &str) -> Option<SessionMeta> {
    let batch = files::read_lines(path, 0, FIRST_LINE_STEP, &[]).ok()?;

    meta_of_line(first_line(&batch.lines)?)
}

/// The version of `codex` that started the session in the file at `path`,
/// such as "0.169.0", as its `session_meta` record names it. `None` when
/// the file can not be read or its first line names no version.
pub(crate) fn read_cli_version(path: &str) -> Option<String> {
    let batch = files::read_lines(path, 0, FIRST_LINE_STEP, &[]).ok()?;

    cli_version_of_line(first_line(&batch.lines)?)
}

/// The first line of `bytes`, without its newline; `None` when it is
/// longer than [`MAX_FIRST_LINE`].
fn first_line(bytes: &[u8]) -> Option<&[u8]> {
    let line = bytes.split(|byte| *byte == b'\n').next().unwrap_or(bytes);

    (line.len() <= MAX_FIRST_LINE).then_some(line)
}

/// The session the first line `line` tells about.
fn meta_of_line(line: &[u8]) -> Option<SessionMeta> {
    let first_line = serde_json::from_slice::<FirstLine>(line).ok()?;
    let payload = first_line.payload?;
    if first_line.kind.as_deref() != Some("session_meta")
        || payload.is_helper_session()
    {
        return None;
    }

    let id = payload.id.filter(|id| !id.is_empty())?;
    let folder = payload.cwd.filter(|cwd| !cwd.is_empty())?;

    Some(SessionMeta { id, folder })
}

/// The version of `codex` the first line `line` names.
fn cli_version_of_line(line: &[u8]) -> Option<String> {
    let first_line = serde_json::from_slice::<FirstLine>(line).ok()?;
    if first_line.kind.as_deref() != Some("session_meta") {
        return None;
    }

    first_line
        .payload?
        .cli_version
        .filter(|version| !version.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{BLOG_SESSION, SHOP_SESSION, as_part, host, path};

    const ID: &str = "019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b";
    const META: &str = r#"{"timestamp":"2026-11-16T10:00:00.000Z","type":"session_meta","payload":{"id":"019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b","timestamp":"2026-11-16T10:00:00.000Z","cwd":"/home/user/otty","originator":"codex_cli_rs","cli_version":"0.163.0","source":"cli","git":{"commit_hash":"0123456789abcdef","branch":"main","repository_url":"https://example.com/otty.git"}}}"#;
    const MESSAGE: &str = r#"{"timestamp":"2026-11-16T10:00:05.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Fix the tabs"}]}}"#;

    fn expected() -> SessionMeta {
        SessionMeta {
            id: String::from(ID),
            folder: String::from("/home/user/otty"),
        }
    }

    /// What a file of `lines` tells, as its first line is read.
    fn meta_of_plain(lines: &[&str]) -> Option<SessionMeta> {
        let content = lines.join("\n");

        meta_of_line(first_line(content.as_bytes())?)
    }

    fn meta_with_payload(payload: &str) -> Option<SessionMeta> {
        let line = format!(r#"{{"type":"session_meta","payload":{payload}}}"#);

        meta_of_plain(&[&line])
    }

    fn meta_of_fixture(relative: &str) -> Option<SessionMeta> {
        let host = host();
        let file = path(&host, relative);

        as_part(&host, || read_session_meta(&file))
    }

    #[test]
    fn a_plain_file_tells_its_id_and_folder() {
        let meta = meta_of_plain(&[META, MESSAGE, "{\"broken"]);

        assert_eq!(meta, Some(expected()));
        assert_eq!(
            meta_of_fixture(&format!(
                ".codex/sessions/2026/11/16/rollout-2026-11-16T10-00-00-{SHOP_SESSION}.jsonl"
            )),
            Some(SessionMeta {
                id: String::from(SHOP_SESSION),
                folder: String::from("/home/dev/shop"),
            })
        );
    }

    #[test]
    fn a_compressed_file_tells_its_id_and_folder() {
        let meta = meta_of_fixture(&format!(
            ".codex/sessions/2026/11/15/rollout-2026-11-15T09-00-00-{BLOG_SESSION}.jsonl.zst"
        ));

        assert_eq!(
            meta,
            Some(SessionMeta {
                id: String::from(BLOG_SESSION),
                folder: String::from("/home/dev/blog"),
            })
        );
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
            folder: String::from("/home/user/otty"),
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

    /// The host decodes a file that starts as a zstd frame; one that
    /// breaks after that start can not be read.
    #[test]
    fn a_broken_or_missing_compressed_file_tells_nothing() {
        let broken = meta_of_fixture(
            ".codex/sessions/2026/11/13/rollout-2026-11-13T08-00-00-019a7ffe-0000-7000-8000-000000000000.jsonl.zst",
        );
        let missing = meta_of_fixture(
            ".codex/sessions/2026/11/13/rollout-missing.jsonl.zst",
        );

        assert_eq!(broken, None);
        assert_eq!(missing, None);
    }

    #[test]
    fn a_first_line_longer_than_the_limit_is_not_read_whole() {
        let padding = "x".repeat(MAX_FIRST_LINE);
        let line = META.replace("\"cli\"", &format!("\"{padding}\""));

        assert_eq!(meta_of_plain(&[&line]), None);
    }

    #[test]
    fn the_version_is_the_cli_version_of_the_first_line() {
        let host = host();
        let file = path(
            &host,
            &format!(
                ".codex/sessions/2026/11/16/rollout-2026-11-16T10-00-00-{SHOP_SESSION}.jsonl"
            ),
        );

        let version = as_part(&host, || read_cli_version(&file));

        assert_eq!(version.as_deref(), Some("0.168.0"));
        assert_eq!(
            cli_version_of_line(META.as_bytes()).as_deref(),
            Some("0.163.0")
        );
        assert_eq!(cli_version_of_line(MESSAGE.as_bytes()), None);
        assert_eq!(
            cli_version_of_line(
                META.replace("\"cli_version\":\"0.163.0\",", "").as_bytes()
            ),
            None
        );
    }
}
