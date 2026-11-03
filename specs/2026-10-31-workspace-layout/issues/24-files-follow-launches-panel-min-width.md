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
- The panel buttons (and their shortcuts) open a panel at the width it had,
  like focus mode, which restores the widths it hid. A panel never resized has
  the minimum width. (This first said the buttons always open a panel at the
  minimum width; see Comments.) The left panel still starts at 20% when the
  app opens.

Not verified: the look in the running app. Local launches of other programs
(htop) do not report a folder, so Files keeps the last one.

## Comments

- 2026-11-04: rule changed at the owner's request. `SidebarIntent::ToggleLeft`
  and `ToggleRight`, which the buttons, shortcuts and palette all use, now
  bring a closed panel back at the width it had. So do `RevealRight` and
  `RevealLaunches`. If the other panel is open, it gives way, and both stay at
  least at the minimum width, so a very wide panel may come back narrower. A
  closed panel dragged open still starts at the minimum width and follows the
  pointer, which replaces the old width.
