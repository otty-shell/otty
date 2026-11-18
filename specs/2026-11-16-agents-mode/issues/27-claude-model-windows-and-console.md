# 27 Claude: every model window, Console profiles billed by money

Status: ready-for-agent
Blocked by: 26

## Why

Spec Decision 11. The owner's subscription shows a Fable weekly limit in
Claude, but OTTY draws only the windows it knows by name. And the owner's
second profile (`~/.claude-semrush`) is logged in to a Console organization:
its `oauthAccount.billingType` is `prepaid` (the main profile says
`stripe_subscription`), it has no subscription windows, and OTTY shows
neither limits nor money for it, while VS Code shows the dollars spent.

## What to do

1. The usage answer (`claude_code/usage.rs`): every model-specific weekly
   window becomes a bar, named after its model: the `seven_day_<model>` keys
   (`seven_day_opus`, `seven_day_sonnet`, and any other such key, such as
   Fable) and the entries of `limits[]` with `weekly_scoped`, as CodexBar
   maps them (read its Claude usage code, MIT,
   https://github.com/steipete/CodexBar, with `gh api` or WebFetch). Keys
   that are not windows (`seven_day_oauth_apps` as a fallback,
   `seven_day_routines`, `seven_day_cowork`, `extra_usage`, `iguana_necktie`,
   `cedar_ember`) keep the meaning CodexBar gives them or are skipped. The
   `/usage` screen reader (issue 10) also reads every "Current week
   (<model>)" block. Names: "7 days · Fable", like the others.
2. `detect` for Claude Code: `billingType` `prepaid` (and any other Console
   billing type CodexBar or Claude Code treats as API billing; check) with no
   API key in the files is an API key profile (spec Decision 9): no limits
   read, the money estimate from the session files. Never read or keep a
   token for this.
3. The header of such a profile says "Anthropic API, pay per token" as for
   other API key profiles.

## Done when

- Tests, written first, cover: an answer with Opus, Sonnet and a third model
  key, an answer with `limits[]` weekly entries, a key that is not a window;
  the `/usage` screen with three week blocks; a `.claude.json` with
  `billingType` `prepaid` and one with `stripe_subscription`.
- Test data is made up; no real file or answer is copied.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

### 2026-11-17 (coordinator)

From issue 26, a small one to do here too: the path field of the profile
dialog is labelled with the picked way ("Type a path"); label it "Config
folder" instead, and keep the way's word only on the segmented choice.

### 2026-11-17

Built in `otty-agents` and `otty-app-agents`, tests first. The reading of
the usage answer moved from `claude_code/usage.rs` into its own
`claude_code/usage_answer.rs` (the request and the token stay in
`usage.rs`), with its tests; the window names live in
`claude_code/window_name.rs`, shared by the answer and the `/usage` screen.

What CodexBar's code showed (`ClaudeOAuthUsageFetcher.swift`,
`ClaudeUsageFetcher.swift`, `ClaudeScopedWeeklyLimitMapper.swift`,
`ClaudeWebExtraRateWindowParser.swift`, `ClaudePlan.swift`): it reads only
`seven_day_opus` and `seven_day_sonnet` as flat model keys; any other
model's week, such as Fable, comes from the `limits` list: an entry with
`kind` `weekly_scoped` and `group` `weekly`, a `percent` from 0 to 100,
`resets_at`, and `scope.model.id` / `display_name`. An entry whose model
or id is "all models" is skipped, `is_active` is not looked at, and the
same model is shown once. `seven_day_oauth_apps` stands in for
`seven_day` when that is missing. `seven_day_routines`,
`seven_day_claude_routines` and `seven_day_cowork` are a "Daily Routines"
window, `iguana_necktie` the cloud credits and `cedar_ember` the reset
status; OTTY skips all of them. CodexBar never reads `billingType` of
`.claude.json`: it uses the organization's `billing_type` of the claude.ai
web answer only to see a Pro plan (`stripe` with a Claude tier). So nothing
in CodexBar names a Console billing type; only `prepaid`, the owner's
value, is taken as one.

Windows: "5 hours", "7 days" (or the apps' week), then the week of each
model: the `seven_day_<model>` keys, Opus and Sonnet first so the old
names keep their place, then the others by key name, each named from the
key ("seven_day_fable" is "7 days · Fable", words split at `_`, the first
letter in upper case); then the `weekly_scoped` entries of `limits`, named
"7 days · <display_name>". A model already shown (case does not matter) is
not shown again, so the key wins over the list. The `/usage` screen reads
every "Current week (<model>)" block as before, now keeping the model's
letters as the screen shows them ("Example Model", not "Example model",
the same as the list's `display_name`), and also reads the compact
"Currentweek(Fable)" CodexBar tests for.

Console: `detect` takes a profile as an API key profile when the
`oauthAccount` of its `.claude.json` has `billingType` `prepaid` and its
`.credentials.json` names no plan (`subscriptionType`). A plan in the
token file wins, as a subscription the files tell for sure. The billing
type is read as text; the token is still only checked for being there. The
app already reads no limits for an API key profile, shows the money
estimate and says "Anthropic API, pay per token" in the header, so it did
not change for this. The coordinator's note is done: the path field of the
dialog is labelled "Config folder" under the segmented choice; the caption
"Config folder" over the choice is gone, so the words do not show twice.

Tests seen failing first: the five new answer tests (a third model key, a
key of several words, the `limits` entries, a model in both, the apps'
week), the two new screen tests, the two prepaid `detect` tests and the
dialog label test. Not seen failing, as they guard what already worked:
the entries of `limits` that are not a model week, the keys that are not
a window, the `stripe_subscription` login and the plan that wins over
`prepaid`; the small `window_name` test was written with its function.

Open: `billingType` values other than `prepaid` that mean Console billing
are not known (CodexBar names none); a value such as
`stripe_subscription_contracted` or `apple_subscription` stays a
subscription.
