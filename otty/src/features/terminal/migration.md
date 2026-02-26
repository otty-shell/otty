# Terminal Feature Migration

## Goal

Convert `terminal` from legacy `terminal_reducer(&mut State, event)` pattern to
`TerminalFeature` implementing the shared `Feature` trait, per CONVENTIONS.md.

## Current Violations

- No `feature.rs` file — reducer lives in `event.rs`.
- `TerminalState` is owned by global `State` (`state.rs:208`), not by a feature struct.
- `terminal_to_tab: HashMap<u64, u64>` lives in global `State` (`state.rs:210`).
- `next_terminal_id: u64` lives in global `State` (`state.rs:211`).
- `terminal_reducer` receives `&mut State` — full mutable access to all state.
- `services::shell_cwd_for_active_tab` takes `&State` to traverse terminal + tab data.

## Inventory of External Access Points

Places in `app.rs` that read/write terminal state:

| Location | Access | Reads/Writes |
| --- | --- | --- |
| `subscription()` | `state.terminal.tabs()` | read terminal entries for subscriptions |
| `view()` / `tab_content::TabContentProps` | `&state.terminal` | read-only for rendering |
| `any_context_menu_open()` | `state.terminal.tabs()` | read context menus |
| `context_menu_layer()` | `state.terminal.tabs()` | read context menus |
| `apply_settings()` | calls `terminal_reducer` | write: apply theme |
| `close_all_context_menus()` | calls `terminal_reducer` | write: close menus |
| `dispatch_event` / `Terminal(event)` | calls `terminal_reducer` | write: main dispatch |
| `terminal_sync_followup()` | reads `TerminalEvent` variants | read-only pattern match |
| `open_terminal_tab()` etc. | `state.allocate_terminal_id()` | write: id allocation |

Places in `features/explorer/feature.rs`:

| Location | Access |
| --- | --- |
| `ExplorerCtx.app_state` | `shell_cwd_for_active_tab(&State)` reads terminal state |

## Migration Steps

### Step 1: Create `feature.rs`

Create `features/terminal/feature.rs` with:

```rust
pub(crate) struct TerminalFeature {
    state: TerminalState,
    terminal_to_tab: HashMap<u64, u64>,
    next_terminal_id: u64,
}
```

Define `TerminalCtx`:

```rust
pub(crate) struct TerminalCtx<'a> {
    pub(crate) pane_grid_size: Size,
    pub(crate) screen_size: Size,
    pub(crate) sidebar_cursor: Point,
}
```

Implement read-only query APIs:

- `tabs()` — iterator for subscriptions and view
- `tab(tab_id)` — single tab lookup
- `active_terminal_tab(active_tab_id)` — for explorer sync
- `has_any_context_menu()` — for `any_context_menu_open()`
- `context_menu_tab()` — for `context_menu_layer()`
- `allocate_terminal_id()` — moved from global State
- `terminal_tab_id(terminal_id)` — lookup from `terminal_to_tab`
- `register_terminal_for_tab()`, `remove_tab_terminals()`, `reindex_terminal_tabs()`

### Step 2: Move reducer logic into `Feature::reduce`

Convert `terminal_reducer` from free function in `event.rs` to
`impl Feature for TerminalFeature` in `feature.rs`.

The `event.rs` file keeps only the `TerminalEvent` enum and `Debug` impl.

Helper functions (`open_tab`, `reduce_widget_event`, `split_pane`, `close_pane`,
`copy_selection`, etc.) move to `feature.rs` as methods or private functions.

The `TerminalCommand` → `Task<AppEvent>` conversion (`execute_command`) stays in
`feature.rs` since it orchestrates iced widget tasks.

### Step 3: Move ownership out of global `State`

Remove from `state.rs`:

- `terminal: TerminalState` field
- `terminal_to_tab: HashMap<u64, u64>` field
- `next_terminal_id: u64` field
- Methods: `allocate_terminal_id`, `register_terminal_for_tab`,
  `remove_tab_terminals`, `terminal_tab_id`, `reindex_terminal_tabs`,
  `active_terminal_tab`, `terminal_tab`, `sync_tab_grid_sizes`, `pane_grid_size`

