# 15 — Update the design system to match the code

Status: resolved
Type: task
Model: human (owner review)
Blocked by: 14
Spec: [../spec.md](../spec.md)

## What

Bring the design system artifact (https://claude.ai/artifact/7zMwx63cZnNmTUeTL1FQF3) in line with what was built. The owner reviews before it is published.

## Changes

- README "In code": crate is `otty-ui-kit` at `crates/ui/kit`; show the real `UiPalette` and component names.
- README "Principles" and "Themes": the app has two fixed themes, OTTY Dark and OTTY Light. Mark "Colors come from the active theme" for custom themes and the "Custom themes follow the same split … `UiPalette::derive`" rule as future work.
- `tokens.json`: add the full 29-color terminal palette of both themes (the 20 OTTY Light colors chosen in ticket 03 and the matching OTTY Dark ones from `ColorPalette::default`).
- README Themes section: the claim that light status colors pass 4.5:1 on `surface-chrome` is wrong (4.39); fix the claim or the colors.
- Raise the known token issues from the spec (muted on hover and selected, light status colors on chrome and raised) and fix the tokens if the owner agrees; then flip the expected failures in ticket 02's tests.
- Note the i18n decision: English source strings follow the writing rules; translations stay.

## Answer

- Published on 2026-10-31 after the owner's review (design system version 6, files `project/README.md`, `project/tokens.json`, `project/components/Themes/README.md`, index `lastChange`).
- `tokens.json`: all 29 terminal colors of both themes as `ansi-*` tokens (checked against `ColorPalette::otty_dark()` / `otty_light()`, 0 mismatches); values from ticket 16; new `tree-indent` 18px; size and radius notes point at `otty_ui_kit` constants instead of the removed `BUTTON_*` ones.
- README: two fixed themes picked in Settings › Appearance, the setting stores the theme name; "Terminal colors" rules for OTTY Light (including the `ansi-bright-white` exception); custom themes and `UiPalette::derive` marked as future work; fonts; i18n decision; "In code" rewritten with the real `otty-ui-kit` modules and components, the gallery command and the gaps (focus ring, ellipsis, tab scrolling).
- The owner chose to fix the known contrast issues instead of documenting them (ticket 16), so the design system has no known-issues section.
