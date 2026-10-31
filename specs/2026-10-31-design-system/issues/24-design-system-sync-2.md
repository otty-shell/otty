# 24 — Update the design system with the new kit parts

Status: resolved
Type: task
Model: Opus 5.5 (claude-opus-5-5), owner review before publishing
Blocked by: 23
Spec: [../spec.md](../spec.md)

## Changes

- `tokens.json`: color `scrim`; size `line` 1px (usage: rule width; name to avoid clashing with color `line`, e.g. `line-width`), `split-grab` 8px, `form` 720px, `dialog` 420px, `logo` 20px.
- New component docs: `Dialog` (README and preview), and a short note on `Divider`, `Surface` and `FormPage` in the README "In code" table.
- README Layout: form pages are `form` wide with `space-7` padding; dialogs dim the window with `scrim`.

## Answer

- Published on 2026-10-31 after the owner's review (design system version 8): `tokens.json`, `README.md`, new `components/Dialog/README.md` and `preview.html`, index `lastChange`.
- New tokens: color `scrim` (dark `rgba(0,0,0,0.5)`, light `rgba(17,20,28,0.16)`); sizes `line-width` 1px, `split-grab` 8px, `form` 720px, `dialog` 420px, `logo` 20px. Every name was checked against `otty_ui_kit`.
- README: rules for form pages, dialogs and splits; `scrim` in Color; "In code" lists `FormPage`, `Dialog`, `divider`, `split`, `surface`, `MONO_BOLD`, `scrim()` and the new size constants.
