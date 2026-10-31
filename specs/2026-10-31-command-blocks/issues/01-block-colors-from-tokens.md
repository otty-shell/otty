# 01 — Block colors from tokens

Status: ready-for-agent
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: —
Spec: [../spec.md](../spec.md)

## What

The terminal draws block chrome with `bright_foreground` at 10% opacity.
Take the colors from the theme's tokens instead (spec question 2, recommended
answer).

## Details

- `crates/ui/terminal/src/theme.rs`: replace `block_highlight` with named block colors in the terminal theme settings: `block_selected`, `block_hover`, `block_line`. Defaults for embedders without a kit: today's look.
- `app/src/theme.rs`: fill them from `UiPalette`: `surface_selected()`, `surface_header()`, `line()`.
- `crates/ui/terminal/src/view.rs`: draw the selected fill, the hovered fill and the line between blocks with these colors, no alpha changes; line thickness 1 px (the app's `size::LINE`; the crate keeps its own constant with a doc comment pointing to the token). Draw fills under cell backgrounds and text; check with `ls --color` and a TUI (`htop`) that colored cells stay visible.
- Make the hovered action button background visible (`block_hover`, not alpha 0).

## Tests first

- App: for each theme, the terminal theme's block colors equal the matching `UiPalette` tokens.

## Done when

- Spec AC1; screenshots of a selected and a hovered block in both themes.
