# 51 — Icon buttons light up on the whole button; danger has a visible hover

Status: ready-for-agent
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
