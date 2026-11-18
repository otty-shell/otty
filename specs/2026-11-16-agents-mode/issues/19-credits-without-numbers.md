# 19 Credits that come without numbers

Status: ready-for-agent

## Why

The owner's Codex account shows no credits at all: no number in the left
panel row and an empty "Credits" line on the dashboard.

Cause, checked read-only in the owner's Codex session records on
2026-11-16: the account is on the plan `enterprise_cbp_usage_based`, and
every one of about 33 000 `rate_limits` records says
`"credits": {"has_credits": true, "unlimited": false, "balance": null}`
with no usage windows. So the source answers "credits" (issue 11), but
with no number at all, and the app draws a credits block with nothing in
it and no number in the row. Other records of the same machine show
`{"has_credits": false, "balance": "0"}` (the older `prolite` plan, which
has windows) and `{"has_credits": true, "unlimited": true}`.

## What to do

1. `Credits` knows that credits are on even when no number came: a plain
   "available" state next to the optional numbers (spec Decision 9: show
   what is known, guess nothing).
2. Dashboard, credits block: with no number, the line says "Credits:
   available" and a muted note: "This plan does not report a balance."
   With a balance, a pool or "unlimited", it stays as it is.
3. Left panel row of a credits profile with no number: the tokens spent
   today from issue 09 ("1.8 M today", muted), the same text the dashboard
   uses; nothing while they are not counted yet. With a number it stays
   "3 760 left" or "Unlimited".
4. Plan names of Codex read like words, not codes: a small table for the
   plan types the sources give (`enterprise_cbp_usage_based` → "Enterprise,
   usage-based", `prolite` → "Pro Lite", `plus`, `pro`, `team`, `business`,
   `enterprise`, `edu`, `free`), and for an unknown code the current
   fallback (underscores to spaces, first letter upper case).
5. `{"has_credits": false, "balance": "0"}` with no windows is "no credits",
   not a credits block with "0 left": the source then says what is true
   ("unexpected answer: no usage window and no credits"), as today.

## Done when

- Tests, written first, cover: the three credit shapes above as answers of
  each Codex source; the dashboard line and note; the row with tokens and
  with nothing counted yet; the plan names.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Built in `otty-agents`, `otty-app-agents` and `otty-app-i18n`, tests first
and seen failing. `Credits` got `Credits::available()` and
`is_only_available()`: a `Credits` value always means the credits are on, and
"only available" means no balance, no credits used, no pool and no
"unlimited". A separate flag was not added, because it would always be true.
`codex_credits` gives `Credits::available()` for `has_credits: true` with a
`null` or missing balance. `{"has_credits": false, "balance": "0"}` is still
"no credits", so with no window the source says "unexpected answer: no usage
window and no credits", and next to windows the windows win. Tests cover the
three shapes in all three Codex sources (usage request, app-server with
`hasCredits`, session file).

Plan names: a small table in `codex/rate_limits.rs` for
`enterprise_cbp_usage_based` ("Enterprise, usage-based"), `prolite` ("Pro
Lite"), `plus`, `pro`, `team`, `business`, `enterprise`, `edu` and `free`;
any other code keeps the old fallback (underscores to spaces, first letter
upper case).

Dashboard: credits with no number say "Credits  Available" with the muted
note "This plan does not report a balance." under them (a renewal time, if
one ever comes, still shows). A balance, a pool or "Unlimited" look as
before. Left panel row: credits with no number show the tokens spent today,
muted, such as "1.8M today", written by the same `short_count` the
dashboard's Today uses; nothing until the tokens are counted. New texts
`agents_credits_available`, `agents_credits_no_balance` and
`tpl_agents_tokens_today` are in all ten languages.

Deviations: the value is "Available" with a capital letter, like
"Unlimited" in the same place, not "available" as written above. The
number is "1.8M", without the space shown in "1.8 M", because that is how
the dashboard writes it. The tokens text has its own template instead of
reusing the money one: both read "{amount} today" now, but they mean
different things.
