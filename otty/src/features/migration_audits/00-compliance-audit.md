# Features Convention Compliance Audit

Reference: `otty/src/features/CONVENTIONS.md`

---

## 1. Overall Compliance

| Feature | Score | Percentage |
|---------|-------|------------|
| **explorer** | 12 / 14 | 86% |
| **quick_launches** | 12 / 14 | 86% |
| **settings** | 12 / 14 | 86% |
| **tab** | 13 / 14 | 93% |
| **terminal** | 10 / 14 | 71% |
| **OVERALL** | **59 / 70** | **84%** |

### Criteria Used (14 items, derived from Sections 3-13)

| # | Criterion | Source |
|---|-----------|--------|
| C1 | Canonical file layout | Section 3 |
| C2 | mod.rs declaration order | Section 3 |
| C3 | mod.rs thin, no business logic | Section 4 |
| C4 | Exactly one primary state / event / reducer | Section 5 |
| C5 | Reducer signature `pub(crate)` returning `Task<AppEvent>` | Section 5.2 |
| C6 | mod.rs re-exports limited to stable API | Section 6 |
| C7 | Internal modules private (`mod`, not `pub mod`) | Section 6 |
| C8 | No wildcard re-exports | Section 6 |
| C9 | Intra-feature module dependency rules (state.rs → model/errors only; external crates allowed) | Section 7.1 |
| C10 | Cross-feature imports only through mod.rs re-exports | Section 7.2 |
| C11 | model.rs UI-runtime agnostic | Section 7.1 |
| C12 | Naming conventions | Section 9 |
| C13 | No anti-patterns (unwrap, logic in mod.rs, etc.) | Section 11 |
| C14 | Tests exist | Section 10 |

---

## 2. Per-Feature Compliance Matrix

Legend: `✓` = pass, `✗` = fail

| Criterion | explorer | quick_launches | settings | tab | terminal |
|-----------|----------|----------------|----------|-----|----------|
| C1 file layout | ✓ | ✓ | ✓ | ✓ | ✓ |
| C2 declaration order | ✗ | ✓ | ✓ | ✓ | ✗ |
| C3 mod.rs thin | ✓ | ✓ | ✓ | ✓ | ✓ |
| C4 one state/event/reducer | ✓ | ✓ | ✓ | ✓ | ✓ |
| C5 reducer signature | ✓ | ✓ | ✓ | ✓ | ✓ |
| C6 re-exports stable API only | ✗ | ✗ | ✗ | ✗ | ✗ |
| C7 private modules | ✓ | ✓ | ✓ | ✓ | ✓ |
| C8 no wildcard re-exports | ✓ | ✓ | ✓ | ✓ | ✓ |
| C9 intra-feature deps | ✓ | ✗ | ✗ | ✓ | ✗ |
| C10 cross-feature deps | ✓ | ✓ | ✓ | ✓ | ✓ |
| C11 model.rs agnostic | ✓ | ✓ | ✓ | ✓ | ✗ |
| C12 naming | ✓ | ✓ | ✓ | ✓ | ✓ |
| C13 no anti-patterns | ✓ | ✓ | ✓ | ✓ | ✓ |
| C14 tests exist | ✓ | ✓ | ✓ | ✓ | ✓ |

### Systemic violations (affect multiple features)

- **C6** fails for all 5 features — every feature re-exports types beyond the primary API.
- **C9** fails for 3 features — `state.rs` imports from `storage.rs`, `crate::app`, or cross-feature types.

---

## 3. Detailed Violations Per Feature

### 3.1 explorer (86%)

#### V-EXP-1 — mod.rs declaration order (C2)

**File:** `explorer/mod.rs:1-5`

```rust
// current
mod errors;
mod event;
mod model;
mod services;   // ← wrong position
mod state;
```

Convention Section 3 requires: `errors, event, model, state, storage, services, subfeatures`.
`services` is declared before `state`.

#### V-EXP-2 — Leaked internal type in re-exports (C6)

**File:** `explorer/mod.rs:10-11`

```rust
pub(crate) use event::{
    ExplorerDeps, ExplorerEvent, ExplorerLoadTarget, explorer_reducer,
};
```

| Re-export | Used outside feature? | Verdict |
|-----------|-----------------------|---------|
| `ExplorerError` | No external consumers (currently unused) | Primary type — keep |
| `ExplorerEvent` | `app.rs` | Primary — required |
| `explorer_reducer` | `app.rs` | Primary — required |
| `ExplorerState` | `state.rs`, UI | Primary — required |
| `ExplorerDeps` | `app.rs` (reducer contract) | Reducer contract — keep (see Decision D8) |
| `FileNode` | `ui/widgets/sidebar_workspace/explorer.rs` | Extended stable API — keep (see Decision D10) |
| **`ExplorerLoadTarget`** | **Only used inside `explorer/event.rs`** | **Leaked internal** |

