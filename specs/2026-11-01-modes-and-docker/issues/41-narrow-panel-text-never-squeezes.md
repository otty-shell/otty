# 41 — Text in narrow panels stays on one line, nothing overlaps

Status: resolved
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

## Answer

Tests were written first and seen failing (kit: 2 of 3 new `SessionGroup`
tests, 2 of 3 `SessionRow` layout tests, 2 of 3 `PanelSection` layout tests),
then the code.

### Kit

The rule of ticket 36 now holds in the three panel parts. Each uses the kit's
`EllipsisText`: the main text takes the room it needs first and ends with "…"
only when it truly does not fit; notes give way first, ending with "…" or
disappearing; everything is one line.

- `SessionGroup`: name first, note on the right takes what is left.
- `SessionRow`: title first, then the state word, the folder takes what is
  left. The room for the close cross is now right padding of the row, not a
  space at the end of the line (a space at the end was squeezed to nothing
  when the texts were wide). The title colour follows the selection as before.
- `PanelSection`: the title (and its caret) first, the count or note on the
  right takes what is left. Action buttons are laid out before the title, so
  they keep their size and the title never reaches them.
- `SplitButton`, `IconButton`, `TreeRow` and the other rows needed no change:
  they have fixed sizes or already follow the rule.
- Gallery: a "PanelSection narrow" section (170 and 128 wide), in both themes.

### Tests

- Kit `iced_test`, comparing bounds, for each of the three parts at 170 px, at
  the 128 px minimum and at a very narrow width: label keeps its width, note
  shrinks and stays one line, no overlap, text stays clear of the action or
  the close cross, the action keeps its size and place and still sends its
  message.
- App: a long folder name in the Files panel at 128 px stays inside the panel.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 87.99% (71373
lines, 8572 missed) -> 88.03% (71538 lines, 8563 missed). fmt, clippy,
workspace tests and `cargo deny check` pass.

### Notes and limits

- At 128 px the section titles "Sessions" and "Launches" end with "…" ("Ses…",
  "Lau…"): the caret, the plus and the chevron leave about 38 px, and the
  title does not fit. One line and no overlap hold; the whole title returns from
  about 165 px.
- The Files panel folder line (`app/.../explorer/view/sidebar_tree.rs`) is cut
  at the panel edge without "…": `EllipsisText` is private to the kit. It does
  not wrap or overlap. A public kit piece for it was left out as a separate
  decision.
- Snapshots (throwaway, not committed): `scratchpad/shots41/panel-128-dark-wgpu.png`,
  `panel-128-light-wgpu.png`, `panel-170-dark-wgpu.png`, `panel-170-light-wgpu.png`.
