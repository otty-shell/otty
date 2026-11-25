# 06 The agent part does agent things only

Status: done
Blocked by: 05

## Why

Decision 2: an agent part gives profiles, projects, sessions and tokens,
and the launch and resume commands. It knows nothing about accounts:
`logins`, `login-refused`, the `login` of an answer and the `connection`
of a profile leave the agent contract. It keeps the `not-logged-in`
status with its need, since whether its own agent can log in is the
agent's business (Decision 9). Decision 11: the OpenCode plugin is an
agent only, its login code goes.

Since issue 05 OTTY no longer calls these functions, so this issue
removes them from the contract, the SDK, the host and the plugins.

## What to build

1. **The agent contract**
   (`crates/plugins/sdk/wit/agents-agent/0.1.0/agent.wit` and
   `types.wit`), as the spec's WIT sketch:
   - Gone: `logins`, `login-refused`, the record `login`, `login-ref`,
     the `login` of `answer`, `connection`, `billing`, and in
     `types.wit` `identity` and `login-kind`. `provider-ref` stays for
     the provider of an answer, which prices it.
   - `profile-status` is: found (the profile can be used, with no word
     on whose login it is), `not-logged-in(need)`, or
     `config-folder-not-found`. Name the first case for what it means,
     such as `found`.
   - `types.wit` keeps only what both contracts share (`read-error`).
   - The doc comments say the agent part knows nothing of accounts.
2. **The SDK and the host.** `crates/plugins/sdk/src/agents/agent.rs`,
   the re-exports of `agents.rs`, `AgentCalls`
   (`crates/plugins/host/src/agents/agent.rs`: `logins`, `login_refused`
   go) and the agent-side conversions of `crates/agents/src/`
   (`plugin_agent.rs`, `plugin_values.rs`, `ProfileStatus` of
   `profile.rs` or where it lives) follow. `ProfileStatus::Connected`
   carries no connection any more.
3. **OTTY's screen.** Whatever showed the connection of a profile (whose
   login, plan, billing) goes from the profile page and the profile rows
   (`crates/app/features/agents/core/src/model/profile_page.rs`,
   `profile_state.rs`). The profile page's Profile info is Agent,
   Version, Folder, Projects, Sessions ("Before → after", Profile page).
4. **The official plugins.** Bump each plugin's patch version and rebuild
   with `plugins/build.sh`.
   - **OpenAI** (`plugins/openai`): the Codex part's `detect` says only
     found, not logged in (no login in `auth.json`) or folder not found
     (`detect.rs`). The login listing of the agent side goes (what issue
     05 has not moved yet).
   - **Anthropic** (`plugins/anthropic`): the same for Claude Code
     (`detect.rs`, what is left of `logins.rs` on the agent side).
   - **OpenCode** (`plugins/opencode`): `logins.rs` and its login needs
     go, with the key permissions of its manifest that only they used.
     OpenCode works with no login (its free models), so its part
     reports no `not-logged-in` (Decision 9: the plugin decides).
   - **Template**: the example agent follows, and shows `not-logged-in`
     with its need.
   - The agent part's `keys` permissions stay where `detect` needs them
     to tell whether a login is there (Codex `auth.json`, Claude Code
     `.credentials.json`), with the keys as handles as today.
5. Module docs follow. The public docs are issue 10.

## Done when

- Tests first: (host, over the template) the agent part has no `logins`
  or `login-refused` call, a profile with no login is `not-logged-in`
  with the part's need, one with a login is found. (OTTY) a found
  profile shows no account, plan or billing, a profile with no login
  shows its need as today. (Plugins) OpenCode lists its profile,
  sessions and tokens with no login need, Codex and Claude Code say "No
  login" for a folder with no login.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-23, owner answer, added to this issue: **money comes from the
  agent part.** When the agent writes the cost itself (OpenCode), the part
  gives it as it is, with no mark. Otherwise the part asks OTTY for the
  price of a model through a new host call (OTTY answers from the price
  list it keeps and renews, LiteLLM today, `prices.rs`) and prices the
  tokens itself, and that money is marked "at API prices". The spend the
  part returns carries money per model next to the tokens, and whether
  it is exact or at API prices. OTTY no longer multiplies tokens by prices
  itself. See spec Decision 3.

