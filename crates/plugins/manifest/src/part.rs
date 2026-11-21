//! The parts of a plugin: the domains they belong to, their kinds and
//! how a call names one.

use std::fmt;

use crate::contract::Contract;

/// A domain of OTTY that plugins extend, with extension points of its
/// own (issue 21). Each domain names its parts in the manifest under its
/// key, such as `[[agents.agent]]`, so two domains may each have a
/// point with the same name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Domain {
    /// Agents mode: coding agents and the providers they pay through.
    Agents,
}

impl Domain {
    /// The key of the domain in the manifest, such as `agents`.
    pub fn key(self) -> &'static str {
        match self {
            Domain::Agents => "agents",
        }
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.key())
    }
}

/// The kind of a plugin part: one extension point of one domain, with its
/// own versioned contract (spec 2026-11-19-agent-plugins, Decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PartKind {
    /// An `[[agents.agent]]` part: reads an agent's profiles, sessions
    /// and tokens.
    Agent,
    /// An `[[agents.provider]]` part: reads an account's numbers at a
    /// provider.
    Provider,
}

impl PartKind {
    /// Every kind, in the order the manifest lists them.
    pub const ALL: [PartKind; 2] = [PartKind::Agent, PartKind::Provider];

    /// The domain the kind belongs to.
    pub fn domain(self) -> Domain {
        match self {
            PartKind::Agent | PartKind::Provider => Domain::Agents,
        }
    }

    /// The name of the extension point within its domain, such as
    /// `agent`; the kind in plain words, as in "agent part".
    pub fn point(self) -> &'static str {
        match self {
            PartKind::Agent => "agent",
            PartKind::Provider => "provider",
        }
    }

    /// The name of the kind in the manifest, both as the table of its
    /// parts (`[[agents.agent]]`) and as its key in `[api]`
    /// (`agents.agent = "0.1"`).
    pub fn key(self) -> &'static str {
        match self {
            PartKind::Agent => "agents.agent",
            PartKind::Provider => "agents.provider",
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
    /// Writes the point, such as `agent`, for plain reasons such as
    /// "agent part `codex`".
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.point())
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

    /// Issue 21: the kind of a part is its domain and its extension
    /// point, `agents.agent`.
    #[test]
    fn a_kind_is_found_by_its_manifest_key() {
        assert_eq!(PartKind::from_key("agents.agent"), Some(PartKind::Agent));
        assert_eq!(
            PartKind::from_key("agents.provider"),
            Some(PartKind::Provider)
        );
        assert_eq!(PartKind::from_key("agent"), None);
        assert_eq!(PartKind::from_key("agents.panel"), None);
    }

    #[test]
    fn a_kind_is_a_domain_and_a_point() {
        assert_eq!(PartKind::Agent.domain(), Domain::Agents);
        assert_eq!(PartKind::Provider.domain(), Domain::Agents);
        assert_eq!(PartKind::Agent.point(), "agent");
        assert_eq!(PartKind::Provider.point(), "provider");
        assert_eq!(Domain::Agents.key(), "agents");
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
