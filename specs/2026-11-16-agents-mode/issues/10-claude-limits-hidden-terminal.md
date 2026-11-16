# 10 Claude limits from the hidden terminal

Status: ready-for-agent
Blocked by: 07

## Why

The HTTPS request of issue 07 needs the token file. It is missing when the
login is kept elsewhere (the macOS Keychain) and can be empty on Linux: the
owner's second profile, `~/.claude-semrush`, has an empty credentials file.
Claude Code can still show the limits itself, with its `/usage` command.
This is the second source for Claude limits (spec Decision 5), the same
fallback CodexBar has.

## What to do

Study CodexBar's Claude CLI probe first (`docs/claude.md`, section "CLI
PTY", and the `ClaudeCLISession` code it names) and follow its logic.

1. Start `claude` in a pseudo-terminal with `otty-pty`, not in a tab, with:
   - the profile's `CLAUDE_CONFIG_DIR`;
   - a working folder of its own inside OTTY's cache folder, so the probe
     never shows up as a session of a real project;
   - `--allowed-tools ""`, `--strict-mcp-config` and
     `--settings '{"remoteControlAtStartup":false,"disableAllHooks":true}'`,
     so it can run nothing;
   - a wide and tall screen (CodexBar uses 160 by 200), so the panel is not
     cut.
2. Feed its output to OTTY's own terminal engine to get the screen as text.
   Send `/usage`, wait for the panel, read it, close it with Escape and end
   the program.
3. Read from the text the "Current session" and "Current week" blocks: the
   percent, whether it says used or left, and the reset text. Turn them
   into `Limits`. The reading is one pure function from the screen text, so
   it is tested without a terminal.
4. Failures: the program is not found; it asks to log in or shows a
   question instead of the prompt ("not logged in" or "not available" —
   never answer the question for the user); the panel does not appear in
   time; the text is not what the reading expects ("unexpected answer").
   Always stop the program, also on a time limit.
5. Remove the session files the probe left in the profile's `projects`
   folder for the probe's working folder, and only those.
6. This source is slow. It runs only when the first source failed, and not
   more often than the five minutes of issue 07.
7. `ClaudeCode::limit_sources()` returns HTTPS first, this one second.

## Done when

- Tests, written first, cover the reading of the screen text: both blocks;
  one block only; "used" and "left"; extra blocks and hints around them; a
  login screen; an unknown layout. The screen texts are typed by hand from
  a real `/usage` panel, with no account data in them.
- Tests cover the run with a fake terminal: the panel appears; it never
  appears and the program is stopped; the program is not found.
- Run by hand on a profile with no token file: the limits show, with this
  source named, and no probe session appears in the projects.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Built in `otty-agents`, the io crate and the binary. The tests of
`otty-agents` (the screen reading, the run with a fake terminal, the
removal of the session files and the five-minute memory) were written
first and seen failing; the test of the reset text in the app was written
first but not run before the change; the tests of the real hidden terminal
in the io crate were written together with it.

`ClaudeCode::limit_sources()` now gives "usage request", then "hidden
terminal" (`claude_code/usage_probe.rs`). `Agent::limit_sources` takes a
second starter, the new public `StartTerminal` / `RunningTerminal` traits
(`src/hidden_terminal.rs`): keys go in, the screen comes back as text once
it changed and stayed still, or "unchanged" or "ended". The real one lives
in `otty-app-agents-io` (`hidden_terminal.rs`), because the crate rules let
only an io crate run a process; `otty-agents` stays free of the terminal
crates. It runs the program through the user's login shell, as the agent
tab does, in a pseudo-terminal of `otty-pty` of 160 by 200 with
`TERM=xterm-256color`, and feeds the output to a `TerminalEngine` of
`otty-libterm` with a plain `Surface`, which also answers the terminal's
questions (cursor position); the screen text is built from the engine's
snapshot. The io crate now depends on the workspace crate `otty-libterm`;
no outside crate was added. Stopping sends the engine's shutdown: `SIGHUP`,
a second's wait, then a kill; it also happens on drop.

The probe runs `claude --allowed-tools "" --strict-mcp-config --settings
'{"remoteControlAtStartup":false,"disableAllHooks":true}'` with
`CLAUDE_CONFIG_DIR` as for the agent tab and `DISABLE_AUTOUPDATER=1` (as
CodexBar, so the probe never updates `claude`), in
`~/.cache/otty/agent-probe`. It types `/usage` and Enter two seconds after
the screen first shows something and is still, types Enter again every
three seconds (at most three times) while no panel shows, as CodexBar does
when the first Enter only picks the command, reads the panel, types Escape
and stops the program. The whole reading has 30 seconds. A login screen is
"not logged in"; a question (trust this folder, use this API key, the
first-run look, "Press Enter") is "not available: claude asks …" and no key
is ever typed for it; a program that ends is "not available" ("claude is
not found" when the screen says so); no panel in time is "not available";
a panel with no readable percent at the time limit is "unexpected answer".

The reading (`claude_code/usage_screen.rs`) is one pure function, after
CodexBar's `ClaudeStatusProbe.swift`: the newest panel is the text after
the last "Settings:" and before "What's contributing to your limits
usage?"; a line that starts with "Current session" is the "5 hours" window,
"Current week" or "Current week (all models)" the "7 days" one, "Current
week (Opus)" "7 days · Opus" ("only" is dropped), the names the usage
request gives; the first percent with "used" (or "left", turned into used)
within 12 lines, and the first line with "Reset(s)" within 14 lines, both
before the next block. A panel still "Loading usage" or "Refreshing…" is
waited for. The panel gives no plan.

`LimitWindow` got `reset_text()` / `with_reset_text`: the panel tells the
reset only as text ("Resets 5pm (Europe/Berlin)"), with no date the crate
could turn into a time without a time zone database. The dashboard shows
that text as it is when a window has no reset time.

After every run that started, the `*.jsonl` files directly in
`<config>/projects/<the probe folder's name>/` are removed (every character
but ASCII letters and digits becomes `-`), then that folder when it is
empty; nothing else is touched, a symlink is not followed, and a name
longer than 200 characters, which Claude Code cuts and hashes, is left
alone. The answer of each profile (also a failure) is remembered by its
config folder for 4 min 30 s from the start of the reading; a read in that
time gets it again and starts nothing. The gap is a little under five
minutes because the timer read reaches this source only after the first
source has failed, up to 30 seconds later each time.

Open: Claude Code asks whether to trust a new folder, and OTTY never
answers, so the first reading of a profile says "claude asks whether to
trust its folder" until the user runs `claude` once in
`~/.cache/otty/agent-probe` with that profile and answers. CodexBar accepts
the trust itself in its own probe folder; doing so would write to the
profile's `.claude.json`. Claude Code may also record the typed `/usage` in
the profile's `history.jsonl` and its start in `.claude.json`; these are
not removed.
