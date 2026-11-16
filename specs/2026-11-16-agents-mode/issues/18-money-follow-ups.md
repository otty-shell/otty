# 18 Follow-ups of the money work

Status: ready-for-agent
Blocked by: 17

## Why

Issue 17 left three small choices that the coordinator decided on
2026-11-16.

## What to do

1. **The admin key variable is read through the user's login shell**, the
   same way programs are found (issue 05), so a variable set in `.zshrc`
   is seen when OTTY was started from the desktop. The value is read on the
   read thread, kept only for the request and never logged.
2. **The number in the left panel row of an API key profile is the
   profile's own estimate** ("$12.40 today"), not the organization's bill,
   because the row is about this profile. The dashboard keeps the bill in
   its Spending block.
3. **"By model, this month" says it is the estimate** while the bill
   answers: "By model, this month (estimate at list prices)".

The `anthropic-version` header stays `2023-06-01`.

## Done when

- Tests, written first, cover each point.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Built tests first; each new test was seen failing before the change, except
three of the shell variable tests (a wrong name, a slow shell, a shell that
can not start), which passed against the first empty stub that gave
nothing. No test reads a real key or the real environment, and none
reaches the network: they use made-up shells.

1. The admin key variable is now read through the user's login shell, the
   way programs are found (issue 05): `otty-app-agents-io` runs
   `<shell> -l -i -c "printf '%s\n' '<mark>'; printenv '<NAME>'"` on the
   bill's read thread, with a 10 second limit (`shell_variable.rs`). The
   value is what follows the mark line, so a greeting printed by
   `~/.zshrc` is not taken for the key. Only the shell and the variable's
   name are ever logged; the value goes straight into the `AdminKey` of
   the request and is dropped with it. `load_bill` takes the shell like
   `load_limits`. The name check (letters, digits, `_`, not starting with a
   digit) moved from the profile dialog to
   `AdminKeyRef::is_variable_name` in `otty-agents`; the shell is never
   started for a name that fails it, and `AdminKeyRef::read` answers "the
   name of the admin key variable is not valid" without showing the name,
   since a key pasted into the wrong field of a hand-edited `agents.json`
   would else show in the error.
2. The left panel number of an API key profile is its own estimate only;
   the bill no longer gives it. Before the tokens are counted the row shows
   no number, even when the bill answered.
3. While the bill answers, the dashboard's table is titled "By model, this
   month (estimate at list prices)" (new text `agents_by_model_estimate`
   in all ten languages); with no bill it stays "By model, this month".

The `anthropic-version` header is unchanged.
