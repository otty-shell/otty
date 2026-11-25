# 12 Anthropic plugin, Claude Code part: profiles, sessions, tokens, logins

Status: ready-for-agent
Blocked by: 07

## Why

Spec Decisions 10 and 11. The spike ported the token reader
(`git show adca59f3:crates/plugins/claude-code/src/lib.rs`, identical
totals on 1.46 GB); this issue ports the rest except the `/usage` probe.
Claude Code is the agent part `claude-code` of the official Anthropic
plugin; its provider part `anthropic` comes in issue 15.

## What to build

1. `plugins/anthropic/`: own cargo workspace, `README.md`, manifest
   (plugin id `anthropic`, name "Anthropic"; `[api] agent = "0.1"`; one
   agent part `claude-code` with `tested_version` `2.1.328` and its own
   permissions), the icon of `assets/svg/agent-claude-code.svg`, made-up
   fixtures. The plugin ends with `export!(Anthropic with agent)`.
2. Port from `crates/agents/src/claude_code/`: profiles (`~/.claude`,
   `CLAUDE_CONFIG_DIR`, extra `~/.claude*`), detect, version, sessions
   (`sessions.rs`, `session_file.rs`, `project_folder.rs`), spend places
   with markers `"usage"` and `"type":"assistant"` and `read-answers`
   (`spend_file.rs`, answer identity of message id and request id, the
   version and field names of a changed format), stats (`stats.rs`),
   commands (`agent.rs`).
3. Logins (`accounts.rs`, `config_files.rs`, `credentials.rs`,
   `plan.rs`): the Claude login of `.credentials.json` (provider
   `anthropic`, key place `/claudeAiOauth/accessToken`; plan from the
   file) and `oauthAccount` of `.claude.json`; `primaryApiKey`;
   `settings.json` `env` keys with `ANTHROPIC_BASE_URL` reported as the
   host the key is for (so a DeepSeek key behind
   `https://api.deepseek.com/anthropic` goes to DeepSeek).
4. Not here: `/usage` probe, embedding, deleting core code (issue 13);
   the provider part `anthropic` with the usage API and cost report
   (issue 15).

## Done when

- Tests first: the tests of the ported modules with the same made-up data
  and expectations; a `settings.json` key with a DeepSeek base URL is a
  login for host `api.deepseek.com`; no key text reaches the plugin.
- The checks of `AGENTS.md` pass; the plugin's own tests pass.

## Comments
