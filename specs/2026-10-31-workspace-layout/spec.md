# Workspace Layout: the V4 Screens on Today's Features

Created: 2026-10-31
Author: holland.deception@gmail.com
Agent: Claude Code
Status: IN PROGRESS
Approved: Yes
Worktree: No
Type: Build

## Summary

**Goal:** Rebuild the OTTY window to match the "v4 today" page of the redesign
canvas: sessions and launches in a left panel, files in a right panel, a status
bar, a title bar with panel toggles, focus mode, an empty state and a command
palette. The app keeps the features it has today and keeps its architecture:
every widget stays `state → reducer → vm() → view(Props) → Intent`, and events
keep flowing through `app/src/events/`.

**References:**

- Screens: the redesign canvas (https://claude.ai/artifact/5QkFQQUDrqMuqVSFyBZCh6), page "v4 today": `V4Workspace`, `V4Focus`, `V4Empty`, `V4FileOpen`, `V4Palette`, `V4Passphrase`, `V4Launch`, `V4Settings`, `V4WorkspaceLight`, `V4SettingsLight`. `V4FutureModes` is out of scope.
- Design system (https://claude.ai/artifact/7zMwx63cZnNmTUeTL1FQF3): `README.md` (Layout and sizes, Content, Iconography) and `components/{SessionRow,StatusBar,TargetChip,CommandPalette}`.
- Previous spec: `specs/2026-10-31-design-system` (kit, tokens, themes; done).

Where a screen and this spec disagree, the Decisions section wins: the screens
show some features OTTY does not have yet (see Out of Scope).

### Current state (research, 2026-10-31)

| Fact | Location |
|---|---|
| The left side is a 48px widget rail (Terminal, Explorer, Settings) and one side pane that shows either Quick launch or Explorer | `app/src/widgets/sidebar/state.rs` (`SidebarItem`, `hidden`, `workspace_open`), `app/src/view.rs:174-232` |
| Tabs are a top tab strip; a tab holds a terminal, Settings, the launch editor or a launch error | `app/src/widgets/tabs/view/tab_bar.rs`, `app/src/widgets/tabs/types.rs` (`TabContent`) |
| The title bar shows the logo, the active tab title and the window buttons | `app/src/widgets/chrome/view/action_bar.rs`, `ChromeIntent` |
| With no tabs, the content area is an empty container | `app/src/view.rs:345-348` |
| App-level keyboard events are received and dropped | `app/src/events/mod.rs:63` (`AppEvent::Keyboard(_event) => Task::none()`) |
| The only app shortcut is split pane (Super Shift D on Linux, Cmd D on macOS); it reaches the app through the terminal widget's app-defined bindings | `app/src/widgets/terminal_workspace/shortcuts.rs`, `crates/ui/terminal/src/bindings.rs` (`BindingAction::Action`) |
| Explorer already follows the active shell's folder | `app/src/events/explorer.rs:26-29` (`shell_cwd_for_active_tab`) |
| The "+" add menu offers New tab, New folder, New launch | `app/src/view.rs:395-453` |
| SSH launches know their host; local ones do not name the machine | `app/src/widgets/quick_launch/types.rs` (`SshCommand::host`) |
| Settings have a section tree on the left and a form on the right | `app/src/widgets/settings/types.rs` (`SettingsSection`, `SettingsNodeKind`), `settings/view/settings_form.rs` |
| Kit components exist for buttons, menus, fields, rows, tabs, rail, dialog, surface, divider, split | `crates/ui/kit/src/components/` |
| `iced_test` 0.14.0 exists (MIT, rust-version 1.88), matching our `iced = "0.14"` | crates.io |

## Decisions

1. **Same features, same architecture.** No new user-facing feature except the
   command palette (decision 5) and the layout itself (panels, focus mode, app
   shortcuts). No new layers: new code follows the existing widget pattern. A
   ticket that changes a reducer or a state file must say so on its `Class:`
   line (see Ticket classes).
2. **Side session list only.** The top tab strip is removed. Open tabs show as
   `SessionRow`s in the Sessions section of the left panel, grouped by machine.
   There is no Settings › Layout switch. Settings, the launch editor and a
   launch error are rows in the same list, like a terminal.
3. **Shortcuts do not take keys from the terminal.** Ctrl B, Ctrl K, Ctrl T,
   Ctrl W, Ctrl \ and Ctrl - drawn on the screens belong to the shell (tmux
   prefix, kill line, delete word, SIGQUIT). On Linux, app shortcuts use
   Ctrl Shift; on macOS, Cmd. The split pane shortcut does not change.

   | Action | Linux | macOS |
   |---|---|---|
   | Command palette | Ctrl Shift K | Cmd K |
   | Left panel | Ctrl Shift B | Cmd B |
   | Files panel | Ctrl Shift E | Cmd Shift E |
   | Focus mode | Ctrl Shift F | Cmd Shift F |
   | Settings | Ctrl , | Cmd , |
   | New terminal | Ctrl Shift T | Cmd T |
   | Close session | Ctrl Shift W | Cmd W |
   | Split pane | Super Shift D (unchanged) | Cmd D (unchanged) |

   The design system and the V4 screens are updated to this table in the last
   ticket.
4. **Terminal widget unchanged.** `crates/ui/terminal` is not edited. Command
   blocks keep today's look; the block header and the input line with chips on
   the screens belong to `specs/2026-10-31-command-blocks` and later work. Pane
   headers are drawn by the app around the widget (pane grid title bar).
5. **Command palette is in scope.** It searches only what exists today: open
   sessions (switch to it), launches (run it) and app actions that already
   exist (New terminal, New launch, New folder, Settings, Split pane, Close
   session, toggle panels, focus mode). Searching files is out of scope.
6. **Kit first.** Every new interface element is a component in `otty-ui-kit`
   with a gallery section in both themes before any app screen uses it. App
   views only compose kit components and pass `UiPalette`.
7. **`iced_test` is added** as a dev-dependency of `otty-ui-kit` and `otty`
   (owner approved, 2026-10-31). It drives views headless: find a widget by its
   text, click or type, and check which message comes out.
8. **Side panels belong to the sidebar widget.** `SidebarWidget` keeps its job
   (the panels around the main area) and now owns both sides: left open, right
   open, focus mode and both widths. The rail and `SidebarItem` go away. No new
   "layout manager" type.
9. **Status bar and pane headers are views, not widgets.** They have no state
   of their own; they render from the view models of tabs, terminal workspace
   and explorer. The status bar view lives in `widgets/chrome/view/`.
10. **This computer is named.** The local machine group and the status bar
    show the host name (e.g. "fedora") with the note "this computer". The
    `hostname` feature of `nix` is enabled for it (owner approved,
    2026-10-31); no new crate. If the name cannot be read, the label is
    "This computer".

## Ticket classes

Each ticket has a `Class:` line. Review checks the diff against it.

| Class | May change | Safety net |
|---|---|---|
| **A, restyle** | `view/*` files, kit | Existing tests unchanged and passing; `iced_test` checks the view's intents |
| **B, new view of existing data** | `vm()`, `model.rs`, views | Unit test on the new view model fields |
| **C, behavior** | state, intents, reducers, event routing | Tests written first (AGENTS.md); characterization tests from ticket 01 still pass |

## Acceptance Criteria

- [ ] AC1: Every flow in the regression checklist (ticket 01) works in the final build: open, split, focus, close a terminal; run a command launch and an SSH launch with a passphrase; edit, rename, duplicate, delete, drag and drop launches; explorer follows `cd` and opens a file in the editor; Settings saves; theme and language switch without a restart.
- [ ] AC2: The window has the slots of `V4Workspace`: title bar, left panel (Sessions over Launches, both collapsible), main area, right panel (Files, closed by default), status bar. No widget rail and no top tab strip.
- [ ] AC3: Every open tab appears as one `SessionRow` under its machine group (this computer by its host name, or the SSH host); clicking it activates the tab; the active tab has `aria-current`-style selection (`surface-selected`); closing works from the row and from the shortcut.
- [ ] AC4: The shortcuts in Decision 3 work while a terminal has focus and while it does not, and typing Ctrl B, Ctrl K, Ctrl W, Ctrl \ in a terminal still reaches the shell (manual check in zsh with `cat -v`, plus a unit test on the keymap).
- [ ] AC5: Focus mode hides both panels, shows the "Focus mode" note with its shortcut, and restores the previous panel state when left.
- [ ] AC6: With no tabs, the main area shows the `V4Empty` card with New terminal and New launch; both run the existing actions.
- [ ] AC7: The command palette opens from the title bar and its shortcut, filters sessions, launches and actions as you type, moves with ↑ ↓, runs with Enter, closes with Esc, and every item runs the same intent as its button elsewhere (`iced_test` and reducer tests).
- [ ] AC8: The status bar shows machine, shell, folder of the focused pane, pane count and session count; values update when focus or folder changes (view model test).
- [ ] AC9: Every new element (Decision 6 list in ticket 02–05) exists in `otty-ui-kit`, is shown in `cargo run -p otty-ui-kit --example gallery` in both themes, and has at least one `iced_test` interaction test.
- [ ] AC10: `crates/ui/terminal` has no changes (`git diff --stat main -- crates/ui/terminal` is empty).
- [ ] AC11: Screenshots of every V4 screen in scope, in OTTY Dark and OTTY Light, are attached to the PR next to the canvas artboards.
- [ ] AC12: `cargo +nightly fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` and `cargo deny check` pass; line coverage from `cargo llvm-cov --workspace --all-features` is strictly greater than on the merge base.

## Out of Scope

Drawn on the V4 screens but new features; each needs its own spec.

- Command block header, sticky header, input line with target/folder/branch chips, ghost completion (`specs/2026-10-31-command-blocks` and later).
- Settings that apply without Save ("Changes apply right away", "✓ Saved"): the Save button stays.
- "Terminal in a folder…", recent launches on the empty screen, Alt 1–9 launch shortcuts.
- Agent states (working, needs you, done) in rows, tabs and the status bar.
- Launch editor additions: Browse buttons, live checks ("Found at /usr/bin/htop", "No such file. Did you mean…"), Ctrl S, the "Unsaved changes" footer. The editor keeps today's fields, checks and buttons in the new look.
- File search in the command palette; git status letters in the Files tree; "Collapse all" in Files.
- Docker and Kubernetes modes, the mode rail, Inspector, DataTable (`V4FutureModes`).

## Tickets

| # | Ticket | Class | Model | Blocked by |
|---|---|---|---|---|
| 01 | Add `iced_test`; characterization tests and the regression checklist | C (tests only) | Opus | — |
| 02 | Kit: `TargetDot`, `Chip`, `StatusBar` | A | Sonnet | 01 |
| 03 | Kit: `PanelSection` (collapsible header with actions), `SessionRow`, `SessionGroup`, `SplitButton` | A | Sonnet | 01 |
| 04 | Kit: `PaneHeader`, `TitleSearch` (title-bar palette trigger), `ActionTile`, `FloatingNote`, `EmptyCard` | A | Sonnet | 01 |
| 05 | Kit: `CommandPalette` (input, groups, items, match highlight, footer) and its tested selection model | A + tested model | Opus | 01 |
| 06 | App shortcuts: one tested keymap, routed through terminal bindings and the keyboard subscription | C | Opus | 01 |
| 07 | Side panel state: left, right, focus mode, widths; remove the rail and `SidebarItem` | C | Opus | 01, 06 |
| 08 | Right panel: Files with "follows <shell>" line | A | Sonnet | 07 |
| 09 | Left panel: Launches section | A | Sonnet | 03, 07 |
| 10 | Left panel: Sessions section replaces the tab strip; machine grouping in the tabs view model | B | Opus | 03, 07 |
| 11 | Title bar: panel toggles, palette trigger, focus, Settings; no logo | A | Sonnet | 04, 07 |
| 12 | Status bar | B | Sonnet | 02, 10 |
| 13 | Pane headers and the restyled pane context menu | B | Sonnet | 04 |
| 14 | Focus mode note | A | Sonnet | 04, 07 |
| 15 | Empty state | A | Sonnet | 04, 10 |
| 16 | New session split button and its menu (replaces the add menu) | A | Sonnet | 03, 10 |
| 17 | Command palette widget | C | Opus | 05, 06, 10 |
| 18 | Launch editor and launch context menu in the V4 look | A | Sonnet | 09 |
| 19 | Settings as one page (Appearance, General, Terminal); section tree removed | A | Sonnet | 07 |
| 20 | SSH passphrase dialog and the "connecting" row state | B | Sonnet | 10 |
| 21 | OTTY Light pass and screenshots | A | Sonnet | 08–20 |
| 22 | Update the design system and the V4 screens (new kit parts, shortcut table) | — | Opus, owner review | 21 |

Order: 01 → (02, 03, 04, 05, 06 in parallel; kit tickets touch separate files) → 07 → (08, 09, 10, 11, 13, 14, 19) → (12, 15, 16, 17, 18, 20) → 21 → 22. App tickets after 07 run one at a time when they touch `app/src/view.rs`. One subagent and one commit per ticket; each commit leaves the app working.

Tickets live in `issues/`. Each ticket names the model that runs it on its `Model:` line; a subagent started for a ticket is launched with that model.

## Progress Tracking

- [x] 01 — `iced_test`, characterization tests, regression checklist · Opus
- [ ] 02 — Kit: TargetDot, Chip, StatusBar · Sonnet
- [ ] 03 — Kit: PanelSection, SessionRow, SessionGroup, SplitButton · Sonnet
- [ ] 04 — Kit: PaneHeader, TitleSearch, ActionTile, FloatingNote, EmptyCard · Sonnet
- [ ] 05 — Kit: CommandPalette · Opus
- [ ] 06 — App shortcuts · Opus
- [ ] 07 — Side panel state · Opus
- [ ] 08 — Right panel: Files · Sonnet
- [ ] 09 — Left panel: Launches · Sonnet
- [ ] 10 — Left panel: Sessions · Opus
- [ ] 11 — Title bar · Sonnet
- [ ] 12 — Status bar · Sonnet
- [ ] 13 — Pane headers and pane menu · Sonnet
- [ ] 14 — Focus mode note · Sonnet
- [ ] 15 — Empty state · Sonnet
- [ ] 16 — New session split button · Sonnet
- [ ] 17 — Command palette widget · Opus
- [ ] 18 — Launch editor and launch menu · Sonnet
- [ ] 19 — Settings page · Sonnet
- [ ] 20 — Passphrase dialog and connecting state · Sonnet
- [ ] 21 — OTTY Light pass · Sonnet
- [ ] 22 — Design system sync · Opus, owner review

## Round Log

- Draft (2026-10-31): owner answers before the draft: side session list only; Ctrl Shift shortcuts; command blocks and the terminal widget out of scope; command palette in scope; `iced_test` approved; new elements go into the kit first; features and architecture stay as they are.
- Round 1 (2026-10-31, owner): show the host name (Decision 10, `nix` feature `hostname`); spec approved; tickets written.
