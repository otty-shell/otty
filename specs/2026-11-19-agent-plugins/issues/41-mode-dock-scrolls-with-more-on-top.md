# 41 The mode dock scrolls sideways; More stays on top at its size

Status: done
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

2026-11-23, done. A new private kit widget, `ModeDock`
(`crates/ui/kit/src/components/mode_dock.rs`), holds the tabs of the
`ModeSwitcher` in a horizontal `scrollable` and draws More over its right
end. The row of tabs ends in a free space as wide as More, so the last
tab can be scrolled fully out from under it; in a wide dock that space is
where More sits, so nothing moves from before. More keeps its size, has the
dock color behind it and its 2 px gap, and takes the presses there, so a
tab under it is never pressed. A wheel or trackpad move over the dock
scrolls the tabs sideways along its larger part, one tab per notch of the
wheel. When the active tab changes (click, shortcut, More), the dock
scrolls just enough to show it left of More, as the floating list keeps
its highlighted row in view (it reuses `offset_showing`).

No bar: the kit's 4 px bar floats over the content, so in the 32 px dock it
would lie over the bottom of the tabs, and its right end, where the
scroller sits when scrolled to the end, would hide under More. A soft fade
(12 px, from the dock color) shows a cut edge instead: at the left edge
while tabs lie past it, and just left of More while tabs lie under it. The
`scrollable` itself is the only scrolling; its bar is hidden.

More still sends `ModesIntent::ToggleMenu`; the app test presses it at the
right end of a narrow left panel through `update_input`, and it opens and
closes the All modes menu.
