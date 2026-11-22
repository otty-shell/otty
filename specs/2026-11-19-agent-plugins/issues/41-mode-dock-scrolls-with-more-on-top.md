# 41 The mode dock scrolls sideways; More stays on top at its size

Status: ready
Blocked by: 39

## Why

Owner, 2026-11-23: when the mode dock (the row of mode icons, kit
`ModeSwitcher` in `crates/ui/kit/src/components/mode_switcher.rs`, used
by `crates/app/features/modes/core/src/view/mode_switcher.rs`) has too
little room, its tabs and the "⋯" More button start to shrink. The owner
wants the mode tabs to scroll sideways instead, with More kept on top of
them (overlaying the right end of the row) at its own size, never
shrinking.

## What to build

1. **Tabs keep their size.** No tab and not More ever shrinks below its
   normal size, whatever room the dock has.
2. **The tabs scroll sideways.** When the tabs do not fit, the row of tabs
   scrolls left and right: by the mouse wheel (vertical wheel scrolls it
   sideways too) and a trackpad's sideways move, with the kit's thin
   scrollbar (or none, if a bar does not fit the dock's look; then a soft
   fade at the cut edge shows there is more). The active tab is scrolled
   into view when the mode changes (by click, shortcut or More).
3. **More stays on top.** The More button stays fixed at the right end of
   the dock, drawn over the scrolling tabs, with the dock's background
   behind it so the tabs pass under it cleanly; the last tab can be
   scrolled fully out from under it (the scroll range leaves room for
   More). More opens its menu as today.
4. This is a kit change (`ModeSwitcher`), reusing the kit's `scrollable`
   style and `scrollbar` helpers; no second scrolling mechanism. Keep the
   existing tests' behaviour (temporary tabs, tooltips, shortcuts).

## Done when

- Tests first: in a narrow dock every tab and More keep their normal
  width; the tabs scroll sideways and the wheel moves them; More stays at
  the right end over the tabs and its bounds do not change with the
  room; the active tab is brought into view after a mode change; in a
  wide dock nothing scrolls and nothing changes from today.
- The checks of `AGENTS.md` pass.

## Comments
