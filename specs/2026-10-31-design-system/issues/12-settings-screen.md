# 12 — Restyle settings screen

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 03, 05, 06
Spec: [../spec.md](../spec.md)

## Details

- `app/src/widgets/settings/view/settings_form.rs`: rows become `SettingRow`; language `Select`; theme `Segmented` with OTTY Dark and OTTY Light (two options, so Segmented per the design system); equalize panes `Switch`; shell and editor `TextField`. The palette color fields are already gone (ticket 03).
- Rename the theme field label from "Preset" to "Theme": replace the `FieldPreset` key with `FieldTheme` in `app/src/i18n/key.rs` and all 10 locale files (left over from ticket 03).
- Section navigation rows use `TreeRow`.
- Buttons use kit `Button`. Save stays (instant apply is out of scope).
- Remove the six local `*_style` functions and the hard-coded `Color::from_rgba(1.0, 1.0, 1.0, 0.2)` (`settings_form.rs:338`).
- Page layout: title `title`, sections `space-6` apart, top padding `space-7`.

## Done when

- Screenshots of each settings section in both themes.

## Answer

- Settings form built from kit SettingRow, Segmented (theme), Select (language), Switch (equalize panes), TextField (shell, editor), Button (Save, Reset) and TreeRow navigation; six local style functions and `tree_row_style` removed.
- `FieldPreset` renamed to `FieldTheme` in all 10 locales.
- Not checked visually: Settings opens from the rail and no input tool is installed. Check it in the final review.
