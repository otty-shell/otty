# Features Conventions

## 1. Scope

This document is a normative specification for `otty/src/features`.

- A feature module MUST encapsulate one domain slice: event contract, state contract, and reduction logic.
- The goal is deterministic architecture and near-identical module topology across features.

## 2. Compliance Profiles

- MUST satisfy all sections in this document.
- MUST have canonical file topology from section 3.
- MUST expose exactly one external reducer entrypoint and events subset from `mod.rs`.
- Feature reducers MUST NOT call other feature reducers directly.
- Cross-feature influence MUST be expressed only by returning `Task<AppEvent>` that contains routed events.
- Feature state MUST be stored inside the global App `State`.
- Feature state MUST be mutated only by the feature reducer defined in `event.rs`.
- External code MAY access feature state only through public getter methods.
- `pub` fields on `<Feature>State` are forbidden.

## 3. Canonical Layout

Each feature under `otty/src/features/<feature_name>/` MUST follow:

```text
├── mod.rs         # required: public surface only
├── event.rs       # required: <Feature>Event + <feature>_reducer
├── state.rs       # required: <Feature>State + state helpers
├── model.rs       # required: domain data
├── errors.rs      # optional: feature errors
├── storage.rs     # optional: persistence boundary only
├── services.rs    # optional: non-UI integrations, adapters, process glue
└── subfeature/    # optional: subfeature must mirror the same contract
  ├── mod.rs
  ├── event.rs
  ├── state.rs
  ├── model.rs
  ├── errors.rs
  └── storage.rs
```

Files outside required/optional lists MUST NOT be added without updating this specification.

## 4. Module Responsibilities

- `mod.rs`: module declarations, curated re-exports. Business logic MUST NOT be added here.
- `event.rs`: public feature event enum, reducer entrypoint, event routing.
- `state.rs`: state struct, deterministic state mutation helpers.
- `model.rs`: domain data structs declarations, domain data structs implementations and trait's implementations for domain data structs (e.g Display/Debug or etc).
- `errors.rs`: explicit error type for feature boundaries.
- `storage.rs`: serialization/deserialization and filesystem I/O only.
- `services.rs`: bounded external interactions that are not pure `event/model/state/storage` logic.

## 5. Primary Contracts

### 5.1 Required Primary Types

Each feature MUST expose:

- Exactly one primary state type named `<Feature>State`.
- Exactly one primary event enum named `<Feature>Event`.
- Exactly one external reducer function named `<feature>_reducer`.

`Exactly one` means one external entrypoint visible outside the feature.
Internal helper reducers MAY exist but MUST be private.

### 5.2 Reducer Entrypoint Rules

- The reducer entrypoint MUST be `pub(crate)` and re-exported from `mod.rs`.
- The reducer MUST be the only external write-entry point for feature state.
- Side effects MUST be returned as `Task<AppEvent>` and never hidden.

Signatures:

```rust
pub(crate) fn feature_reducer(
    state: &mut State,
    event: FeatureEvent,
) -> Task<AppEvent>
```

or when explicit runtime dependencies are needed:

```rust
pub(crate) struct FeatureDeps<'a> {
    pub(crate) terminal_settings: &'a Settings,
}

pub(crate) fn feature_reducer(
    state: &mut State,
    deps: FeatureDeps<'_>,
    event: FeatureEvent,
) -> Task<AppEvent>
```

### 5.3 State Rules

- The feature's State MUST be `pub(crate)` and re-exported from `mod.rs`.
- The feature's State fields MUST be private (without `pub`) and have getter methods.
- The feature's State MUST NOT be mutated outside the feature.

## 6. API Exposure

- `features/mod.rs` MUST register features as `pub(crate) mod <feature>;`.
- A feature's `mod.rs` MUST be the only import surface for sibling features.
- Feature-internal module declarations inside `<feature>/mod.rs` MUST use private `mod ...;`.
- Feature-internal `mod.rs` MUST re-export only stable API items:
  - primary event enum
  - primary reducer entrypoint
  - primary state type
  - feature error type (if exists)
  - dependency struct for the reducer entrypoint (e.g., `FeatureDeps`), when required
  - domain models from `model.rs`
  - service structs and functions from `services.rs` (if exists and required)
- Wildcard re-exports (`pub use ...::*`) MUST NOT be used.
- Storage internals and temporary helper types MUST NOT be re-exported.

## 7. Dependency Graph Rules

### 7.1 Allowed Intra-Feature Dependencies

| from \ to | model | errors | state | services | storage | event |
| --------- | ----: | -----: | ----: | -------: | ------: | ----: |
| model     |     - |      ✅ |     ❌ |        ❌ |       ❌ |     ❌ |
| state     |     ✅ |      ✅ |     - |        ❌ |       ❌ |     ❌ |
| services  |     ✅ |      ✅ |     ❌ |        - |       ❌ |     ❌ |
| storage   |     ✅ |      ✅ |     ❌ |        ❌ |       - |     ❌ |
| event     |     ✅ |      ✅ |     ✅ |        ✅ |       ✅ |     - |

