# 51 — Icon buttons light up on the whole button; danger has a visible hover

Status: resolved
Type: bug
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 50
Spec: [../spec.md](../spec.md)

## What

Owner report (2026-11-03). Two visual bugs in the kit.

1. **The icon lights up only over the icon.** The icon's color comes from
   `svg::Status`, which is hovered only while the pointer is over the 16 px of the
   svg, not over the button around it. Pointing at the button's edge changes the
   background but not the icon. The icon must change as soon as the pointer is over
   any part of the button.

   This is not only `IconButton`: find every button in `crates/ui/kit/src/components/`
   that colors an svg by `svg::Status` (`mode_switcher`, `mode_menu`, `overflow_menu`,
   `split_button`, `notice`, `title_search`, `action_tile`, `data_table`, `data_tree`
   and any other) and fix them all through one kit-private piece, not one by one.
   Pressed and disabled keep today's rules. Icon sizes and sharpness must not change:
   do not stretch the svg to fill the button (see `ICON_SIZE` in `icon_button.rs`).

2. **Danger has no visible hover.** `IconTone::Danger` draws `surface-danger` while
   hovered, which nearly melts into the panel, and its icon barely changes. The hover
   must be plainly visible in both themes, on the surfaces danger buttons sit on
   (`surface-panel`, `surface-canvas`, `surface-raised`), through palette tokens only.
   Keep the 3:1 contrast of the icon and add a test that the hovered background really
   differs from those surfaces by a visible amount. Check `Button` with the danger
   style (the confirm dialogs) for the same problem.

Tests first, gallery update in both themes. Run fmt, clippy, tests, `cargo deny check`
and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Kit `iced_test` by picture: an icon button with the pointer on its corner, outside
  the svg, draws the hovered icon (same picture as with the pointer on the icon, and
  different from idle). The same for each other fixed component.
- Contrast and difference tests for the danger hover in both themes.

## Answer

Tests were written first and seen failing (7 picture tests, one per fixed
component), then the code.

### Icons light up over the whole button

One kit-private piece, `components/icon_hover.rs`: `IconHover`. It wraps a
control (a button, or the container that acts as one) in a small widget that
records whether the pointer is over the control's bounds, before it updates and
before it draws. `IconHover::icon_style(idle, hover)` gives the svg style that
reads that record, so the icon follows the whole control. The svg keeps its
size and position, so icon sizes and sharpness are unchanged. Pressed and
disabled keep their rules (the colors still come from each component).

Fixed through it: `IconButton` (so `SplitButton`'s primary part too),
`SplitButton` chevron, `ModeSwitcher` tabs, the `ModeMenu` pin toggle, the
`Notice` dismiss cross, the "⋯" of `OverflowMenu` (toolbar) and the "⋯" row
button of `DataTable`. `title_search`, `action_tile`, `data_tree` and the other
svgs ignore the status, so they had nothing to fix.

### Danger hover

New palette token `surface-danger-hover` (this is a token addition; the design
system file must get it too): dark `#4a2a33`, light `#f0bcc2`. Used by
`IconButton` danger, `Button` danger (hover and pressed; idle stays
`surface-danger`) and the danger item of `Menu`, which had the same problem.
Contrast against `surface-panel` / `surface-canvas` / `surface-raised`: dark
1.48 / 1.41 / 1.30, light 1.46 / 1.60 / 1.34 (`surface-danger` gave 1.16 / 1.10
/ 1.02 and 1.05 / 1.14 / 1.04). Idle danger icon on the hover background: dark
3.93, light 3.63 (needs 3). Hovered icon (`text-danger`) on it: dark 8.01,
light 5.28.
