# 17 — Command palette widget

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 05, 06, 10
Spec: [../spec.md](../spec.md)

## What

A new widget `widgets/palette/` with the usual files (`event`, `state`,
`reducer`, `model`, `view`, `mod`), registered in `Widgets`. Tests first.

- State: open or closed, query, active index. Opens from the shortcut (ticket 06) and the title bar `TitleSearch` (ticket 11); Esc or a click outside closes it; running an item closes it.
- Items, built when it opens and on query change from other widgets' view models:
  - Sessions: every open tab ("<title>" and the machine), runs `ActivateTab`;
  - Launches: every launch (path in the tree as detail), runs the existing run-launch intent;
  - Actions: New terminal, New launch…, New folder, Settings, Split pane, Close session, Show/hide left panel, Show/hide files, Focus mode — each runs the intent its button already sends.
  No file search (spec Out of Scope).
- Routing: the palette emits an effect naming the chosen item; `app/src/events/palette.rs` turns it into the existing events of the owning widget. The palette never changes other widgets' state itself.
- Keyboard: the input has focus when the palette opens; ↑ ↓ Enter Esc handled by the kit model from ticket 05. While open, terminal panes do not receive keys.
- Search text in the title bar: active session title, or "Search launches and actions" with no tabs.

## Done when

- Reducer tests: open/close, query, run each item kind produces the right effect, close after run.
- Routing tests: each effect maps to the existing event.
- `iced_test` on the view: type, arrow, Enter (spec AC7).

## Answer

New widget `app/src/widgets/palette/` (`event`, `state`, `reducer`,
`model`, `view`, `mod`, plus `types` for the entry key and `items` for
building the list), registered as `Widgets::palette` and
`AppEvent::Palette`. Class C: new intents, state, reducer and routing.

### State and intents

- `PaletteState` holds `Option<PaletteList<PaletteTarget>>`: `None` is
  closed; open keeps the entries, the query and the active entry in the kit
  `PaletteList` (ticket 05).
- `PaletteIntent`: `Open`, `Dismiss` (click outside), `FocusInput` (click on
  the palette background) and `Action(PaletteAction)` straight from the kit
  view. The app does not route ↑ ↓ Enter Esc; the kit input sends them.
- `PaletteEffect`: `Run(PaletteTarget)` (the palette is already closed) and
  `Dismissed` (closed without running).
- `Open` builds the entries and focuses the input with
  `operation::focus(command_palette::input_id())`; while already open it only
  focuses the input again (the query stays). Every `QueryChanged` rebuilds
  the entries from the other widgets, then filters. Actions while closed are
  ignored; Enter with no match and a click past the last match do nothing.

### Entries (`items.rs`, from other widgets' view models)

`PaletteCtx { sessions: TabsViewModel, launches, can_split_pane }`, built by
`events/palette.rs` on every palette intent.

- **Sessions**: every open tab, title, machine name as detail, its target dot
  → `Session { tab_id }`.
- **Launches**: every launch in the tree (new
  `QuickLaunchTreeViewModel::launches()`, depth first, folders skipped),
  folder path as detail (`servers/prod`, none at the top), SSH or local dot →
  `Launch { path }`.
- **Actions**: New terminal, New launch…, New folder, Settings, Split pane
  (only when the active tab is a terminal with a focused pane), Close session
  (only with an active tab), Show or hide the left panel, Show or hide files,
  Focus mode, each with its `AppShortcut` label when it has one.

No file search (spec Out of Scope).

### Routing (`app/src/events/palette.rs`)

