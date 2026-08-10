use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

pub(crate) const MAX_DECODED_JSON_BYTES: usize = 32 * 1024;
pub(crate) const MAX_ENCODED_EVENT_BYTES: usize = MAX_DECODED_JSON_BYTES * 2;
pub(crate) const MAX_COMMAND_BYTES: usize = 16 * 1024;
const MAX_ID_BYTES: usize = 512;

/// Semantic event decoded from an OTTY protocol-v2 envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolEvent {
    terminal_session_id: String,
    shell_instance_id: String,
    sequence: u64,
    sent_at_unix_ms: Option<i64>,
    kind: ProtocolEventKind,
}

impl ProtocolEvent {
    /// Return the terminal session namespace asserted by the sender.
    pub fn terminal_session_id(&self) -> &str {
        &self.terminal_session_id
    }

    /// Return the root or nested shell identity that emitted this event.
    pub fn shell_instance_id(&self) -> &str {
        &self.shell_instance_id
    }

    /// Return the monotonic sequence scoped to the shell instance.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Return the sender timestamp when supplied.
    pub fn sent_at_unix_ms(&self) -> Option<i64> {
        self.sent_at_unix_ms
    }

    /// Return the typed lifecycle payload.
    pub fn kind(&self) -> &ProtocolEventKind {
        &self.kind
    }
}

/// Supported shell lifecycle payloads in protocol v2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolEventKind {
    /// Register one root or nested shell context.
    ShellHello {
        shell: String,
        shell_version: Option<String>,
        parent_shell_instance_id: Option<String>,
        capabilities: Vec<String>,
    },
    /// Prepare the stable block used by the next command.
    PromptPrepare {
        block_id: String,
        cwd: Option<String>,
    },
    /// Begin execution in the prepared block.
    CommandStart {
        block_id: String,
        command: Option<String>,
        cwd: Option<String>,
        command_truncated: bool,
    },
    /// Complete one exact command block.
    CommandEnd {
        block_id: String,
        exit_code: Option<i32>,
        signal: Option<i32>,
        pipe_status: Vec<i32>,
        next_block_id: Option<String>,
        cwd: Option<String>,
    },
    /// Patch shell context fields outside normal prompt transitions.
    ContextUpdate { cwd: Option<String> },
    /// Close one root or nested shell context.
    ShellExit {
        status: Option<i32>,
        active_block_id: Option<String>,
    },
    /// Record a safe shell-side error code.
    IntegrationError { code: String },
}

/// Safe protocol-level diagnostic that can be validated by the terminal model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolDiagnostic {
    /// A structurally valid event used an unsupported protocol major.
    UnsupportedVersion {
        terminal_session_id: String,
        version: u16,
    },
}

#[derive(Debug, Error)]
pub(super) enum EventV2ParsingError {
    #[error("event-v2 payload must use hex encoding")]
    UnsupportedEncoding,

    #[error("event-v2 encoded or decoded payload exceeded its limit")]
    PayloadLengthExceeded,

    #[error("event-v2 payload contains malformed hex")]
    MalformedHex,

    #[error("event-v2 decoded payload is not UTF-8")]
    InvalidUtf8,

