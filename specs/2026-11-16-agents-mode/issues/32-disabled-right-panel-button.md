# 32 The right panel button looks pressable in Agents mode

Status: ready-for-agent
Blocked by: 31

## Why

In Agents mode the right panel does not exist (issue 06), and the title bar
button that toggles it does nothing. The owner's screenshot (2026-11-17,
OTTY Light) shows that button drawn like the other title bar buttons, so it
looks pressable. The design system says a disabled icon is faded to 45 %
and does not light up under the pointer; the canvas draws it that way.

The kit's `IconButton` already fades a button that has no message
(`crates/ui/kit/src/components/icon_button.rs`, test
`given_disabled_button_then_icon_is_faded_and_does_not_brighten`), and the
title bar builds this button without a message in Agents mode
(`crates/app/otty/src/frame/title_bar/view.rs`, `toggle_button`,
`right_panel_enabled`). So either the button still gets a message at run
time, or the faded colour is too close to the idle one to see. Find which.

## What to do

1. Find the cause: check what `right_panel_enabled` is at run time in Agents
   mode (follow it from `view.rs` of the binary), and compare the faded
   colour with the idle one in both themes.
2. Fix it so that in Agents mode the button is drawn at 45 % of its idle
   colour, does not brighten under the pointer, shows no hover background,
   and its tooltip says the Files panel is not used in this mode. If the
   kit's fade is the problem, fix it in the kit with its test, and check the
   other disabled icon buttons still look right.

## Done when

- Tests, written first: in Agents mode the title bar's right panel button
  has no message and its icon colour is the faded one in both themes; in
  Terminal mode it is unchanged.
- Run by hand: in Agents mode the button is visibly fainter than its
  neighbours and nothing happens under the pointer.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments
