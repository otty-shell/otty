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

### 2026-11-16

The pane remembers the folder it was started in, in the terminal feature.
`TerminalEntry` (`types.rs`) keeps a `start_folder`, taken from the working
folder of the pane's session settings when the pane is created
(`TerminalTabState::new` for a new tab, `split_pane_with` for a split; an
SSH session gives none). `TerminalEntry::folder()` is the one rule: the
folder the shell reported last, else the starting one. The pane label (pane
header, status bar, session row through `TitleChanged`) and the folder Files
follows (`active_local_cwd`, still only for tabs on this computer) both read
it. Nothing changed in the tabs feature or in the binary's dispatch: an agent
tab, a local launch, the editor tab and an agent in a split already start
with a working folder.

Tests were written first. Seen failing: a command tab started in a folder
gives it to Files when it opens and when it comes back to the front (and
the session row gets its folder name); a split started in a folder gives
its folder when focused; in the binary, a continued session shows its
project folder in Files and the status bar after a shell tab was in front.
Passed from the start, as they check rules that already held: a reported
folder wins over the starting one; an SSH tab does not move Files.

A side effect: a shell tab starts in a folder too (the home folder, or the
folder of Open in terminal), so Files and the session row show that folder
as soon as the tab opens, before the first prompt reports it. A shell
without the OTTY shell integration keeps showing its starting folder after
a `cd`, where it showed none before. The test
`each_open_terminal_is_listened_to` now lets Files follow a folder first,
because the new shell tab's folder would add a Files watch to the count.
