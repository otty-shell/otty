# 14 — Focus mode note

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 04, 07
Spec: [../spec.md](../spec.md)

## What

In focus mode a kit `FloatingNote` sits at the top center of the main area:
"Focus mode · Panels are hidden · <shortcut> to show them", as in `V4Focus`.
It is an overlay layer in `view.rs` and does not take mouse input from the
terminal outside its own bounds.

## Done when

- Visible only in focus mode; strings in all 10 locales; screenshot in both themes (spec AC5).

## Answer

Added `app/src/widgets/chrome/view/focus_note.rs`: `view(focus_mode, theme)` returns `None` outside focus mode, and in focus mode a full-size `container` (padding top 12, centered horizontally) holding the kit `FloatingNote`: "Focus mode" (reuses `window_focus_mode`), muted "Panels are hidden", `AppShortcut::FocusMode.label()` as the key hint, muted "to show them". `view.rs` pushes it as a layer in the content stack, before the menus. The layer has no mouse area; iced's `Stack` only blocks lower layers when a layer reports a mouse interaction, and a plain container does not, so the terminal keeps its mouse input.

New keys `FocusNoteHidden` and `FocusNoteShow` (`ALL` is now 119) in all 10 locales.

Tests (written first; first run failed to compile): no note outside focus mode; in focus mode the four parts are found with `iced_test`.

## Comments

- Not verified: the look (no GUI); no screenshots taken (AC5 needs the owner).
- Regression checklist for the owner: (1) press Ctrl Shift F: both panels hide, note appears at top center, 12px from the top of the main area, in dark and light themes; (2) in focus mode click, select text, scroll and right-click the terminal beside the note: all work; (3) hover and click the note itself: nothing happens; (4) press Ctrl Shift F again: panels return, note disappears; (5) switch languages: note text is translated and the shortcut shows the platform keys; (6) open the add menu or a context menu in focus mode: the menu stays above the note.
- `cargo deny check`: ok. No new dependencies.
