# 07 A login need shows wherever it is met, and is fixed from any of them

Status: ready
Blocked by: 06

## Why

Decision 9 (revised with the owner on 2026-11-23): a login need ("No
login", "Login expired", with its fix command such as `codex login` in
that folder) is shown on every page it concerns, the user may act from
any of them, and once fixed it goes from all of them.

- A profile with no login: the agent part's need, on the profile page,
  as a mark on its row and in All profiles → Needs you. Nothing in
  Accounts, since there is no account yet.
- A profile whose login expired: the agent part's need on the profile,
  and the provider part's need on the account found in that profile.
  Both run the same command in the same folder.
- An account with no profile (OTTY's own login, issue 08): only on the
  account, as "Log in again" (issue 09 gives it).

The spec's "Before → after" adds: All accounts → Needs you holds the
login needs of accounts, All profiles → Needs you those of profiles, and
Found by OTTY lists profiles and accounts in two separate groups with no
"found in" links.

## What to build

1. **Expired on the profile.** The agent parts of the official plugins
   say "Login expired" with `not-logged-in` and the agent's login
   command for that folder when their own files tell that the login can
   no longer be used. Each plugin writes in its code what it checks.
   When the files can not tell, the agent part reports nothing and only
   the account shows the need. See the comment below before starting.
2. **Expired on the account.** The provider parts of OpenAI and
   Anthropic give "Login expired" on an account when the provider
   refuses its login in a profile folder, with the agent's login command
   for that folder (`run`, such as `codex login` with `CODEX_HOME`). The
   title, text and command are the same as the agent part's, so the two
   read as one need.
3. **Fixed from either place.** When the tab of a login need closes
   (`crates/app/features/agents/core/src/need.rs`, the watched tabs of
   issue 32), OTTY reads the profiles and the accounts again, and their
   numbers, so the need goes from every page it was on.
4. **Needs you.** All accounts → Needs you lists the needs of accounts,
   All profiles → Needs you the needs of profiles, login needs included
   (`model/needs.rs`, `model/overview.rs`). The profile keeps its other
   needs (format changed, folder not found, agent missing), the plugin
   needs (update waits, part off, plugin failed, format changed) stay.
5. **Found by OTTY in two groups** (`model/found_report.rs`,
   `view/found_report.rs`): Profiles (agent, version, folder) and
   Accounts (provider, name, kind, source), with no count of accounts
   per profile and no "found in" link from one group to the other. The
   intro line counts both.
6. Words in all ten locales.

## Done when

- Tests first: a profile with no login shows "No login" on its page,
  its row and All profiles → Needs you, and nothing in Accounts, an
  expired login of `~/.codex` shows on the profile (when its files tell)
  and on the account, closing the tab of the need on either page reads
  both again and the need goes from both, All accounts → Needs you holds
  the account's need, and Found by OTTY shows the two groups with no
  links.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-23, written with the issues: the spec says the agent part
  shows "Login expired" on the profile, but an agent part sees only its
  files, and a login the server refuses usually looks fine there (Codex
  keeps a refresh token, Claude Code an `expiresAt` it renews). So in
  most cases only the provider part knows. Ask the owner before
  starting whether the profile should also show the need the provider
  part of the same plugin gives for an account found in that profile's
  folder (same plugin, same folder, so no link across plugins), or show
  it only when the agent part's files tell.