Move `pane_grid_size` calculation into a standalone function or into
`TerminalCtx` construction (app.rs computes it and passes as context field).

Move `sync_tab_grid_sizes` into `TerminalFeature::set_grid_size(size)`.

### Step 4: Register in `Features` container

Add to `features/mod.rs`:

```rust
pub(crate) struct Features {
    explorer: explorer::ExplorerFeature,
    quick_launch: quick_launch::QuickLaunchFeature,
    terminal: terminal::TerminalFeature,  // new
}
```

Add `terminal()` and `terminal_mut()` accessors.

### Step 5: Update `app.rs` dispatch

Replace:

```rust
Terminal(event) => {
    let sync_task = self.terminal_sync_followup(&event);
    let terminal_task = terminal_reducer(&mut self.state, event);
    Task::batch(vec![terminal_task, sync_task])
}
```

With:

```rust
Terminal(event) => {
    let sync_task = self.terminal_sync_followup(&event);
    let terminal_task = self.features.terminal_mut().reduce(event, &ctx);
    Task::batch(vec![terminal_task, sync_task])
}
```

Update `close_all_context_menus`, `apply_settings` to use
`features.terminal_mut().reduce(...)`.

### Step 6: Update `subscription()`

Replace `self.state.terminal.tabs()` with `self.features.terminal().tabs()`.

### Step 7: Update `view()`

Replace `&self.state.terminal` in `TabContentProps` with
`self.features.terminal()` (expose `TerminalState` or typed view proxy).

Replace `self.state.terminal.tabs()` in `context_menu_layer` with
`self.features.terminal()`.

### Step 8: Update tab lifecycle in `app.rs`

Replace `self.state.allocate_terminal_id()` with
`self.features.terminal_mut().allocate_terminal_id()`.

### Step 9: Decouple `shell_cwd_for_active_tab`

Current signature: `fn shell_cwd_for_active_tab(state: &State) -> Option<PathBuf>`

This reads:
1. `state.active_tab_id()` — from TabState
2. `state.terminal_tab(tab_id)` — from TerminalState
3. Terminal's focused entry blocks

After migration, change to:

```rust
pub(crate) fn shell_cwd_for_active_tab(
    active_tab_id: Option<u64>,
    terminal: &TerminalFeature,
) -> Option<PathBuf>
```

Update `ExplorerCtx` to pass `active_tab_id` + `&TerminalFeature` reference
instead of `app_state: &State` (this also resolves the P2 item about narrowing
ExplorerCtx).

### Step 10: Update `mod.rs` exports

```rust
mod errors;
mod event;
mod feature;  // new
mod model;
mod services;
mod state;

pub(crate) use event::TerminalEvent;
pub(crate) use feature::{TerminalCtx, TerminalFeature};
pub(crate) use model::{ShellSession, TerminalEntry, TerminalKind};
pub(crate) use services::{
    fallback_shell_session_with_shell, setup_shell_session_with_shell,
    shell_cwd_for_active_tab, terminal_settings_for_session,
};
pub(crate) use state::{TerminalState, TerminalTabState};
```

Remove `terminal_reducer` from exports.

### Step 11: Update tests

- Tests in `event.rs` that use `State::default()` + `terminal_reducer()`
  should be rewritten to use `TerminalFeature::new()` + `feature.reduce()`.
- Explorer tests that build `State` with terminal data need to construct
  `TerminalFeature` + pass it to updated `ExplorerCtx`.

### Step 12: Verify

- `cargo +nightly fmt`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo llvm-cov --workspace --all-features --fail-under-lines 80`
- `cargo deny check`

## Risk Assessment

**High complexity.** Terminal is the largest feature with the most external
touchpoints. The `subscription()` and `view()` rewiring are load-bearing and
must not break event flow.

**Key invariant:** after migration, `App` must never hold a reference to both
`features.terminal_mut()` and `features.explorer_mut()` simultaneously (borrow
checker enforces this naturally through `Features` accessor pattern).

**Recommended approach:** migrate in two phases:
1. First: create `TerminalFeature` struct, move state ownership, keep reducer as
   a method that delegates to existing helper functions.
2. Second: clean up helper functions, remove global State dependencies from
   function signatures, update tests.
