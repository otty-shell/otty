# 01 — Add iced_test, characterization tests and the regression checklist

Status: resolved
Type: task
Class: C (tests only, no behavior change)
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: —
Spec: [../spec.md](../spec.md)

## What

Pin today's behavior before the layout changes, so later tickets show at once
when they break something.

- Add `iced_test = "0.14"` as a `[dev-dependencies]` entry of `crates/ui/kit` and `app` (owner approved, spec Decision 7). Run `cargo deny check`; record any license or duplicate warning in Comments.
- Characterization tests (unit tests next to the reducers, no view code), one per flow the redesign touches:
  - tabs: open terminal, settings, launch editor and error tabs; activate; close the active and a background tab; which tab becomes active after a close;
  - sidebar: toggle visibility, resize, collapse below the threshold (`SIDEBAR_COLLAPSE_WORKSPACE_RATIO`) — these move to the new state in ticket 07 and must keep their numbers;
  - explorer: follows `shell_cwd_for_active_tab` when the active tab changes;
  - quick launch: run a command launch and an SSH launch up to the secret prompt; cancel the prompt.
  Reuse tests that already exist; add only the missing ones.
- One `iced_test` smoke test on a kit component (e.g. `Button` click gives its message) to prove the setup, in both crates.
- Write `regression-checklist.md` in this spec folder: the manual flows of spec AC1, each as a numbered step with the expected result. Later tickets tick it in their Comments.

## Done when

- `cargo test --workspace --all-features` passes with the new tests; `cargo deny check` passes.
- `regression-checklist.md` exists and was walked once on the current build (note results in Comments).

## Answer

`iced_test = "0.14"` is a dev-dependency of `otty-ui-kit` and `otty`. No
production code changed. The regression checklist is
[`../regression-checklist.md`](../regression-checklist.md) (23 numbered steps
covering spec AC1).

### Tests added (34)

| Where | Tests | What they pin |
|---|---|---|
| `crates/ui/kit/src/components/button.rs` | 2 | `iced_test` smoke: clicking an enabled `Button` sends its message; a button without a message sends nothing |
| `app/src/widgets/tabs/view/tab_bar.rs` | 1 | `iced_test` smoke in the app: clicking a background tab title gives `TabsIntent::ActivateTab` |
| `app/src/task_outputs.rs` (new, test-only) | 2 | the `collect_outputs` helper itself |
| `app/src/widgets/tabs/reducer.rs` | 11 | opening terminal, launch editor (wizard), error and Settings tabs and their effects; activating a background tab; closing a background tab (active stays), the active middle tab (the tab before it wins), the active first tab (the **last** tab wins, not the right neighbour), the only tab (`Closed { new_active_id: None, remaining: 0 }`), an unknown tab (no-op); tab ids are never reused |
| `app/src/widgets/sidebar/reducer.rs` | 10 | visibility toggle (ratio 0 while hidden, back to 0.2); workspace toggle; resize follows the drag and emits `QuickLaunchResetInteractionState` + `SyncTerminalGridSizes`; width capped at 0.8; below 0.04 the panel collapses, at 0.05 it stays open; dragging a closed panel open sets 0.04 (`0.2 * 0.2`, compared with a tolerance); a small drag on a closed panel does not resync the grid; `SelectExplorer` reopens at 0.2; `SelectTerminal` on an open panel emits nothing |
| `app/src/events/tabs.rs` | 4 | explorer follows `shell_cwd_for_active_tab`: activating a background shell tab emits `ExplorerIntent::SyncRoot` with that tab's folder plus `FocusActive` and `SyncSelection`; closing the active tab syncs the explorer to the new active tab and notifies terminal workspace and quick launch; activating Settings does not move the explorer; closing the last tab requests no focus |
| `app/src/widgets/quick_launch/reducer.rs` | 4 | command launch: click (press + release) starts a launch, `SetupCompleted(Prepared)` emits exactly one `OpenCommandTerminalTab`; a second click while launching starts nothing; `Failed` emits `OpenErrorTab`; SSH launch: click, `SecretRequired(KeyPassphrase)` opens the prompt and ends the launch, `SecretPromptCancel` closes it with no launch |

Already covered before this ticket and reused, not duplicated: sidebar state
numbers (`sidebar/state.rs`), explorer `SyncRoot` reducer behavior
(`explorer/reducer.rs`), secret prompt typing/submit/empty submit/cancel
(`quick_launch/reducer.rs`), Settings tab de-duplication and title refresh
(`tabs/reducer.rs`).

### How to use `iced_test` 0.14 in this repo

`iced_test::simulator(element)` builds a headless UI (default 1024×768) from
any `Element<'_, Message, iced::Theme, iced::Renderer>`, so kit components and
app `view(props)` functions work as they are. Select widgets by visible text
(`&str`), by `widget::Id`, by `Point`, or with a closure. `click` points the
cursor at the centre of the found text and sends press + release; the message
of the button around that text is recorded. `into_messages()` returns what was
produced.

