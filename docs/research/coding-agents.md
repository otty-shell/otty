# Coding agents on the user's machine: config, accounts, sessions, tokens

Checked on 2026-11-19. These tools change every week; every claim below links to the doc page or
source file it comes from. Source links point at the exact commit read that day. "Unknown" means
no primary source was found; "unconfirmed" means the fact is likely but only partly backed.

## 1. Summary

- Every agent keeps its state in one home folder that one environment variable can move
  (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `GEMINI_CLI_HOME`, `QWEN_HOME`, `KIMI_CODE_HOME`,
  `COPILOT_HOME`, `CLINE_DIR`, `CONTINUE_GLOBAL_DIR`, `GOOSE_PATH_ROOT`, `CURSOR_CONFIG_DIR`).
  The exceptions are OpenCode and Kilo (XDG folders plus a database variable), Crush (data in each
  project), Aider (files in each repository) and Amp (threads live on Amp's server).
- Every agent can continue a session from the command line, so OTTY's `launch(profile, session)`
  holds everywhere; only the flags differ.
- Most agents are multi-provider today. Even Claude Code and Codex can be pointed at another
  endpoint, and Codex, OpenCode, Kilo, Crush, Goose, Cline, Continue, Qwen Code and Kimi Code treat
  providers as configuration. A fixed `provider` string per agent no longer holds.
- Open-source multi-provider agents record the **provider id and model on each message** (OpenCode,
  Kilo, Cline, Crush) or on each session (Codex, Goose); many record **cost**. Almost none record
  **which account or key** paid. Only Codex writes an account id (`creator_account_id`).
- Limits, balances and bills belong to a login or key at a provider, not to an agent folder. One
  provider login is now shared by several agents (a ChatGPT plan can run Codex, OpenCode and Amp;
  a Copilot plan can run Copilot CLI and OpenCode).
- Closed agents that resell many vendors' models through their own account (Cursor, Copilot, Amp,
  Factory, Augment) keep little documented local data; their usage is readable only through the
  vendor.
- **Recommendation:** keep the agent profile (program + folder: sessions, projects, tokens) as the
  main unit, count tokens per (provider id, model) inside it, and add a separate, optional
  **provider account** that owns limits, balance and bill. Link a profile to its accounts per
  provider; do not try to attribute tokens to an account from records, because the records do not
  say which account paid.

## 2. Comparison table

"Prov. per msg" means the record names the provider (not only the model). "Sess." means the field
is per session, not per message.

| Agent | Config folder (move var) | Auth kinds | Multi-provider | Sessions on disk | Token kinds in records | Cost recorded | Prov. per msg | Limits / usage readable | Continue a session |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Claude Code | `~/.claude` + `~/.claude.json` (`CLAUDE_CONFIG_DIR`) | Claude plan OAuth, Console key or OAuth, Bedrock/Vertex/Foundry, gateway token | No; any Anthropic-compatible endpoint via `ANTHROPIC_BASE_URL` | JSONL `projects/<cwd-slug>/<id>.jsonl` | input, output, cache read, cache write (5m/1h), thinking (observed) | No | No | `/usage`; Anthropic cost report (API orgs) | `-c`, `-r <id\|name>` |
| Codex CLI | `~/.codex` (`CODEX_HOME`, `CODEX_SQLITE_HOME`) | ChatGPT OAuth, API key, provider env keys | Yes, one per session | JSONL `sessions/YYYY/MM/DD/rollout-*.jsonl` + SQLite index | input, cached input, cache write, output, reasoning, total | No | Sess. `model_provider` + account id | Rate limits in every token event; `/status`; OpenAI Costs API | `codex resume [id]`, `--last` |
| Gemini CLI | `~/.gemini` (`GEMINI_CLI_HOME`) | Google OAuth, Gemini API key, Vertex AI | No | JSONL `tmp/<project-id>/chats/session-*.jsonl` | input, output, cached, thoughts, tool, total | No | No | `/stats model` (tokens and quota) | `--resume [n\|id]` |
| OpenCode (V2) | XDG: `~/.config/opencode`, `~/.local/share/opencode` (`OPENCODE_DB`) | API keys, OAuth (ChatGPT, Copilot), env, cloud chains; several accounts per provider | Yes, per message | SQLite `opencode.db` | input, output, reasoning, cache read, cache write | Yes, per message and session | Yes (no account) | `opencode stats` (local) | `-c`, `-s <id>` |
| Kilo Code | XDG: `~/.local/share/kilo` (`KILO_DB`) | Kilo account (credits) or own keys, `auth.json` | Yes, per message | SQLite `kilo.db` (OpenCode fork) | as OpenCode | Yes | Yes | Kilo balance API | `-c`, `-s <id>` |
| Cline | Extension: editor storage; CLI `~/.cline` (`CLINE_DIR`, `CLINE_DATA_DIR`) | Cline account or own keys | Yes, can switch mid-task | Ext: JSON per task; CLI: SQLite `sessions.db` + message files | input, output, cache read, cache write | Yes | Yes | Unknown | CLI `--id <id>` |
| Aider | none; `.aider.conf.yml` in home/repo/cwd (`AIDER_*`) | API keys only | Yes (LiteLLM) | Markdown `.aider.chat.history.md` per repo; optional analytics JSONL | prompt, completion (analytics log only) | Analytics log only | No (model only) | `/tokens` (context only) | `--restore-chat-history` |
| Cursor CLI | `~/.cursor/cli-config.json` (`CURSOR_CONFIG_DIR`) | Cursor login, `CURSOR_API_KEY` | Cursor's models, one account | Unknown | Unknown | Unknown | Unknown | Unknown | `--resume [chatId]`, `--continue` |
| Copilot CLI | `~/.copilot` (`COPILOT_HOME`) | GitHub OAuth (keychain), BYOK `providers.json` | Yes (GitHub models + BYOK) | JSONL `session-state/<id>/events.jsonl` + SQLite | Unknown (format undocumented) | Unknown | Unknown | `/usage` (premium requests, AI credits) | `--resume [id\|name]`, `--continue` |
| Goose | `~/.config/goose`, `~/.local/share/goose` (`GOOSE_PATH_ROOT`) | Keys / OAuth per provider; OS keyring or secrets file | Yes, one per session | SQLite `sessions/sessions.db` | input, output, total, cache read, cache write (sess. totals) | Yes (sess.) | Sess. `provider_name` | Unknown | `goose session -r`, `--session-id` |
| Amp | `~/.config/amp/settings.json`; threads on ampcode.com | Amp accounts, `AMP_API_KEY`; can link ChatGPT/Claude plans | Amp routes models | Server | Unknown | Per thread, in web UI | Unknown | ampcode.com billing | `amp threads continue T-…` |
| Crush | `~/.config/crush`, `~/.local/share/crush`, per project `.crush/` (`CRUSH_GLOBAL_CONFIG`, `CRUSH_GLOBAL_DATA`, `CRUSH_DATA_DIR`) | API keys, Charm Hyper | Yes | SQLite `.crush/crush.db` per project | prompt, completion (sess.) | Yes (sess.) | Yes | Unknown | `-C`, `-s <id>` |
| Qwen Code | `~/.qwen` (`QWEN_HOME`, `QWEN_RUNTIME_DIR`) | Qwen OAuth (plan), ModelStudio plans, keys | Yes | JSONL `projects/<cwd-slug>/chats/` | Gemini usage metadata (prompt, output, cached, thoughts, total) | No | No (model only) | Unconfirmed | `--continue`, `--resume <id>` |
| Kimi Code | `~/.kimi-code` (`KIMI_CODE_HOME`) | Kimi Code OAuth, API keys | Yes | JSONL `sessions/<workspace>/<id>/agents/main/wire.jsonl` | input, output, cache read, cache creation (type; per-record unconfirmed) | Unknown | Unknown | `/usage` (tokens, quota) | `-c`, `-S [id]` |
| Continue (`cn`, IDE) | `~/.continue` (`CONTINUE_GLOBAL_DIR`) | Continue account or keys in `config.yaml` | Yes | JSON `sessions/<id>.json`; `dev_data/0.2.0/tokensGenerated.jsonl` | prompt, generated (events) | Unknown | Yes, in events | Unknown | `cn --resume` |
| Factory Droid | `~/.factory/settings.json` | Factory login, `FACTORY_API_KEY`, BYOK in settings | Yes (Factory + BYOK) | Unknown | Unknown | Unknown | Unknown | `/cost` | `droid --resume [id]` |
| Auggie (Augment) | `~/.augment/settings.json` | Augment login, `AUGMENT_SESSION_AUTH` | Augment's models, one account | Unknown | Unknown | Unknown | Unknown | Unknown | `-c`, `-r` |

