# 24 — Files follows local launches; wider panel minimum

Status: resolved
Type: task
Class: B
Blocked by: 07, 08, 23
Spec: [../spec.md](../spec.md)

## What

- A launch that runs a shell in a folder (for example `/usr/bin/zsh` in
  `~/…/the-time-of-horde`) left the Files panel on the old folder. Files
  followed shell tabs only, and the launched shell had no OTTY integration,
  so it never reported its folder. Files must follow the open shell whenever
  it can.
- The smallest width of a side panel was 4% of the window: too narrow.

## Answer

- A custom launch of zsh or bash with no arguments gets the same OTTY shell
  integration as a shell tab (`terminal_workspace::services::with_shell_integration`,
  used by `quick_launch::services::custom_session`). Launches with arguments
  run exactly as saved.
- Files follows every terminal tab that runs on this computer
  (`TerminalTabState::runs_locally`, `local_cwd_for_active_tab`); SSH tabs
  are skipped because their folders are remote. Command tabs now ask for a
  Files sync when they open.
- Side panels: an open panel is at least 16% of the window wide
  (`SIDE_PANEL_MIN_RATIO`, set by the owner); a drag below that stops at it,
  and a drag below half of it closes the panel. A closed panel dragged open
  starts at the minimum width.
- The panel buttons (and their shortcuts) always open a panel at the minimum
  width; they no longer bring back the last dragged width. Focus mode still
  restores the widths it hid. The left panel still starts at 20% when the
  app opens.

Not verified: the look in the running app. Local launches of other programs
(htop) do not report a folder, so Files keeps the last one.

## Comments
