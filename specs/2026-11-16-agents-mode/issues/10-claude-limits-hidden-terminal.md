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
