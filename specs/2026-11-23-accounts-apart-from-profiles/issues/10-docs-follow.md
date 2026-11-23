# 10 Docs follow: profiles and accounts apart

Status: ready
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
