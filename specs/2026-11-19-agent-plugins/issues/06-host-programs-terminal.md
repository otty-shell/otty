# 06 Host operations: programs, line programs, hidden terminal, file removal

Status: ready-for-agent
Blocked by: 03

## Why

Spec Decision 6. Agents tell their version with `--version`, Codex its
limits through `codex app-server` (JSON-RPC over lines), Claude Code its
limits only on the `/usage` screen. Today's `program.rs` (`StartProgram`)
and `hidden_terminal.rs` (`StartTerminal`, implemented by the app with
OTTY's terminal engine) do this for in-core readers.

## What to build

1. `programs.run(program, args, env, timeout)`: only a program the called
   part declares, with arguments matching its declared pattern; only the
   variables that part lists are passed (plus the profile's config-folder
   variable the plugin sets); 10 s; 1 MiB of output.
2. `programs.talk`: a resource over a running program: `send-line`,
   `next-line(timeout)`; stopped and killed when the call ends; 60 s in
   all. The host does not look at the lines (spec Decision 7: which
   methods are sent is the plugin's choice; the rule of the `codex` part
   of the official OpenAI plugin is in issue 11).
3. `terminal.hidden-terminal`: a program the called part declares,
   started unseen in OTTY's own working folder, through a
   `StartTerminal`-like trait the app gives the host (the host must not
   depend on the app); `send-keys`, `screen` (visible text),
   `wait-for-change(timeout)`; stopped when the call ends; 60 s.
4. `files.remove(path)`: only paths matching the called part's `write`
   globs; removes one file, or one folder only when empty.
5. Host-side pacing: a part's limit source that uses a program or a
   terminal is run at most once in five minutes per profile (today's
   `claude_code/probe_memory.rs`), applied by the host to any plugin.

## Done when

- Tests first, with `mockall` mocks of the program and terminal starters:
  an undeclared program or argument, or a program only another part
  declares, is refused; variables not listed are not passed; a program
  that never answers is stopped at its timeout; a talk ends with the
  call; a hidden terminal's screen text reaches the plugin; removal
  outside `write` globs is refused and a non-empty folder is kept; pacing
  skips a second probe within five minutes.
- The checks of `AGENTS.md` pass.

## Files

`crates/plugins/host/src/` (programs, terminal, remove, pacing);
`crates/agents/src/program.rs`, `hidden_terminal.rs` (traits moved or
shared); the app's implementation of the terminal starter in
`crates/app/features/agents/io/`.

## Comments
