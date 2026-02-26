# Features Conventions

## 1. Scope

This document is a normative specification for `otty/src/features`.

- A feature module MUST encapsulate one domain slice: event contract, state contract, and reduction logic.
- The goal is deterministic architecture and near-identical module topology across features.
- This specification defines the struct-based flow: each feature is a struct that owns its state and exposes a single `reduce` method.

## 2. Compliance Profile

- MUST satisfy all sections in this document.
- MUST have canonical file topology from section 3.
- MUST implement one feature struct and one feature event enum.
- MUST expose `reduce` as the only write entrypoint.
- Feature state MUST be owned by the feature struct, not by global app state.
- Feature state MUST be mutated only inside that feature's `reduce` implementation.
- Cross-feature influence MUST be expressed only by returning `Task<AppEvent>` with routed events.
- Feature internals MUST NOT be mutated directly from `app.rs` or sibling features.

## 3. Canonical Layout

Each feature under `otty/src/features/<feature_name>/` MUST follow:

```text
├── mod.rs         # required: public surface only
├── feature.rs     # required: <Feature>Feature + reducer implementation
├── event.rs       # required: <Feature>Event
├── state.rs       # required: <Feature>State + state helpers
├── model.rs       # required: domain data
├── errors.rs      # optional: feature errors
├── storage.rs     # optional: persistence boundary only
├── services.rs    # optional: side-effect adapters/integrations
└── subfeature/    # optional: subfeature mirrors same contract
  ├── mod.rs
  ├── feature.rs
  ├── event.rs
  ├── state.rs
  ├── model.rs
  ├── errors.rs
  ├── storage.rs
  └── services.rs
```

Files outside required/optional lists MUST NOT be added without updating this specification.

## 4. Module Responsibilities

- `mod.rs`: module declarations, curated re-exports. Business logic MUST NOT be added here.
- `feature.rs`: feature struct, owned state, reducer implementation, event orchestration.
- `event.rs`: public feature event enum and event-only helpers.
- `state.rs`: internal runtime state struct and deterministic state mutation helpers.
- `model.rs`: domain data structs and their trait implementations. A type belongs in `model.rs` if and only if it is used across **two or more** of the feature's modules (e.g. `state`, `services`, `event`, UI). If a type is used only inside a single module, it MUST be defined in that module instead.
- `errors.rs`: explicit error type for feature boundaries.
- `storage.rs`: serialization/deserialization and filesystem I/O only.
- `services.rs`: bounded external interactions and side-effect adapters.

## 5. Reducer Method Contract

### 5.1 Method Shape

All features MUST expose a reducer method on the primary feature struct.

```rust
impl ExampleFeature {
    pub(crate) fn reduce(
        &mut self,
        event: ExampleEvent,
        ctx: &ExampleCtx<'_>,
    ) -> iced::Task<crate::app::Event> {
        // ...
    }
}
```

### 5.2 Context Rules

- `reduce` MUST always accept a read-only context reference.
- Features without runtime dependencies MUST accept `&()` and ignore context.
- Features with runtime dependencies MUST define a feature-specific context type (e.g. `ExplorerCtx<'a>` or `ExampleCtx`).
- Context MUST be read-only and MUST NOT expose mutable access to other features.

### 5.3 Primary Types

Each feature MUST define:

- Exactly one primary feature struct named `<Feature>Feature`.
- Exactly one primary state type named `<Feature>State`.
- Exactly one primary event enum named `<Feature>Event`.

`Exactly one` means one external `reduce` entrypoint per feature.

## 6. State Ownership And Mutation

- `<Feature>Feature` MUST own `<Feature>State` as a private field.
- `<Feature>State` fields MUST be private.
- `<Feature>State` MUST NOT be stored as a separate mutable field in global app state.
- External code MAY read feature data only through read-only feature APIs.
- External code MUST NOT receive raw mutable references to feature state.
- Reusable mutation logic SHOULD live on `<Feature>State` methods and be called only from `reduce`.
- **Test exception:** `#[cfg(test)]` code within the feature's own module tree MAY access `<Feature>State` methods directly to set up test fixtures without going through `reduce`. This exception MUST NOT be used outside the feature's own module tree.

