# 46 — Fuzzy suggestions with `frizbee`

Status: resolved
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

## Answer

`rank` in `crates/ui/kit/src/components/tag_search.rs` now uses `frizbee` 0.13,
added to `otty-ui-kit` only. Filtering rows is still a substring match.

- Case is ignored. Both the query and the candidates are lower-cased first
  with `FoldedText`, because `frizbee` folds only ASCII case.
- The letters of the query may be apart in the candidate (`cntnr` finds
  `container`). The best score comes first.
- A query of up to three letters allows no typo; a longer one allows one.
- An empty query keeps the given order and has no highlight.
- Highlights: `frizbee` reports matched bytes; `FoldedText::originals_of` maps
  them back to byte ranges of the shown text and joins letters that touch.
- Keys still return to column order after ranking.
- Build: no required dependencies. It picks a SIMD backend at run time and
  falls back to plain code, so no target-specific setup is needed. `Cargo.lock`
  gains only the `frizbee` entry (`serde` was already locked).
  `cargo deny check` is clean.
