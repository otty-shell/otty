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
   `effects/startup.rs`.
6. **Who calls what.** Until the rules are written again (decision 9), review checks this:
   - `XWidget::reduce` is called only from `dispatch/x.rs`;
   - `effects/x.rs` is called only from `dispatch/mod.rs`;
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
3. **Generator.** First `xtask/tests/new_widget.rs` expects `dispatch/x.rs`,
   `effects/x.rs` and the wiring in `dispatch/mod.rs`; then the generator.
4. **Docs.** Decision 8, plus every place that names `router/`: `AGENTS.md`,
   `clippy.toml`, comments in `xtask`.
5. **Coverage.** `cargo llvm-cov --workspace --all-features` on the merge
   base and on the result; the result must be strictly higher.

## Out of scope

- `update.rs` also tells the Containers widget whether the right panel is on
  screen. It is App-layer flow and stays where it is.
- Splitting large effect files such as `router/containers.rs` (1200 lines)
  further. They move as they are.
