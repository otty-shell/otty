//! The parts of a plugin: their kinds and how a call names one.

use std::fmt;

use crate::contract::Contract;

/// The kind of a plugin part. Each kind is one extension point with its
/// own versioned contract (spec 2026-11-19-agent-plugins, Decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PartKind {
    /// An `[[agent]]` part: reads an agent's profiles, sessions and
    /// tokens.
    Agent,
    /// A `[[provider]]` part: reads an account's numbers at a provider.
    Provider,
}

impl PartKind {
    /// Every kind, in the order the manifest lists them.
    pub const ALL: [PartKind; 2] = [PartKind::Agent, PartKind::Provider];

    /// The name of the kind in the manifest, both as the table of its
    /// parts (`[[agent]]`) and as its key in `[api]` (`agent = "0.1"`).
    pub fn key(self) -> &'static str {
        match self {
            PartKind::Agent => "agent",
            PartKind::Provider => "provider",
        }
    }

    /// The kind whose manifest key is `key`.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }

    /// The contract the parts of this kind serve.
    pub fn contract(self) -> Contract {
        match self {
            PartKind::Agent => Contract::Agent,
            PartKind::Provider => Contract::Provider,
        }
    }
}

impl fmt::Display for PartKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.key())
    }
}

/// One part of a plugin, as a call names it: agent ids and provider ids
/// are separate lists, so the kind is part of the name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PartRef {
    kind: PartKind,
    id: String,
}

impl PartRef {
    /// The kind of the part.
    pub fn kind(&self) -> PartKind {
        self.kind
    }

    /// The id of the part, unique among the parts of its kind.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The agent part `id`.
    pub fn agent(id: &str) -> Self {
        Self {
            kind: PartKind::Agent,
            id: id.to_string(),
        }
    }

    /// The provider part `id`.
    pub fn provider(id: &str) -> Self {
        Self {
            kind: PartKind::Provider,
            id: id.to_string(),
        }
    }
}

impl fmt::Display for PartRef {
    /// Writes `agent part `codex``, for the plain reasons of refusals.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} part `{}`", self.kind, self.id)
    }
}

/// Whether `id` is a valid plugin or part id: lowercase ASCII letters,
/// digits and dashes, not empty, with no dash at either end.
pub(crate) fn is_valid_id(id: &str) -> bool {
    let allowed = !id.is_empty()
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
        });

    allowed && !id.starts_with('-') && !id.ends_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_is_found_by_its_manifest_key() {
        assert_eq!(PartKind::from_key("agent"), Some(PartKind::Agent));
        assert_eq!(PartKind::from_key("provider"), Some(PartKind::Provider));
        assert_eq!(PartKind::from_key("panel"), None);
    }

    #[test]
    fn a_part_names_its_kind_and_id_in_plain_words() {
        assert_eq!(PartRef::agent("codex").to_string(), "agent part `codex`");
    }

    #[test]
    fn ids_are_lowercase_letters_digits_and_dashes() {
        for id in ["codex", "claude-code", "open-router-2"] {
            assert!(is_valid_id(id), "{id}");
        }
        for id in ["", "Codex", "claude_code", "-codex", "codex-", "co dex"] {
            assert!(!is_valid_id(id), "{id}");
        }
    }
}
