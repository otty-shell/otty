# 19 — Settings as one page

Status: resolved
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

## Answer

Settings is now one page: `FormPage` with the title, Reset and Save on top, then
Appearance (Theme), General (Language) and Terminal (Shell, Default editor,
Equalize panes). Every setting is a `SettingRow` with a muted description. The
section tree on the left is gone. Save and Reset work as before (no auto-apply).

- `Shell` and `Default editor` are `TextField`s inside a `SettingRow`. The kit
  field always drew a label row, so `field_chrome::labelled` now skips the label
  row when the label is empty (one small kit change, no visible change for
  fields that have a label). It was needed because a `SettingRow` already shows the name, and a second label above the input would double it; the default (non-empty label) output is unchanged.
- i18n: new keys `desc_theme`, `desc_language`, `desc_shell`,
  `desc_default_editor`, `desc_equalize_panes`, in all 10 locales.
  `option_equalize_panes` is now the short title ("Equalize panes");
  `field_pane_layout` was removed (unused).
- Removed because only the tree used them: `SettingsSection`, `SettingsNodeKind`,
  `SettingsNode`, the `NodePressed` / `NodeHovered` intents and their reducer
  branches, the tree / selection / hover fields of `SettingsState`, and
  `SettingsViewModel`'s tree fields. Their tests were removed with them.

Tests (written first, seen failing against an empty view, then passing), in
`app/src/widgets/settings/view/settings_form.rs`: Save click sends
`SettingsIntent::Save`; Save on a clean draft sends nothing; Reset click sends
`Reset`; clicking "OTTY Light" sends `ThemeSelected(OttyLight)`; all three
sections and the five descriptions are visible. The i18n coverage tests check
the new keys in every locale.

Screenshots in both themes were not taken (no display in the agent run); the
owner should look at the page in both themes.

## Comments

- Class C for the removed tree part (`SettingsSection`, `SettingsNode*`, tree
  intents, their reducer branches and tests).
- Owner must walk regression checklist steps 19 (Settings tab opens, a second
  click re-activates it, Shell / Default editor saved and kept after restart),
  20 (theme switch and Save) and 21 (language switch and Save, then back).
  Also check that Reset and Save are disabled while nothing changed, the Equalize
  panes switch still saves, and the page in OTTY Light.
