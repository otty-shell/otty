# 21 — Bold mono font in the kit and small token fixes

Status: ready-for-agent
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 20
Spec: [../spec.md](../spec.md)

## Details

- Move `TERM_FONT_JET_BRAINS_BYTES` (`app/src/fonts.rs`, JetBrainsMono Nerd Font Mono Bold) to `crates/ui/kit/src/fonts.rs` as `JETBRAINS_MONO_BOLD_BYTES` plus `MONO_BOLD: Font` (weight Bold), next to `MONO`; load it where the app loads the other kit fonts.
- `TerminalFonts::default()` size: `typography::TERM.size()` instead of `typography::BODY.size()` (same 13 px today; the token was wrong).
- `app/src/widgets/explorer/view/sidebar_tree.rs` `TREE_PADDING_BOTTOM = 20` is off the spacing scale → `spacing::SPACE_6` (24).
- `app/src/geometry.rs` `MENU_MARGIN = 6.0` → `spacing::SPACE_2`.
- `app/src/widgets/chrome/view/action_bar.rs` `LOGO_HEIGHT = 20` → kit `size::LOGO: f32 = 20.0` ("Logo height in the title bar."); `LOGO_WIDTH` stays local (aspect ratio of this file).
- Visual change: 4 px more space under the file tree.