- `event.rs` is the orchestration layer and MAY depend on `state/model/errors/services/storage`.
- No other module within the feature MAY depend on `event.rs`.

### 7.2 Cross-Feature Rules

- Cross-feature imports MUST go through `crate::features::<other>` re-exports only.
- Direct imports of sibling internals are forbidden, including:
  - `crate::features::<other>::event::...`
  - `crate::features::<other>::state::...`
  - `crate::features::<other>::model::...`
  - `crate::features::<other>::storage::...`
  - `crate::features::<other>::errors::...`
- If required behavior is missing, the owning feature MUST add explicit re-exports in its `mod.rs`.

### 7.3 Forbidden Behaviors

- Cyclical dependencies between feature modules.
- Blocking I/O in reducers and hot state paths.
- Unmanaged background threads from reducers.

## 8. State Ownership And Mutation

- Each domain datum MUST have one canonical owner feature.
- Duplicate ownership of mutable domain data across features is forbidden.
- Reusable mutation logic SHOULD live on `<Feature>State` methods.
- Feature internals MUST NOT be mutated directly from `app.rs` or sibling features.
- State initialization MUST define explicit defaults and avoid hidden globals.

## 9. Naming Rules

- Directory names MUST be `snake_case`.
- File names MUST be lower `snake_case`.
- Event enum: `<Feature>Event`.
- Reducer function: `<feature>_reducer`.
- State type: `<Feature>State`.
- Error type: `<Feature>Error`.
- Constants: `UPPER_SNAKE_CASE`.
- Boolean fields/functions: `is_`, `has_`, or `can_` prefix.
- Public helper APIs SHOULD be feature-prefixed to avoid collisions.

## 10. Testing Matrix

Each feature MUST include deterministic tests for:

- Model validation and normalization paths.
- State transitions for each event variant that mutates state.
- Reducer success path.
- Reducer ignored/invalid-event path.
- Reducer error/failure path.
- Storage round-trip (when `storage.rs` exists).
- Storage corruption/fallback handling (when `storage.rs` exists).

Test naming MUST use:
`given_<context>_when_<action>_then_<outcome>`

Tests MUST NOT require network or user-specific environment state.

## 11. Anti-Patterns

Forbidden:

- Business logic in `mod.rs`.
- Direct sibling internal imports.
- Direct mutation of another feature's state.
- Unbounded cloning when borrowing is sufficient.
- Stringly-typed ad-hoc event channels replacing typed enums.
- Hidden side effects in constructors/getters.

## 12. Canonical Strict Template

```rust
// otty/src/features/example/mod.rs
mod errors;
mod event;
mod model;
mod state;

pub(crate) use errors::ExampleError;
pub(crate) use event::{ExampleEvent, example_reducer};
pub(crate) use state::ExampleState;

// otty/src/features/example/errors.rs
use thiserror::Error;

/// Errors emitted by the example feature.
#[derive(Debug, Error)]
pub(crate) enum ExampleError {
    #[error("validation failed: {message}")]
    Validation { message: String },
}

// otty/src/features/example/model.rs
/// Domain entity for the example feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExampleItem {
    id: u64,
}

impl ExampleItem {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }
}

// otty/src/features/example/state.rs
use super::model::ExampleItem;

/// Runtime state for the example feature.
#[derive(Debug, Default)]
pub(crate) struct ExampleState {
    items: Vec<ExampleItem>,
}

impl ExampleState {
    pub(crate) fn items(&self) -> &[ExampleItem] {
        &self.items
    }

    pub(crate) fn push_item(&mut self, id: u64) {
        self.items.push(ExampleItem { id });
    }
}

// otty/src/features/example/event.rs
use iced::Task;

use crate::app::Event as AppEvent;
use crate::state::State;

/// Events emitted by the example feature UI.
#[derive(Debug, Clone)]
pub(crate) enum ExampleEvent {
    AddItem { id: u64 },
}

/// Reduce example events into state updates and side effects.
pub(crate) fn example_reducer(
    state: &mut State,
    event: ExampleEvent,
) -> Task<AppEvent> {
    match event {
        ExampleEvent::AddItem { id } => {
            state.example.push_item(id);
            Task::none()
        },
    }
}
```

## 13. Compliance Checklist

A feature is compliant only if all checks pass:

- Canonical file layout is satisfied.
- `mod.rs` is thin and exports only stable API.
- Exactly one external reducer entrypoint exists.
- Event/state/reducer/error naming matches required patterns.
- No forbidden dependency edges exist.
- No direct external state mutation bypasses reducer boundary.
- Required tests from section 10 exist and pass.
