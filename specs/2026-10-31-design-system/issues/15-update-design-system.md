# 15 — Update the design system to match the code

Status: ready-for-human
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