## 7. Feature Container And Registration

- `features/mod.rs` MUST register features as `pub(crate) mod <feature>;`.
- App composition SHOULD use a dedicated container struct (e.g. `Features`) that owns all `<Feature>Feature` instances.
- `app.rs` MUST route `AppEvent` to the owning feature's `reduce` implementation.
- Feature reducers MUST NOT call other features' `reduce` directly.

## 8. API Exposure

- A feature's `mod.rs` MUST be the only import surface for sibling features and app wiring.
- Feature-internal module declarations inside `<feature>/mod.rs` MUST use private `mod ...;`.
- Feature-internal `mod.rs` MUST re-export only items that external consumers (sibling features, app wiring, or UI) actually need to name. Re-export a type only when it is confirmed to be referenced outside the feature — not pre-emptively because it appears inside another exported type:
  - primary feature struct — if referenced by app wiring or container
  - primary event enum — if routed from `app.rs`
  - feature context type — if constructed outside the feature
  - feature error type — if handled outside the feature
  - types embedded in exported types (e.g. enum payload types) — only if external code needs to pattern-match or name them directly
  - read-only query/view models — only if consumed by UI or sibling features
- Wildcard re-exports (`pub use ...::*`) MUST NOT be used.
- Mutable state internals and temporary helper types MUST NOT be re-exported.

## 9. Dependency Graph Rules

### 9.1 Allowed Intra-Feature Dependencies

| from \ to | model | errors | state | services | storage | event | feature |
| --------- | ----: | -----: | ----: | -------: | ------: | ----: | ------: |
| model     |     - |      ✅ |     ❌ |        ✅ |       ❌ |     ❌ |      ❌ |
| state     |     ✅ |      ✅ |     - |        ✅ |       ❌ |     ❌ |      ❌ |
| services  |     ✅ |      ✅ |     ❌ |        - |       ✅ |     ❌ |      ❌ |
| storage   |     ✅ |      ✅ |     ❌ |        ❌ |       - |     ❌ |      ❌ |
| event     |     ✅ |      ✅ |     ❌ |        ❌ |       ❌ |     - |      ❌ |
| feature   |     ✅ |      ✅ |     ✅ |        ✅ |       ✅ |     ✅ |       - |

- `feature.rs` is the only orchestration layer and MAY depend on all feature modules.
- No other module within the feature MAY depend on `feature.rs`.

### 9.2 Cross-Feature Rules

- Cross-feature imports MUST go through `crate::features::<other>` re-exports only.
- Direct imports of sibling internals are forbidden, including:
  - `crate::features::<other>::feature::...`
  - `crate::features::<other>::event::...`
  - `crate::features::<other>::state::...`
  - `crate::features::<other>::model::...`
  - `crate::features::<other>::storage::...`
  - `crate::features::<other>::errors::...`
- If required behavior is missing, the owning feature MUST add explicit re-exports in `mod.rs`.

### 9.3 Forbidden Behaviors

- Cyclical dependencies between feature modules.
- Blocking I/O in reducers and hot state paths.
- Unmanaged background threads from reducers.

## 10. Services And Side Effects

- Pure deterministic helpers MAY remain free functions.
- Side-effecting operations (filesystem/process/network/env/time) SHOULD be abstracted behind feature-specific service interfaces.
- Feature context SHOULD provide service dependencies required by `reduce`.
- Service abstractions MUST remain bounded to feature needs; avoid global god-services.
- `services.rs` is the right home for a helper when it: (a) is used from more than one module inside the feature, OR (b) directly crosses an external I/O boundary (filesystem, process, network, env, time).
- Private orchestration helpers used exclusively inside `feature.rs` (e.g. `Task` builders, local command parsers called from a single `reduce` arm) MAY remain as private free functions in `feature.rs`. Moving them to `services.rs` would be premature if no other module needs them.

