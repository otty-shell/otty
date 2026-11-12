# 01 Report the wheel to programs that ask for mouse events

Status: needs-triage

## Why

In Claude CLI and other full-screen programs that handle the mouse, the
wheel walks through the prompt history instead of scrolling the output. OTTY
sends arrow keys where the program expects a wheel report. See
[the spec](../spec.md) for the cause.

## How to see it

1. Open a terminal tab in OTTY and start Claude CLI in its full-screen mode.
2. Produce enough output to fill more than one screen.
3. Turn the wheel up over the output.

Now: the prompt shows earlier inputs. Expected: the output scrolls up.

## What to do

1. Write the tests first, next to the existing `handle_wheel_scrolled_tests`
   in `crates/ui/terminal/src/input.rs`:
   - mouse reports on, wheel up by 3 lines: three `Event::MouseReport` with
     `MouseButton::ScrollUp`, `pressed: true`, the current grid position and
     the held modifiers; no `Event::Scroll`;
   - mouse reports on, wheel down: the same with `MouseButton::ScrollDown`;
   - mouse reports off: one `Event::Scroll`, as today (the existing tests
     cover it; keep them passing);
   - mouse reports on, a pixel delta smaller than one line: nothing is
     published.
2. Pass the terminal snapshot into `handle_wheel_scrolled`
   (`handle_mouse_event` already has it). When the mode intersects
   `SurfaceMode::MOUSE_MODE`, publish one `Event::MouseReport` per scrolled
   line instead of `Event::Scroll`. Otherwise keep publishing
   `Event::Scroll`.
3. Leave `Engine::scroll_delta` and `Engine::process_mouse_report` as they
   are. The first keeps serving the arrow-key and history cases, the second
   already encodes buttons 64 and 65 in both report formats.
4. Add one engine test in `crates/ui/terminal/src/engine.rs` that pins the
   bytes of a wheel report: `ESC [ < 64 ; column ; line M` in the SGR format
   (the modern format with decimal numbers).

## Done when

- In Claude CLI in full-screen mode the wheel scrolls the output, and the
  prompt history does not move.
- In `htop`, `lazygit` and `vim` with `set mouse=a` the wheel scrolls the
  list or the buffer.
- In `less` and `man` (no mouse reports) the wheel still scrolls by sending
  arrow keys.
- At the shell prompt the wheel still scrolls OTTY's history.
- Bringing a block into sight still scrolls OTTY's history and sends nothing
  to the program.

## Open questions

- `handle_wheel_scrolled` turns line deltas (a wheel) and pixel deltas (a
  touchpad) into lines with opposite signs: `lines = y` for lines, but
  `scroll_pixels -= y` for pixels. This was noticed while reading the code
  and not checked on a touchpad. Check the direction on a touchpad before
  choosing which sign means `ScrollUp`; if the pixel path is inverted, fix
  it here.
- Should Shift + wheel scroll OTTY's history while a program holds the
  mouse? Left out of this issue; the owner decides whether it is wanted.
