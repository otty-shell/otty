# Split the router into dispatch and effects

Created: 2026-11-09
Author: ilyashvy@gmail.com
Agent: Claude Code
Status: IN PROGRESS
Approved: Yes
Worktree: No
Type: Refactor

## Summary

**Goal:** every layer of `app` has one job. Today a widget route
`router/x.rs` has two:

- **delivery:** it builds `XCtx` from other widgets and calls
  `XWidget::reduce` with an intent;
- **effects:** it runs what the reducer asked for: background work, file and
  process I/O, messages to other widgets.

After this change, delivery belongs to the App layer, in `dispatch/`, and
effects get their own layer, in `effects/`. The name "router" goes away,
because no code in either folder only routes.

Nothing changes for the user: same behaviour, same tests passing.

## Decisions

1. **Two folders.**
   - `dispatch/` is part of the App layer. `dispatch/mod.rs` holds `AppEvent`
     and picks the widget an event belongs to. `dispatch/x.rs` delivers the
     intents of widget `x`: it builds `XCtx` and calls `XWidget::reduce`,
     nothing else. Its effects go to `effects/x.rs`.
   - `effects/` is the Effects layer. `effects/x.rs` runs the effects of
     widget `x`. It never calls `reduce`; it sends results and messages back
     as `Task::done(AppEvent::...)`.
2. **One file per widget in each folder.** Every widget has `dispatch/x.rs`
   and `effects/x.rs`, even when its `XEffect` has no variants yet, so the
   layout stays the same for every widget and the generator always creates
   both.
3. **Delivery makes no decisions.** A decision that a route makes today on an
   intent moves into the widget's reducer as an effect. The only one found is
   in `router/terminal_workspace.rs`: after `PaneClicked`, `SplitPane`,
   `SplitFocusedPane`, `ClosePane` and `Widget(ContentSync)` the route sends
   `TerminalWorkspaceEffect::SyncExplorer` itself. The reducer returns that
   effect instead.
4. **Commands are delivery.** `router/command.rs` turns an `AppCommand` into
   intents of widgets. It moves to `dispatch/command.rs`.
5. **App events.** Window events, `SyncTerminalGridSizes` and `ResizeWindow`
   stay in `dispatch/mod.rs`. The I/O of `AppEvent::IcedReady` (writing the
   shell integration files, reading the host name) moves to
   `effects/startup.rs`, which then sends the first
   `TabsIntent::OpenTerminalTab` with `Task::done`, as today.
6. **Who calls what.** Until the rules are written again (decision 9), review checks this:
   - `XWidget::reduce` is called only from `dispatch/x.rs`;
   - the `run` of `effects/x.rs` is called only from `dispatch/mod.rs`. The
     one helper other effects files call is `push` in
     `effects/notifications.rs`, which only sends
     `NotificationsIntent::Push`;
   - the dispatch entry point is called only from `update.rs` and
     `dispatch/mod.rs`;
   - the I/O shared modules (`services`, `shell_integration`) are used only by
     `effects/`, by a widget's `services`, `storage` and `subscription`, and by
     each other;
   - a widget's `services` and `storage` are used only by `effects/x.rs`, and
     `storage` also by `XWidget::load`;
   - the clippy ban on tasks, files and processes is lifted on `mod effects`
     and not on `mod dispatch`;
   - code in `dispatch/` and `effects/` picks what to do by the kind of event
     only, not by widget state (today's `route-branches-on-state`);
   - parameters typed `XIntent` and `XEffect` are named `intent` and `effect`
     in both folders.
7. **Rule names.** When the rules come back, none is named after routes.
8. **Docs.** The layers in `docs/internal/architecture/app/` become App (now
   with dispatch), Effects (instead of Routing), Screens, Widgets, Domain and
   Shared code.
9. **No architecture rules during the change.** All rules of the
   architecture test were removed before the move (owner, 2026-11-09): they
   describe the old layers and would only get in the way. The test, the
   allowlist (now empty) and the report stay. The rules are written again,
   in a separate spec, once the owner is happy with the new layers. The
   coverage drop of that removal is accepted: the rules come back with their
   tests.
10. **Effects change no state.** An effects file only does I/O and sends
    events. Widget state changes only in its reducer, and App fields that
    belong to no widget change only in `dispatch/mod.rs`, from an app event
    the effect sends (for example the host name read at start, or the theme,
    terminal settings and shell session applied after Settings are saved).
    Today some route code writes these App fields itself; step 2 moves it as
    it is and step 2b fixes it (owner, 2026-11-09).
11. **The pull handle lives in the widget state.** `effects/containers.rs`
    no longer calls `ContainersWidget::set_pull_task` or `cancel_pull`. The
    effect that starts a pull sends the abort handle back as an intent, and
    the reducer keeps it in the state. On cancel the reducer takes the
    handle out of the state and puts it in the `CancelPull` effect, and the
    effects file only aborts it (owner, 2026-11-09). The handle lives in the
    state of the child that owns the pull, `run` or `create`, as
    `PullHandle`. One difference: before, one slot for the whole widget meant
    a create-dialog pull would stop a running Run-form pull; now each child
    keeps its own. No way to reach that case was found, since the Run form
    replaces the table the dialog opens from.
12. **Command availability leaves delivery.** `dispatch/command.rs` asks
    `AppCommand::is_available` before it runs a command; that is a decision
    in delivery, and it goes (owner, 2026-11-09). Where the decision goes is
    designed in step 2d and confirmed with the owner before the code changes.
    Outcome (owner, 2026-11-09): the sources check it. The input filter
    skips an unavailable shortcut command as its last step, after menus are
    closed and an inline edit is cancelled; the palette reducer checks a
    picked entry again and closes without running it; header buttons send
    only always-available commands. `command_ctx` lives in `guards.rs`, and
    `dispatch/command.rs` only translates.

## Steps

Each step ends with `cargo +nightly fmt`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo deny check` and
`cargo test --workspace --all-features` passing.

1. **Decisions out of delivery.** Tests first in the `terminal_workspace`
   reducer: each of the intents in decision 3 returns `SyncExplorer`. Then
   move the decision from the route into the reducer.
2. **The move.** `router/` becomes `dispatch/` and `effects/` as in
   decisions 1, 4 and 5, with the tests of each route moving next to the
   code they test. There are no architecture rules to change (decision 9).
2b. **Effects stop changing App fields** (decision 10). Tests first in
    `dispatch/`: each new app event writes its field. Then the effects send
    those events instead of writing the fields.
2c. **Pull handle** (decision 11). Tests first in the containers reducer.
2d. **Command availability** (decision 12). First a proposal for where the
    decision goes, for every command; then the change, tests first.
3. **Generator.** First `xtask/tests/new_widget.rs` expects `dispatch/x.rs`,
   `effects/x.rs` and the wiring in `dispatch/mod.rs` and `effects/mod.rs`;
   then the generator.
4. **Docs.** Decision 8, plus every place that names `router/`: `AGENTS.md`,
   `clippy.toml`, comments in `xtask`.
5. **Coverage.** `cargo llvm-cov --workspace --all-features` on the merge
   base and on the result; the result must be strictly higher.

## Out of scope

- `update.rs` also tells the Containers widget whether the right panel is on
  screen. It is App-layer flow and stays where it is.
- Splitting large effect files such as `router/containers.rs` (1200 lines)
  further. They move as they are.