`ExplorerLoadTarget` is a helper enum used solely within `explorer_reducer` for async load results. It has zero external consumers.

---

### 3.2 quick_launches (86%)

#### V-QL-1 — Excessive re-exports beyond primary API (C6)

**File:** `quick_launches/mod.rs:16-33`

Primary types (compliant):
- `QuickLaunchError`, `QuickLaunchEvent`, `quick_launches_reducer`, `QuickLaunchState`

Additional re-exports (all have external consumers in `app.rs` or `ui/widgets/`):

| Re-export | External consumer |
|-----------|-------------------|
| `ContextMenuAction` | `ui/widgets/quick_launches/context_menu.rs` |
| `QUICK_LAUNCHES_TICK_MS` | `app.rs` |
| `QuickLaunchSetupOutcome` | `app.rs` |
| `CommandSpec` | `ui/widgets/` |
| `CustomCommand` | `ui/widgets/` |
| `EnvVar` | `ui/widgets/` |
| `NodePath` | `ui/widgets/` |
| `QuickLaunch` | `ui/widgets/` |
| `QuickLaunchFolder` | `ui/widgets/` |
| `QuickLaunchNode` | `ui/widgets/` |
| `QuickLaunchType` | `ui/widgets/` |
| `SSH_DEFAULT_PORT` | `ui/widgets/` |
| `quick_launch_error_message` | `ui/widgets/quick_launches/error.rs` |
| `ContextMenuState` | `ui/widgets/` |
| `ContextMenuTarget` | `ui/widgets/` |
| `DropTarget` | `ui/widgets/` |
| `InlineEditKind` | `ui/widgets/` |
| `InlineEditState` | `ui/widgets/` |
| `LaunchInfo` | `ui/widgets/` |
| `QuickLaunchErrorState` | `ui/widgets/quick_launches/error.rs` |

All types have external consumers, but the re-export surface (24+ types) far exceeds the convention's "only stable API" rule. The convention lists exactly 4 categories of allowed re-exports (Section 6). Section 7.2 permits additions when cross-feature needs exist, but the volume indicates an architectural gap where UI widgets directly consume feature internals.

**Editor subfeature** (`editor/mod.rs:8`) also re-exports `QuickLaunchEditorMode` from `state.rs` which is a state-internal enum, though it has consumers in `ui/widgets/quick_launches/editor.rs`.

#### V-QL-2 — state.rs depends on storage.rs (C9)

**File:** `quick_launches/state.rs:9`

```rust
use super::storage::load_quick_launches;
```

Section 7.1: "state.rs MAY depend on model.rs and errors.rs."
`state.rs` imports `load_quick_launches` from `storage.rs`, violating the allowed dependency set.
The load function should be called from `event.rs` (the reducer), not from state.

#### Note — `#[rustfmt::skip]` on all module declarations

**File:** `quick_launches/mod.rs:1-13`

Every module declaration has `#[rustfmt::skip]`. This is an artifact with no purpose — to be removed (see Decision D5).

---

### 3.3 settings (86%)

#### V-SET-1 — Re-exports beyond primary API (C6)

**File:** `settings/mod.rs:10-13`

Primary types (compliant):
- `SettingsError`, `SettingsEvent`, `settings_reducer`, `SettingsState`

Additional re-exports:

| Re-export | External consumer | Verdict |
|-----------|-------------------|---------|
| `SettingsData` | `app.rs` (in AppEvent variant) | Extended stable API — keep |
| `is_valid_hex_color` | `ui/widgets/settings.rs` | Move to services.rs (see Decision D7) |
| `palette_label` | `ui/widgets/settings.rs` | Move to services.rs (see Decision D7) |
| `SettingsNode` | `ui/widgets/settings.rs` | Extended stable API — keep (see Decision D10) |
| `SettingsPreset` | `ui/widgets/settings.rs` | Extended stable API — keep (see Decision D10) |
| `SettingsSection` | `ui/widgets/settings.rs` | Extended stable API — keep (see Decision D10) |

#### V-SET-2 — state.rs depends on storage.rs (C9)

**File:** `settings/state.rs:5`

```rust
use super::storage::{SettingsLoad, SettingsLoadStatus, load_settings};
```

Section 7.1: "state.rs MAY depend on model.rs and errors.rs."
`state.rs` imports three items from `storage.rs`. The `load_settings` call and `SettingsLoad`/`SettingsLoadStatus` types should be wired through the reducer in `event.rs`.

---

### 3.4 tab (93%)

#### V-TAB-1 — Re-exports beyond primary API (C6)

**File:** `tab/mod.rs:5-7`

Primary types (compliant):
- `TabEvent`, `tab_reducer`, `TabState`

Additional re-exports:

