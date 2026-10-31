# 03 — Two themes: add OTTY Light, remove other presets and the palette editor, store the theme name

Status: resolved
Type: task
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 02
Spec: [../spec.md](../spec.md)

## What

The app has exactly two themes, OTTY Dark and OTTY Light. The user picks one in
Settings; the settings file stores its name.

## Details

- Replace `SettingsPreset` (`app/src/widgets/settings/types.rs:612`) with a theme enum holding `OttyDark` and `OttyLight` only, with:
  - `name()` → `"otty-dark"` / `"otty-light"` (the stored value), `title()` → `"OTTY Dark"` / `"OTTY Light"`;
  - `terminal_palette()` → the 29-color `ColorPalette`;
  - `ui_palette()` → `UiPalette::otty_dark()` / `UiPalette::otty_light()`.
- Remove One Dark, Solarized Dark, Dracula, `palette_from_hexes`, `SettingsPreset::from_palette`, the palette text inputs (`palette_inputs`, `set_palette_input`, `SettingsIntent` variant for palette edits) and their tests and locale strings.
- Settings data: `theme.name` replaces `theme.palette` (spec Decision 3). Old files with only `theme.palette`, a missing or an unknown name → OTTY Dark. Saving writes `theme.name` only.
- `ThemeManager::set_custom_palette` becomes "set theme"; `AppTheme` gets the UI palette from the theme (ticket 02).
- Switching the theme in Settings rebuilds `AppTheme` and the terminal palette without a restart.
- Add the OTTY Light title to all 10 locale files only if theme titles are translated today; otherwise keep the titles as product names in English.

## OTTY Light terminal palette

Fixed by the design system (`tokens.json`): background, foreground, red, green,
yellow, blue, magenta, cyan, bright_black. The rest was chosen on 2026-10-31
with these rules: every normal and bright color except `bright_black` and `bright_white` reaches
4.5:1 on the background; `white` and `bright_white` are greys readable on a
light background; `dim_*` = 60% of the normal color + 40% background, which
gives about 2.5:1, the same as the dim colors of OTTY Dark; the four UI-facing
fields mirror their OTTY Dark roles.

| Field | Value | Contrast on `#fafbfc` | Source |
|---|---|---|---|
| foreground | `#2a2f3a` | 12.94 | design system |
| background | `#fafbfc` | — | design system |
| black | `#2a2f3a` | 12.94 | chosen |
| red | `#b23641` | 5.80 | design system |
| green | `#377028` | 5.77 | design system |
| yellow | `#8a6200` | 5.30 | design system |
| blue | `#1a64b0` | 5.80 | design system |
| magenta | `#8e3fae` | 5.84 | design system |
| cyan | `#0b6e79` | 5.76 | design system |
| white | `#5d6575` | 5.66 | chosen (= `text-muted`) |
| bright_black | `#8a92a2` | 3.02 | design system (ghost text) |
| bright_red | `#cb3e4b` | 4.68 | chosen |
| bright_green | `#2e7d32` | 4.95 | chosen |
| bright_yellow | `#946800` | 4.78 | chosen |
| bright_blue | `#1f6fc5` | 4.90 | chosen |
| bright_magenta | `#9c47bf` | 4.98 | chosen |
| bright_cyan | `#0a7581` | 5.23 | chosen |
| bright_white | `#7a8292` | 3.73 | chosen (= `text-faint`) |
| bright_foreground | `#11141c` | 17.77 | mirrors OTTY Dark: `text-strong` |
| dim_foreground | `#7a8292` | 3.73 | mirrors OTTY Dark: `text-faint` |
| dim_black | `#e9ebf0` | — | mirrors OTTY Dark: `surface-chrome` |
| dim_red | `#cf858c` | 2.74 | 60/40 mix |
| dim_green | `#85a87d` | 2.57 | 60/40 mix |
| dim_yellow | `#b79f65` | 2.48 | 60/40 mix |
| dim_blue | `#74a0ce` | 2.65 | 60/40 mix |
| dim_magenta | `#b98acd` | 2.67 | 60/40 mix |
| dim_cyan | `#6ba6ad` | 2.64 | 60/40 mix |
| dim_white | `#9ca1ab` | 2.50 | 60/40 mix |
| overlay | `#ffffff` | — | mirrors OTTY Dark: `surface-overlay` |

Changed by ticket 16.

## Tests first

- The theme enum lists exactly OTTY Dark and OTTY Light; OTTY Dark is the default.
- Spec AC3: for each theme, `ui_palette()` alias tokens equal the `terminal_palette()` colors (canvas = background, text-body = foreground, accent and focus = blue, status-ok = green, status-warn = yellow, status-error = red, target-local = cyan, target-ssh = magenta, target-staging = yellow, target-prod = red).
- OTTY Light terminal palette: the contrast rules above.
- Spec AC5: settings loading and saving cases.
- Update or delete the existing preset and palette-input tests in `settings/state.rs` and `settings/reducer.rs` so they describe the new behavior.

## Done when

- Settings › Appearance offers only OTTY Dark and OTTY Light; selecting OTTY Light redraws the terminal in light colors (the UI chrome follows after tickets 07–13).

## Answer

- `BuiltinTheme { OttyDark, OttyLight }` lives in `app/src/theme.rs` (not in settings, so the theme module does not depend on a widget); `ColorPalette::otty_dark()` / `otty_light()`; `AppTheme::new(BuiltinTheme)`; `ThemeManager::set_theme`.
- Settings file: `{"theme": {"name": "otty-dark" | "otty-light"}}`; old palette-only files load as OTTY Dark and are rewritten on the next save.
- Removed: other presets, palette editor, `settings/services.rs`, `palette_labels` in all locales and their keys.
- `otty_ui_kit::contrast::contrast_ratio` is now public: the app tests the OTTY Light terminal palette with it.
- The theme field label still uses the `FieldPreset` key ("Preset"); renamed in ticket 12.