- 2026-11-23, done. The agent part does agent things only, and prices
  its own answers.

  Contract (changed in place): `agent.wit` has no `logins`,
  `login-refused`, `login`, `login-ref`, `connection`, `billing`, and
  the `login` of `answer` is gone. `profile-status` is `found`,
  `not-logged-in(need)` or `config-folder-not-found`. `types.wit` keeps
  only `read-error`. A new imported interface `prices.wit`
  (`otty:agents-agent/prices`) holds `provider-ref` (moved from `agent`),
  `token-prices`, `long-prompt-price`, `model-price` and the host call
  `price-of(model, provider) -> option<model-price>`. An `answer` now
  carries `cost: option<answer-cost>`, where `answer-cost` is
  `written(u64)` (the cost the agent wrote, exact) or
  `at-api-prices(u64)` (millionths of a dollar); none when the model has
  no price. The money is per answer, so the long prompt price is picked
  by the part for each request (OTTY's `PROMPT_LIMITS` and the long
  prompt buckets of the token count are gone).

  SDK: the `Agent` trait lost `logins` and `login_refused`.
  `agents::price_of`, `agents::cost_at` and `agents::answer_cost` (written
  cost as it is, else the tokens at the price OTTY gives) do the pricing,
  and `TestHost::with_price` gives a plugin test its price list.

  Host: `AgentCalls` lost `logins` and `login_refused`;
  `read_answers(profile, item, prices)` takes the app's
  `agents::prices::PriceList` (a trait, since the host does not know
  where OTTY keeps prices) and answers `prices.price-of` for that call
  only. Tests over the template: no `logins` or `login-refused` export, a
  profile with a login is `found`, one with none is `not-logged-in` with
  the part's need, and the answers are priced at the price list given.

  `otty-agents`: `ProfileStatus::Connected` carries nothing.
  `ApiPrices` (the `PriceTable` and the `Registry` that names a provider
  to it) answers the parts. `SpendCounter::spend`,
  `StatsSource::session_stats`/`project_stats`, `read_session_stats` and
  `read_project_stats` take `&Arc<ApiPrices>`; the part spend forgets
  what it kept when the prices change, so every file is read again and
  the money follows. `ModelTokens` carries `money()` (a `MoneyTotal`:
  cost, requests with no price, requests at API prices) in place of
  `cost()` and the long prompt buckets; `MoneyTotal::of_models(models)`
  only adds up. `ModelPrice::cost`, `TokenPrices::cost` and the
  `PROMPT_LIMITS` filter of the LiteLLM reader are gone; the "served by"
  pricing tests moved to `api_prices.rs`.

  Screen: the money of the usage block, the statistics dialog and the
  project summary is the money the parts gave; the mark "at API prices"
  shows when any of it is at API prices or unpriced, and the Money choice
  stays off until the prices are read. `Agents::api_prices()` gives the
  binary the prices for `load_spend`, `load_session_stats` and
  `load_project_stats`. When the prices or the registry change, every
  shown profile is counted again, and a count on its way then is made
  again once it ends. With the connection gone, a profile counts as
  paying with an API key when the accounts its provider part found there
  are only keys or organizations (no limits read, bills read as before).
  The profile page already showed no account, plan or billing, so no
  text changed.

  Plugins: `openai` 0.2.8 -> 0.2.9 (Codex `detect` is found or "Needs
  login", no billing; the spend file no longer fills the answer's login,
  `creator_account_id` is no longer read; answers priced at the price
  OTTY gives), `anthropic` 0.2.8 -> 0.2.9 (Claude Code `detect` found or
  "Needs login", answers priced), `opencode` 0.1.7 -> 0.1.8 (agent only:
  `logins.rs`, `needs.rs`, `plan.rs`, `provider_ids.rs`, the credential
  reads, the language receiver and the `serde` and `base64` dependencies
  go; `detect` is found whenever the data folder holds a file; the cost
  OpenCode writes is given as `written`), template `example` 0.1.4 ->
  0.1.5 (found or not logged in with its need, `login_refused` and its
  texts go, answers priced, a made-up `cost_microdollars` shows the
  written cost), `deepseek` and `openrouter` 0.1.3 -> 0.1.4 (rebuilt for
  the changed types). `template.wasm` and the four test plugins of the
  host were rebuilt.

  Kept on purpose: OpenCode's manifest still declares its logins as keys
  (`auth.json` and the `credential` column). Its `read` and `sqlite`
  permissions cover that file and that table, and a declared key file is
  refused as bytes and a declared key column reads as `NULL`; without
  the declarations the part could read the keys as text. The `keys`
  permissions of Codex and Claude Code stay for `detect`. A count made
  before the prices are read is made again once they are, so the start
  reads the session files twice; the start could read the prices before
  the tokens to avoid it.

  Left for later issues: "Login expired" on the profile and on the
  account and fixing it from either (07), OTTY's own logins (08, 09),
  the public docs (10).