| Re-export | External consumer | Verdict |
|-----------|-------------------|---------|
| `TabDeps` | `app.rs` (reducer contract) | Reducer contract — keep (see Decision D8) |
| `TabContent` | `app.rs`, `state.rs`, `ui/widgets/`, `quick_launches/editor` | Extended stable API — keep (see Decision D9) |
| `TabItem` | `app.rs`, `state.rs`, `quick_launches/editor` | Extended stable API — keep (see Decision D9) |
| `TabOpenRequest` | `app.rs`, `quick_launches/event.rs` | Extended stable API — keep (see Decision D9) |

All additional re-exports are heavily used by external consumers and represent core domain types that other features depend on. This is the mildest C6 violation — every extra type is justified by genuine cross-feature need.

**Note:** Tab has no `errors.rs`. No explicit error types are defined in the feature, so this is correct per convention ("errors.rs required when explicit feature error exists").

---

### 3.5 terminal (71%)

#### V-TERM-1 — mod.rs declaration order (C2)

**File:** `terminal/mod.rs:1-5`

```rust
// current
mod errors;
mod event;
mod model;
mod services;   // ← wrong position
mod state;
```

Convention Section 3 requires: `errors, event, model, state, storage, services, subfeatures`.
`services` is declared before `state` (same issue as explorer).

#### V-TERM-2 — Re-exports beyond primary API (C6)

**File:** `terminal/mod.rs:9-17`

Primary types (compliant):
- `TerminalError`, `TerminalEvent`, `terminal_reducer`, `TerminalState`

Additional re-exports:

| Re-export | External consumer | Verdict |
|-----------|-------------------|---------|
| `shell_cwd_for_active_tab` | `tab/event.rs`, `explorer/event.rs` | Move to services.rs (see Decision D3) |
| `ShellSession` | `app.rs`, `tab/event.rs` | Extended stable API — keep |
| `TerminalEntry` | `ui/widgets/terminal/view.rs` | Extended stable API — keep |
| `TerminalKind` | `tab/event.rs`, `explorer/event.rs` | Extended stable API — keep |
| `fallback_shell_session_with_shell` | `app.rs` | Extended stable API — keep |
| `setup_shell_session_with_shell` | `app.rs` | Extended stable API — keep |
| `terminal_settings_for_session` | `explorer/event.rs`, `quick_launches/services.rs` | Extended stable API — keep |

All additional re-exports have external consumers. Like tab, this is a mild C6 violation where every type is genuinely needed.

#### V-TERM-3 — model.rs uses UI-runtime types (C11)

**File:** `terminal/model.rs:1`

```rust
use iced::widget::pane_grid;
```

Section 7.1: "model.rs MUST remain UI-runtime agnostic."
`TerminalEntry` (line 29) contains `pane_grid::Pane`, an iced-specific widget type. This couples the domain model directly to the iced runtime.

#### V-TERM-4 — state.rs illegal dependencies (C9)

**File:** `terminal/state.rs:3,9`

```rust
use iced::{Point, Size, Task, widget::Id, widget::pane_grid};
use crate::{app::Event as AppEvent, features::tab::TabEvent};
```

Section 7.1: "state.rs MAY depend on model.rs and errors.rs."

Violations:
- `iced::Task` + `crate::app::Event` — state.rs produces `Task<AppEvent>`, meaning reduction logic has leaked from `event.rs` into `state.rs`.
- `crate::features::tab::TabEvent` — cross-feature dependency in state.rs, beyond allowed model/errors scope.
- `iced::widget::pane_grid` — UI-runtime type in state operations.

This is the most significant architectural violation in the codebase. State methods are performing reducer-level work (dispatching events, creating tasks) instead of being pure mutation helpers.

---

## 4. Architectural Decisions

These decisions were taken during audit review and govern all migration plans below.