### The list and why

Stars from the GitHub API on 2026-11-19 ([gh-api]): OpenCode 227.8k, Claude Code 156.2k (issue
tracker, code not published), Codex 135.8k, Gemini CLI 108.9k, Cline 71.8k, Goose 56.8k, Aider
50.2k, Continue 36.5k, Qwen Code 30.8k, Crush 29.1k, Kilo Code 28.1k, Copilot CLI 11.4k (issue
tracker), Kimi Code 8.3k. Cursor, Amp, Factory and Augment publish no code; they are included as
official products of large vendors with their own CLI.

Dropped:

- **Roo Code**: repository archived (`archived: true`, last push 2026-05-15). Its storage was the
  Cline extension layout, covered under Cline.
- **Kimi CLI (Python)**: archived; the README sends users to Kimi Code, which is included.
- **Warp agent**: part of the Warp terminal itself, not a program OTTY could run in a tab.
- **Zed agent**: built into the Zed editor; its threads live in Zed's own database. Zed also runs
  external agents (Claude Code, Codex, Gemini) over ACP, which are covered here.

## 3. Patterns

### A. One vendor, one subscription, local JSONL logs

Claude Code, Codex, Gemini CLI, Qwen Code, Kimi Code.

- One home folder, one moving variable, one login per folder. Several logins on one machine means
  several folders; Claude Code documents exactly this ([cc-auth] "Log in with multiple accounts").
- Sessions are append-only JSONL files keyed by project folder; each assistant record has the model
  and token counts. No cost field, no provider field (Codex is the exception: provider and account
  id once per session).
- Subscription limits are readable: from the agent's own command (`/usage`, `/status`,
  `/stats model`) or by calling the vendor endpoint with the OAuth token the agent stored. Codex
  writes the limits into its own log.
- All of them now also accept other providers: Claude Code through `ANTHROPIC_BASE_URL`
  ([cc-env]), Codex through `model_providers` ([cx-adv]), Qwen Code and Kimi Code through built-in
  provider lists ([qw-auth], [km-prov]). When that happens the folder's tokens are no longer the
  vendor's tokens, and nothing in Claude Code's records shows it.
- **OTTY can read:** sessions, projects, tokens per model per day, limits. **Cannot read** from
  records: which endpoint or account served a Claude Code message.

### B. Multi-provider, bring your own key, open source

OpenCode, Kilo Code, Crush, Goose, Cline, Continue, Aider.

- Providers and models are configuration; keys come from a saved store, environment variables or
  cloud credential chains. OpenCode V2 keeps **several saved accounts per provider** and one active
  ([oc-cliprov]).
- Records name provider and model, and most store cost in USD computed by the agent from a model
  catalog (models.dev for OpenCode, Catwalk for Crush).
- No record names the account or key, so tokens split by provider but not by account.
- **OTTY can read:** sessions, projects, tokens per (provider, model), the agent's own cost.
  **Cannot read:** limits (there are none for plain keys) or the real bill, except through the
  provider's own API (see pattern E).

### C. Vendor account that resells many vendors' models

Cursor CLI, Copilot CLI, Amp, Factory Droid, Augment Auggie; Kilo, Cline, OpenCode (Zen/Go) and
Crush (Hyper) also offer this as one option.

- One login pays for models from Anthropic, OpenAI, Google and others. The "provider account" is
  the agent vendor's account, not the model maker's.
- Local session formats are undocumented (Cursor, Factory, Augment) or the threads live on the
  vendor's server (Amp). Copilot documents the folder but not the event format ([cp-dir]).
- Usage is shown by the agent (`/usage`, `/cost`) or the vendor's dashboard; no documented local
  file or public endpoint for an individual's balance was found, except Kilo's balance endpoint in
  its open source ([kc-profile]).
- **OTTY can read:** sessions list and resume (Copilot, Cursor, Factory, Auggie have flags).
  **Cannot read** reliably: tokens, money, limits.

### D. Where the data lives

- **JSONL per session:** Claude Code, Codex, Gemini CLI, Qwen Code, Kimi Code, Copilot CLI. Easy to
  read incrementally; formats are internal and change between versions ([cc-sess] says so for
  Claude Code).
