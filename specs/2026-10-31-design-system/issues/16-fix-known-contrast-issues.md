# 16 — Fix the known contrast issues

Status: ready-for-agent
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 14
Spec: [../spec.md](../spec.md)

## What

The owner decided on 2026-10-31 to fix the token pairs listed in the spec's
"Known Token Issues" instead of keeping them as known exceptions. After this
ticket every pair in that table reaches 4.5:1 and ticket 02's "known issues"
test is gone.

## New values

| Token | Theme | Old | New | Why |
|---|---|---|---|---|
| `text-muted` | OTTY Dark | `#8b93a1` | `#939ba8` | 4.60 on `surface-hover` (was 4.16); same hue, a bit lighter, still clearly dimmer than `text-body` |
| `surface-hover` | OTTY Light | `#dde1e9` | `#e0e4eb` | `text-muted` on it reaches 4.59 (was 4.47); still visible on the white overlay (1.28:1) |
| `ansi-green` = `status-ok` | OTTY Light | `#3c7a2b` | `#377028` | Same hue, 8% darker: 5.01 on `surface-chrome`, 4.83 on `surface-raised`, 4.56 on `surface-hover`, 4.64 on `surface-selected` |
| `ansi-red` = `status-error` = `target-prod` | OTTY Light | `#c23b47` | `#b23641` | Same hue, 8% darker: 5.03 on `surface-chrome`, 4.85 on `surface-raised`, 4.58 on `surface-hover`, 4.65 on `surface-selected` |
| `dim_green` | OTTY Light | `#88ae7f` | `#85a87d` | Keeps the rule dim = 60% normal + 40% background |
| `dim_red` | OTTY Light | `#d8888f` | `#cf858c` | Same rule |

OTTY Dark `text-muted` is not part of the terminal palette. OTTY Light
`text-muted` is not changed because `ansi-white` equals it.

## Details

- `crates/ui/kit/src/palette.rs`: the four `UiPalette` values (`text_muted` in `otty_dark`; `surface_hover`, `status_ok`, `status_error`, `target_prod` in `otty_light`) and the expected hex tables in its tests.
- `app/src/theme.rs`: `ColorPalette::otty_light()` `red`, `green`, `dim_red`, `dim_green`.
- `specs/2026-10-31-design-system/issues/03-two-themes.md`: update the four rows of the OTTY Light table (new hex and contrast on `#fafbfc`: red 5.80, green 5.77, dim_red 2.74, dim_green 2.57) and add a line under the table: "Changed by ticket 16."
- Nothing else uses these values (checked with `grep`).

## Tests first

- Change the expected hex values in the token tables first, see them fail, then change the palette values.
- Replace `given_known_token_issues_when_contrast_checked_then_ratio_is_as_recorded` with passing tests:
  - in each theme, `text-muted` on `surface-hover` reaches 4.5:1;
  - in OTTY Light, `status-ok` and `status-error` reach 4.5:1 on `surface-chrome` and `surface-raised` (the OTTY Dark twin of this test already exists: `given_otty_dark_when_status_on_chrome_and_raised_then_reaches_4_5`; extend it to both themes rather than adding a copy).
- The existing AC3 alias tests (UI aliases equal the terminal palette) and the OTTY Light terminal contrast rules from ticket 03 must still pass without changes to their logic.

## Done when

- `cargo +nightly fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` and `cargo deny check` pass.
- No test or doc in the repo refers to a known contrast exception any more.
