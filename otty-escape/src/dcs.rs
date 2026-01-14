use log::warn;

use crate::block;
use crate::{Action, EscapeActor};

pub(crate) const DCS_PREFIX: &[u8] = b"otty;";
pub(crate) const MAX_DCS_KIND_LEN: usize = 32;
pub(crate) const MAX_DCS_CONTENT_BYTES: usize = 4096;

pub(crate) const fn max_dcs_buffer_len() -> usize {
    DCS_PREFIX.len() + MAX_DCS_KIND_LEN + 1 + MAX_DCS_CONTENT_BYTES
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DcsKind {
    Block,
}

impl DcsKind {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"block" => Some(Self::Block),
            kind_bytes => {
                let kind = String::from_utf8_lossy(kind_bytes);
                warn!("unsupported otty DCS kind: {kind}");
                None
            },
        }
    }
}

impl std::fmt::Display for DcsKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Block => "block",
        };
        f.write_str(name)
    }
}

struct DcsEnvelope<'a> {
    kind: DcsKind,
    payload: &'a [u8],
}

impl<'a> DcsEnvelope<'a> {
    fn parse(buffer: &'a [u8]) -> Option<Self> {
        if !buffer.starts_with(DCS_PREFIX) {
            warn!("otty DCS is missing the dcs prefix");
            return None;
        }

        let remaining = &buffer[DCS_PREFIX.len()..];
        let Some(separator_idx) = remaining.iter().position(|&b| b == b';')
        else {
            warn!("otty DCS is missing the kind separator");
            return None;
        };

        let (kind_bytes, rest) = remaining.split_at(separator_idx);
        if kind_bytes.is_empty() {
            warn!("otty DCS is missing the kind name");
            return None;
        }

        Some(Self {
            kind: DcsKind::from_bytes(kind_bytes)?,
            payload: &rest[1..],
        })
    }
}

pub(crate) fn perform<A: EscapeActor>(actor: &mut A, state: &[u8]) {
    let Some(dcs) = DcsEnvelope::parse(state) else {
        return;
    };

    match dcs.kind {
        DcsKind::Block => {
            if let Some(event) = block::parse_block_payload(dcs.payload) {
                actor.handle(Action::BlockEvent(event));
            } else {
                warn!("otty-block DCS payload could not be parsed");
            }
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
        let json = r#"{"v":1,"id":"1","phase":"preexec","cmd":"ls","cwd":"/","time":42}"#;
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
        let json = r#"{"v":1,"id":"1","phase":"preexec","cmd":"ls","cwd":"/","time":42"#;
        let payload = format!("\x1bPotty;block;{json}\x1b\\");
        let actions = parse_with_bytes(payload.as_bytes());

        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, Action::BlockEvent(_))),
            "invalid JSON should not produce BlockEvent"
        );
    }
}