    #[error("event-v2 JSON deserialization failed")]
    Deserialization(#[from] serde_json::Error),

    #[error("unsupported event-v2 protocol major {version}")]
    UnsupportedVersion {
        version: u16,
        terminal_session_id: String,
    },

    #[error("required event-v2 field is empty: {0}")]
    MissingField(&'static str),

    #[error("event-v2 sequence must be greater than zero")]
    InvalidSequence,

    #[error("unsupported event-v2 event: {0}")]
    UnsupportedEvent(String),
}

#[derive(Debug, Deserialize)]
struct RawEnvelope {
    v: u16,
    event: String,
    terminal_session_id: String,
    shell_instance_id: String,
    seq: u64,
    #[serde(default)]
    block_id: Option<String>,
    #[serde(default)]
    sent_at_unix_ms: Option<i64>,
    payload: Value,
}

impl RawEnvelope {
    fn validate(&self) -> Result<(), EventV2ParsingError> {
        if self.v != 2 {
            return Err(EventV2ParsingError::UnsupportedVersion {
                version: self.v,
                terminal_session_id: self.terminal_session_id.clone(),
            });
        }
        validate_required_id(&self.terminal_session_id, "terminal_session_id")?;
        validate_required_id(&self.shell_instance_id, "shell_instance_id")?;
        if self.seq == 0 {
            return Err(EventV2ParsingError::InvalidSequence);
        }

        Ok(())
    }

    fn required_block_id(&self) -> Result<String, EventV2ParsingError> {
        let block_id = self
            .block_id
            .as_deref()
            .ok_or(EventV2ParsingError::MissingField("block_id"))?;
        validate_required_id(block_id, "block_id")?;

        Ok(block_id.to_string())
    }
}

#[derive(Debug, Deserialize)]
struct ShellHelloPayload {
    shell: String,
    #[serde(default)]
    shell_version: Option<String>,
    #[serde(default)]
    parent_shell_instance_id: Option<String>,
    #[serde(default)]
    capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PromptPreparePayload {
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CommandStartPayload {
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    command_truncated: bool,
}

#[derive(Debug, Deserialize)]
struct CommandEndPayload {
    #[serde(default)]
    exit_code: Option<i32>,
    #[serde(default)]
    signal: Option<i32>,
    #[serde(default)]
    pipe_status: Vec<i32>,
    #[serde(default)]
    next_block_id: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContextUpdatePayload {
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShellExitPayload {
    #[serde(default)]
    status: Option<i32>,
    #[serde(default)]
    active_block_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IntegrationErrorPayload {
    code: String,
}

pub(super) fn parse_event_v2_payload(
    payload: &[u8],
) -> Result<ProtocolEvent, EventV2ParsingError> {
    let encoded = payload
        .strip_prefix(b"h;")
        .ok_or(EventV2ParsingError::UnsupportedEncoding)?;
    let json = decode_hex_json(encoded)?;
    let raw: RawEnvelope = serde_json::from_str(&json)?;
    raw.validate()?;

    let kind = parse_event_kind(&raw)?;

    Ok(ProtocolEvent {
        terminal_session_id: raw.terminal_session_id,
        shell_instance_id: raw.shell_instance_id,
        sequence: raw.seq,
        sent_at_unix_ms: raw.sent_at_unix_ms,
        kind,
    })
}

fn decode_hex_json(encoded: &[u8]) -> Result<String, EventV2ParsingError> {
    if encoded.len() > MAX_ENCODED_EVENT_BYTES {
        return Err(EventV2ParsingError::PayloadLengthExceeded);
    }
    if !encoded.len().is_multiple_of(2) {
        return Err(EventV2ParsingError::MalformedHex);
    }

    let mut decoded = Vec::with_capacity(encoded.len() / 2);
    for pair in encoded.chunks_exact(2) {
        let high = decode_nibble(pair[0])?;
        let low = decode_nibble(pair[1])?;
        decoded.push((high << 4) | low);
    }
    if decoded.len() > MAX_DECODED_JSON_BYTES {
        return Err(EventV2ParsingError::PayloadLengthExceeded);
    }

    String::from_utf8(decoded).map_err(|_| EventV2ParsingError::InvalidUtf8)
}

fn decode_nibble(byte: u8) -> Result<u8, EventV2ParsingError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(EventV2ParsingError::MalformedHex),
    }
}

fn parse_event_kind(
    raw: &RawEnvelope,
) -> Result<ProtocolEventKind, EventV2ParsingError> {
    match raw.event.as_str() {
        "shell_hello" => {
            let payload: ShellHelloPayload =
                serde_json::from_value(raw.payload.clone())?;
            if payload.shell.is_empty() {
                return Err(EventV2ParsingError::MissingField("payload.shell"));
            }
            if let Some(parent) = &payload.parent_shell_instance_id {
                validate_required_id(
                    parent,
                    "payload.parent_shell_instance_id",
                )?;
            }

            Ok(ProtocolEventKind::ShellHello {
                shell: payload.shell,
                shell_version: payload.shell_version,
                parent_shell_instance_id: payload.parent_shell_instance_id,
                capabilities: payload.capabilities,
            })
        },
        "prompt_prepare" => {
            let payload: PromptPreparePayload =
                serde_json::from_value(raw.payload.clone())?;
            Ok(ProtocolEventKind::PromptPrepare {
                block_id: raw.required_block_id()?,
                cwd: payload.cwd,
            })
        },
        "command_start" => {
            let payload: CommandStartPayload =
                serde_json::from_value(raw.payload.clone())?;
            let (command, was_truncated) = payload
                .command
                .map(|command| truncate_utf8(command, MAX_COMMAND_BYTES))
                .map_or((None, false), |(command, truncated)| {
                    (Some(command), truncated)
                });

            Ok(ProtocolEventKind::CommandStart {
                block_id: raw.required_block_id()?,
                command,
                cwd: payload.cwd,
                command_truncated: payload.command_truncated || was_truncated,
            })
        },
        "command_end" => {
            let payload: CommandEndPayload =
                serde_json::from_value(raw.payload.clone())?;
            if let Some(next_block_id) = &payload.next_block_id {
                validate_required_id(next_block_id, "payload.next_block_id")?;
            }

            Ok(ProtocolEventKind::CommandEnd {
                block_id: raw.required_block_id()?,
                exit_code: payload.exit_code,
                signal: payload.signal,
                pipe_status: payload.pipe_status,
                next_block_id: payload.next_block_id,
                cwd: payload.cwd,
            })
        },
        "context_update" => {
            let payload: ContextUpdatePayload =
                serde_json::from_value(raw.payload.clone())?;
            Ok(ProtocolEventKind::ContextUpdate { cwd: payload.cwd })
        },
        "shell_exit" => {
            let payload: ShellExitPayload =
                serde_json::from_value(raw.payload.clone())?;
            if let Some(active_block_id) = &payload.active_block_id {
                validate_required_id(
                    active_block_id,
                    "payload.active_block_id",
                )?;
            }

            Ok(ProtocolEventKind::ShellExit {
                status: payload.status,
                active_block_id: payload.active_block_id,
            })
        },
        "integration_error" => {
            let payload: IntegrationErrorPayload =
                serde_json::from_value(raw.payload.clone())?;
            if payload.code.is_empty() {
                return Err(EventV2ParsingError::MissingField("payload.code"));
            }

            Ok(ProtocolEventKind::IntegrationError { code: payload.code })
        },
        event => Err(EventV2ParsingError::UnsupportedEvent(event.to_string())),
    }
}

fn validate_required_id(
    value: &str,
    field: &'static str,
) -> Result<(), EventV2ParsingError> {
    if value.is_empty() || value.len() > MAX_ID_BYTES {
        return Err(EventV2ParsingError::MissingField(field));
    }

    Ok(())
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> (String, bool) {
    if value.len() <= max_bytes {
        return (value, false);
    }

    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    (value, true)
}

#[cfg(test)]
mod tests {
    use super::{
        EventV2ParsingError, MAX_COMMAND_BYTES, ProtocolEventKind,
        decode_hex_json, parse_event_v2_payload,
    };

    fn encode_json(json: &str) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(json.len() * 2 + 2);
        encoded.extend_from_slice(b"h;");
        for byte in json.bytes() {
            encoded.extend_from_slice(format!("{byte:02x}").as_bytes());
        }
        encoded
    }

    #[test]
    fn parses_valid_command_end_envelope() {
        let json = r#"{"v":2,"event":"command_end","terminal_session_id":"session","shell_instance_id":"shell","seq":18,"block_id":"session:shell:7","sent_at_unix_ms":1785680123456,"payload":{"exit_code":7,"pipe_status":[0,7],"next_block_id":"session:shell:8","cwd":"/after"}}"#;

        let event = parse_event_v2_payload(&encode_json(json))
            .expect("valid event-v2 payload");

        assert_eq!(event.terminal_session_id(), "session");
        assert_eq!(event.shell_instance_id(), "shell");
        assert_eq!(event.sequence(), 18);
        assert_eq!(event.sent_at_unix_ms(), Some(1785680123456));
        assert!(matches!(
            event.kind(),
            ProtocolEventKind::CommandEnd {
                block_id,
                exit_code: Some(7),
                pipe_status,
                next_block_id: Some(next),
                cwd: Some(cwd),
                ..
            } if block_id == "session:shell:7"
                && pipe_status == &[0, 7]
                && next == "session:shell:8"
                && cwd == "/after"
        ));
    }

    #[test]
    fn rejects_malformed_hex_json_and_utf8() {
        assert!(matches!(
            decode_hex_json(b"xyz"),
            Err(EventV2ParsingError::MalformedHex)
        ));
        assert!(matches!(
            decode_hex_json(b"ff"),
            Err(EventV2ParsingError::InvalidUtf8)
        ));
        assert!(matches!(
            parse_event_v2_payload(b"x;00"),
            Err(EventV2ParsingError::UnsupportedEncoding)
        ));
    }

    #[test]
    fn rejects_unsupported_version_and_missing_required_fields() {
        let unsupported = r#"{"v":9,"event":"shell_hello","terminal_session_id":"session","shell_instance_id":"shell","seq":1,"payload":{"shell":"bash"}}"#;
        let missing_session = r#"{"v":2,"event":"shell_hello","shell_instance_id":"shell","seq":1,"payload":{"shell":"bash"}}"#;

        assert!(matches!(
            parse_event_v2_payload(&encode_json(unsupported)),
            Err(EventV2ParsingError::UnsupportedVersion {
                version: 9,
                terminal_session_id,
            }) if terminal_session_id == "session"
        ));
        assert!(matches!(
            parse_event_v2_payload(&encode_json(missing_session)),
            Err(EventV2ParsingError::Deserialization(_))
        ));
    }

    #[test]
    fn rejects_empty_ids_and_zero_sequence() {
        let empty_session = r#"{"v":2,"event":"shell_hello","terminal_session_id":"","shell_instance_id":"shell","seq":1,"payload":{"shell":"bash"}}"#;
        let zero_sequence = r#"{"v":2,"event":"shell_hello","terminal_session_id":"session","shell_instance_id":"shell","seq":0,"payload":{"shell":"bash"}}"#;

        assert!(matches!(
            parse_event_v2_payload(&encode_json(empty_session)),
            Err(EventV2ParsingError::MissingField("terminal_session_id"))
        ));
        assert!(matches!(
            parse_event_v2_payload(&encode_json(zero_sequence)),
            Err(EventV2ParsingError::InvalidSequence)
        ));
    }

    #[test]
    fn bounds_decoded_payload_and_command_text() {
        let oversized_hex = vec![b'0'; (32 * 1024 * 2) + 2];
        assert!(matches!(
            decode_hex_json(&oversized_hex),
            Err(EventV2ParsingError::PayloadLengthExceeded)
        ));

        let command = "x".repeat(MAX_COMMAND_BYTES + 100);
        let json = format!(
            r#"{{"v":2,"event":"command_start","terminal_session_id":"session","shell_instance_id":"shell","seq":2,"block_id":"session:shell:1","payload":{{"command":"{command}","cwd":"/tmp"}}}}"#
        );
        let event = parse_event_v2_payload(&encode_json(&json))
            .expect("long commands are bounded, not rejected");

        assert!(matches!(
            event.kind(),
            ProtocolEventKind::CommandStart {
                command: Some(command),
                command_truncated: true,
                ..
            } if command.len() == MAX_COMMAND_BYTES
        ));
    }

    #[test]
    fn preserves_unicode_newlines_quotes_and_control_like_text() {
        let json = r#"{"v":2,"event":"command_start","terminal_session_id":"session","shell_instance_id":"shell","seq":2,"block_id":"session:shell:1","payload":{"command":"printf 'λ\n\\u001b\\\\'","cwd":"/tmp"}}"#;

        let event = parse_event_v2_payload(&encode_json(json))
            .expect("encoded command should parse");

        assert!(matches!(
            event.kind(),
            ProtocolEventKind::CommandStart { command: Some(command), .. }
                if command == "printf 'λ\n\\u001b\\\\'"
        ));
    }
}