| ID | Question | Decision |
|----|----------|----------|
| D1 | TerminalEntry location | **Stays in model.rs** — `otty_ui_term::Terminal` is domain logic, not UI-runtime. `pane_grid::Pane` violates C11. Fix: remove the `pane` field from `TerminalEntry` entirely; `TerminalState` adds a private `pane_for_terminal(terminal_id: u64) -> Option<pane_grid::Pane>` reverse-lookup helper (iterates `pane_grid::State<u64>` which maps panes to terminal IDs). |
| D2 | state.rs reducer logic extraction strategy | **Command pattern** — state.rs methods return a lightweight `TerminalCommand` enum (may use `iced::widget::Id`; must not use `iced::Task` or `AppEvent`), event.rs maps commands to `Task<AppEvent>`. |
| D3 | `shell_cwd_for_active_tab` location | **Move to services.rs** — accepts `&State` (app-level); services.rs may take full state as a parameter. |
| D4 | quick_launches 24+ re-exports | **Accept as extended stable API** — all have external consumers. Document in convention. |
| D5 | `#[rustfmt::skip]` on mod declarations | **Remove** — artifact with no purpose. |
| D6 | `SettingsState::load()` bootstrap flow | **Sync factory in event.rs** — remove `load()` from state.rs; expose `pub(crate) fn bootstrap_settings() -> SettingsState` in event.rs (calls storage synchronously). state.rs gets a pure `from_data(SettingsData, SettingsLoadStatus) -> Self` constructor. Same pattern for quick_launches. |
| D7 | `is_valid_hex_color`, `palette_label` | **Exported functions go through services.rs.** `is_valid_hex_color` stays in model.rs (used internally by model.rs and state.rs); services.rs re-exports it so mod.rs sources it from services. `palette_label` has no internal usage — moves its definition to services.rs directly. |
| D8 | Deps structs as re-exports | **Part of reducer contract** — add deps structs to convention as an allowed re-export category. |
| D9 | `TabContent`, `TabItem`, `TabOpenRequest` | **Keep in tab** — document as extended stable API. Ownership stays with tab feature. |
| D10 | `features/` ↔ `ui/widgets/` contract | **UI is consumer** — features export whatever UI needs. No view-model indirection required. |

---

## 5. Migration Plans

### 5.1 explorer → 100%

**Violations to fix:** V-EXP-1, V-EXP-2

#### Step 1 — Fix declaration order (V-EXP-1)

In `explorer/mod.rs`, reorder module declarations:

```rust
// before
mod errors;
mod event;
mod model;
mod services;
mod state;

// after
mod errors;
mod event;
mod model;
mod state;
mod services;
```

#### Step 2 — Remove leaked re-export (V-EXP-2)

Remove `ExplorerLoadTarget` from `mod.rs` re-exports. Since it is only used inside `explorer/event.rs`, no external code needs to be updated.

```rust
// before
pub(crate) use event::{
    ExplorerDeps, ExplorerEvent, ExplorerLoadTarget, explorer_reducer,
};

// after
pub(crate) use event::{ExplorerDeps, ExplorerEvent, explorer_reducer};
```

Ensure `ExplorerLoadTarget` in `event.rs` is `pub(super)` or `pub(crate)` only within the feature (it currently is — no change needed in `event.rs`).

---

### 5.2 quick_launches → 100%

**Violations to fix:** V-QL-1, V-QL-2

#### Step 1 — Accept re-exports as extended stable API (V-QL-1) [Decision D4]

All 24+ re-exported types have verified external consumers in `app.rs` or `ui/widgets/`. Per Decision D4, these are accepted as extended stable API. No types need removal.

Actions:
- Remove `#[rustfmt::skip]` from all module declarations (Decision D5).
- No structural changes to re-exports — all are accepted as extended stable API per Decisions D4/D10.

#### Step 2 — Sync bootstrap factory and remove storage from state.rs (V-QL-2) [Decision D6]

Same pattern as settings. Bootstrap loading stays synchronous, storage call moves from state.rs to event.rs.

1. Remove `use super::storage::load_quick_launches;` from `state.rs`.
2. Remove `pub(crate) fn load() -> Self` from `state.rs`.
3. Add a pure data constructor to `state.rs`:

```rust
// state.rs
pub(crate) fn from_data(data: Option<QuickLaunchFile>) -> Self {
    // current from_loaded logic without the storage call
}
```

4. Add a sync bootstrap factory to `event.rs`:

```rust
// event.rs
pub(crate) fn bootstrap_quick_launches() -> QuickLaunchState {
    QuickLaunchState::from_data(storage::load_quick_launches().ok().flatten())
}
```

5. Update the bootstrap call site to call `quick_launches::bootstrap_quick_launches()` instead of `QuickLaunchState::load()`.

---

### 5.3 settings → 100%

**Violations to fix:** V-SET-1, V-SET-2

#### Step 1 — Adjust re-exports (V-SET-1) [Decisions D7, D10]

| Current re-export | Source after migration |
|-------------------|----------------------|
| `SettingsData` | `model` — unchanged |
| `is_valid_hex_color` | `services` — re-exported from model.rs (implementation stays in model.rs) |
| `palette_label` | `services` — definition moves to services.rs |
| `SettingsNode` | `model` — unchanged, extended stable API for UI |
| `SettingsPreset` | `model` — unchanged, extended stable API for UI |
| `SettingsSection` | `model` — unchanged, extended stable API for UI |

Migration steps:
1. Move `palette_label` definition from `model.rs` to `services.rs` (no internal consumers in model.rs or state.rs).
2. In `services.rs`, add `pub(crate) use super::model::is_valid_hex_color;` — re-exports the model.rs implementation.
3. Update `mod.rs`: re-export both functions from `services` instead of `model`:

```rust
// before
pub(crate) use model::{SettingsData, is_valid_hex_color, palette_label};

// after
pub(crate) use model::SettingsData;
pub(crate) use services::{is_valid_hex_color, palette_label};
```

4. `ui/widgets/settings.rs` imports are unchanged (they go through `mod.rs`).

#### Step 2 — Sync bootstrap factory and remove storage from state.rs (V-SET-2) [Decision D6]

Bootstrap loading stays synchronous. The fix moves the storage call from state.rs to event.rs.

`SettingsState::from_settings(SettingsData) -> Self` already exists in state.rs — no new constructor needed. Status handling (logging invalid/missing) lives in the bootstrap function.

1. Remove from `state.rs`:
   - `use super::storage::{SettingsLoad, SettingsLoadStatus, load_settings};`
   - `pub(crate) fn load() -> Self`
   - `pub(super) fn read_settings_payload<Load>(...)` (moves to event.rs)

2. Add a sync bootstrap factory to `event.rs` (not an event variant — a plain function). Absorbs `read_settings_payload` logic inline:

```rust
// event.rs
pub(crate) fn bootstrap_settings() -> SettingsState {
    let data = match storage::load_settings() {
        Ok(load) => {
            let (data, status) = load.into_parts();
            if let SettingsLoadStatus::Invalid(message) = status {
                log::warn!("settings file invalid: {message}");
            }
            data
        }
        Err(err) => {
            log::warn!("settings read failed: {err}");
            SettingsData::default()
        }
    };
    SettingsState::from_settings(data)
}
```

3. Re-export from `mod.rs`:

```rust
pub(crate) use event::{SettingsEvent, bootstrap_settings, settings_reducer};
```

4. Update the bootstrap call site to call `settings::bootstrap_settings()` instead of `SettingsState::load()`.

---

### 5.4 tab → 100%

**Violations to fix:** V-TAB-1

#### Step 1 — Document re-exports as extended stable API (V-TAB-1) [Decisions D8, D9]

All re-exported model types (`TabContent`, `TabItem`, `TabOpenRequest`) and the deps struct (`TabDeps`) are heavily used across `app.rs`, `state.rs`, `ui/widgets/`, and sibling features.

Per Decision D9, these remain in tab with ownership preserved. Per Decision D8, `TabDeps` is part of the reducer contract.

No code changes required — tab is effectively compliant. The convention will be updated to explicitly allow these re-export categories (see Section 7).

---

### 5.5 terminal → 100%

**Violations to fix:** V-TERM-1, V-TERM-2, V-TERM-3, V-TERM-4

This is the most significant migration. Violations V-TERM-3 and V-TERM-4 are architectural issues, not simple refactors.

#### Step 1 — Fix declaration order (V-TERM-1)

In `terminal/mod.rs`, reorder module declarations:

```rust
// before
mod errors;
mod event;
mod model;
mod services;
mod state;

// after
mod errors;
mod event;
mod model;
mod state;
mod services;
```

#### Step 2 — Move `shell_cwd_for_active_tab` to services.rs and document re-exports (V-TERM-2) [Decision D3]

1. Move `shell_cwd_for_active_tab` from `event.rs` to `services.rs`. The function signature stays unchanged — it accepts `&State` (app-level), which is acceptable for services.rs.
2. Update `mod.rs` re-export to source from `services` instead of `event`:

```rust
// before
pub(crate) use event::{TerminalEvent, shell_cwd_for_active_tab, terminal_reducer};

// after
pub(crate) use event::{TerminalEvent, terminal_reducer};
pub(crate) use services::shell_cwd_for_active_tab;
```

3. All other additional re-exports have external consumers and are accepted as extended stable API. No other changes needed.

#### Step 3 — Make model.rs UI-runtime agnostic (V-TERM-3) [Decision D1]

`TerminalEntry` stays in `model.rs`. Remove the `pane: pane_grid::Pane` field — it is the only iced dependency. `pane_grid::Pane` is opaque (no public numeric ID), so a `PaneId(u64)` newtype is not feasible without keeping the iced dependency. The reverse-lookup pattern eliminates the need for the field entirely.

1. Remove `pane: pane_grid::Pane` from `TerminalEntry`.
2. Remove `use iced::widget::pane_grid;` from `model.rs`.

```rust
// model.rs — iced import removed
pub(crate) struct TerminalEntry {
    pub(crate) terminal: otty_ui_term::Terminal,  // domain logic — unchanged
    pub(crate) title: String,
}
```

3. In `state.rs`, add a private reverse-lookup helper (pane grid is tiny, O(n) is fine):

```rust
// state.rs
fn pane_for_terminal(&self, terminal_id: u64) -> Option<pane_grid::Pane> {
    self.panes
        .layout()
        .panes()
        .find(|(_, &id)| id == terminal_id)
        .map(|(pane, _)| pane)
}
```

