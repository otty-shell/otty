# Mouse wheel reports

## Problem

In a full-screen program that handles the mouse itself, the mouse wheel does
not scroll the program's output. In Claude CLI the wheel walks through the
prompt history instead. The same happens in any program that asks for mouse
events and treats the arrow keys as something other than scrolling.

Other terminals scroll the output of these programs correctly.

## Cause

A program can ask the terminal to report mouse events to it ("mouse
reports"). The wheel is part of that: the terminal must send it as mouse
button 64 (wheel up) or 65 (wheel down), and the program scrolls its own
output.

OTTY reports clicks and mouse movement, but never the wheel:

- `handle_wheel_scrolled` in `crates/ui/terminal/src/input.rs` always
  publishes `Event::Scroll`. It does not look at whether the program asked
  for mouse reports. The handlers for clicks and movement do.
- `Engine::scroll_delta` in `crates/ui/terminal/src/engine.rs` then sees the
  alternate screen (the separate screen without history that full-screen
  programs draw on) together with the "alternate scroll" mode, which OTTY
  turns on by default, and sends the arrow keys `ESC O A` / `ESC O B`.
- Claude CLI reads the arrow keys as "previous / next prompt".

The buttons `MouseButton::ScrollUp = 64` and `MouseButton::ScrollDown = 65`
already exist, and `Engine::process_mouse_report` already encodes them
correctly. Nothing sends them.

## Goal

The wheel does what the program asked for:

| Program state | Wheel does |
| --- | --- |
| Asked for mouse reports | Sends a wheel report to the program |
| No mouse reports, alternate screen, alternate scroll on | Sends arrow keys (as today) |
| Anything else | Scrolls OTTY's own history (as today) |

## Decisions

- **Mouse reports win over arrow keys.** This is the order xterm, Alacritty,
  kitty and WezTerm use. The check is the same one clicks use:
  `SurfaceMode::MOUSE_MODE`.
- **One report per scrolled line.** A wheel step of three lines sends three
  reports, so the program scrolls as far as OTTY would have.
- **The wheel has no release.** Only the "pressed" report is sent, as in
  other terminals.
- **The report carries the cell under the mouse pointer and the held
  modifier keys**, the same way click reports do.
- **The decision is made where the wheel event arrives.** `Event::Scroll`
  is also published by the view to bring a block into sight
  (`crates/ui/terminal/src/view.rs`). That is not a wheel event and must
  never turn into a mouse report.

## Out of scope

- Shift + wheel to scroll OTTY's history while a program holds the mouse.
- Which arrow key form the alternate scroll sends (`ESC O A` or `ESC [ A`).
- Reports for the middle and right buttons.

## Issues

- [01 report the wheel to programs that ask for mouse events](issues/01-report-wheel-as-mouse-event.md)
