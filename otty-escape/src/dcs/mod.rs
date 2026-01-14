mod block;

use log::error;
use memchr::memchr;
use thiserror::Error;

use crate::{Action, EscapeActor};
pub use block::{BlockEvent, BlockKind, BlockMeta, BlockPhase};

pub(crate) const DCS_PREFIX: &[u8] = b"otty;";
pub(crate) const MAX_DCS_KIND_LEN: usize = 32;
pub(crate) const MAX_DCS_CONTENT_BYTES: usize = 4096;

pub(crate) const fn max_dcs_buffer_len() -> usize {
    DCS_PREFIX.len() + MAX_DCS_KIND_LEN + 1 + MAX_DCS_CONTENT_BYTES
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum DcsMessageKind {
    Block,
}

impl DcsMessageKind {
    fn from_bytes(bytes: &[u8]) -> Result<Self, DcsMessageParsingError> {
        match bytes {
            b"block" => Ok(Self::Block),
            kind_bytes => {
                let kind = String::from_utf8_lossy(kind_bytes).to_string();
                Err(DcsMessageParsingError::UnsupportedKind(kind))
            },
        }
    }
}

impl std::fmt::Display for DcsMessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Block => "block",
        };
        f.write_str(name)
    }
}

struct DcsMessage<'a> {
    kind: DcsMessageKind,
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

        Ok(Self {
            kind: DcsMessageKind::from_bytes(kind_bytes)?,
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

    match message.kind {
        DcsMessageKind::Block => {
            match block::parse_block_payload(message.payload) {
                Ok(event) => actor.handle(Action::BlockEvent(event)),
                Err(e) => {
                    error!("[OTTY DCS] failed to parse otty block payload: {e}")
                },
            };
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, BlockKind, BlockPhase, EscapeParser, Parser};

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
    fn parses_block_event_from_dcs() {
        let json =
            r#"{"id":"1","phase":"preexec","cmd":"ls","cwd":"/","time":42}"#;
        let payload = format!("\x1bPotty;block;{json}\x1b\\");
        let actions = parse_with_bytes(payload.as_bytes());

        assert!(
            actions.iter().any(|action| match action {
                Action::BlockEvent(event) => {
                    assert_eq!(event.phase, BlockPhase::Preexec);
                    assert_eq!(event.meta.id, "1");
                    assert_eq!(event.meta.kind, BlockKind::Command);
                    assert_eq!(event.meta.cmd.as_deref(), Some("ls"));
                    assert_eq!(event.meta.cwd.as_deref(), Some("/"));
                    assert_eq!(event.meta.started_at, Some(42));
                    assert_eq!(event.meta.finished_at, None);
                    true
                },
                _ => false,
            }),
            "expected at least one BlockEvent action"
        );
    }

    #[test]
    fn ignores_invalid_block_dcs() {
        let json =
            r#"{"id":"1","phase":"preexec","cmd":"ls","cwd":"/","time":42"#;
        let payload = format!("\x1bPotty;block;{json}\x1b\\");
        let actions = parse_with_bytes(payload.as_bytes());

        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, Action::BlockEvent(_))),
            "invalid JSON should not produce BlockEvent"
        );
    }

    #[test]
    fn parses_exit_block_event_from_dcs() {
        let json = r#"{"id":"2","phase":"exit","time":99,"exit_code":3,"shell":"zsh"}"#;
        let payload = format!("\x1bPotty;block;{json}\x1b\\");
        let actions = parse_with_bytes(payload.as_bytes());

        assert!(
            actions.iter().any(|action| match action {
                Action::BlockEvent(event) => {
                    assert_eq!(event.phase, BlockPhase::Exit);
                    assert_eq!(event.meta.kind, BlockKind::Command);
                    assert_eq!(event.meta.id, "2");
                    assert_eq!(event.meta.started_at, None);
                    assert_eq!(event.meta.finished_at, Some(99));
                    assert_eq!(event.meta.exit_code, Some(3));
                    assert_eq!(event.meta.shell.as_deref(), Some("zsh"));
                    true
                },
                _ => false,
            }),
            "expected BlockEvent for exit phase"
        );
    }

    #[test]
    fn parses_precmd_block_event_from_dcs() {
        let json = r#"{"id":"3","phase":"precmd","time":7,"cwd":"/tmp"}"#;
        let payload = format!("\x1bPotty;block;{json}\x1b\\");
        let actions = parse_with_bytes(payload.as_bytes());

        assert!(
            actions.iter().any(|action| match action {
                Action::BlockEvent(event) => {
                    assert_eq!(event.phase, BlockPhase::Precmd);
                    assert_eq!(event.meta.kind, BlockKind::Prompt);
                    assert_eq!(event.meta.id, "3");
                    assert_eq!(event.meta.cmd, None);
                    assert_eq!(event.meta.cwd.as_deref(), Some("/tmp"));
                    assert_eq!(event.meta.started_at, None);
                    assert_eq!(event.meta.finished_at, Some(7));
                    true
                },
                _ => false,
            }),
            "expected BlockEvent for precmd phase"
        );
    }

    #[test]
    fn ignores_dcs_with_unsupported_kind() {
        let json = r#"{"id":"1","phase":"preexec","time":1}"#;
        let payload = format!("\x1bPotty;unknown;{json}\x1b\\");
        let actions = parse_with_bytes(payload.as_bytes());

        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, Action::BlockEvent(_))),
            "unsupported DCS kind should be ignored"
        );
    }

    #[test]
    fn ignores_dcs_with_wrong_prefix() {
        let json = r#"{"id":"1","phase":"preexec","time":1}"#;
        let payload = format!("\x1bPnotty;block;{json}\x1b\\");
        let actions = parse_with_bytes(payload.as_bytes());

        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, Action::BlockEvent(_))),
            "wrong DCS prefix should be ignored"
        );
    }

    #[test]
    fn ignores_dcs_with_empty_payload() {
        let payload = "\x1bPotty;block;\x1b\\";
        let actions = parse_with_bytes(payload.as_bytes());

        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, Action::BlockEvent(_))),
            "empty payload should be ignored"
        );
    }

    #[test]
    fn parses_multiple_block_events_in_stream() {
        let preexec = r#"{"id":"10","phase":"preexec","time":1,"cmd":"ls"}"#;
        let exit = r#"{"id":"10","phase":"exit","time":2,"exit_code":0}"#;
        let payload = format!(
            "\x1bPotty;block;{preexec}\x1b\\\x1bPotty;block;{exit}\x1b\\"
        );
        let actions = parse_with_bytes(payload.as_bytes());

        let block_events = actions
            .into_iter()
            .filter(|action| matches!(action, Action::BlockEvent(_)))
            .count();

        assert_eq!(block_events, 2, "expected two BlockEvent actions");
    }

    #[test]
    fn dcs_message_parse_errors() {
        assert!(matches!(
            DcsMessage::parse(b"block;{}"),
            Err(DcsMessageParsingError::PrefixMissed)
        ));

        assert!(matches!(
            DcsMessage::parse(b"otty;block"),
            Err(DcsMessageParsingError::KindSeparatorMissed)
        ));

        assert!(matches!(
            DcsMessage::parse(b"otty;unknown;{}"),
            Err(DcsMessageParsingError::UnsupportedKind(_))
        ));
    }
}
