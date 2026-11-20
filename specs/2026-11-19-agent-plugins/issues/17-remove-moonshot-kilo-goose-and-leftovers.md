# 17 Remove Moonshot, Kilo, Goose and every leftover agent or provider name

Status: ready-for-agent
Blocked by: 10, 11, 12, 13, 14, 15, 16

## Why

Spec Decision 1: nothing in OTTY by agent or provider name. Moonshot,
Kilo, Kilo Gateway and Goose are not ported (Decision 10, owner
2026-11-20).

## What to build

1. Delete `account_numbers/moonshot.rs` and every Moonshot mention in
   code, tests, i18n and the Add account dialog.
2. Delete `account_numbers/kilo.rs` (the Kilo Gateway balance and the
   Kilo team header) and whatever Kilo code issue 14 left: `Provider::Kilo`
   uses, Kilo commands, the Kilo icon `assets/svg/agent-kilo.svg`, Kilo
   i18n and tests.
3. Delete the Goose code: the Goose keyring entry (`goose` / `secrets`),
   `KNOWN_ENTRIES`, `KEY_VARIABLES` and the rest of `keyring_entry.rs`,
   and any Goose detection or constant. The generic keyring host
   operation (issue 05) stays. The keyring Allow in Agents mode is asked
   only when an installed part declares a keyring entry.
4. No compatibility with earlier agents.json: Agents mode was never released.
5. Delete the built-in registry entries and whatever is left of
   `claude_code/`, `codex/`, `open_code/`, `account_numbers/`, and the
   agent icons in `assets/svg/` that only the built-in entries used (the
   plugins carry their own).
6. Remove from `otty-agents` the dependencies it no longer uses (`ureq`,
   `rusqlite`, `keyring`, `ruzstd`, `base64` if moved to the host);
   update its `README.md` (it now holds the generic agent logic and the
   plugin parts, not agents by name).
7. Search the whole tree outside `plugins/`, `specs/`, `docs/` and test
   data for agent and provider names and program names (`claude`,
   `codex`, `kilo`, `opencode`, `goose`, `anthropic`, `openai`,
   `deepseek`, `openrouter`, `moonshot`, `chatgpt`) and remove or
   justify each hit in the comments (for example a made-up test
   fixture).
8. Check the binary size and the dependency count against the spike
   numbers (the owner accepted about +16.5 MB) and note them.

## Done when

- Tests first: an account added by hand for a provider with no plugin
  is listed and not read; no keyring Allow is asked with only the
  official plugins installed.
- The search of step 7 has no unexplained hit.
- The checks of `AGENTS.md` pass.

## Comments
- 2026-11-20, issue 13: Claude-only leftovers after the core reader went: `ClaudeQuestion`, `Unavailable::{ClaudeAsks, ClaudeRateLimited, NoLoginToken, LoginMayNotReadUsage}`, `Unexpected::UsagePanelUnreadable`, `SourceId::{AnthropicUsageApi, ClaudeUsageCommand}` and their i18n texts; `otty_agents::fixtures::claude_code_stand_in` (test-only) and the Claude icon of the built-in Anthropic entry.
- 2026-11-20, issue 11: Codex-only leftovers after the core reader went: `SourceId::{CodexAppServer, LatestCodexSession, SessionFiles}` uses, `Unavailable::{ApiKeyLogin, NoLimitsInSessionFiles, NotStarted, NotWritten, Refused, NoAnswerInTime}` and `SourceError::ProgramStopped` where only Codex made them, and their i18n texts; `otty_agents::fixtures::codex_stand_in` (test-only), the `CodexLike` test agent, `profile_command` and `default_config_dir` (now only for test stand-ins), and the OpenAI icon of the built-in `openai` entry.
- 2026-11-20, issue 14: the core Kilo code is gone (the `open_code` Kilo variant, `account_numbers/kilo.rs`, the Kilo entries, `KILO_ICON` and `assets/svg/agent-kilo.svg`, `SourceId::KiloBalance` and its texts). Left: Kilo-named made-up test data in `otty-app-agents` `reads.rs` (`kilo()`, `kilo_login`) and doc comments that name Kilo (`agent.rs`, `spend.rs`, `money_estimate.rs`, `state.rs`, `usage_block.rs`, `usage_groups.rs`, `codex/`); the `#[expect(unused_imports)]` on `mod codex;` in `otty-agents` `lib.rs` goes with `codex/` (issue 11).
- 2026-11-20, issue 15: Anthropic/OpenAI leftovers after their core readers went: `SourceId::{AnthropicUsageApi, ChatGptUsageApi, AnthropicCostReport, OpenAiCosts}` (now only in app tests and the stand-in fixtures' limit sources), their i18n names and tips, and `Unexpected::{NoLimits, NotCostReport, TooManyPages, NotJsonObject}` if nothing else makes them. What is left of `account_numbers/` is Moonshot only (`moonshot.rs`, the `Reach::read` built-in path, `has_own_balance`, `ask_with_key`, `number_in`), together with `https.rs` (`UreqClient`), `Agent::account_key` and `AccountKey`, which only that path uses. `keyring_entry.rs` names `"anthropic"` and `"openai"` as text in `KEY_VARIABLES`. `known_agents()`/`Registry::built_in` now hold only the Moonshot provider entry.
