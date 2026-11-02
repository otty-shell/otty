# 41 — Text in narrow panels stays on one line, nothing overlaps

Status: ready-for-agent
Type: bug
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 40
Spec: [../spec.md](../spec.md)

## What

Owner report with a screenshot (2026-11-02,
`/tmp/claude-1000/-home-kemokempo-projects-main-ottyrun-otty/01492649-6afb-4868-b239-d18c47b01579/images/2.png`).
In a narrow left panel in Terminal mode:

- the machine group cuts its name ("fed") and wraps its note ("this" /
  "computer") to two lines;
- the section title "Sessions" runs into its "+" button.

Text must not squeeze or wrap in rows and headers of a panel. Apply the rule of
ticket 36 (`TreeRow`) to every kit part with the same layout, not only these two:
the main label takes the room first and ends with "…" only when it truly does
not fit; notes and counts give way first, ending with "…" or hiding; everything
stays on one line; action buttons keep their size and never overlap text.

Check at least `SessionGroup`, `SessionRow`, `PanelSection` (title, caret,
actions, `SplitButton`) and any other panel row or header in
`crates/ui/kit/src/components/`, and how the app uses them in
`app/src/widgets/tabs/view/` and `app/src/widgets/quick_launch/view/`. Use the
kit's `EllipsisText`. Docker mode and the Files panel must stay correct too.

Tests first. Gallery updates in both themes. Run fmt, clippy, tests, `cargo deny
check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Kit `iced_test` for each part at a narrow width: the label stays one line tall,
  the note gives way before the label, and text bounds do not overlap the
  action buttons (compare bounds), which still emit their messages.
- A snapshot of the left panel at the panel's minimum width in Terminal mode,
  Dark and Light, looks right.
