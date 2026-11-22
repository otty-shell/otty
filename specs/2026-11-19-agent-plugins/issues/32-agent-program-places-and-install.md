# 32 An agent part says where its program is installed and how to install it

Status: done
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

- 2026-11-22, issue 32, done. An agent part's manifest may list
  `program_places` (one file each, from `~`, a variable of its `env` or
  `/`, no `..`, no pattern) and `[agents.agent.install]` with a `command`
  (program and arguments) and/or an `https` `page`, at least one; both are
  checked when the manifest is read (`ProgramPlace`, `ProgramInstall` in
  `otty-plugin-manifest`). The order is the login shell's `PATH`, then the
  first place that holds a program, else "program not found"
  (`profile_list.rs`). The plugin agent tells its loaded plugin where it
  found the program; from then on the plugin's calls that run it by its
  name, or by that exact path in a found profile's command, start that
  path with the permissions of the name (`ProgramPaths` of
  `otty-plugin-host`), and OTTY starts it by that path for session tabs,
  the commands of needs and the version check. "Program not found" names
  the program and the places OTTY looked at; with an install command it
  shows **Install <agent>**, the line says what it runs, and the page is a
  second button; with only a page that page is the button; with neither,
  no button. "Set the path of the program with Edit" is gone with its
  key: Edit has no such field. Every need's tab and Log in now open as a
  tab the feature watches, and closing it reads the profiles again.
  Official plugins: OpenCode 0.1.1 (`~/.opencode/bin/opencode`, page
  `https://opencode.ai/docs`), Anthropic 0.2.2 (`~/.local/bin/claude`,
  page `https://docs.anthropic.com/en/docs/claude-code/setup`), OpenAI
  0.2.2 (page `https://github.com/openai/codex`); no install command,
  since none could be confirmed from the repository; the template shows
  both fields. Tests: the fields and their refusals (manifest); a found
  path runs by name or by that path and nothing else (host); a program at
  a place is found and started by its full path for a session, a login
  need and a found profile, the name wins when the shell finds it, and
  nothing found is "program not found" (agents, over the template); the
  need with the install button and the page, with only the page, and
  with no button (core); the profiles are read again once a need's tab
  closes (core, app). Spec ("The agent's program"), Write a Plugin, the
  three plugin pages and READMEs follow.