## 11. Naming Rules

- Directory names MUST be `snake_case`.
- File names MUST be lower `snake_case`.
- Feature struct: `<Feature>Feature`.
- Event enum: `<Feature>Event`.
- State type: `<Feature>State`.
- Context type: `<Feature>Ctx` (or explicit domain name, e.g. `ExplorerCtx`).
- Error type: `<Feature>Error`.
- Constants: `UPPER_SNAKE_CASE`.
- Boolean fields/functions: `is_`, `has_`, or `can_` prefix.
- Public helper APIs SHOULD be feature-prefixed to avoid collisions.

## 12. Testing Matrix

Each feature MUST include deterministic tests for:

- Model validation and normalization paths.
- State transitions for each event variant that mutates state.
- Reducer success path.
- Reducer ignored/invalid-event path.
- Reducer error/failure path.
- Service adapter behavior (when `services.rs` contains side effects).
- Storage round-trip (when `storage.rs` exists).
- Storage corruption/fallback handling (when `storage.rs` exists).

Test naming MUST use:
`given_<context>_when_<action>_then_<outcome>`

Tests MUST NOT require network or user-specific environment state.

## 13. Anti-Patterns

Forbidden:

- Business logic in `mod.rs`.
- Direct sibling internal imports.
- Direct mutation of another feature's state.
- Exposing mutable feature state to external modules.
- Unbounded cloning when borrowing is sufficient.
- Stringly-typed ad-hoc event channels replacing typed enums.
- Hidden side effects in constructors/getters.

## 14. Canonical Strict Template

```rust
// otty/src/features/example/mod.rs
mod errors;
mod event;
mod feature;
mod model;
mod state;

pub(crate) use event::ExampleEvent;
pub(crate) use feature::{ExampleCtx, ExampleFeature};

// otty/src/features/mod.rs (registrations)
pub(crate) mod example;

// otty/src/features/example/state.rs
#[derive(Debug, Default)]
pub(crate) struct ExampleState {
    items: Vec<u64>,
}

impl ExampleState {
    fn items(&self) -> &[u64] {
        &self.items
    }

    fn push_item(&mut self, id: u64) {
        self.items.push(id);
    }
}

// otty/src/features/example/event.rs
#[derive(Debug, Clone)]
pub(crate) enum ExampleEvent {
    AddItem { id: u64 },
}

// otty/src/features/example/feature.rs
use iced::Task;

use crate::app::Event as AppEvent;

/// Runtime dependencies for example feature.
pub(crate) struct ExampleCtx<'a> {
    pub(crate) workspace_root: &'a str,
}

/// Feature root that owns state and reduction logic.
pub(crate) struct ExampleFeature {
    state: ExampleState,
}

impl ExampleFeature {
    pub(crate) fn new() -> Self {
        Self {
            state: ExampleState::default(),
        }
    }

    pub(crate) fn items(&self) -> &[u64] {
        self.state.items()
    }
}

impl ExampleFeature {
    pub(crate) fn reduce(
        &mut self,
        event: ExampleEvent,
        _ctx: &ExampleCtx<'_>,
    ) -> Task<AppEvent> {
        match event {
            ExampleEvent::AddItem { id } => {
                self.state.push_item(id);
                Task::none()
            },
        }
    }
}
```

## 15. Compliance Checklist

A feature is compliant only if all checks pass:

- Canonical file layout is satisfied.
- `mod.rs` is thin and exports only stable API.
- Exactly one feature struct, one event enum, and one state type exist.
- Feature struct exposes `reduce` as an inherent method.
- `reduce` accepts read-only context (`&()` allowed for no-deps features).
- No forbidden dependency edges exist.
- No external mutable access bypasses feature reducer boundary.
- Required tests from section 12 exist and pass.
