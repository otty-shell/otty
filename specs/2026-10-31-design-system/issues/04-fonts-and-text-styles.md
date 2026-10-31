# 04 — Bundle fonts and switch UI and terminal text styles

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 01
Spec: [../spec.md](../spec.md)

## What

Ship the two font families the design system uses and make them the defaults.

## Details

- The owner approved the fonts on 2026-10-31 (spec Decision 5): Noto Sans Regular, Medium and SemiBold (SIL OFL 1.1) and JetBrainsMono Nerd Font Mono Regular (SIL OFL 1.1). Prefer the variable or subset build if it is clearly smaller, and report the added size in the PR.
- Files go to `assets/fonts/NotoSans/` and `assets/fonts/JetBrains/`; the license text goes next to them.
- `otty-ui-kit` gets `fonts.rs`: font byte constants via `include_bytes!` and `Font` values `UI`, `UI_MEDIUM`, `UI_SEMIBOLD`, `MONO`. The app loads them at startup where `TERM_FONT_JET_BRAINS_BYTES` is loaded today.
- `app/src/fonts.rs`: `UiFonts` default → Noto Sans, `body` size 13. `TerminalFonts` default → Regular weight, size 13.
- Settings do not store fonts today (`theme`, `general`, `terminal.shell/editor/equalize_panes` only), so only the defaults change.
- The Bold file stays only if something still uses it; otherwise remove it.

## Tests

None: font loading and defaults are wiring, not business logic (AGENTS.md).

## Done when

- The app opens with Noto Sans in the chrome and Regular JetBrains Mono in the terminal.

## Answer

- Noto Sans v2.015 static unhinted Regular, Medium, SemiBold (notofonts/latin-greek-cyrillic) and JetBrainsMono Nerd Font Mono Regular from nerd-fonts v3.5.1, each with OFL.txt; about 3.9 MB added.
- The Bold JetBrains file stays: the terminal draws bold cells with it, and `crates/ui/terminal/examples/fonts.rs` uses it.
- CJK: iced 0.14 / cosmic-text 0.15 still load system fonts and fall back by script (Noto Sans CJK names first, then any installed face), so CJK locales render as before when a CJK font is installed.
