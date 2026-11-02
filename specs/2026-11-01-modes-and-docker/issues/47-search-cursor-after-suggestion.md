# 47 — After a suggestion is applied the cursor goes to the end of the field

Status: resolved
Type: bug
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 46
Spec: [../spec.md](../spec.md)

## What

Owner report (2026-11-03). After applying a suggestion in the tag search field,
the text cursor stays at the start of the field, so the next letter lands before
the text: `status:` plus `r` becomes `rstatus:`.

The text changes from outside the input, and iced 0.14 leaves the cursor where it
was. After any change that replaces the text (a key, a name or a value applied by
Enter, Tab or a click), the cursor must be at the end of the text and the field
must keep focus, also after a click on a suggestion. Typing, ↑ and ↓ must not move
it.

Fix it so that every widget using `TagSearchField` gets this without its own code,
if iced allows it; otherwise the smallest step a widget must do, documented on the
kit type. Look at `text_input::move_cursor_to_end` and `focus` in iced 0.14, and
how `InputKeys` already wraps the input.

Add steps for tag search to `regression-checklist.md`: typing and Enter on a key, a
value and a name; the cursor at the end; a click on a suggestion; tags kept when
switching lists and back; a cross and Clear all; the Compose link; a typo in the
suggestions; the list over the tags row; Tab, Esc and the wheel in a long list.

Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo
llvm-cov` with the merge base before the commit.

## Done when

- `iced_test` on the real flow: apply a key by Enter, type `r`, and the text is
  `status:r`, not `rstatus:`. The same after a click on a suggestion, and after
  applying a name.
- Typing in the middle of a text keeps the cursor where it is.

## Answer

Fixed in the kit, so every widget using `TagSearchField` gets it with no code of
its own. `iced` 0.14 allows it: `text_input::State::move_cursor_to_end` is
public, and `InputKeys` already owns the input's widget state.

- `InputKeys::follow_text(text)` (opt-in, `input_keys.rs`). The wrapper
  remembers the text the app gave at the last rebuild. In `diff`, if the text
  changed and the input did not get a key since the last rebuild, the text was
  replaced from outside, so the cursor goes to the end. A key press or input
  method event in the focused input marks the change as its own edit; a mouse
  press or a bound key (Enter, Tab, arrows) clears that mark. So typing,
  pasting and Left/Right never move the cursor, and an edit in the middle stays.
- `TagSearchField` calls `follow_text(search.input())`. `TokenField` and
  `CommandPalette` do not, so they are unchanged.
- Focus: a press on a suggestion is already captured by the list, so the input
  keeps focus. A new test covers it with typing after the click.

Tests (`tag_search_field.rs`) run the real flow: a small harness keeps the
widget state between events, as `iced` does between frames, and feeds every
message back to the `TagSearch`. They failed first (`startus:`, `webx-1`). Cases:
key by Enter, by Tab and by click then `r` gives `status:r`; a name by Enter and
by click then `x` gives `web-1x`; typing in the middle (`abdc`); ↑ and ↓ keep the
cursor; a value by click leaves the field empty and focused.

Also added tag search steps 25 to 33 to `regression-checklist.md`.
