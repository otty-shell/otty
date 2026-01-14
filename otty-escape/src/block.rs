use std::fmt;

use log::warn;
use serde::Deserialize;

use crate::dcs::MAX_DCS_CONTENT_BYTES;

const MAX_CMD_LEN: usize = 1024;
const MAX_CWD_LEN: usize = 512;

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
pub(crate) fn parse_block_payload(payload: &[u8]) -> Option<BlockEvent> {
    if payload.is_empty() {
        warn!("otty-block DCS has empty JSON payload");
        return None;
    }

    if payload.len() > MAX_DCS_CONTENT_BYTES {
        warn!(
            "otty-block JSON payload too large: {len} bytes (max {max})",
            len = payload.len(),
            max = MAX_DCS_CONTENT_BYTES
        );
        return None;
    }

    let raw: BlockDcsPayload = match serde_json::from_slice(payload) {
        Ok(raw) => raw,
        Err(err) => {
            warn!("failed to parse otty-block JSON payload: {err}");
            return None;
        },
    };

    let phase = match raw.phase.as_str() {
        "preexec" => BlockPhase::Preexec,
        "exit" => BlockPhase::Exit,
        "precmd" => BlockPhase::Precmd,
        other => {
            warn!("unknown otty-block phase: {other}");
            return None;
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

    Some(BlockEvent { phase, meta })
}
