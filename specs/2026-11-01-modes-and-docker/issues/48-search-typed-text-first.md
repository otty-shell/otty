# 48 — The text you typed is the first suggestion

Status: ready-for-agent
Type: bug
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 47
Spec: [../spec.md](../spec.md)

## What

Owner report (2026-11-03). The search does not work as a filter: typing offers only
existing values, so a partial value cannot be applied to get everything containing
it. The first suggestion must be the text as typed. Rules (spec Decision 21 is
updated):

- `sta`: the first suggestion is `sta` itself, a name filter. Enter applies it: the
  text stays in the field, the table is filtered by name, the list closes. Below it
  come the matching keys, then the matching names.
- `status:ru`: the first suggestion is `status:ru` itself. Enter adds the tag
  `status:ru`, which keeps every row whose status contains `ru`, and clears the
  field. Below it come the matching values.
- `status:` with nothing after the colon, and an empty field: no typed-text row. The
  first real suggestion is highlighted, as today.
- When the typed text equals a suggestion, ignoring case, there is one row for it,
  not two.
- **Enter** applies the highlighted row. **Tab** completes to the first real
  suggestion below the typed-text row, and does what Enter does if there is none:
  `sta` Tab gives `status:`, `run` Tab gives the tag `status:Running`.
- ↓ moves from the typed-text row to the real ones.
- The typed-text row looks plainly different from a real suggestion, and works in
  `TagSearchField` without any widget code.

Update `regression-checklist.md` for the new Enter and Tab rules.

Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov`
with the merge base before the commit.

## Done when

- Model tests for each rule above, including no duplicate row, `redis:7` as a name
  filter, and an unknown key `foo:bar` as a name filter.
- `iced_test` on the real Docker flow: `status:ru` Enter adds the tag and shows only
  running containers; `otty` Enter filters by name with the text kept; `sta` Tab,
  `run` Tab adds `status:Running`.
