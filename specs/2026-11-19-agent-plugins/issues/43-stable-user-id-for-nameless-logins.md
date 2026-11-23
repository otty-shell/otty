# 43 A login known only by its user is kept by a user id

Status: ready
Blocked by: 42

## Why

Owner, 2026-11-23: why can the "unknown" OpenCode account not be hidden?

The OpenCode login by `chatgpt-token-sharing` keeps its account id and
email encrypted, so the plugin tells no account id, organization id or
email. OTTY then knows the account by the fingerprint of its key
(`account_of` in `crates/agents/src/plugin_agent.rs`), and an account
known by a key fingerprint can not be saved (`Account::can_be_saved`):
Hide and Rename are off. Worse, the key is a ChatGPT access token that
changes about every hour, so the fingerprint, and with it the account,
changes too: a hidden or renamed account would come back after a token
refresh.

The token does name a stable user: its `sub` claim (issue 34 compares it
to tell that two logins are one person). The owner agreed to keep such an
account by that id: it is not a secret (nobody can log in with it), it is
kept only in `agents.json` on this computer next to the names and hidden
marks, and it is never shown.

## What to build

1. **WIT.** `identity` in `otty:agents-agent@0.1.0`
   (`crates/plugins/sdk/wit/agents-agent/0.1.0/types.wit`) gets
   `user-id: option<string>`: a stable id of the user at the provider, for
   a login whose files name no account, organization or email. Unreleased
   contract: change it in place, no new version (owner rule: no backward
   compatibility for Agents mode before its release). SDK, host and
   template follow.
2. **OTTY.** `AccountIdentity` gets `UserId(String)`; `account_of` takes,
   in order, the account id, the organization id, the email, the user id,
   and only then the key fingerprint. An account known by a user id can
   be saved (`agents.json` keeps it as the other ids), so Rename and Hide
   work and stay after a token refresh. Two logins with one user id and
   one provider are one account.
3. **OpenCode.** Its ChatGPT logins give the token's `sub` as `user-id`
   when the token names no account id and no email. Bump the patch
   version, rebuild with `plugins/build.sh opencode`. Other official
   plugins give it where their logins can have no other identity (check
   OpenAI's Codex logins; do not invent one where there is none).
4. Spec ("identity"), Write a Plugin and the plugin pages follow.

## Done when

- Tests first: a login with only a user id is an account known by it,
  can be saved, its name and hidden mark are kept in `agents.json` and
  read back; a new token with the same user id is the same account (still
  hidden, same name); the order account id → organization → email → user
  id → key fingerprint; OpenCode gives `sub` as the user id only when the
  token names nothing else.
- The checks of `AGENTS.md` pass.

## Comments
