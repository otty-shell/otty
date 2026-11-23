# 06 The agent part does agent things only

Status: ready
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
