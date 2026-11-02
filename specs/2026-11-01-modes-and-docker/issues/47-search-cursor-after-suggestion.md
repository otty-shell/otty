# 47 — After a suggestion is applied the cursor goes to the end of the field

Status: ready-for-agent
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
