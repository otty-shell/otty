//! The stable ids of the sources of the data (spec Decisions 5 and 10).

use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError};

/// The names plugin parts gave their sources, each kept once for the whole
/// run, so that a [`SourceId`] stays a small value that copies.
static SOURCE_NAMES: Mutex<BTreeSet<&'static str>> =
    Mutex::new(BTreeSet::new());

/// Which source a piece of data came from, or failed in.
///
/// The id is stable and carries no text a person reads: the app turns it
/// into a translated name, such as "LiteLLM prices", or shows the name a
/// plugin part gave its source.
///
/// ```
/// use otty_agents::SourceId;
///
/// let id = SourceId::LiteLlmPrices;
///
/// assert_ne!(id, SourceId::KeptPrices);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    /// The money estimated from the tokens of the session files at the
    /// list prices of the LiteLLM price table.
    SessionFilesAtListPrices,
    /// The money the agent wrote for each answer in its database: exact,
    /// not an estimate.
    CostWrittenByAgent,
    /// The list prices of the LiteLLM price table, fetched at most once a
    /// day.
    LiteLlmPrices,
    /// The copy of the LiteLLM price table kept from the last good fetch.
    KeptPrices,
    /// A part of a plugin: its sessions, tokens, limits, statistics or the
    /// numbers of an account (spec 2026-11-19-agent-plugins). The registry
    /// names the plugin.
    Plugin,
    /// One of the sources of limits an agent part names, or the provider
    /// part named after its provider (issue 15 of spec
    /// 2026-11-19-agent-plugins). The registry names its plugin.
    Named(SourceName),
}

/// The name a plugin part gives one of its sources, such as `example
/// app-server`, shown as the plugin wrote it. Each name is kept once for
/// the whole run: parts name a handful of sources.
///
/// ```
/// use otty_agents::SourceName;
///
/// let name = SourceName::new("example app-server");
///
/// assert_eq!(name.as_str(), "example app-server");
/// assert_eq!(name, SourceName::new("example app-server"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceName(&'static str);

impl SourceName {
    /// The name as the plugin wrote it.
    pub fn as_str(&self) -> &'static str {
        self.0
    }

    /// The source named `name`.
    pub fn new(name: &str) -> Self {
        let mut names =
            SOURCE_NAMES.lock().unwrap_or_else(PoisonError::into_inner);

        if let Some(kept) = names.get(name) {
            return Self(kept);
        }
        let kept: &'static str = Box::leak(name.to_string().into_boxed_str());
        names.insert(kept);
        Self(kept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_name_is_kept_once() {
        let first = SourceName::new("made-up source");
        let again = SourceName::new(&String::from("made-up source"));

        assert_eq!(first, again);
        assert!(std::ptr::eq(first.as_str(), again.as_str()));
        assert_ne!(first, SourceName::new("another made-up source"));
    }

    #[test]
    fn a_named_source_is_not_the_plain_plugin_source() {
        let named = SourceId::Named(SourceName::new("made-up session files"));

        assert_ne!(named, SourceId::Plugin);
        assert_eq!(
            named,
            SourceId::Named(SourceName::new("made-up session files"))
        );
    }
}
