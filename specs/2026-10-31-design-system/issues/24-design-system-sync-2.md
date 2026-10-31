# 24 — Update the design system with the new kit parts

Status: ready-for-human
Type: task
Model: Opus 5.5 (claude-opus-5-5), owner review before publishing
Blocked by: 23
Spec: [../spec.md](../spec.md)

## Changes

- `tokens.json`: color `scrim`; size `line` 1px (usage: rule width; name to avoid clashing with color `line`, e.g. `line-width`), `split-grab` 8px, `form` 720px, `dialog` 420px, `logo` 20px.
- New component docs: `Dialog` (README and preview), and a short note on `Divider`, `Surface` and `FormPage` in the README "In code" table.
- README Layout: form pages are `form` wide with `space-7` padding; dialogs dim the window with `scrim`.
