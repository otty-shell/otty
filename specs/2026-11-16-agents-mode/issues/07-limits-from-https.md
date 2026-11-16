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
