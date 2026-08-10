mod event_v2;

pub use event_v2::{ProtocolDiagnostic, ProtocolEvent, ProtocolEventKind};
use log::error;
use memchr::memchr;
use thiserror::Error;

use crate::{Action, EscapeActor};

pub(crate) const DCS_PREFIX: &[u8] = b"otty-dcs;";
pub(crate) const MAX_DCS_KIND_LEN: usize = 32;

pub(crate) const fn max_dcs_buffer_len() -> usize {
    DCS_PREFIX.len()
        + MAX_DCS_KIND_LEN
        + 1
        + event_v2::MAX_ENCODED_EVENT_BYTES
        + 2
}

#[derive(Debug, Error)]
enum DcsMessageParsingError {
    #[error("message is missing the DCS prefix")]
    PrefixMissed,

    #[error("message is missing the kind separator")]
    KindSeparatorMissed,

    #[error("unsupported otty DCS kind")]
    UnsupportedKind(String),
}

struct DcsMessage<'a> {
    payload: &'a [u8],
}

impl<'a> DcsMessage<'a> {
    fn parse(buffer: &'a [u8]) -> Result<Self, DcsMessageParsingError> {
        if !buffer.starts_with(DCS_PREFIX) {
            return Err(DcsMessageParsingError::PrefixMissed);
        }

        let remaining = &buffer[DCS_PREFIX.len()..];
        let separator_idx = memchr(b';', remaining)
            .ok_or(DcsMessageParsingError::KindSeparatorMissed)?;

        let (kind_bytes, rest) = remaining.split_at(separator_idx);
        if kind_bytes != b"event-v2" {
            let kind = String::from_utf8_lossy(kind_bytes).to_string();
            return Err(DcsMessageParsingError::UnsupportedKind(kind));
        }

        Ok(Self {
            payload: &rest[1..],
        })
    }
}

pub(crate) fn perform<A: EscapeActor>(actor: &mut A, raw_message: &[u8]) {
    let message = match DcsMessage::parse(raw_message) {
        Ok(msg) => msg,
        Err(e) => {
            error!("[OTTY DCS] failed to parsing message: {e}");
            return;
        },
    };

    match event_v2::parse_event_v2_payload(message.payload) {
        Ok(event) => actor.handle(Action::ProtocolEvent(event)),
        Err(event_v2::EventV2ParsingError::UnsupportedVersion {
            version,
            terminal_session_id,
        }) => actor.handle(Action::ProtocolDiagnostic(
            ProtocolDiagnostic::UnsupportedVersion {
                terminal_session_id,
                version,
            },
        )),
        Err(e) => {
            error!("[OTTY DCS] failed to parse event-v2 payload: {e}")
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, EscapeParser, Parser};

    #[derive(Default)]
    struct CollectingActor {
        actions: Vec<Action>,
    }

    impl EscapeActor for CollectingActor {
        fn handle(&mut self, action: Action) {
            self.actions.push(action);
        }
    }

    fn parse_with_bytes(input: &[u8]) -> Vec<Action> {
        let mut parser = Parser::<otty_vte::Parser>::new();
        let mut actor = CollectingActor::default();
        parser.advance(input, &mut actor);
        actor.actions
    }

    #[test]
    fn legacy_block_frames_are_ignored_without_panicking() {
        let valid = b"\x1bPotty-dcs;block;{\"id\":\"legacy\",\"phase\":\"preexec\",\"time\":1}\x1b\\";
        let malformed = b"\x1bPotty-dcs;block;{not-json}\x1b\\";
        let mut parser = Parser::<otty_vte::Parser>::new();
        let mut actor = CollectingActor::default();

        for byte in valid {
            parser.advance(std::slice::from_ref(byte), &mut actor);
        }
        parser.advance(malformed, &mut actor);
        let json = r#"{"v":2,"event":"shell_hello","terminal_session_id":"session","shell_instance_id":"shell","seq":1,"payload":{"shell":"bash"}}"#;
        let encoded = json
            .bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let v2 = format!("\x1bPotty-dcs;event-v2;h;{encoded}\x1b\\");
        parser.advance(v2.as_bytes(), &mut actor);

        assert!(
            matches!(actor.actions.as_slice(), [Action::ProtocolEvent(_)]),
            "legacy frames must be ignored and parser recovery must accept v2",
        );
    }

    #[test]
    fn ignores_dcs_with_unsupported_kind() {
        let json = r#"{"id":"1","phase":"preexec","time":1}"#;
        let payload = format!("\x1bPotty-dcs;unknown;{json}\x1b\\");
        let actions = parse_with_bytes(payload.as_bytes());

        assert!(actions.is_empty(), "unsupported DCS kind should be ignored");
    }

    #[test]
    fn reports_unsupported_protocol_version_without_exposing_payload() {
        let json = r#"{"v":9,"event":"shell_hello","terminal_session_id":"session","shell_instance_id":"shell","seq":1,"payload":{"shell":"bash"}}"#;
        let encoded = json
            .bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let payload = format!("\x1bPotty-dcs;event-v2;h;{encoded}\x1b\\");

        let actions = parse_with_bytes(payload.as_bytes());

        assert!(actions.iter().any(|action| matches!(
            action,
            Action::ProtocolDiagnostic(
                crate::ProtocolDiagnostic::UnsupportedVersion {
                    terminal_session_id,
                    version: 9,
                }
            ) if terminal_session_id == "session"
        )));
    }

    #[test]
    fn ignores_dcs_with_wrong_prefix() {
        let json = r#"{"id":"1","phase":"preexec","time":1}"#;
        let payload = format!("\x1bPnotty-dcs;block;{json}\x1b\\");
        let actions = parse_with_bytes(payload.as_bytes());

        assert!(actions.is_empty(), "wrong DCS prefix should be ignored");
    }

    #[test]
    fn ignores_dcs_with_empty_payload() {
        let payload = "\x1bPotty-dcs;block;\x1b\\";
        let actions = parse_with_bytes(payload.as_bytes());

        assert!(actions.is_empty(), "empty legacy payload should be ignored");
    }

    #[test]
    fn dcs_message_parse_errors() {
        assert!(matches!(
            DcsMessage::parse(b"block;{}"),
            Err(DcsMessageParsingError::PrefixMissed)
        ));

        assert!(matches!(
            DcsMessage::parse(b"otty-dcs;block"),
            Err(DcsMessageParsingError::KindSeparatorMissed)
        ));

        assert!(matches!(
            DcsMessage::parse(b"otty-dcs;unknown;{}"),
            Err(DcsMessageParsingError::UnsupportedKind(_))
        ));
    }

    #[test]
    fn parses_fragmented_event_v2_hex_frame() {
        let json = r#"{"v":2,"event":"command_start","terminal_session_id":"session","shell_instance_id":"shell","seq":3,"block_id":"session:shell:1","sent_at_unix_ms":10,"payload":{"command":"printf ok","cwd":"/tmp"}}"#;
        let encoded = json
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let frame = format!("\x1bPotty-dcs;event-v2;h;{encoded}\x1b\\");
        let mut parser = Parser::<otty_vte::Parser>::new();
        let mut actor = CollectingActor::default();

        for byte in frame.bytes() {
            parser.advance(&[byte], &mut actor);
        }

        assert!(actor.actions.iter().any(|action| matches!(
            action,
            Action::ProtocolEvent(event)
                if event.sequence() == 3
                    && event.terminal_session_id() == "session"
        )));
    }
}
