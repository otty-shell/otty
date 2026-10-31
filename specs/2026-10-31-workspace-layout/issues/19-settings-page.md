# 19 — Settings as one page

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 07
Spec: [../spec.md](../spec.md)

## What

`widgets/settings/view/settings_form.rs` as in `V4Settings`: one `FormPage`
with sections Appearance (Theme), General (Language), Terminal (Shell, Default
editor, Equalize panes), each a `SettingRow` with its muted description. The
section tree on the left is removed. Save and Reset stay where they are today
in the new look (no auto-apply, spec Out of Scope).

- If `SettingsSection` / `SettingsNodeKind` exist only to drive the removed tree, remove them and their reducer branches with their tests updated; note this in Comments (Class becomes C for that part).

## Done when

- Settings flows of the regression checklist pass; descriptions in all 10 locales; screenshots in both themes.