| Entry | Event sent (the one its button sends) |
|---|---|
| Session | `TabsIntent::ActivateTab { tab_id }` |
| Launch | `QuickLaunchIntent::LaunchNode { path }` + `FocusActive` |
| New terminal | `TabsIntent::OpenTerminalTab { title: shell name }` |
| New launch… | `QuickLaunchIntent::HeaderCreateCommand` |
| New folder | `QuickLaunchIntent::HeaderCreateFolder` |
| Settings | `TabsIntent::OpenSettingsTab` |
| Split pane | `TerminalWorkspaceIntent::SplitPane { active tab, its focused pane, Vertical }` (the pane menu's "Split right"); nothing without one |
| Close session | `TabsIntent::CloseTab { active tab }`; nothing without one |
| Left panel / files / focus mode | `SidebarIntent::ToggleLeft` / `ToggleRight` / `ToggleFocusMode` + `FocusActive` |
| `Dismissed` (Esc, click outside) | `TerminalWorkspaceIntent::FocusActive` |

`FocusActive` gives the keyboard back to the active terminal, which lost it
to the palette input. It is only added where the event itself does not move
focus (tab activation, a new terminal, a split and the inline folder name
move it themselves; Settings and the launch editor need no terminal focus),
so it never races with their own focus change.

Triggers: `AppShortcut::CommandPalette` (`events/shortcuts.rs`) and the title
bar `TitleSearch` (`ChromeIntent::OpenCommandPalette` →
`ChromeEffect::OpenCommandPalette` → `PaletteIntent::Open`). The search
button now has a press message, so it captures the click and no window drag
starts. With no tab open it reads "Search launches and actions".

### Terminal keys while open

Two parts:

1. Focusing the palette input unfocuses every other focusable widget, the
   terminal widgets included (`TerminalViewState` implements `Focusable`),
   so they stop handling keys.
2. Guard: `guards::is_terminal_input` (a terminal `Write` event: typed
   keys, paste, IME text) is dropped in `update` while the palette is open.
   This covers a terminal that is focused anyway, for example one created
   while the palette is open (new terminal widgets start focused).

### View

`widgets/palette/view.rs`: the kit `CommandPalette` (placeholder, "No
matches", hints ↑↓ move, Enter open), 14px below the title bar, centered, on
a transparent full-size dismiss layer (`Idle` cursor, so the hidden panels
get no hover). The palette sits in a `mouse_area` that sends `FocusInput`
for clicks on its background, so those do not close it and keys keep
working. `app/src/view.rs` gets only the overlay layer (before the SSH
secret dialog, which stays on top) and `title: active_tab_title()`.

i18n (all 10 locales): `title_search_no_sessions`, `palette_placeholder`,
`palette_no_matches`, `palette_hint_move`, `palette_hint_open`,
`palette_group_actions`, `palette_new_terminal`, `palette_new_launch`,
`palette_new_folder`, `palette_split_pane`, `palette_close_session`,
`palette_toggle_left_panel`, `palette_toggle_files_panel`. Group names and
Settings / Focus mode reuse `sessions_panel_title`, `launches_panel_title`,
`settings_tab_title`, `window_focus_mode`.

Follow-up: a launch now runs with one `QuickLaunchIntent::LaunchNode` (ticket 16) instead of a chained press and release, so a cursor move cannot start a drag in between.

### Tests (written first; 46 failed on `todo!()` before the code)

| Where | Tests | What they pin |
|---|---|---|
| `palette/reducer.rs` | 14 | open lists all with the first active and focuses; reopen keeps the query; typing filters; typing rebuilds from fresh view models; ↑ ↓; Enter runs the active entry and closes; clicks run a session, a launch and actions and close; no match / bad index run nothing; Esc and click outside close with `Dismissed`; closing twice emits nothing; actions while closed ignored; `FocusInput` only when open |
| `palette/items.rs` | 6 | session detail and dot; launch folder path and dot; every action with its shortcut; no Split pane / Close session without tabs; non-terminal tab: close yes, split no; group order |
| `palette/view.rs` (`iced_test`) | 5 | spec AC7: type, ↓, Enter runs the second match; Esc closes; clicking an entry runs it; click outside dismisses; click on the palette background refocuses and does not dismiss |
| `events/palette.rs` | 13 | each effect maps to the existing event (table above); Open reads the tabs widget; Split pane is offered with a terminal tab |
| `quick_launch/model.rs` | 1 | `launches()` walks the tree in order and skips folders |
| `chrome` reducer, router, `action_bar` view | 4 | search press → `OpenCommandPalette` only (no drag); no-tab text; effect → `PaletteIntent::Open` |
| `events/shortcuts.rs` | 1 (+1 rewritten) | palette shortcut opens the palette; split pane from the subscription still does nothing |
| `guards.rs` | 2 | terminal `Write` is terminal input; redraw, binding actions and other events are not |

## Comments

- **Checklist walk: not done** (the agent cannot run the GUI). Owner, please
  walk `regression-checklist.md` steps 1–7 (terminal focus and typing still
  work with the palette closed), 8–9 (launches run from the tree as before),
  10 (New launch…), 19 (Settings), 21 (language switch: palette texts and the
  title bar text change), 22–23 (title bar: empty space still drags the
  window, double click still toggles full screen). Then these palette
  checks:
  1. Ctrl Shift K (Cmd K on macOS) with a terminal focused: the palette
     opens 14px under the title bar with the input focused; typing goes to
     the palette, nothing reaches the shell (`cat -v` shows nothing).
  2. Click the title bar search: the palette opens and the window does not
     move. With every tab closed it reads "Search launches and actions".
  3. Type, move with ↑ ↓, run with Enter: a session switches tab and focuses
     its terminal; a command launch and an SSH launch (passphrase dialog)
     run like a click in the tree; each action does what its button does
     (New terminal, New launch…, New folder starts the inline name, Settings,
     Split pane splits the focused pane right, Close session, panels, focus
     mode).
  4. Esc and a click outside close it and the terminal gets keys again; a
     click on the palette background keeps it open and typing still works.
  5. Look at both themes (OTTY Dark and Light).
- **Resolved in 2327fba:** a launch run from the palette now sends one `LaunchNode` intent, so the press/release race is gone.
- Ticket 16 also lists launches (top level only); `launches()` lists the
  whole tree and could be shared.
- Opening the palette while a context menu is open only closes the menu
  (the existing context menu guard dismisses on unknown events); a second
  press opens the palette.
- The kit list does not scroll to the active entry when it moves past
  twelve entries (ticket 05 open item); with today's counts it rarely
  matters.
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok (the
  three old `yanked` warnings remain). No new dependencies.
- Coverage (`cargo llvm-cov --workspace --all-features`, lines): base
  `a7e1449` 78.47% → this ticket 79.23%.