4. Update the one internal call site in `state.rs` that used `entry.pane` (`handle_terminal_event::Shutdown`):

```rust
// before
Shutdown { .. } => {
    if let Some(entry) = self.terminals.get(&terminal_id) {
        let pane = entry.pane;
        return self.close_pane(pane);
    }
}

// after
Shutdown { .. } => {
    if let Some(pane) = self.pane_for_terminal(terminal_id) {
        return self.close_pane(pane);
    }
}
```

5. Update `TerminalEntry` construction sites in `state.rs` to remove the `pane` field:

```rust
// before
TerminalEntry { pane: initial_pane, terminal, title: ... }

// after
TerminalEntry { terminal, title: ... }
```

Note: `ui/widgets/terminal/view.rs` receives `&HashMap<u64, TerminalEntry>` and renders via `PaneGrid::new(&state.panes(), ...)` — pane association is handled by the pane_grid widget itself, not via `entry.pane`. No changes needed in view.rs.

#### Step 4 — Extract reducer logic from state.rs using command pattern (V-TERM-4) [Decision D2]

This is the largest refactor. `terminal/state.rs` currently produces `Task<AppEvent>` and dispatches cross-feature events, which is reducer-level work.

**Command pattern approach (Decision D2):**

1. Define a `TerminalCommand` enum in `state.rs`. It MAY use `iced::widget::Id` (state.rs external crate imports are allowed per Decision D12/C9 clarification). It MUST NOT use `iced::Task` or `AppEvent`:

```rust
// state.rs
use iced::widget::Id;

pub(crate) enum TerminalCommand {
    None,
    FocusWidget(Id),
    SelectAll(Id),
    CloseTab { tab_id: u64 },
    Batch(Vec<TerminalCommand>),
}
```

2. Refactor state methods: `&mut self → TerminalCommand` instead of `&mut self → Task<AppEvent>`.

3. In `event.rs`, add a private mapper from command to Task:

```rust
// event.rs
fn execute_command(command: TerminalCommand) -> Task<AppEvent> {
    match command {
        TerminalCommand::None => Task::none(),
        TerminalCommand::FocusWidget(id) => TerminalView::focus(id),
        TerminalCommand::SelectAll(id) => TerminalView::focus(id), // + select task
        TerminalCommand::CloseTab { tab_id } => {
            Task::done(AppEvent::Tab(TabEvent::CloseTab(tab_id)))
        }
        TerminalCommand::Batch(cmds) => {
            Task::batch(cmds.into_iter().map(execute_command))
        }
    }
}
```

4. Remove from `state.rs`:
   - `use iced::Task;`
   - `use crate::app::Event as AppEvent;`
   - `use crate::features::tab::TabEvent;`

5. Target `state.rs` imports after migration:

```rust
use std::collections::HashMap;
use iced::{Point, Size, widget::Id, widget::pane_grid};
use otty_ui_term::{BlockCommand, SurfaceMode, TerminalView, settings::{Settings, ThemeSettings}};
use super::errors::TerminalError;
use super::model::{BlockSelection, TerminalEntry, TerminalKind};
```

Note: `iced::widget::pane_grid`, `iced::widget::Id`, and `otty_ui_term` in `state.rs` are acceptable — C9 restricts intra-feature module dependencies only, external crate imports are allowed (Decision D12). `TerminalEntry` is still imported — the struct remains in model.rs, only the `pane` field is removed.

---

## 6. Priority Order

Ordered by impact and risk (lowest risk first):

| Priority | Feature | Effort | Risk |
|----------|---------|--------|------|
| 1 | **explorer** | Low | Minimal — declaration reorder + one re-export removal |
| 2 | **tab** | Minimal | None — document extended API |
| 3 | **settings** | Medium | Low — palette_label to services, is_valid_hex_color re-exported via services, sync bootstrap factory in event.rs |
| 4 | **quick_launches** | Medium | Low — remove `#[rustfmt::skip]`, sync bootstrap factory in event.rs |
| 5 | **terminal** | High | Medium — PaneId newtype in model.rs, shell_cwd_for_active_tab to services.rs, command pattern extraction from state.rs |

---

## 7. Convention Updates Required

The following changes are needed in `CONVENTIONS.md` to align the specification with the decided architecture:

### 7.1 Clarify C9 scope — external crate imports in state.rs (Decision D12)

**Section 7.1** says "state.rs MAY depend on model.rs and errors.rs." This currently reads as if all external crate imports are forbidden in state.rs.

Resolution: Add clarification that C9 governs **intra-feature module dependencies** only. External crate imports (`iced`, `std`, `otty_ui_term`, etc.) are allowed in all modules. The restriction is: within the feature's own module graph, state.rs may only import from its `model.rs` and `errors.rs` — not from `storage.rs`, `services.rs`, or sibling features.

