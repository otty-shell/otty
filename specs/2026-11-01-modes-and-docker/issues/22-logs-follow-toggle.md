# 22 — Logs Follow toggle

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 11, 13
Spec: [../spec.md](../spec.md)

## What

Ticket 13 shipped a Logs tab that always follows: while a busy container
writes, the user cannot scroll up and read. Add a Follow toggle (spec
Decision 12, owner approved 2026-11-01). Tests first.

- A small toggle in the Logs tab header, next to the "Last 500 lines…" hint,
  on by default. It is a kit part (gallery section in both themes and an
  `iced_test`), per the owner's kit-first rule; check `Switch`, `Segmented`
  and `Button` in `crates/ui/kit` first.
- On: new lines are added and the view stays at the bottom (today).
- Off: the view does not move; new lines wait in `LogViewState` (it already
  holds them back while not following, `pending_count`). Show how many lines
  wait, for example "12 new lines".
- Turning Follow on adds the waiting lines and jumps to the bottom.
- Scrolling: research first and note the answer in `## Comments`. `LogView`
  is built on iced's `text_editor`, which draws no scrollbar and may not
  report its scroll position. If the app can see that the user scrolled up,
  that turns Follow off, and reaching the bottom turns it on. If it cannot
  without replacing the widget, keep the toggle only and say why.
- The 5000-line cap still holds while Follow is off (waiting lines count).
- Changing the tab, the selected container or the mode resets Follow to on.

All new text goes through `i18n` in the ten locales (spec Decision 17). Run
fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the
merge base before the commit.

## Done when

- Reducer tests: toggle off keeps the view and counts waiting lines; toggle
  on adds them; the line cap with Follow off; reset on tab, selection and
  mode change.
- `iced_test`: pressing the toggle emits its intent; the waiting counter shows.
- Kit: the toggle's gallery section and `iced_test`.
