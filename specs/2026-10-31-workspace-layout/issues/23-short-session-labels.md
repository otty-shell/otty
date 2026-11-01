# 23 — Short program and folder labels

Status: resolved
Type: task
Class: B
Blocked by: 08, 10, 12, 13
Spec: [../spec.md](../spec.md)

## What

The session row, the pane header, the status bar and the title bar all show
the terminal title the shell sends (OSC 0/2). Shells put the folder in it
(`user@host:~/path`, `~/path`), and the pane header and status bar then add
the folder again. Long folder paths repeat up to three times.

Decisions (owner, 2026-11-01):

- Ignore the titles the shell or a program sends. The program name comes
  from the shell integration blocks: the first word of the running command
  (`cargo`, `nvim`), or the shell name at the prompt. Without a running
  command the pane keeps the name it opened with (shell or launch name).
  Leading `NAME=value` words are skipped, and a path is cut to its last part.
- Each place has one job; only the status bar shows the folder path.

| Place | Shows |
|---|---|
| Session row | program + muted folder name (`zsh  otty`) |
| Pane header | program + folder name + machine chip |
| Status bar | machine + short folder path + counts; no program |
| Files | machine dot + folder name; no "follows <program>" |

- Short folder path: home is `~`; with more than three folders after `~`
  or `/`, only the last two stay, after `…` (`~/…/ottyrun/otty`).
- Folder name: the last folder; home is `~`, the root is `/`.

## Done when

- Tests: program name from a command line and from blocks, folder name, short
  folder path, OSC title events no longer change the tab title, the tab keeps
  the folder name, the session row shows the folder, the status bar shows the
  short path and no program, Files no longer says "follows".

## Answer

- `terminal_workspace/model.rs`: `PaneLabel` (program, folder name, short
  folder path) replaces `PaneHeaderViewModel`; the machine is passed to the
  header chip directly. `services::running_program_from_blocks` reads the
  running command of the latest block.
- `TerminalTabState` names the tab after its focused pane (`title` =
  program, `folder` = folder name) after every terminal event and focus
  change. OSC `TitleChanged` / `ResetTitle` are ignored, so the reducer no
  longer emits an extra title effect for them.
- `TitleChanged` effect and `TabsIntent::SetTitle` carry `folder`;
  `TabItem` keeps it; the session row shows it muted in mono (new kit
  `SessionRow::folder`).
- Pane header: program + folder name + chip. Status bar: machine + short
  path + counts. Files: dot + folder name; `followed_program`,
  `focused_program` and the `tpl_files_follows` string (10 locales) are gone.
- Coverage: 69.80% at the merge base, 78.88% after.

Not verified: the look in the running app; shells without the OTTY
integration (fish) now show only the shell name, with no folder.

## Comments
