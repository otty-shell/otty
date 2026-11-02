# 46 — Fuzzy suggestions with `frizbee`

Status: ready-for-agent
Type: task
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 45
Spec: [../spec.md](../spec.md)

## What

The owner approved the new dependency (2026-11-02): `frizbee` 0.13, MIT, no
required dependencies, a SIMD fuzzy matcher that tolerates typos.

Replace the body of the private ranking function of ticket 43 with `frizbee`, in
`otty-ui-kit` only. Only suggestions change: filtering the table stays substring.
Check `cargo deny check`.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Ranking tests: a typo still finds the item; a better match comes first; an
  empty query keeps the given order.
