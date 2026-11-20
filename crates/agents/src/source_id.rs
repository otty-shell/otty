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
/// into a translated name, such as "Anthropic usage API".
///
/// ```
/// use otty_agents::SourceId;
///
/// let id = SourceId::AnthropicUsageApi;
///
/// assert_ne!(id, SourceId::ClaudeUsageCommand);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    /// The sessions or the tokens read from the agent's session files.
    SessionFiles,
    /// Claude Code's limits from `GET api.anthropic.com/api/oauth/usage`.
    AnthropicUsageApi,
    /// Claude Code's limits read from its `/usage` panel in a hidden
    /// terminal.
    ClaudeUsageCommand,
    /// Codex's limits from `GET chatgpt.com/backend-api/wham/usage`.
    ChatGptUsageApi,
    /// Codex's limits from `account/rateLimits/read` of `codex
    /// app-server`.
    CodexAppServer,
    /// Codex's limits from the last `rate_limits` record of its newest
    /// session files.
    LatestCodexSession,
    /// The money of an Anthropic organization from the Admin API cost
    /// report.
    AnthropicCostReport,
    /// The money of an OpenAI organization from
    /// `GET api.openai.com/v1/organization/costs`, read with an admin key.
    OpenAiCosts,
    /// The money left on a Moonshot account from
    /// `GET api.moonshot.ai/v1/users/me/balance`.
    MoonshotBalance,
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
    /// One of the sources of limits an agent part names, such as `codex
    /// app-server` (issue 15 of spec 2026-11-19-agent-plugins). The
    /// registry names its plugin.
    Named(SourceName),
}

/// The name a plugin part gives one of its sources, such as `codex
/// app-server`, shown as the plugin wrote it. Each name is kept once for
/// the whole run: parts name a handful of sources.
///
/// ```
/// use otty_agents::SourceName;
///
/// let name = SourceName::new("codex app-server");
///
/// assert_eq!(name.as_str(), "codex app-server");
/// assert_eq!(name, SourceName::new("codex app-server"));
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
        let named = SourceId::Named(SourceName::new("Codex session files"));

        assert_ne!(named, SourceId::Plugin);
        assert_eq!(
            named,
            SourceId::Named(SourceName::new("Codex session files"))
        );
    }
}
