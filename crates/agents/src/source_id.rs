//! The stable ids of the sources of the data (spec Decisions 5 and 10).

/// Which source a piece of data came from, or failed in.
///
/// The id is stable and carries no text a person reads: the app turns it
/// into a translated name, such as "LiteLLM prices", where it says why a
/// read failed. Where a plugin part reads its numbers from is the part's
/// business (issue 19 of spec 2026-11-19-agent-plugins): OTTY knows only
/// which part answered.
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
    /// The list prices of the LiteLLM price table, fetched at most once a
    /// day.
    LiteLlmPrices,
    /// The copy of the LiteLLM price table kept from the last good fetch.
    KeptPrices,
    /// An agent part of a plugin: its sessions, tokens, statistics or the
    /// limits the agent itself shows (spec 2026-11-19-agent-plugins). The
    /// registry names the plugin.
    AgentPart,
    /// A provider part of a plugin: the numbers of an account, such as
    /// the limits of a subscription login, which it is asked for before
    /// the agent part (issue 15 of spec 2026-11-19-agent-plugins). The
    /// registry names the plugin.
    ProviderPart,
}
