use std::fmt;

use serde::Deserialize;
use thiserror::Error;

use crate::dcs::MAX_DCS_CONTENT_BYTES;

const MAX_CMD_LEN: usize = 1024;
const MAX_CWD_LEN: usize = 512;

#[derive(Debug, Error)]
pub(super) enum BlockPayloadParsingError {
    #[error("payload is empty")]
    EmptyPayload,

    #[error("payload length is exceeded")]
    PayloadLengthExceeded,

    #[error("payload deserialization error")]
    DeserializationError(#[from] serde_json::Error),

    #[error("unsuported phase")]
    UnsupportedPhase(String),
}

/// Kind of a terminal block in the session history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// Command output block started by a `preexec` event.
    Command,
    /// Prompt block corresponding to a rendered shell prompt.
    Prompt,
    /// Full-screen application block (ALT_SCREEN / TUI).
    FullScreen,
}

/// Phase of the block lifecycle as reported by the shell hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockPhase {
    /// Command is about to start executing.
    Preexec,
    /// Command finished executing.
    Exit,
    /// Prompt is about to be rendered.
    Precmd,
}

/// Metadata describing a single terminal block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockMeta {
    pub id: String,
    pub kind: BlockKind,
    pub cmd: Option<String>,
    pub cwd: Option<String>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub exit_code: Option<i32>,
    pub shell: Option<String>,
    pub is_alt_screen: bool,
}

/// High-level block event parsed from the DCS JSON payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockEvent {
    pub phase: BlockPhase,
    pub meta: BlockMeta,
}

#[derive(Debug, Deserialize)]
struct BlockDcsPayload {
    id: String,
    phase: String,
    #[serde(default)]
    cmd: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    time: Option<i64>,
    #[serde(default)]
    exit_code: Option<i32>,
    #[serde(default)]
    shell: Option<String>,
}

impl fmt::Display for BlockPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Preexec => "preexec",
            Self::Exit => "exit",
            Self::Precmd => "precmd",
        };
        f.write_str(s)
    }
}

fn truncate_opt(value: Option<String>, max_chars: usize) -> Option<String> {
    value.map(|s| s.chars().take(max_chars).collect())
}

/// Try to parse a block event from a completed DCS payload.
///
/// The payload is expected to be a UTF-8 JSON string for the `otty-block` DCS.
pub(crate) fn parse_block_payload(
    payload: &[u8],
) -> Result<BlockEvent, BlockPayloadParsingError> {
    if payload.is_empty() {
        return Err(BlockPayloadParsingError::EmptyPayload);
    }

    if payload.len() > MAX_DCS_CONTENT_BYTES {
        return Err(BlockPayloadParsingError::PayloadLengthExceeded);
    }

    let raw: BlockDcsPayload = serde_json::from_slice(payload)?;

    let phase = match raw.phase.as_str() {
        "preexec" => BlockPhase::Preexec,
        "exit" => BlockPhase::Exit,
        "precmd" => BlockPhase::Precmd,
        other => {
            return Err(BlockPayloadParsingError::UnsupportedPhase(
                other.to_string(),
            ));
        },
    };

    let kind = match phase {
        BlockPhase::Preexec | BlockPhase::Exit => BlockKind::Command,
        BlockPhase::Precmd => BlockKind::Prompt,
    };

    let started_at = match phase {
        BlockPhase::Preexec => raw.time,
        _ => None,
    };

    let finished_at = match phase {
        BlockPhase::Exit | BlockPhase::Precmd => raw.time,
        _ => None,
    };

    let cmd = truncate_opt(raw.cmd, MAX_CMD_LEN);
    let cwd = truncate_opt(raw.cwd, MAX_CWD_LEN);

    let meta = BlockMeta {
        id: raw.id,
        kind,
        cmd,
        cwd,
        started_at,
        finished_at,
        exit_code: raw.exit_code,
        shell: raw.shell,
        is_alt_screen: false,
    };

    Ok(BlockEvent { phase, meta })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_payload() {
        let err = parse_block_payload(b"").unwrap_err();
        assert!(matches!(err, BlockPayloadParsingError::EmptyPayload));
    }

    #[test]
    fn rejects_oversized_payload() {
        let payload = vec![b'a'; MAX_DCS_CONTENT_BYTES + 1];
        let err = parse_block_payload(&payload).unwrap_err();
        assert!(matches!(
            err,
            BlockPayloadParsingError::PayloadLengthExceeded
        ));
    }

    #[test]
    fn rejects_unknown_phase() {
        let json = r#"{"id":"1","phase":"later","time":1}"#;
        let err = parse_block_payload(json.as_bytes()).unwrap_err();
        assert!(matches!(
            err,
            BlockPayloadParsingError::UnsupportedPhase(phase) if phase == "later"
        ));
    }

    #[test]
    fn rejects_invalid_utf8_json() {
        let err = parse_block_payload(b"{\xff}").unwrap_err();
        assert!(matches!(
            err,
            BlockPayloadParsingError::DeserializationError(_)
        ));
    }

    #[test]
    fn truncates_cmd_and_cwd_fields() {
        let cmd = "a".repeat(MAX_CMD_LEN + 10);
        let cwd = "b".repeat(MAX_CWD_LEN + 10);
        let json = format!(
            r#"{{"id":"1","phase":"preexec","cmd":"{cmd}","cwd":"{cwd}","time":1}}"#
        );
        let event =
            parse_block_payload(json.as_bytes()).expect("valid payload");

        assert_eq!(event.meta.cmd.as_deref().unwrap().len(), MAX_CMD_LEN);
        assert_eq!(event.meta.cwd.as_deref().unwrap().len(), MAX_CWD_LEN);
    }

    #[test]
    fn parses_exit_payload_fields() {
        let json = r#"{"id":"2","phase":"exit","time":5,"exit_code":7,"shell":"fish"}"#;
        let event =
            parse_block_payload(json.as_bytes()).expect("valid payload");

        assert_eq!(event.phase, BlockPhase::Exit);
        assert_eq!(event.meta.kind, BlockKind::Command);
        assert_eq!(event.meta.finished_at, Some(5));
        assert_eq!(event.meta.exit_code, Some(7));
        assert_eq!(event.meta.shell.as_deref(), Some("fish"));
    }
}
