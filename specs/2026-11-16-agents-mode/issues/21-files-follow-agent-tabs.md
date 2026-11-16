# 21 Files follows the folder of an agent tab

Status: ready-for-agent
Blocked by: 20

## Why

The owner continues a Claude Code session from Agents mode. The tab opens
in the project folder, but the Files panel does not show that folder; it
stays on the folder of the previous shell tab.

Cause (read in the code on 2026-11-16): Files follows the folder of the
focused pane, and a pane learns its folder only from what the shell
reports through OTTY's shell integration (`focused_folder_path` in
`crates/app/features/terminal/src/lib.rs`, `follows_folder` and
`folder_followed` in its `reducer.rs`, `SyncRoot` in
`crates/app/otty/src/dispatch/explorer.rs`). An agent tab is a command
tab started without the shell integration (issue 05: it would replace the
user's start files), so its pane never reports a folder, and Files keeps
the old one. The design system says Files follows "the folder of the
focused tab when that tab runs on this computer: shell tabs and local
launches"; an agent tab is a local tab with a known folder too.

## What to do

1. A pane remembers the folder it was started in (the working folder of
   its session settings, as `OpenCommandTab` and `OpenTerminalTab` give it;
   the split of issue 15 too).
2. The folder of a pane is the folder its shell reported, else the folder it
   was started in. Only for panes on this computer; an SSH pane keeps
   today's rule (Files stays on the last local folder).
3. So a Claude Code or Codex tab opened from Agents mode, and a local
   launch or the editor tab with a folder, makes Files follow that folder
   as soon as it comes to the front; the status bar and the session row
   show its folder name as for a shell tab.
4. A pane whose shell reports a folder later keeps using the reported one,
   as today.

## Done when

- Tests, written first, cover: a command tab started in a folder gives that
  folder to Files when it comes to the front; a reported folder wins over
  the starting one; an SSH tab does not move Files; a split started in a
  folder gives its folder when focused.
- Run by hand: continue a Claude Code session from Agents mode; the Files
  panel shows the project folder; switch to a shell tab and back.
- The checks of `AGENTS.md` pass.

## Comments