### 7.2 Add deps structs to allowed re-exports (Decision D8)

**Section 6** currently lists 4 allowed re-export categories. Add a 5th:

> - dependency struct for the reducer entrypoint (e.g., `FeatureDeps`)

This aligns with the existing reducer signature pattern already shown in Section 5.2.

### 7.3 Clarify extended stable API for UI consumers (Decisions D4, D9, D10)

**Section 6** and **Section 7.2** have a tension: Section 6 limits re-exports strictly, while Section 7.2 requires re-exports when sibling features need them.

Resolution: Add explicit language that re-exports with verified external consumers (`app.rs`, `ui/widgets/`, sibling features) are compliant extended stable API. The convention MUST NOT require view-model indirection — `ui/widgets/` is a first-class consumer and features MAY re-export any types it needs directly.

---

## 8. Acceptance Criteria

### 8.1 Global (all features)

| # | Criterion | Verification |
|---|-----------|-------------|
| G1 | Project compiles | `cargo build` exits 0 |
| G2 | No clippy warnings | `cargo clippy --workspace --all-targets` exits 0 with no warnings |
| G3 | Formatting clean | `cargo fmt --check` exits 0 |
| G4 | All tests pass | `cargo test --workspace` exits 0 |
| G5 | No behavioral regressions | Application launches, all features function as before migration |
| G6 | Compliance matrix all green | Re-run audit — every feature scores 14/14, overall 70/70 (100%) |

### 8.2 explorer

| # | Criterion | Verification |
|---|-----------|-------------|
| A-EXP-1 | Declaration order fixed | `explorer/mod.rs` declares modules in order: `errors, event, model, state, services` |
| A-EXP-2 | `ExplorerLoadTarget` not re-exported | `grep "ExplorerLoadTarget" explorer/mod.rs` returns nothing |
| A-EXP-3 | `ExplorerLoadTarget` still usable internally | `grep "ExplorerLoadTarget" explorer/event.rs` returns matches — type is defined and used within the module |

### 8.3 quick_launches

| # | Criterion | Verification |
|---|-----------|-------------|
| A-QL-1 | `#[rustfmt::skip]` removed | `grep "rustfmt::skip" quick_launches/mod.rs` returns nothing |
| A-QL-2 | state.rs has no storage import | `grep "super::storage" quick_launches/state.rs` returns nothing |
| A-QL-3 | state.rs has no `load()` method | `grep "fn load" quick_launches/state.rs` returns nothing |
| A-QL-4 | `from_data` constructor exists in state.rs | `grep "fn from_data" quick_launches/state.rs` returns a match |
| A-QL-5 | Bootstrap factory exists in event.rs | `grep "fn bootstrap_quick_launches" quick_launches/event.rs` returns a match |
| A-QL-6 | Bootstrap factory re-exported from mod.rs | `grep "bootstrap_quick_launches" quick_launches/mod.rs` returns a match |
| A-QL-7 | Bootstrap call site updated | `grep "QuickLaunchState::load()" otty/src/` returns nothing; `grep "bootstrap_quick_launches()" otty/src/` returns the call site |

### 8.4 settings

| # | Criterion | Verification |
|---|-----------|-------------|
| A-SET-1 | `palette_label` defined in services.rs | `grep "fn palette_label" settings/services.rs` returns a match |
| A-SET-2 | `palette_label` not defined in model.rs | `grep "fn palette_label" settings/model.rs` returns nothing |
| A-SET-3 | `is_valid_hex_color` re-exported via services | `grep "is_valid_hex_color" settings/services.rs` returns a re-export line |
| A-SET-4 | mod.rs sources functions from services | `grep "use services::" settings/mod.rs` shows `is_valid_hex_color` and `palette_label` |
| A-SET-5 | mod.rs does NOT source functions from model | `grep "use model::.*palette_label\|use model::.*is_valid_hex_color" settings/mod.rs` returns nothing |
| A-SET-6 | state.rs has no storage import | `grep "super::storage" settings/state.rs` returns nothing |
| A-SET-7 | state.rs has no `load()` method | `grep "fn load" settings/state.rs` returns nothing |
| A-SET-8 | state.rs has no `read_settings_payload` | `grep "read_settings_payload" settings/state.rs` returns nothing |
| A-SET-9 | Bootstrap factory exists in event.rs | `grep "fn bootstrap_settings" settings/event.rs` returns a match |
| A-SET-10 | Bootstrap factory re-exported from mod.rs | `grep "bootstrap_settings" settings/mod.rs` returns a match |
| A-SET-11 | Bootstrap call site updated | `grep "SettingsState::load()" otty/src/` returns nothing; `grep "bootstrap_settings()" otty/src/` returns the call site |
| A-SET-12 | UI imports unchanged | `ui/widgets/settings.rs` still imports `is_valid_hex_color` and `palette_label` via `crate::features::settings::` — no path changes needed |

