# 22 — Update the design system and the V4 screens

Status: resolved
Type: task
Class: —
Model: Opus 5.5 (claude-opus-5-5), owner review
Blocked by: 21
Spec: [../spec.md](../spec.md)

## What

- Design system (https://claude.ai/artifact/7zMwx63cZnNmTUeTL1FQF3): add cards for the new kit parts (TargetDot, Chip, PanelSection, SessionGroup, SplitButton, PaneHeader, TitleSearch, ActionTile, FloatingNote, EmptyCard); update SessionRow, StatusBar, CommandPalette to match the code; remove or mark RailButton and TabStrip as not in the app if tickets 07 and 10 deleted them; README "In code" table and the Layout section (no top tab mode).
- Replace the shortcut notation everywhere with spec Decision 3's table.
- Redesign canvas (https://claude.ai/artifact/5QkFQQUDrqMuqVSFyBZCh6), page "v4 today": shortcut labels to Decision 3.
- Show the drafts to the owner before publishing.

### Changes since the first draft (2026-11-01)

The first draft was lost with its session scratchpad and is redone from
today's code. It must also cover:

- Ticket 23 labels: SessionRow shows the program + a muted mono folder name
  (`SessionRow::folder`); PaneHeader shows program + folder name + machine
  chip on one line, cut at the right edge in a narrow pane; StatusBar shows
  machine + short folder path (`~/…/ottyrun/otty`) + counts, no program;
  Files shows the dot + folder name, no "follows <program>" line.
- Ticket 24: side panels are at least 16% of the window wide; the panel
  buttons open a panel at that width.
- Sessions and Launches share the left panel height (`panel_height`): open
  sections take their content height, and split the free height when they
  do not fit; each scrolls inside its share.
- Launches header has a SplitButton like Sessions (plus = New launch,
  chevron = menu with New launch and New folder); plus icon centered.
- SessionRow close cross shows only on hover, with no background.
- Shortcut labels order modifiers Ctrl, Alt, Super/Cmd, Shift; macOS says
  Cmd (Round 5).
- TitleSearch: content centered, `surface-overlay` background with a `line`
  border, `surface-hover` on hover.
- Chip: 24 px high from padding, so a wrapped label keeps its padding.
- Segmented: frame `surface-chrome`, selected segment `surface-overlay` with
  a `line` border.
- FloatingNote: the focus mode note stays 2 s, then fades out in 0.4 s
  (`FloatingNote::opacity`, `UiPalette::with_opacity`).
- New size tokens go into `tokens.json` (Round 5).

## Done when

- Owner approved and both artifacts published.

## Answer

- Published on 2026-11-01 after the owner's review: design system version 11
  (`1793541467-b234`), canvas version 32 (`1793541516-2467`).
- Design system: size tokens from `otty_ui_kit::size` (`pane-header`,
  `group`, `session-indent`, `dot`, `split-chevron`, `status-gap`,
  `status-button`, `title-search-*`, `palette*`, `floating-note`,
  `empty-card`, `empty-logo-*`, `action-tile`) and `side-panel-min` 16%;
  `tab` removed. New cards: TargetDot, Chip (replaces TargetChip),
  SessionGroup, PanelSection, SplitButton, PaneHeader, TitleSearch,
  ActionTile, EmptyCard, FloatingNote. SessionRow, StatusBar,
  CommandPalette, Kbd, Segmented, IconButton follow the code; TabStrip
  removed; RailButton marked "not in the app yet". README: Decision 3
  shortcut table and modifier order, Layout without top tabs, session
  labels (ticket 23), the "In code" table. `logo-full.svg` dropped from
  Logos.
- Canvas, page "v4 today": shortcut labels follow Decision 3; session rows,
  pane headers, status bar, Files header and Launches split button follow
  tickets 23 and 24; the palette shows only what the app searches. The
  canvas copy of the design system and `otty-theme.css` are updated.
- Left as drawn (owner, 2026-11-01): out-of-scope screens keep Ctrl 1-3 on
  the mode rail, Alt 1-3 on recent launches and Ctrl S in the launch
  editor.

Not verified: how the cards and screens look in the browser.

## Comments

- Draft ready at `/tmp/claude-1000/-home-kemokempo-projects-main-ottyrun-otty/901c711b-99ab-4ffe-9031-c7cc34df6eeb/scratchpad/ticket22/` (`ds/project/…` for the design system, `canvas/project/V4*.dc.html` for the canvas); waiting for owner review before publishing.
- 2026-11-01: the first draft folder is gone (old session scratchpad); the draft is redone against the changes listed above.
- 2026-11-01: second draft ready at `/tmp/claude-1000/-home-kemokempo-projects-main-ottyrun-otty/8bea13c1-cc9b-45ff-8596-44f37d8281d7/scratchpad/ticket22/` (`ds/project/…`, `canvas/project/…`, review diffs `ds.diff` and `canvas.diff`); waiting for owner review before publishing.
