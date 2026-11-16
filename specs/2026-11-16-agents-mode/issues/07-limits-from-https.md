# 07 Limits from the HTTPS requests, both agents

Status: ready-for-agent
Blocked by: 05

## Why

The user wants to see how much of each limit is used (spec Goal 2). The
simple request is the first source for both agents (spec Decision 5). This
issue also draws the limits, so the later sources only add data.

## The requests

Taken from the CodexBar docs (`docs/claude.md`, `docs/codex.md`); neither
address is documented by its owner, so the reading must be tolerant.

- Claude Code: `GET https://api.anthropic.com/api/oauth/usage` with
  `Authorization: Bearer <accessToken>` and
  `anthropic-beta: oauth-2025-04-20`. The token is `claudeAiOauth.accessToken`
  in `<config>/.credentials.json`. The answer has `five_hour`, `seven_day`,
  `seven_day_opus`, `seven_day_sonnet`, each with a percent used and a
  reset time, and `extra_usage`. The plan name comes from
  `subscriptionType` and `rateLimitTier` in the credentials file.
- Codex: `GET https://chatgpt.com/backend-api/wham/usage` with
  `Authorization: Bearer <access_token>`. The token is `tokens.access_token`
  in `<config>/auth.json`. The answer has `rate_limit.primary_window` and
  `secondary_window` (percent used, window length, reset time) and the
  credits. A `chatgpt_base_url` in `<config>/config.toml` replaces the
  host.

## What to do

1. Add the `ureq` dependency to `otty-agents` (approved by the owner, spec
   Decision 8) with TLS and without what is not needed. `cargo deny check`
   must pass. Put the request behind a small trait so the sources are
   tested without a network.
2. One `LimitSource` for each agent. It reads the token, makes the request
   and turns the answer into `Limits`. It only reads the token and never
   writes or refreshes it. A missing or expired token is "not logged in";
   a refused request with a valid-looking token is "not logged in" too; no
   connection is "network"; an answer it can not read is "unexpected
   answer". A window missing in the answer is left out, not an error.
3. In the app: read the limits of each connected profile when the mode
   opens, on "Refresh", and every five minutes while the mode is on
   screen. Draw the windows as bars in the profile's row of the left panel
   and in full in the profile's details: each window with its percent and
   reset time, the credits, the plan, the source that answered and, when it
   is not the first one, why the earlier ones failed.
4. The rules of spec Decision 5 for a failed read: the last good answer
   stays, greyed, with its time; with none, "not available" with the
   reasons, and "open the agent to log in again" when the reason is the
   login.
5. Send nothing but the request above: no OTTY version with user data, no
   account name in a header.

## Done when

- Tests, written first with a `mockall` request, cover for each agent: a
  full answer; an answer with a window missing; an answer with unknown
  fields; an account with credits only; a missing token file; an expired
  token; a refused request; no connection; an unreadable answer. And for
  the reducer: bars from an answer; the last good answer kept after a
  failure; the reasons shown when there never was one.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Built tests first, with a `mockall` request and made-up files; nothing
reaches the network. `otty-agents` now depends on `ureq` 3.5.2 without its
default features and with only `rustls`: TLS from rustls 0.23 with `ring`
and the Mozilla roots of `webpki-roots`, no OpenSSL, no `aws-lc-rs`, no
gzip. `webpki-roots` is under CDLA-Permissive-2.0, so that licence was
added to `deny.toml` with a comment. The request sits behind the small
`HttpsGet` trait in `src/https.rs`; the real client follows no redirect,
gives up after 30 seconds and sends `User-Agent: otty/<version>`. Each
agent has one source, "usage request" (`claude_code/usage.rs`,
`codex/usage.rs`). In the app, the limits of each connected profile are
read when the mode opens, on Refresh and on a five-minute timer while the
mode is on screen, and drawn as bars under the profile row and in full in
the details, by the rules of Decision 5.

What CodexBar's code showed, and choices: Claude's `utilization` is a
percent from 0 to 100 and `resets_at` an RFC 3339 time; `expiresAt` is in
milliseconds. A token whose `scopes` lack `user:profile` is refused by the
server, so it is "not available" before any request. A token with no
`expiresAt` is tried (CodexBar counts it as expired, but OTTY never
refreshes, so the server decides). Claude's `extra_usage` is in cents; it
shows only as an "Extra usage" window with its percent while it is turned
on, since OTTY showed no money yet (issue 11 changes this). An answer with
no window at all is "unexpected answer", as in CodexBar. Codex's
`used_percent` is 0 to 100, `reset_at` a Unix time in seconds, and the
window is named from `limit_window_seconds` ("5 hours", "7 days"), the
shorter one first. CodexBar sends `ChatGPT-Account-Id` with
`tokens.account_id`, so OTTY does too when the file has one. `auth.json`
keeps no expiry: the access token is a JWT and its `exp` claim is read
(a small base64url reader, no new crate). A login with only an API key is
"not available": it has no usage limits. `chatgpt_base_url` is read from
the top-level keys of `config.toml` line by line, as CodexBar does; a
ChatGPT address gets `/backend-api`, any other server is asked at
`/api/codex/usage`. CodexBar's Claude request also sends a `claude-cli`
User-Agent and a `cedar_ember` query; OTTY sends neither. A 401 or 403 is
"not logged in", no answer is "network", any other status or a body that
is not JSON is "unexpected answer". As the issue says, a missing token
file is "not logged in", even when `detect` found the account in
`.claude.json`.

### 2026-11-16 (coordinator)

Accepted. Its drawing (bars under the profile rows, the limits in the
details) is the old look and is replaced by issues 14 and 16; `Limits` is
reshaped by issue 11. Follow-up in issue 11: a missing or empty Claude token
file is "not available", not "not logged in".
