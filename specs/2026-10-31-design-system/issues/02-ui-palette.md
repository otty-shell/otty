# 02 — UiPalette for OTTY Dark and OTTY Light, contrast tests

Status: resolved
Type: task
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 01
Spec: [../spec.md](../spec.md)

## What

One struct, `UiPalette`, that holds every color token by role. Widgets will
read colors only from it. There are two fixed instances, one per theme.

## Details

- `crates/ui/kit/src/palette.rs`:
  - `UiPalette::otty_dark()` and `UiPalette::otty_light()` — every color token from `tokens.json`, including the ANSI aliases written out as plain values. No input, no computation.
  - Getters named after tokens: `surface_chrome()`, `surface_panel()`, `surface_canvas()`, `text_muted()`, `accent()`, `on_accent()`, `status_error()`, `target_ssh()`, `line()`, `diff_add()`, …
  - `shadow_popover()` and `shadow_sticky()` returning `iced::Shadow`.
- `crates/ui/kit/src/contrast.rs` — `contrast_ratio(a, b) -> f32` (WCAG 2.1). Used only by tests today; keep it `#[cfg(test)]` unless a component needs it.
- In `app/src/theme.rs`: `AppTheme` keeps a `UiPalette` chosen by the theme (ticket 03 adds the theme name); until then OTTY Dark. Add the `AppTheme::ui()` getter.

## Tests first (AGENTS.md)

- Table test: every token for both themes equals its hex in `tokens.json` (spec AC2).
- Contrast matrix (spec AC4). Record the pairs from the spec's "Known token issues" as explicit expected failures, so a later fix of the token flips the test on purpose.
- `contrast_ratio` against two known pairs (black on white = 21, equal colors = 1).
- AC3 (aliases equal the terminal palette) is tested in ticket 03, where both terminal palettes exist.

## Done when

- Tests pass; `AppTheme::ui()` is available; no widget uses it yet.
