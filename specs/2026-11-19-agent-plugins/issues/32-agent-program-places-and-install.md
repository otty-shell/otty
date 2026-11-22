# 32 An agent part says where its program is installed and how to install it

Status: open
Blocked by: 30

## Why

Owner, 2026-11-22: OpenCode's profile is shown (the plugin found
`~/.local/share/opencode`), but its state is "program not found" with a
line that does not help and no button. The program is there: OpenCode's
installer put it at `~/.opencode/bin/opencode`, and that folder is not in
the `PATH` of the owner's login shell, so `command -v opencode` (how OTTY
looks, `crates/app/features/agents/io/src/program_lookup.rs`) finds
nothing, and a tab that runs `opencode` would fail the same way.

## What to build

1. **Program places.** An agent part's manifest may list the places its
   official installer puts the program (`program_places`, with `~` and
   `{VAR}` as in permissions), such as `~/.opencode/bin/opencode` for
   OpenCode. When the login shell does not find the program by name, OTTY
   checks these places (an executable file) and then starts the agent by
   that full path everywhere it starts it (session tabs, login and other
   need commands, hidden terminals, version check). OTTY has no
   agent-specific place of its own.
2. **Install button.** An agent part's manifest may give how to install
   its program: a command (run in a visible terminal tab, as other need
   commands are) and/or a page. "Program not found" then shows a button
   "Install <agent>" that runs the command, and a link to the page; with
   neither, no button. After the tab closes OTTY reads the profiles again.
3. **Words.** The "program not found" line names the program and says
   where OTTY looked (the login shell's `PATH` and the program places), in
   all ten locales. Drop "set the path of the program with Edit" unless
   Edit really does that.
4. Official plugins: OpenCode (`~/.opencode/bin/opencode`, its official
   install command and page), and the places and install of Claude Code
   and Codex where their official installers have them. Rebuild with
   `plugins/build.sh`. Spec, Write a Plugin and the plugin pages follow.

## Done when

- Tests first: a program not on `PATH` but at a program place is found
  and started by its full path; not found anywhere → the need with the
  install button and the page; a part with no install info → no button.
- The checks of `AGENTS.md` pass.

## Comments
