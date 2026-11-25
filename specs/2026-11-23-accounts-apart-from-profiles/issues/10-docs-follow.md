# 10 Docs follow: profiles and accounts apart

Status: done
Blocked by: 09

## Why

Issues 01 to 09 change what Agents mode shows and what a plugin gives.
The public pages, the plugin pages and the earlier specs still describe
the links between profiles and accounts, accounts added by key place,
the found extra folders and the old contracts. This spec also says which
issues and decisions of earlier specs it replaces ("What it
supersedes").

## What to build

1. **Public pages** (`docs/public/Agents/`):
   - `Overview.mdx`: profiles and accounts as two independent lists, no
     Pays through, Used by or By account, money "at API prices", the
     default and added profiles only, the account page with Source, Add
     account with "Log in with <agent>", Log in again and Remove of
     OTTY's own login, login needs on the profile and on the account.
   - `What OTTY Reads and Sends.mdx`: a provider part reads logins only
     in its own plugin's profile folders and OTTY's own login folders
     (`<data dir>/accounts/<id>/`), OTTY runs the agent's login command
     there and never refreshes a token, what `agents.json` keeps.
   - `Plugins.mdx`: the "can access" group of a provider part's logins.
   - `Write a Plugin.mdx`: the new agent and provider contracts (no
     logins in the agent part, `accounts`, `numbers` and `read-bill` of
     an account, `log-in`), the stable account id, the manifest's
     `logins` entries with their keys, read globs, folder variable and
     "Log in with", the default profile only in `profiles`, and no
     `add_by_hand`.
2. **Plugin pages** (`docs/public/Agents/Official Plugins/*.mdx`) and the
   READMEs of `plugins/*`: OpenAI and Anthropic find accounts in their
   agent's profiles and their own logins, OpenCode is an agent only,
   DeepSeek and OpenRouter have no accounts until "paste a key", the
   template shows both parts.
3. **Crate READMEs and module docs** that still describe joining,
   `merge_accounts` or added accounts (`crates/agents/README.md`,
   `crates/plugins/sdk/README.md`, the lib docs of `crates/agents` and
   of the agents feature crates).
4. **Earlier specs.** At each decision and section this spec supersedes
   (its "What it supersedes" list), add a short note that points to
   `specs/2026-11-23-accounts-apart-from-profiles/spec.md`:
   - `specs/2026-11-19-agents-accounts/spec.md`: Decision 2, Decision 4,
     the Used by, Pays through, By account and Account line parts of
     Decision 6, Add account by key place in Decision 7.
   - `specs/2026-11-19-agent-plugins/spec.md`: in Decision 4 the bullets
     "Agent part" (logins), "Provider part" (`login-facts`, `found-in`),
     "No fight over names", "Accounts and profiles are not plugins" and
     "Hidden accounts are left out of every total", the sections
     "Identity and links do not depend on a provider plugin" and the
     account rows of "What the UI shows".
   - A comment on issues 29, 31 and 34 of
     `specs/2026-11-19-agent-plugins/issues/` saying which part this spec
     replaces. Issue 43 was dropped before it was written.
5. **Owner's open questions.** The spec's "Open questions" keep the ones
   the owner has not answered, and each answered one moves into its
   decision with the date.

## Done when

- Every page above reads true against the app after issue 09, with no
  word left on Pays through, Used by, By account, account copies,
  accounts added by key place, found extra folders or the agent part's
  logins, checked with
  `grep -rn "Pays through\|Used by\|By account\|added by you\|login-facts\|merge_accounts" docs/public/Agents plugins/*/README.md crates/agents/README.md crates/plugins/*/README.md`.
- The checks of `AGENTS.md` are not needed unless a Rust file changes.

## Comments

- 2026-11-23, done. The docs describe what issues 01 to 09 and 11 built.

  Public pages: Overview tells profiles and accounts as two lists with no
  link, the default and added profiles only, accounts found by the
  provider part of the same plugin, Found by OTTY in two groups, the
  account page with Source, the usage block with By project and By model,
  money "at API prices", login needs wherever met, and Add account with
  "Log in with <agent>", Log in again and Remove of OTTY's own login. What
  OTTY Reads and Sends: a provider part reads logins only in the folders of
  a call, OTTY never renews a token, the login command OTTY runs, and
  `agents.json` and `<data dir>/accounts/<id>/` in what OTTY writes. Plugins:
  the group "Logins in <Agent> profiles" in place of "Found logins", and
  what happens when a part is off or missing now. Write a Plugin: the
  agent and provider contracts as built (`accounts`, `numbers`,
  `read_bill`, `log_in`, `LoginFolder`, the stable account id, needs on
  the account), `[[agents.provider.logins]]` with `read`, `write`, `keys`,
  `folder_env` and `log_in`, `[[agents.provider.permissions.keys]]`, the
  money of an answer (`answer_cost`, `price_of`), login needs, OTTY's own
  login, a template-based test, and no `add_by_hand`. The plugin pages give
  the current versions (OpenAI and Anthropic 0.2.12, OpenCode 0.1.8,
  DeepSeek and OpenRouter 0.1.5) and what each part does now, and the
  Anthropic page notes that "Log in with Claude Code" is not checked on
  macOS.

  READMEs: the template README also tells `write` and
  `[[agents.provider.permissions.keys]]` and how its own login stays
  fresh. The other plugin READMEs, `crates/agents`, `crates/plugins/*`
  were already true. `otty-app-agents` now names the accounts and Add
  account, `otty-app-agents-io` no longer reads limits by profile and says
  it deletes only the folders of OTTY's own logins. The lib docs (`.rs`)
  were left as they are, since this issue changed no Rust: the first line
  of `crates/agents/src/lib.rs` ("the providers they pay through") and of
  `crates/app/features/agents/core/src/lib.rs` ("the accounts the coding
  agents pay through") still need a word change in a later Rust change.

  Earlier specs: notes at Decisions 2, 4, 6 and the Add account of
  Decision 7 of `2026-11-19-agents-accounts`, at the bullets Agent part,
  Provider part, No fight over names, Accounts and profiles are not
  plugins and Hidden accounts of Decision 4, the section Identity and
  links and the account rows of What the UI shows of
  `2026-11-19-agent-plugins`, comments on its issues 29, 31 and 34, and
  notes in `docs/research/wasm-plugins.md`. The spec has "Built as" notes
  where the code differs (own login words, issue 11's inner links, the
  answer `cost` and `prices` interface, `is-default`, the `logins` entries,
  the Add account dialog, the own login reads, the shared login need, the
  account menu, the WIT case `own-login` and `option<command>`), its
  answered open questions moved into Decisions 6 and 7, and its status
  says it is built. The grep of "Done when" finds nothing.