- **SQLite:** OpenCode, Kilo, Crush, Goose, Cline CLI, Copilot (`session-store.db`), and Codex as an
  index next to the JSONL. Reading needs a read-only open that survives the agent writing at the
  same time (WAL files are present in Kilo's folder on this machine).
- **Editor storage:** Cline extension (VS Code global storage of `saoudrizwan.claude-dev`). The path
  depends on the editor and its profile.
- **In the project folder:** Crush (`.crush/crush.db`), Aider (`.aider.chat.history.md`). There is no
  single folder to scan; OTTY would have to know the projects first.
- **Server only:** Amp threads.

### E. Provider bills and balances readable with a key

Independent of the agent, several providers expose spend or balance:

- Anthropic: organization cost report, admin key (OTTY uses it already).
- OpenAI: `GET /v1/organization/costs`, admin key ([oa-costs]).
- OpenRouter: `GET /api/v1/key` returns `limit_remaining`, with the normal key ([or-limits]).
- DeepSeek: `GET /user/balance`, normal key ([ds-balance]).
- Moonshot: `GET /v1/users/me/balance`, normal key ([ms-balance]).
- Kilo: `/api/profile/balance` with the Kilo login ([kc-profile]).

These belong to a key or login, not to any agent.

## 4. What this means for OTTY's model

### The current `Agent` trait against the facts

| Part of `Agent` / `AgentDescriptor` | Holds? | Why |
| --- | --- | --- |
| `program`, `launch(profile, session)`, `login(profile)` | Holds | Every agent has a program, a "continue last" and a "continue by id" flag, and most have a login command. Aider has no session ids; Amp continues server threads. |
| `config_dir_name` + `config_dir_variable` | Mostly | 10 of 17 have one folder and a documented variable; Factory and Auggie have one folder but no documented variable. OpenCode/Kilo use XDG roots (moving them moves other apps too) and a separate DB variable; Crush and Aider keep data per project; Amp keeps it on a server. A profile needs "a set of variables that select it", not always one folder. |
| One profile = one account | Breaks | OpenCode V2 and Amp keep several accounts in one folder and switch the active one; a multi-provider folder holds one account per provider; an env-var key outside the folder changes the account without changing the folder (Claude Code `ANTHROPIC_API_KEY`). |
| `provider: &'static str` | Breaks | Codex, OpenCode, Kilo, Crush, Goose, Cline, Continue, Qwen, Kimi pick providers from config; Claude Code can be sent to another endpoint. |
| `session_sources`, `stats_source` | Holds | Sessions with cwd exist in every open agent; formats differ per agent, which is what the trait already hides. |
| `spend_counter` (tokens) | Holds, needs a provider column | Tokens per model exist almost everywhere. Token kinds differ (Gemini: thoughts, tool; Codex: reasoning, cache write; Claude: 5m/1h cache writes). Records give provider in B-type agents and Codex. |
| `limit_sources` | Half | The ways to read limits are agent-specific (run `/usage`, read the agent's token file), but the limits themselves belong to the provider login. Two agents on one ChatGPT plan would show the same limits twice. |
| `bill_source`, `has_bill` | Breaks | A bill is a property of the provider and the key (see pattern E), not of the agent. OpenCode with an Anthropic key and Claude Code with the same key would each need the Anthropic bill reader. |

### Can tokens be attributed to an account?

| Agent | Provider in records | Account in records |
| --- | --- | --- |
| Claude Code | No (model name only) | No |
| Codex | Yes, per session (`model_provider`) | Yes, `creator_account_id` per session |
| Gemini, Qwen, Kimi | No (model only) | No |
| OpenCode, Kilo, Cline, Crush | Yes, per message | No |
| Goose | Yes, per session | No |
| Continue | Yes, per usage event | No |
| Aider | No | No |
| Cursor, Copilot, Amp, Factory, Augment | Unknown | Unknown |

So tokens can be split by provider for most open agents, but only Codex can tie them to an account.
Everywhere else the account is "whatever login the profile has now", which is wrong for history
when the user changed keys or accounts.

### Option 1: agent-centric with per-model accounting

Keep one profile = one folder = one account; add the provider to each token record and show tokens
per (provider, model).

- Good: small change; fits Claude Code and Codex as they are; no new UI concept.
- Bad: a multi-provider profile has no single "account" for limits or a bill, so those parts stay
  empty or wrong. A ChatGPT plan used by Codex and OpenCode shows its limits twice and its usage
  split. The bill reader stays tied to an agent though it is the provider's.

### Option 2: agent profile + provider account

A **profile** is a program plus the variables that select its state: sessions, projects, tokens per
(provider, model), launch and resume. A **provider account** is a login or key at a provider: limits,
balance, bill, price list. A profile links to zero or more accounts, at most one per provider.

- Good: matches where each number really comes from. Limits shown once per plan. Bill readers
  belong to providers and work for any agent that uses them. Fits gateway vendors (Cursor, Amp,
  Kilo) as "providers" too.
- Bad: more concepts in the UI; account totals of tokens are only an estimate (sum of linked
  profiles' tokens for that provider), because records do not name the account.
- Account discovery stays agent-specific: Claude Code's `.credentials.json`, Codex's `auth.json`,
  OpenCode's credential table are where the logins are. The agent finds accounts in a profile; the
  provider reads them.

### Recommendation

Choose option 2, in this limited form:

1. Keep the agent profile as the main unit and keep the `Agent` trait for what is truly per agent:
   program, launch/resume, login, sessions, stats, token counting, and finding the credentials in
   its folder.
2. Make every token record carry `(provider id as recorded or known, model)`. For Claude Code,
   Gemini, Qwen and Kimi, which do not record it, use the provider the profile is configured for at
   read time and say so.
3. Move bills, balances and the subscription-limit endpoints to a provider side (Anthropic, OpenAI
   / ChatGPT, Google, OpenRouter, DeepSeek, Moonshot, Kilo...). A provider account is created from a
   credential the agent found, or added by the user (an admin key for a bill).
4. Do not attribute tokens to accounts beyond the profile → account link. Show account-level token
   totals only as "from the profiles linked to this account".
5. Replace `descriptor.provider` with "providers this agent can use" (or a default provider), and
   let a profile be selected by several variables, not only one folder.

Reasons: the facts that drive the model are (a) limits and bills are per login, (b) one login is
shared by several agents, (c) records name provider and model but not the account. Option 1 cannot
express (a) and (b); full account attribution cannot be built on (c). Option 2 keeps tokens where
the data is (the profile) and limits and money where they are owed (the account).

## 5. Per-agent details

### Claude Code (Anthropic)

- **License:** proprietary; the GitHub repo has no license and holds issues and plugins, not the
  CLI code ([gh-api]).
- **Config:** `~/.claude/settings.json`, `~/.claude.json` (account, MCP, trust); `CLAUDE_CONFIG_DIR`
  moves both ([cc-settings], [cc-dir]).
- **Auth:** Claude Pro/Max/Ultra/Team/Enterprise OAuth; Console login (OAuth "profile" or created
  key); Bedrock, Vertex (Google Cloud's Agent Platform), Foundry; gateway token;
  `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_API_KEY`, `apiKeyHelper`, `CLAUDE_CODE_OAUTH_TOKEN`, with a
  documented precedence. Linux/Windows: `.credentials.json` (0600); macOS: Keychain keyed to the
  config dir. Keyless Console sign-ins are stored outside the config dir, so separate folders do not
  keep them apart ([cc-auth]).
- **Providers:** one API (Anthropic Messages) at several hosts; `ANTHROPIC_BASE_URL` points it at a
  gateway or any compatible endpoint; with only that variable set, a saved claude.ai login stays
  the credential ([cc-env], [cc-gw]).
- **Sessions:** `~/.claude/projects/<cwd with non-alphanumerics as ->/<session-id>.jsonl`, names
  over 200 chars truncated plus a hash; subagents under `<session>/subagents/`; deleted after
  `cleanupPeriodDays` (default 30). The entry format "is internal ... and changes between versions"
  ([cc-sess], [cc-dir]).
- **Record (observed locally in v2.1.327, not documented):** top level `cwd`, `gitBranch`,
  `sessionId`, `timestamp`, `version`, `requestId`, `requestedModel`, `isSidechain`; `message.model`;
  `message.usage` with `input_tokens`, `output_tokens`, `cache_creation_input_tokens`,
  `cache_read_input_tokens`, `cache_creation.ephemeral_5m_input_tokens` / `ephemeral_1h_input_tokens`,
  `output_tokens_details.thinking_tokens`, `service_tier`, `inference_geo`. No provider, no base
  URL, no account, no cost.
- **Usage/limits:** `/usage` shows session cost (computed locally at list price), and for plans the
  usage bars and credits; `stats-cache.json` holds the totals `/usage` shows ([cc-costs], [cc-dir]).
  Org bills: Console cost pages, Claude Code Analytics API, Enterprise Analytics API ([cc-costs]).
  The OAuth usage endpoint OTTY uses is not documented.
- **Resume:** `claude -c` (last in this folder), `claude -r <id|name|path>`, `--fork-session`,
  `--session-id <uuid>` ([cc-cli]).

### Codex CLI (OpenAI)

- **License:** Apache-2.0, Rust ([gh-api]).
- **Config:** `~/.codex/config.toml`, `auth.json`; `CODEX_HOME`; SQLite files under
  `CODEX_SQLITE_HOME` (default the same folder; `state_5.sqlite`, `logs_2.sqlite` etc. seen locally)
  ([cx-storage], [cx-state]).
- **Auth:** ChatGPT login or API key; `cli_auth_credentials_store = file | keyring | auto |
  ephemeral`, file by default; keyring service "Codex Auth" ([cx-types], [cx-storage], [cx-auth]).
- **Providers:** `[model_providers.<id>]` with `base_url`, `env_key`, `wire_api`, `auth` command,
  `gateway_oauth`, `requires_openai_auth`; built-in Bedrock; `model_provider` picks one
  ([cx-adv], [cx-ref]).
- **Sessions:** `~/.codex/sessions/YYYY/MM/DD/rollout-<timestamp>-<id>.jsonl`,
  `archived_sessions/` ([cx-recorder]); thread index in SQLite with `model_provider`, `cwd`,
  `tokens_used` ([cx-thread]).
- **Record:** `session_meta`: `id`, `timestamp`, `cwd`, `originator`, `cli_version`,
  `model_provider`, `creator_account_id`, parent/fork ids ([cx-proto-meta]); `turn_context`: `cwd`,
  `model` ([cx-proto-turn]); `token_count` event: `TokenUsage` = `input_tokens`,
  `cached_input_tokens`, `cache_write_input_tokens`, `output_tokens`, `reasoning_output_tokens`,
  `total_tokens` ([cx-proto-usage]) and `RateLimitSnapshot` = primary/secondary windows, credits,
  spend limit, `plan_type` ([cx-proto-rl]). No cost.
- **Usage/limits:** limits in the log, `/status`, ChatGPT usage endpoint (see OTTY's Codex page);
  OpenAI orgs: Costs API with an admin key ([oa-costs]).
- **Resume:** `codex resume [id]`, `codex resume --last` (scoped to the cwd unless `--all`),
  `codex fork` ([cx-cli]).

### Gemini CLI (Google)

- **License:** Apache-2.0 ([gh-api]).
- **Config:** `~/.gemini/settings.json`; `GEMINI_CLI_HOME` replaces the home folder, so all of
  `~/.gemini` moves ([gm-paths], [gm-storage]).
- **Auth:** Sign in with Google (free tier, Code Assist, AI Pro/Ultra), `GEMINI_API_KEY`, Vertex AI
  ([gm-auth]). OAuth tokens in `~/.gemini/oauth_creds.json`, or an encrypted file when
  `GEMINI_FORCE_ENCRYPTED_FILE_STORAGE=true` ([gm-storage] L22, [gm-oauth], [gm-tokenstore]).
- **Providers:** Google only.
- **Sessions:** JSONL at `<runtime dir>/tmp/<project-id>/chats/session-<time>-<short id>.jsonl`;
  subagents nested under the parent id; old `.json` files are converted on resume. The project id is
  now a short slug kept in `projects.json` (older versions used a hash, which the docs still say)
  ([gm-crs], [gm-storage], [gm-sess]).
- **Record:** session `sessionId`, `projectHash`, `startTime`, `lastUpdated`, `directories`; Gemini
  messages carry `model` and `tokens` = `input`, `output`, `cached`, `thoughts`, `tool`, `total`
  ([gm-crs] L1168, L1244). No cost, no provider.
- **Usage/limits:** `/stats model` shows tokens and quota; `/stats heatmap` daily tokens
  ([gm-cmds]); quotas per plan ([gm-quota]).
- **Resume:** `gemini --resume` (latest), `--resume <n|uuid>`, `--list-sessions`,
  `--delete-session` ([gm-sess]).

### OpenCode (Anomaly, formerly SST)

- **License:** MIT ([gh-api]). Current docs describe "V2"; V1 docs remain ([oc-migrate]).
- **Config:** `~/.config/opencode/opencode.json(c)` and project files ([oc-config]); data in XDG
  folders ([oc-global]); database `~/.local/share/opencode/opencode.db`, moved by `OPENCODE_DB`
  ([oc-trouble]).
- **Auth:** `/connect` or `opencode auth`: API key, OAuth (ChatGPT Plus/Pro, Copilot), command,
  environment; several saved accounts per provider, newest active, `opencode auth switch`; saved
  keys and tokens in the server's SQLite; V1 used `~/.local/share/opencode/auth.json` and V2
  imports it ([oc-cliprov], [oc-prov], [oc-v1cli]).
- **Providers:** many (models.dev catalog), switch per message ([oc-v1cli] L142).
- **Sessions:** tables `session` (project, cwd via project, `cost`, `tokens_input`,
  `tokens_output`, `tokens_reasoning`, `tokens_cache_read`, `tokens_cache_write`, `model` =
  `{id, providerID, variant}`), `session_message`, part table ([oc-sql]).
- **Record:** assistant message: `model` (id, providerID), `cost` (USD), `tokens` = `input`,
  `output`, `reasoning`, `cache.read`, `cache.write`, `time.created` ([oc-msg], [oc-tokens]). No
  account id.
- **Usage/limits:** `opencode stats [--days N] [--models]` from local data ([oc-v1cli] L456).
  Provider limits: unknown.
- **Resume:** `opencode -c`, `-s <id>`, `--fork` ([oc-v1cli]).

### Kilo Code

- **License:** MIT; a fork of OpenCode (`.opencode-version` = v1.18.26) plus VS Code and JetBrains
  extensions ([kc-tree], [gh-api]).
- **Config/data:** XDG folders named `kilo` ([kc-global]); `auth.json` in the data folder
  ([kc-auth]); database `kilo.db`, moved by `KILO_DB` ([kc-db]).
- **Auth/providers:** Kilo account with 500+ models at provider price, or own keys; switch models
  mid-task (README).
- **Record:** OpenCode V1 message: `providerID`, `modelID`, cost, tokens ([kc-msg]).
- **Usage:** Kilo balance via `/api/profile/balance` ([kc-profile]).
- **Resume:** `kilo -c`, `-s <id>`, `--fork`, `--cloud-fork` ([kc-cli]).

### Cline

- **License:** Apache-2.0 ([gh-api]).
- **Extension storage:** editor global storage (VS Code: extension `saoudrizwan.claude-dev`): per
  task `api_conversation_history.json`, `ui_messages.json`, `task_metadata.json` ([cl-disk]).
  History item: `id`, `ts`, `task`, `tokensIn`, `tokensOut`, `cacheWrites`, `cacheReads`,
  `totalCost`, `cwdOnTaskInitialization`, `modelId`, `apiProvider` ([cl-hist]). Key storage in the
  extension: unconfirmed (editor secret storage).
- **CLI storage:** `~/.cline` (`CLINE_DIR`, `--config`), data `~/.cline/data` (`CLINE_DATA_DIR`)
  ([cl-cli]); SQLite `data/db/sessions.db` (`CLINE_DB_DATA_DIR`) with `provider`, `model`, `cwd`,
  `workspace_root`, `started_at`, `messages_path` ([cl-paths], [cl-sqldb]). Provider keys in
  `data/settings/providers.json` (unconfirmed, seen only in test fixtures).
- **Record:** message `modelInfo.{id, provider}`, `metrics.{inputTokens, outputTokens,
  cacheReadTokens, cacheWriteTokens, cost}`, `ts` ([cl-msgs]).
- **Resume:** CLI `--id <session-id>`, `cline history` ([cl-cli]).

### Aider

- **License:** Apache-2.0; last commit 2026-05-22 ([gh-api]).
- **Config:** `.aider.conf.yml` in home, git root, cwd; every option also as `AIDER_*`
  ([ad-conf], [ad-args]). No home data folder.
- **Auth/providers:** API keys only: `--api-key provider=key`, env, `.env`; any LiteLLM provider
  ([ad-args] L97-108).
- **Sessions:** `.aider.chat.history.md` and `.aider.input.history` in the git root
  ([ad-args] L271-276); no session ids; `--restore-chat-history`.
- **Tokens:** only with `--analytics-log FILE`: `message_send` events with `main_model`,
  `prompt_tokens`, `completion_tokens`, `total_tokens`, `cost`, `total_cost` ([ad-coder]).
- **Usage:** `/tokens` reports the current context ([ad-cmds]).

### Cursor CLI

- **License:** proprietary.
- **Config:** `~/.cursor/cli-config.json`; `CURSOR_CONFIG_DIR`, or `$XDG_CONFIG_HOME/cursor/`
  ([cu-config]).
- **Auth:** `agent login` (browser), "credentials are securely stored locally" (where: unknown);
  `CURSOR_API_KEY` or `--api-key` ([cu-auth]).
- **Sessions, records, usage:** not documented; unknown.
- **Resume:** `--resume [chatId]`, `--continue`, `agent resume`, `agent ls` ([cu-params]).

### GitHub Copilot CLI

- **License:** proprietary (repo holds issues and changelog) ([gh-api]).
- **Config:** `~/.copilot` (`COPILOT_HOME`; `--config-dir` is legacy): `settings.json`, `config.json`
  (auth and internal state), `providers.json` (BYOK, plain text keys; `COPILOT_PROVIDERS_CONFIG`),
  `session-state/`, `session-store.db`, `logs/` ([cp-dir]).
- **Auth:** GitHub device login; token in the system keychain, or plain text after consent
  ([cp-log] 1.0.x entries on keychain and plaintext consent).
- **Providers:** GitHub-hosted models plus BYOK through `providers.json` or `COPILOT_PROVIDER_*`
  ([cp-dir], [cp-log]).
- **Sessions:** `session-state/<id>/events.jsonl` plus plans and checkpoints ([cp-dir]); event
  fields undocumented.
- **Usage:** `/usage` shows premium requests and AI credits ([cp-log]).
- **Resume:** `copilot --resume [id|name]`, `--continue` ([cp-dir], [cp-log]).

### Goose (AAIF, formerly Block)

- **License:** Apache-2.0; repo moved to `aaif-goose/goose` ([gh-api]).
- **Config:** platform folders for app "goose" (Linux `~/.config/goose`, `~/.local/share/goose`);
  `GOOSE_PATH_ROOT` puts config/data/state under one absolute root ([gs-paths]).
- **Auth:** secrets in the OS keyring (service "goose"), or a secrets file when
  `GOOSE_DISABLE_KEYRING` is set ([gs-base]).
- **Providers:** many; one provider and model config per session.
- **Sessions:** SQLite `sessions/sessions.db` in the data folder ([gs-sm] L34, L74).
- **Record (session):** `working_dir`, `provider_name`, `model_config`, usage and accumulated usage
  (`input`, `output`, `total`, `cache_read`, `cache_write`), `accumulated_cost` ([gs-sm] L79-140).
- **Resume:** `goose session -r`, `--session-id <id>`, `-n <name>`, `--fork` ([gs-cli]).

### Amp (Sourcegraph / Amp Inc.)

- **License:** proprietary.
- **Config:** `~/.config/amp/settings.json(c)`, workspace `.amp/settings.json` ([amp-settings]).
- **Auth:** `amp login` adds an account per Amp server; `amp account list|switch`; `AMP_API_KEY`
  wins over saved accounts ([amp-cli]). Storage location of accounts: unknown.
- **Providers/billing:** Amp-served models paid from Amp credits; paid tiers include usage; users
  can link a ChatGPT or Claude subscription and teams share BYOK ([amp-pricing]).
- **Sessions:** threads on ampcode.com; the thread menu shows "Cost & Usage" ([amp-threads]).
- **Resume:** `amp threads continue T-…` ([amp-threads]).

### Crush (Charm)

- **License:** FSL-1.1-MIT (source available, becomes MIT later) ([cr-license]).
- **Config/data:** global `~/.config/crush/crush.json` and state `~/.local/share/crush/crush.json`
  (`CRUSH_GLOBAL_CONFIG`, `CRUSH_GLOBAL_DATA`); per-project `.crush/` (`CRUSH_DATA_DIR`, `-D`)
  ([cr-readme], [cr-load]).
- **Auth/providers:** API keys by env or config; Catwalk model catalog; Charm Hyper as own provider
  ([cr-readme]). Where entered keys are saved: unconfirmed.
- **Sessions:** SQLite `.crush/crush.db` ([cr-connect]); `sessions` has `prompt_tokens`,
  `completion_tokens`, `cost` ([cr-init]); `messages` has `model`, `provider`, `model_name`
  ([cr-mig-prov], [cr-mig-model]); stats grouped by model and provider ([cr-stats]).
- **Resume:** `crush -C` / `--continue`, `-s <id>` ([cr-root]).

### Qwen Code (Alibaba)

- **License:** Apache-2.0; fork of Gemini CLI ([gh-api]).
- **Config:** `~/.qwen/settings.json`; `QWEN_HOME`; runtime data `QWEN_RUNTIME_DIR` ([qw-storage]).
- **Auth:** Qwen OAuth with a Plus/Pro plan, Alibaba ModelStudio Coding Plan / Token Plan / API key,
  QwenCloud, third-party keys (DeepSeek, Kimi, OpenRouter...) ([qw-auth]); OAuth file
  `oauth_creds.json` ([qw-storage] L16).
- **Sessions:** JSONL in `<runtime>/projects/<sanitized cwd>/chats/` ([qw-storage] L619, [qw-crs]
  L1512; the class comment still says `tmp/<project_id>/chats`).
- **Record:** `sessionId`, `timestamp`, `cwd`, `usageMetadata` (Gemini format), `model`
  ([qw-crs] L539-562). Provider: not recorded (unconfirmed).
- **Resume:** `qwen --continue`, `qwen --resume <id>`, `/resume` ([qw-headless], [qw-cmds]).

### Kimi Code (Moonshot AI)

- **License:** MIT, TypeScript ([gh-api]).
- **Config:** `~/.kimi-code/config.toml`; `KIMI_CODE_HOME` moves everything ([km-data]).
- **Auth:** `/login` (Kimi Code managed OAuth), files in `credentials/<name>.json` (0600); API keys
  in `config.toml` (`api_key` or `api_key_env`) ([km-data], [km-prov]).
- **Providers:** `kimi`, `anthropic`, OpenAI-compatible; several at once ([km-prov]).
- **Sessions:** `sessions/<workspaceId>/<sessionId>/` with `state.json` and
  `agents/main/wire.jsonl`; `session_index.jsonl` maps ids to work dirs ([km-data]).
- **Record:** token type `inputOther`, `output`, `inputCacheRead`, `inputCacheCreation`
  ([km-usage]); which wire records carry it and the model: unconfirmed.
- **Usage:** `/usage` shows tokens, context and quota ([km-cmd]).
- **Resume:** `kimi -c` / `--continue`, `kimi -S` / `--session [id]` ([km-cmd]).

### Continue (`cn` CLI and IDE extensions)

- **License:** Apache-2.0 ([gh-api]).
- **Config:** `~/.continue/config.yaml`, `--config`; `CONTINUE_GLOBAL_DIR` ([co-paths], [co-cfg]).
- **Auth:** Continue account (hub) or keys in config ([co-quick]).
- **Sessions:** `~/.continue/sessions/<id>.json` and `sessions.json` ([co-paths], [co-cli]); CLI
  session usage `promptTokens`, `completionTokens`, `promptTokensDetails.{cachedTokens,
  cacheWriteTokens}` ([co-cli]).
- **Tokens log:** `dev_data/0.2.0/tokensGenerated.jsonl` events with `timestamp`, `model`,
  `provider`, `promptTokens`, `generatedTokens` ([co-tok], [co-paths] L229).
- **Resume:** `cn --resume`, `/resume` ([co-tui]).

### Factory Droid

- **License:** proprietary.
- **Config:** `~/.factory/settings.json`, `settings.local.json` ([fa-settings]).
- **Auth/providers:** Factory login or `FACTORY_API_KEY`; BYOK `customModels` in
  `settings.json`, keys stay local ([fa-cli], [fa-byok]).
- **Sessions:** local format unknown; sessions also exposed by Factory's API ([fa-llms]).
- **Usage:** `/cost` "Show usage statistics" ([fa-cli] L912).
- **Resume:** `droid --resume [id]` (`-r`), `droid exec -s <id>` ([fa-cli]).

### Auggie (Augment Code)

- **License:** proprietary.
- **Config:** `~/.augment/settings.json` ([au-settings]).
- **Auth:** `auggie auth login`; local token removed by `auggie auth logout`; `auggie token print`;
  `AUGMENT_SESSION_AUTH` ([au-auth]). File location of the token: unknown.
- **Sessions/usage:** location and format unknown; `auggie session list` ([au-ref]).
- **Resume:** `auggie -c` / `--continue`, `auggie -r` / `--resume` ([au-ref]).

## 6. Sources

All read on 2026-11-19.

GitHub and providers:

- `gh-api`: GitHub REST `GET /repos/{owner}/{repo}` for each repository (stars, license, archived).
- `oa-costs`: https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage/methods/costs
- `or-limits`: https://openrouter.ai/docs/api/reference/limits.md
- `ds-balance`: https://api-docs.deepseek.com/api/get-user-balance
- `ms-balance`: https://platform.moonshot.ai/docs/api/balance

Claude Code:

- `cc-settings`: https://code.claude.com/docs/en/settings
- `cc-auth`: https://code.claude.com/docs/en/authentication
- `cc-dir`: https://code.claude.com/docs/en/claude-directory
- `cc-costs`: https://code.claude.com/docs/en/costs
- `cc-cli`: https://code.claude.com/docs/en/cli-reference
- `cc-sess`: https://code.claude.com/docs/en/sessions#where-transcripts-are-stored
- `cc-env`: https://code.claude.com/docs/en/env-vars
- `cc-gw`: https://code.claude.com/docs/en/llm-gateway

Codex (commit `2d58165`):

- `cx-proto-usage`: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/protocol/src/protocol.rs#L2599
- `cx-proto-rl`: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/protocol/src/protocol.rs#L2700
- `cx-proto-meta`: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/protocol/src/protocol.rs#L3543
- `cx-proto-turn`: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/protocol/src/protocol.rs#L3722
- `cx-recorder`: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/rollout/src/recorder.rs
- `cx-storage`: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/login/src/auth/storage.rs#L307
- `cx-types`: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/config/src/types.rs#L139
- `cx-state`: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/state/src/lib.rs#L149
- `cx-thread`: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/state/src/model/thread_metadata.rs#L158
- `cx-auth`: https://developers.openai.com/codex/auth
- `cx-adv`: https://developers.openai.com/codex/config-advanced
- `cx-ref`: https://developers.openai.com/codex/config-reference
- `cx-cli`: https://developers.openai.com/codex/cli/reference

Gemini CLI (commit `615d6bc`):

- `gm-paths`: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/packages/core/src/utils/paths.ts#L22
- `gm-storage`: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/packages/core/src/config/storage.ts
- `gm-crs`: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/packages/core/src/services/chatRecordingService.ts#L796
- `gm-oauth`: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/packages/core/src/code_assist/oauth2.ts#L112
- `gm-tokenstore`: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/packages/core/src/mcp/token-storage/index.ts#L13
- `gm-auth`: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/docs/get-started/authentication.mdx
- `gm-sess`: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/docs/cli/session-management.md
- `gm-cmds`: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/docs/reference/commands.md#stats
- `gm-quota`: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/docs/resources/quota-and-pricing.md

OpenCode (commit `88ca792`, branch `dev`):

- `oc-global`: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/packages/util/src/global.ts
- `oc-sql`: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/packages/core/src/session/sql.ts#L28
- `oc-msg`: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/packages/schema/src/session-message.ts#L221
- `oc-tokens`: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/packages/schema/src/token-usage.ts
- `oc-trouble`: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/content/troubleshooting.mdx#L203
- `oc-config`: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/content/config.mdx
- `oc-prov`: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/content/providers.mdx
- `oc-cliprov`: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/content/cli/providers.mdx#L112
- `oc-migrate`: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/content/migrate-v1.mdx
- `oc-v1cli`: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/v1/content/cli.mdx

Kilo Code (commit `00a5d30`):

- `kc-tree`: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/.opencode-version
- `kc-global`: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/core/src/global.ts#L12
- `kc-auth`: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/opencode/src/auth/index.ts#L11
- `kc-db`: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/opencode/src/storage/db.ts#L35
- `kc-msg`: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/opencode/src/session/message-v2.ts
- `kc-profile`: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/kilo-gateway/src/api/profile.ts#L81
- `kc-cli`: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/kilo-docs/pages/code-with-ai/platforms/cli-reference.md

Cline (commit `eff43e1`):

- `cl-disk`: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/apps/vscode/src/core/storage/disk.ts#L17
- `cl-hist`: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/apps/vscode/src/shared/HistoryItem.ts
- `cl-cli`: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/docs/cli/cli-reference.mdx
- `cl-paths`: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/sdk/packages/shared/src/storage/paths.ts#L247
- `cl-sqldb`: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/sdk/packages/shared/src/db/sqlite-db.ts#L188
- `cl-msgs`: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/sdk/packages/shared/src/llms/messages.ts#L140

Aider (commit `5dc9490`):

- `ad-args`: https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/args.py#L271
- `ad-coder`: https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/coders/base_coder.py#L2114
- `ad-conf`: https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/website/docs/config/aider_conf.md
- `ad-cmds`: https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/website/docs/usage/commands.md

Cursor:

- `cu-config`: https://cursor.com/docs/cli/reference/configuration
- `cu-auth`: https://cursor.com/docs/cli/reference/authentication
- `cu-params`: https://cursor.com/docs/cli/reference/parameters

GitHub Copilot CLI:

- `cp-dir`: https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference
- `cp-log`: https://github.com/github/copilot-cli/blob/main/changelog.md (entries up to 1.0.114, 2026-11-18)

Goose (commit `024e4cd`):

- `gs-paths`: https://github.com/aaif-goose/goose/blob/024e4cd5a8b878d0b45bb8c7dbd83ddf2801a599/crates/goose/src/config/paths.rs#L22
- `gs-base`: https://github.com/aaif-goose/goose/blob/024e4cd5a8b878d0b45bb8c7dbd83ddf2801a599/crates/goose/src/config/base.rs#L90
- `gs-sm`: https://github.com/aaif-goose/goose/blob/024e4cd5a8b878d0b45bb8c7dbd83ddf2801a599/crates/goose/src/session/session_manager.rs#L34
- `gs-cli`: https://github.com/aaif-goose/goose/blob/024e4cd5a8b878d0b45bb8c7dbd83ddf2801a599/documentation/docs/guides/goose-cli-commands.md

Amp:

- `amp-cli`: https://ampcode.com/docs/markdown/cli
- `amp-settings`: https://ampcode.com/docs/markdown/cli/settings
- `amp-threads`: https://ampcode.com/docs/markdown/threads
- `amp-pricing`: https://ampcode.com/docs/markdown/pricing

Crush (commit `b34f2cc`):

- `cr-license`: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/LICENSE.md
- `cr-readme`: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/README.md
- `cr-load`: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/config/load.go#L240
- `cr-connect`: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/db/connect.go#L101
- `cr-init`: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/db/migrations/20250424200609_initial.sql
- `cr-mig-prov`: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/db/migrations/20250627000000_add_provider_to_messages.sql
- `cr-mig-model`: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/db/migrations/20261106000000_add_model_name_to_messages.sql
- `cr-stats`: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/db/sql/stats.sql
- `cr-root`: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/cmd/root.go#L58

Qwen Code (commit `7011932`):

- `qw-storage`: https://github.com/QwenLM/qwen-code/blob/701193272671b0ef1bb26bdc77ca9d93b90c37d8/packages/core/src/config/storage.ts#L194
- `qw-crs`: https://github.com/QwenLM/qwen-code/blob/701193272671b0ef1bb26bdc77ca9d93b90c37d8/packages/core/src/services/chatRecordingService.ts#L1512
- `qw-auth`: https://github.com/QwenLM/qwen-code/blob/701193272671b0ef1bb26bdc77ca9d93b90c37d8/docs/users/configuration/auth.md
- `qw-headless`: https://github.com/QwenLM/qwen-code/blob/701193272671b0ef1bb26bdc77ca9d93b90c37d8/docs/users/features/headless.md
- `qw-cmds`: https://github.com/QwenLM/qwen-code/blob/701193272671b0ef1bb26bdc77ca9d93b90c37d8/docs/users/features/commands.md

Kimi Code (commit `84439d5`):

- `km-data`: https://github.com/MoonshotAI/kimi-code/blob/84439d5f4ec7bcf69f3e1a5ba0e3145d8dddff08/docs/en/configuration/data-locations.md
- `km-prov`: https://github.com/MoonshotAI/kimi-code/blob/84439d5f4ec7bcf69f3e1a5ba0e3145d8dddff08/docs/en/configuration/providers.md
- `km-cmd`: https://github.com/MoonshotAI/kimi-code/blob/84439d5f4ec7bcf69f3e1a5ba0e3145d8dddff08/docs/en/reference/kimi-command.md
- `km-usage`: https://github.com/MoonshotAI/kimi-code/blob/84439d5f4ec7bcf69f3e1a5ba0e3145d8dddff08/packages/core/src/llm/usage.ts

Continue (commit `5522c6f`):

- `co-paths`: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/core/util/paths.ts#L27
- `co-cli`: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/extensions/cli/src/session.ts#L51
- `co-tok`: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/packages/config-yaml/src/schemas/data/tokensGenerated/v0.2.0.ts
- `co-cfg`: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/docs/cli/configuration.mdx
- `co-quick`: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/docs/cli/quickstart.mdx
- `co-tui`: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/docs/cli/tui-mode.mdx

Factory Droid:

- `fa-llms`: https://docs.factory.ai/llms.txt
- `fa-settings`: https://docs.factory.com/droid-cli/settings.md
- `fa-cli`: https://docs.factory.com/droid-cli/cli-reference.md
- `fa-byok`: https://docs.factory.com/model-independence/byok.md

Augment Auggie:

- `au-auth`: https://docs.augmentcode.com/cli/setup-auggie/authentication.md
- `au-settings`: https://docs.augmentcode.com/cli/settings.md
- `au-ref`: https://docs.augmentcode.com/cli/reference.md

[gh-api]: https://docs.github.com/en/rest/repos/repos#get-a-repository
[oa-costs]: https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage/methods/costs
[or-limits]: https://openrouter.ai/docs/api/reference/limits.md
[ds-balance]: https://api-docs.deepseek.com/api/get-user-balance
[ms-balance]: https://platform.moonshot.ai/docs/api/balance
[cc-settings]: https://code.claude.com/docs/en/settings
[cc-auth]: https://code.claude.com/docs/en/authentication
[cc-dir]: https://code.claude.com/docs/en/claude-directory
[cc-costs]: https://code.claude.com/docs/en/costs
[cc-cli]: https://code.claude.com/docs/en/cli-reference
[cc-sess]: https://code.claude.com/docs/en/sessions#where-transcripts-are-stored
[cc-env]: https://code.claude.com/docs/en/env-vars
[cc-gw]: https://code.claude.com/docs/en/llm-gateway
[cx-proto-usage]: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/protocol/src/protocol.rs#L2599
[cx-proto-rl]: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/protocol/src/protocol.rs#L2700
[cx-proto-meta]: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/protocol/src/protocol.rs#L3543
[cx-proto-turn]: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/protocol/src/protocol.rs#L3722
[cx-recorder]: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/rollout/src/recorder.rs
[cx-storage]: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/login/src/auth/storage.rs#L307
[cx-types]: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/config/src/types.rs#L139
[cx-state]: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/state/src/lib.rs#L149
[cx-thread]: https://github.com/openai/codex/blob/2d581656514cf4d87a43e20f3a8be4a0a2696025/codex-rs/state/src/model/thread_metadata.rs#L158
[cx-auth]: https://developers.openai.com/codex/auth
[cx-adv]: https://developers.openai.com/codex/config-advanced
[cx-ref]: https://developers.openai.com/codex/config-reference
[cx-cli]: https://developers.openai.com/codex/cli/reference
[gm-paths]: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/packages/core/src/utils/paths.ts#L22
[gm-storage]: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/packages/core/src/config/storage.ts
[gm-crs]: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/packages/core/src/services/chatRecordingService.ts#L796
[gm-oauth]: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/packages/core/src/code_assist/oauth2.ts#L112
[gm-tokenstore]: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/packages/core/src/mcp/token-storage/index.ts#L13
[gm-auth]: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/docs/get-started/authentication.mdx
[gm-sess]: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/docs/cli/session-management.md
[gm-cmds]: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/docs/reference/commands.md#stats
[gm-quota]: https://github.com/google-gemini/gemini-cli/blob/615d6bc8c95f277a16b30918606c0cd50a6e42cd/docs/resources/quota-and-pricing.md
[oc-global]: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/packages/util/src/global.ts
[oc-sql]: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/packages/core/src/session/sql.ts#L28
[oc-msg]: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/packages/schema/src/session-message.ts#L221
[oc-tokens]: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/packages/schema/src/token-usage.ts
[oc-trouble]: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/content/troubleshooting.mdx#L203
[oc-config]: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/content/config.mdx
[oc-prov]: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/content/providers.mdx
[oc-cliprov]: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/content/cli/providers.mdx#L112
[oc-migrate]: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/content/migrate-v1.mdx
[oc-v1cli]: https://github.com/anomalyco/opencode/blob/88ca792e024aa2085fa6e15288be37037c4871d2/services/site/src/docs/v1/content/cli.mdx
[kc-tree]: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/.opencode-version
[kc-global]: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/core/src/global.ts#L12
[kc-auth]: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/opencode/src/auth/index.ts#L11
[kc-db]: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/opencode/src/storage/db.ts#L35
[kc-msg]: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/opencode/src/session/message-v2.ts
[kc-profile]: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/kilo-gateway/src/api/profile.ts#L81
[kc-cli]: https://github.com/Kilo-Org/kilocode/blob/00a5d30cf76f2a826bdfb305fea631beb9d2ce76/packages/kilo-docs/pages/code-with-ai/platforms/cli-reference.md
[cl-disk]: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/apps/vscode/src/core/storage/disk.ts#L17
[cl-hist]: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/apps/vscode/src/shared/HistoryItem.ts
[cl-cli]: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/docs/cli/cli-reference.mdx
[cl-paths]: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/sdk/packages/shared/src/storage/paths.ts#L247
[cl-sqldb]: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/sdk/packages/shared/src/db/sqlite-db.ts#L188
[cl-msgs]: https://github.com/cline/cline/blob/eff43e19b267e171f597ab03733bfed25090f72d/sdk/packages/shared/src/llms/messages.ts#L140
[ad-args]: https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/args.py#L271
[ad-coder]: https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/coders/base_coder.py#L2114
[ad-conf]: https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/website/docs/config/aider_conf.md
[ad-cmds]: https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/website/docs/usage/commands.md
[cu-config]: https://cursor.com/docs/cli/reference/configuration
[cu-auth]: https://cursor.com/docs/cli/reference/authentication
[cu-params]: https://cursor.com/docs/cli/reference/parameters
[cp-dir]: https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference
[cp-log]: https://github.com/github/copilot-cli/blob/main/changelog.md
[gs-paths]: https://github.com/aaif-goose/goose/blob/024e4cd5a8b878d0b45bb8c7dbd83ddf2801a599/crates/goose/src/config/paths.rs#L22
[gs-base]: https://github.com/aaif-goose/goose/blob/024e4cd5a8b878d0b45bb8c7dbd83ddf2801a599/crates/goose/src/config/base.rs#L90
[gs-sm]: https://github.com/aaif-goose/goose/blob/024e4cd5a8b878d0b45bb8c7dbd83ddf2801a599/crates/goose/src/session/session_manager.rs#L34
[gs-cli]: https://github.com/aaif-goose/goose/blob/024e4cd5a8b878d0b45bb8c7dbd83ddf2801a599/documentation/docs/guides/goose-cli-commands.md
[amp-cli]: https://ampcode.com/docs/markdown/cli
[amp-settings]: https://ampcode.com/docs/markdown/cli/settings
[amp-threads]: https://ampcode.com/docs/markdown/threads
[amp-pricing]: https://ampcode.com/docs/markdown/pricing
[cr-license]: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/LICENSE.md
[cr-readme]: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/README.md
[cr-load]: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/config/load.go#L240
[cr-connect]: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/db/connect.go#L101
[cr-init]: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/db/migrations/20250424200609_initial.sql
[cr-mig-prov]: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/db/migrations/20250627000000_add_provider_to_messages.sql
[cr-mig-model]: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/db/migrations/20261106000000_add_model_name_to_messages.sql
[cr-stats]: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/db/sql/stats.sql
[cr-root]: https://github.com/charmbracelet/crush/blob/b34f2ccba8cfd15bd1d39e32bde7cb43405d869d/internal/cmd/root.go#L58
[qw-storage]: https://github.com/QwenLM/qwen-code/blob/701193272671b0ef1bb26bdc77ca9d93b90c37d8/packages/core/src/config/storage.ts#L194
[qw-crs]: https://github.com/QwenLM/qwen-code/blob/701193272671b0ef1bb26bdc77ca9d93b90c37d8/packages/core/src/services/chatRecordingService.ts#L1512
[qw-auth]: https://github.com/QwenLM/qwen-code/blob/701193272671b0ef1bb26bdc77ca9d93b90c37d8/docs/users/configuration/auth.md
[qw-headless]: https://github.com/QwenLM/qwen-code/blob/701193272671b0ef1bb26bdc77ca9d93b90c37d8/docs/users/features/headless.md
[qw-cmds]: https://github.com/QwenLM/qwen-code/blob/701193272671b0ef1bb26bdc77ca9d93b90c37d8/docs/users/features/commands.md
[km-data]: https://github.com/MoonshotAI/kimi-code/blob/84439d5f4ec7bcf69f3e1a5ba0e3145d8dddff08/docs/en/configuration/data-locations.md
[km-prov]: https://github.com/MoonshotAI/kimi-code/blob/84439d5f4ec7bcf69f3e1a5ba0e3145d8dddff08/docs/en/configuration/providers.md
[km-cmd]: https://github.com/MoonshotAI/kimi-code/blob/84439d5f4ec7bcf69f3e1a5ba0e3145d8dddff08/docs/en/reference/kimi-command.md
[km-usage]: https://github.com/MoonshotAI/kimi-code/blob/84439d5f4ec7bcf69f3e1a5ba0e3145d8dddff08/packages/core/src/llm/usage.ts
[co-paths]: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/core/util/paths.ts#L27
[co-cli]: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/extensions/cli/src/session.ts#L51
[co-tok]: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/packages/config-yaml/src/schemas/data/tokensGenerated/v0.2.0.ts
[co-cfg]: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/docs/cli/configuration.mdx
[co-quick]: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/docs/cli/quickstart.mdx
[co-tui]: https://github.com/continuedev/continue/blob/5522c6f44ca0ac3528b37244818fbfa39b5af470/docs/cli/tui-mode.mdx
[fa-llms]: https://docs.factory.ai/llms.txt
[fa-settings]: https://docs.factory.com/droid-cli/settings.md
[fa-cli]: https://docs.factory.com/droid-cli/cli-reference.md
[fa-byok]: https://docs.factory.com/model-independence/byok.md
[au-auth]: https://docs.augmentcode.com/cli/setup-auggie/authentication.md
[au-settings]: https://docs.augmentcode.com/cli/settings.md
[au-ref]: https://docs.augmentcode.com/cli/reference.md