### 8.5 tab

| # | Criterion | Verification |
|---|-----------|-------------|
| A-TAB-1 | No code changes | `git diff` for `tab/` shows zero changes (tab is already compliant after convention update) |
| A-TAB-2 | Convention updated | `CONVENTIONS.md` Section 6 includes deps structs and extended stable API language (see Section 7 of this document) |

### 8.6 terminal

| # | Criterion | Verification |
|---|-----------|-------------|
| A-TERM-1 | Declaration order fixed | `terminal/mod.rs` declares modules in order: `errors, event, model, state, services` |
| A-TERM-2 | `shell_cwd_for_active_tab` in services.rs | `grep "fn shell_cwd_for_active_tab" terminal/services.rs` returns a match |
| A-TERM-3 | `shell_cwd_for_active_tab` not in event.rs | `grep "fn shell_cwd_for_active_tab" terminal/event.rs` returns nothing |
| A-TERM-4 | mod.rs re-export sourced from services | `grep "services::shell_cwd_for_active_tab" terminal/mod.rs` returns a match |
| A-TERM-5 | model.rs has no iced import | `grep "use iced" terminal/model.rs` returns nothing |
| A-TERM-6 | `TerminalEntry` has no `pane` field | `grep "pane:" terminal/model.rs` returns nothing (in struct definition context) |
| A-TERM-7 | Reverse-lookup helper exists in state.rs | `grep "fn pane_for_terminal" terminal/state.rs` returns a match |
| A-TERM-8 | state.rs has no `iced::Task` import | `grep "iced::Task\|iced::{.*Task" terminal/state.rs` returns nothing |
| A-TERM-9 | state.rs has no `AppEvent` import | `grep "app::Event\|AppEvent" terminal/state.rs` returns nothing |
| A-TERM-10 | state.rs has no `TabEvent` import | `grep "TabEvent" terminal/state.rs` returns nothing |
| A-TERM-11 | `TerminalCommand` enum exists in state.rs | `grep "enum TerminalCommand" terminal/state.rs` returns a match |
| A-TERM-12 | `execute_command` mapper exists in event.rs | `grep "fn execute_command" terminal/event.rs` returns a match |
| A-TERM-13 | State methods return `TerminalCommand` | No state method signature contains `-> Task<AppEvent>` — `grep "-> Task<AppEvent>" terminal/state.rs` returns nothing |
| A-TERM-14 | External callers unaffected | `tab/event.rs`, `explorer/event.rs`, `quick_launches/services.rs` import terminal types through `crate::features::terminal::` — no import path changes |

### 8.7 Convention updates

| # | Criterion | Verification |
|---|-----------|-------------|
| A-CONV-1 | C9 scope clarified | `CONVENTIONS.md` Section 7.1 explicitly states C9 governs intra-feature module deps only; external crate imports are allowed |
| A-CONV-2 | Deps struct category added | `CONVENTIONS.md` Section 6 lists dependency struct (e.g., `FeatureDeps`) as an allowed re-export category |
| A-CONV-3 | Extended stable API documented | `CONVENTIONS.md` states that re-exports with verified external consumers are compliant extended stable API |

### 8.8 Verification procedure

After all migrations are complete, run the following sequence:

```bash
# 1. Build
cargo build

# 2. Format check
cargo fmt --check

# 3. Lint
cargo clippy --workspace --all-targets

# 4. Tests
cargo test --workspace

# 5. Structural grep checks (from otty/src/features/)
# explorer
grep "ExplorerLoadTarget" explorer/mod.rs           # expect: no output

# quick_launches
grep "rustfmt::skip" quick_launches/mod.rs          # expect: no output
grep "super::storage" quick_launches/state.rs       # expect: no output
grep "fn bootstrap_quick_launches" quick_launches/event.rs  # expect: match

# settings
grep "fn palette_label" settings/services.rs        # expect: match
grep "super::storage" settings/state.rs             # expect: no output
grep "fn bootstrap_settings" settings/event.rs      # expect: match

# terminal
grep "fn shell_cwd_for_active_tab" terminal/services.rs    # expect: match
grep "use iced" terminal/model.rs                          # expect: no output
grep "fn pane_for_terminal" terminal/state.rs              # expect: match
grep "enum TerminalCommand" terminal/state.rs              # expect: match
grep "fn execute_command" terminal/event.rs                # expect: match
grep "iced::Task\|AppEvent\|TabEvent" terminal/state.rs    # expect: no output
grep "-> Task<AppEvent>" terminal/state.rs                 # expect: no output
```

All grep checks with "expect: no output" must produce zero matches. All grep checks with "expect: match" must produce at least one match. If any check fails, the migration for that feature is incomplete.
