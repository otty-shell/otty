# 17 — Status colors pass 4.5:1 on every surface they can sit on

Status: ready-for-agent
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 16
Spec: [../spec.md](../spec.md)

## What

After ticket 16, OTTY Light `status-warn` (yellow) still misses 4.5:1 on
`surface-raised` (4.43), `surface-hover` (4.30) and `surface-selected` (4.25).
The owner decided on 2026-10-31:

- darken OTTY Light yellow;
- status colors are never placed on `surface-hover` (it is used only for the
  hovered menu item and the selected segment). This rule is why OTTY Dark
  `status-error` on `surface-hover` (4.03) is not a defect and stays as is.
- `target-*` colors are dots, not text, so they need 3:1, not 4.5:1.

## New values

| Token | Theme | Old | New | Contrast after |
|---|---|---|---|---|
| `ansi-yellow` = `status-warn` = `target-staging` | OTTY Light | `#8a6200` | `#835d00` | chrome 4.99, raised 4.80, selected 4.61, canvas 5.74 |
| `dim_yellow` | OTTY Light | `#b79f65` | `#b39c65` | 2.58 on the background (rule: 60% normal + 40% background) |

`bright_yellow` `#946800` (4.78 on the background) stays lighter than the new
yellow, as before.

## Details

- `crates/ui/kit/src/palette.rs`: `status_warn` and `target_staging` in `otty_light`, and the expected hex table in its tests.
- `app/src/theme.rs`: `ColorPalette::otty_light()` `yellow` and `dim_yellow`.
- `specs/2026-10-31-design-system/issues/03-two-themes.md`: rows `yellow` (`#835d00`, 5.74) and `dim_yellow` (`#b39c65`, 2.58); extend the line under the table to "Changed by tickets 16 and 17."
- Nothing else uses these values (checked with `grep`).

## Tests first

- Change the expected hex values first, see them fail, then change the values.
- Extend `given_each_theme_when_status_on_chrome_and_raised_then_reaches_4_5` to all three status colors (`status-ok`, `status-warn`, `status-error`) and to `surface-selected` as well; rename it to say what it checks now (for example `given_each_theme_when_status_on_chrome_raised_and_selected_then_reaches_4_5`). Its doc comment states the rule that status colors never sit on `surface-hover`.
- Add one test: in each theme every `target-*` color reaches 3:1 on `surface-chrome`, `surface-panel`, `surface-canvas`, `surface-raised`, `surface-overlay`, `surface-hover` and `surface-selected` (dots are non-text graphics, WCAG 1.4.11).
- The AC3 alias tests and the OTTY Light terminal contrast rules from ticket 03 must still pass without changes to their logic.

## Done when

- `cargo +nightly fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` and `cargo deny check` pass.