```rust
use iced_test::simulator;

let palette = UiPalette::otty_dark();
let mut ui = simulator(Button::new("Save", &palette).on_press(Message::Save));

ui.click("Save").expect("button label should be visible");

let messages: Vec<Message> = ui.into_messages().collect();
assert_eq!(messages, vec![Message::Save]);
```

Other calls: `ui.find(selector)` (is it rendered?), `ui.typewrite("text")`,
`ui.tap_key(key)`, `ui.simulate(events)`, `ui.point_at(point)`,
`Simulator::with_size(Settings::default(), size, element)` for a fixed size,
`ui.snapshot(&theme)` for image/hash snapshots (not used yet).

Things to know:

- Text is the easiest selector. Icon-only buttons (`IconButton`, the tab close
  cross, the "+" new tab) have no text in the tree, because their label is a
  tooltip; select them by `Id` or `Point`, or test their message through the
  parent component's text.
- Messages need no `PartialEq`; app intents that hold `Settings` are checked
  with `matches!`.
- The simulator renders with Fira Sans (`iced_renderer/fira-sans`, enabled by
  `iced_test` for test builds only); kit fonts are not loaded, which is fine
  for clicks and lookups, not for pixel snapshots.
- No GPU is needed; it falls back to the software renderer. Set
  `ICED_TEST_BACKEND` to force one.

### Reading reducer effects: `crate::task_outputs::collect_outputs`

Reducers return `Task<Event>`, and `task.units()` only says how many things
were scheduled. The test-only helper `app/src/task_outputs.rs` runs a task
with `iced_test::runtime::task::into_stream` and `block_on`, and returns its
`Action::Output` messages (widget operations such as focus or scroll are
skipped):

```rust
let events = collect_outputs(reduce(&mut state, TabsIntent::CloseTab { tab_id }));
assert!(matches!(
    events.as_slice(),
    [TabsEvent::Effect(TabsEffect::Closed { new_active_id: None, remaining: 0, .. })]
));
```

Only use it on tasks built from `Task::done` / `Task::batch`. A `Task::perform`
would really run (for example a quick launch setup starting a process or SSH
connection), so drop those tasks unread, as the launch tests do. `Task::batch`
does not promise an order; assert with `any(...)` when a batch has several
items.

### Driving a terminal tab's folder in a test

`shell_cwd_for_active_tab` reads the folder from the terminal's latest block
snapshot. Tests in `app/src/events/tabs.rs` (`open_shell_tab_in`) open a real
`/bin/sh` tab through `TerminalWorkspaceIntent::OpenTab`, then send
`TerminalWorkspaceIntent::Widget(Event::ContentSync { id, frame })` with a
`SnapshotOwned` whose `blocks` hold a `BlockMeta { cwd: Some(..) }`. This is
the same path shell integration takes, without waiting for a prompt. Ticket 08
(Files "follows <shell>") and ticket 12 (status bar folder) can reuse it.

### Surprising findings

- Closing the active **first** tab activates the **last** tab, not its right
  neighbour (`previous_tab_id` is `None`, so `last_tab_id` wins). Pinned in
  `close_active_first_tab_activates_the_last_tab`; ticket 10 must decide
  whether the session list keeps this.
- `SIDEBAR_COLLAPSE_WORKSPACE_RATIO` is `0.2 * 0.2 = 0.040000003` in `f32`, so
  tests that pin it use a tolerance.
- Adding `iced_test` made Cargo re-pick `windows-sys 0.61.2` (already in the
  lock) instead of `0.52.0` for `errno`, `rustix`, `tempfile` and
  `winapi-util`. Windows-only, no new versions.

## Comments

- **Checklist walk: not done.** The agent cannot run the GUI. "Walked once on
  the current build" is left for the owner: walk
  `regression-checklist.md` steps 1–23 on this commit and note the result
  here.
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok. New
  crates (all dev-only): `iced_test`, `iced_selector` (MIT), `sha2`, `digest`,
  `block-buffer`, `crypto-common`, `cpufeatures`, `typenum` (MIT OR
  Apache-2.0), `generic-array` (MIT). `iced_test` also uses `png 0.18` and
  `nom`, which were already in the lock. Duplicate versions are allowed by
  `deny.toml`. The three `yanked` warnings (`clipboard-win`, `core2`, `spin`)
  were there before this ticket.
- Not covered by an automated test, because it would need a production change:
  the quick launch router mapping `OpenCommandTerminalTab` →
  `TabsIntent::OpenCommandTab` (`app/src/events/quick_launch.rs`). A test
  there would have to build a `QuickLaunch`, whose fields are private to the
  `quick_launch` module. The reducer side of that effect is pinned.
- The router tests in `app/src/events/tabs.rs` use `App::new()`, which, like
  the tests that existed before, reads `~/.config/otty/settings.json` and
  `quick_launches.json` and writes the shell integration files to
  `~/.config/otty`. They open real `/bin/sh` terminals.
- Coverage (`cargo llvm-cov --workspace --all-features`, line): merge base
  `9544fd8` 69.81% → this ticket 71.80%.
