# 22 — Logs Follow toggle

Status: resolved
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

## Comments

### Can the app see scrolling in `text_editor`? (iced 0.14, researched 2026-11-01)

Read in `iced_widget-0.14.0/src/text_editor.rs`, `iced_graphics-0.14.0/src/text/editor.rs`
and `cosmic-text-0.15.0/src/edit/editor.rs` and `buffer.rs`.

- **Scrolling up: yes.** A mouse wheel over the editor becomes
  `Update::Scroll(lines)` and is sent through `on_action` as
  `Action::Scroll { lines }` (`text_editor.rs`, `update`). A negative `lines`
  is up. `LogView` already passes every action to `LogViewState::perform`, so
  the kit can turn following off there. No widget change is needed.
- **Reaching the bottom: no.** The scroll position lives in cosmic-text's
  buffer (`buffer.scroll()`), behind `Content`'s private `Internal`. `Content`
  offers the text, the lines, the cursor and the selection, but no scroll
  offset, no visible lines and no layout size (`Editor::bounds` and
  `min_bounds` are not reachable from `Content`). The widget keeps no scroll
  state of its own beyond `partial_scroll`, and there is no `scrollable`-like
  `on_scroll` or `Viewport`. Adding up the `lines` of the scroll actions does
  not work either: cosmic-text stops the scroll at the top (and the editor
  layout at the end), so extra wheel turns are lost, and long lines wrap into
  several screen lines whose count depends on the panel width.
- **Cursor position** is not a scroll position: the wheel does not move the
  cursor, so it says nothing about what is shown.

So: scrolling up turns Follow off (kit `LogViewState::perform`), and
scrolling back down does not turn it on; the user presses Follow. Telling
"at the bottom" would mean replacing `text_editor` (for example with a
`scrollable` of selectable text, which iced 0.14 does not have) or a fork.

One more finding: moving the cursor to the end brings the bottom into view
only when the cursor really moves (cosmic-text `cursor_moved`). After a wheel
scroll the cursor is still at the end, so turning Follow on moves it to the
start and back to the end to jump to the bottom.

## Answer

The Logs tab header shows the hint, then (while Follow is off and lines
wait) a muted count such as "12 new lines", then a "Follow" switch, on by
default. Tests were written first and seen failing (kit: 2 of the new
`LogViewState` tests; app: 10 compile errors for the missing intent, keys
and helpers), then the code.

### Kit

- The toggle is the existing kit `Switch` (on/off that applies right away,
  with a label): `Segmented` is for picking one of several values and a
  `Button` does not show a state. `Switch` had a gallery section but no
  `iced_test`; it now has three (on asks for off, off asks for on, no message
  sends nothing, both themes). The gallery's LogView section uses the
  `Switch` labelled "Follow" with the waiting count instead of the old
  "Following: on" button (the theme toggle shows both themes).
- `LogViewState::perform`: a scroll up (`Action::Scroll` with negative
  `lines`) turns following off, so the user can read while lines arrive.
- `LogViewState::set_following(true)` always jumps to the bottom, also when
  no lines wait (see Comments: the cursor goes to the start and back to the
  end, because the editor scrolls only when the cursor moves).

### App

- `DockerIntent::LogFollowToggled(bool)` → `InspectorState::set_log_following`.
- `view/inspector.rs`: the Logs header row (hint, waiting count, `Switch`).
- Reset: nothing new was needed. A tab change, another selected container
  and a mode change already drop the followed logs and start new ones, and
  new logs start with Follow on; tests cover it.
- The 5000-line cap: waiting lines are capped at 5000 while Follow is off
  (the shown lines stay put, the view does not move); turning Follow on keeps
  the last 5000 of shown and waiting lines.
- i18n: `docker_logs_follow`, `docker_logs_one_new_line` and
  `tpl_docker_logs_new_lines` in the ten locales (one and many forms, like
  the status bar counts; Russian uses "Новых строк: {count}" to avoid three
  plural forms). `Key::ALL` is 348 after the rebase onto tickets 15 and 23.

### Tests

- Reducer: Follow is on when the logs open; off keeps the shown text and
  counts waiting lines; on adds them; scrolling up turns Follow off; the
  5000-line cap with Follow off (5100 waiting → 5000, then lines 100 to
  5099 shown); Follow is on again after a tab change, another selection and
  leaving and entering Docker mode; a toggle with no followed logs does
  nothing.
- `iced_test` view: pressing Follow sends `LogFollowToggled(false)`; with
  Follow off "12 new lines" shows and pressing sends
  `LogFollowToggled(true)`, in both themes; no count while following.
- i18n: one and many forms in English and Russian; the template is filled in
  every locale.
- Kit: scroll up stops following, scroll down keeps it, Follow on without
  waiting lines goes to the end; the three `Switch` tests.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 85.49% on
`7884d5c` → 85.57%. `cargo +nightly fmt`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo test --workspace
--all-features` and `cargo deny check` pass.

### Deviations

- Scrolling back to the bottom does not turn Follow on: iced 0.14 does not
  show the scroll position to the app (see Comments). The user presses
  Follow.
- A scroll up turns Follow off even when the log is short enough to fit
  (the app cannot tell whether the text overflows).
- No new kit component: the existing `Switch` is the toggle, the count is a
  muted hint line like the one beside it.
- Not checked by eye: the gallery and the app were not opened; the jump to
  the bottom is checked through the cursor, not pixels.
