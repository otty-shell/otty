# 33 — Opening or activating a tab switches to Terminal mode

Status: resolved
Type: bug
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 31
Spec: [../spec.md](../spec.md)

## What

Bug reported by the owner (2026-11-02): in Docker mode, pressing Settings in
the title bar does nothing visible. The Settings tab opens, but tabs only
show in Terminal mode (spec Decision 5), and nothing switches the mode back.
The same happens for every other way to open or activate a tab from Docker
mode: Ctrl , / Cmd ,, the command palette (Settings, a session, a launch,
New terminal, New launch), New terminal (Ctrl Shift T), running a launch,
opening the launch editor. Only Shell (ticket 15) switches today, by hand.

Fix it in one place, not per caller: when the tabs widget opens a new tab or
makes a tab active while another mode is shown, the app switches to Terminal
mode (`ModesIntent::Select(Mode::Terminal)`), which also syncs the terminal
grid and focus (ticket 06). Find the single point where tabs report "opened"
or "activated" (an effect or the tabs event handler in `app/src/events/tabs.rs`)
and add the switch there. Shell's own switch then becomes redundant; remove it
if it is, keeping Shell's behaviour and tests.

Tests first. Run fmt, clippy, tests, `cargo deny check` and compare
`cargo llvm-cov` with the merge base before the commit.

## Done when

- Tests, from Docker mode: Settings from the chrome button, the shortcut and
  the palette; New terminal; activating an existing session from the palette;
  running a launch — each ends in Terminal mode with that tab active.
- In Terminal mode the same actions do not emit a mode change.
- Closing a tab does not switch modes.

## Answer

Tests were written first and seen failing (7 of 10), then the fix.

- Fix: `handle_effect` in `app/src/events/tabs.rs`. Every opened-tab effect (terminal, command, settings, wizard, error) and `Activated` now also sends `ModesIntent::Select(Mode::Terminal)` when the active mode is not Terminal. Every caller (title bar button, shortcuts, palette, launches, launch editor, Shell) ends in these effects, so none needed a change. In Terminal mode nothing extra is sent; `Closed` and the other effects never switch.
- Shell: its own `Select(Terminal)` in `app/src/events/docker.rs` was redundant and is removed. Shell still opens one command tab and ends in Terminal mode through the tabs handler. Its handler test now checks only the opened tab; the mode switch is covered by the running-a-launch test, which uses the same command-tab path.

Tests (in `events/tabs.rs`, from Docker mode): Settings opened, Settings activated again, New terminal, activating a session, running a launch, the launch editor, an error tab - each emits exactly one switch to Terminal mode. In Terminal mode the same actions emit none. Closing a tab from Docker mode emits none.

Deviations: the Shell handler test no longer asserts the mode switch (it moved to the tabs handler tests). The palette, shortcut and chrome paths are covered through their tab intents, which existing handler tests already check.
