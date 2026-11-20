//! The stable ids of the sources of the data (spec Decisions 5 and 10).

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
    /// The sessions or the tokens read from the agent's own database, such
    /// as `kilo.db`, opened read-only.
    AgentDatabase,
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
    /// The money left on a DeepSeek account from
    /// `GET api.deepseek.com/user/balance`.
    DeepSeekBalance,
    /// The limit of an OpenRouter key and its usage from
    /// `GET openrouter.ai/api/v1/key`.
    OpenRouterKey,
    /// The money left on a Moonshot account from
    /// `GET api.moonshot.ai/v1/users/me/balance`.
    MoonshotBalance,
    /// The money left on a Kilo account from
    /// `GET api.kilo.ai/api/profile/balance`, with the Kilo login.
    KiloBalance,
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
}
