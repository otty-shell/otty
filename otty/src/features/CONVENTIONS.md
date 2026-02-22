# Features Package Conventions

## 1. Scope And Purpose

This document is a normative specification for `otty/src/features`.

- A feature module MUST encapsulate one domain slice: event contract, state contract, and reduction logic.
- New feature modules MUST be created in `strict` profile (see section 2).
- Existing modules MAY temporarily remain in `legacy` profile while being migrated.
- The goal is deterministic architecture and near-identical module topology across features.

### Rationale

- Uniform feature shape reduces onboarding and review ambiguity.
- Deterministic boundaries reduce accidental coupling and generated-code drift.

## 2. Compliance Profiles

### 2.1 Strict Profile (Default)

Use for all new features and all fully migrated features.

- MUST satisfy all sections in this document.
- MUST have canonical file topology from section 3.
- MUST expose exactly one external reducer entrypoint from `mod.rs`.
- MUST interact with other features only through that feature's `mod.rs` re-exports.

### 2.2 Legacy Profile (Temporary)

Use only for pre-existing features during migration.

- MAY keep historical structure temporarily.
- MUST NOT add new violations.
- MUST include a migration task to reach `strict` profile.

### Rationale

- Profile split allows incremental migration without blocking delivery.

## 3. Canonical Feature Layout

Each feature under `otty/src/features/<feature_name>/` MUST follow:

```text
mod.rs                # required: public surface only
event.rs              # required: <Feature>Event + <feature>_reducer
state.rs              # required: <Feature>State + state helpers
model.rs              # required: domain data, validation, pure transforms
errors.rs             # required when explicit feature error exists
storage.rs            # optional: persistence boundary only
```

Optional extensions in strict profile:

```text
services.rs           # optional: non-UI integrations, adapters, process glue
editor/               # optional subfeature; must mirror the same contract
  mod.rs
  event.rs
  state.rs            # required if subfeature has state
  model.rs
  errors.rs           # required when explicit subfeature error exists
  storage.rs          # optional
```

- `mod.rs` MUST declare modules in this stable order if present:
  `errors`, `event`, `model`, `state`, `storage`, `services`, then subfeatures.
- `errors.rs` (plural) MUST be used for error definitions. `error.rs` is forbidden in strict profile.
- Files outside canonical/optional lists MUST NOT be added without updating this specification.

### Rationale

- Stable topology enables mechanical generation and deterministic navigation.

## 4. Module Responsibilities

- `mod.rs`: module declarations, curated re-exports, and zero business logic.
- `event.rs`: public feature event enum, reducer entrypoint, event routing.
- `state.rs`: state struct, deterministic state mutation helpers.
- `model.rs`: domain types, validation, pure mapping/normalization.
- `errors.rs`: explicit error types for feature boundaries.
- `storage.rs`: serialization/deserialization and filesystem I/O only.
- `services.rs`: bounded external interactions that are not pure model/state logic.

### Rationale

- Responsibility isolation improves testability and code search precision.

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

Recommended signatures:

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

### Rationale

- Single, explicit write boundary preserves invariants and auditability.

## 6. API Exposure And Visibility

- `features/mod.rs` MUST register features as `pub(crate) mod <feature>;`.
- A feature's `mod.rs` MUST be the only import surface for sibling features.
- Feature-internal module declarations inside `<feature>/mod.rs` MUST use private `mod ...;`.
- `mod.rs` MUST re-export only stable API items:
  - primary event enum
  - primary reducer entrypoint
  - primary state type
  - feature error type (if exists)
  - dependency struct for the reducer entrypoint (e.g., `FeatureDeps`), when required
  - extended stable API: any types with verified external consumers in `app.rs`, `ui/widgets/`, or sibling
    features. `ui/widgets/` is a first-class consumer; features MAY re-export types it needs directly without
    requiring view-model indirection.
- Wildcard re-exports (`pub use ...::*`) MUST NOT be used.
- Storage internals and temporary helper types MUST NOT be re-exported.

### Rationale

- Minimal surfaces keep refactors safe and prevent API leaks.

## 7. Dependency Graph Rules

### 7.1 Allowed Intra-Feature Dependencies

- `event.rs` MAY depend on `state.rs`, `model.rs`, `errors.rs`, `services.rs`, and shared crate utilities.
- `state.rs` MAY depend on `model.rs` and `errors.rs`.
- `state.rs` MAY import from external crates (`iced`, `std`, `otty_ui_term`, etc.). The restriction is on
  **intra-feature module dependencies** only: within the feature's own module graph, `state.rs` MUST NOT
  import from `storage.rs`, `services.rs`, or sibling features.
- `model.rs` MUST remain UI-runtime agnostic.
- `errors.rs` MAY depend only on std/core types, external error helpers (for example `thiserror`), and feature `model.rs` when needed for typed error payloads.
- `storage.rs` MAY depend on `model.rs` and `errors.rs`.

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

### Rationale

- Explicit dependency edges prevent boundary erosion.

## 8. State Ownership And Mutation

- Each domain datum MUST have one canonical owner feature.
- Duplicate ownership of mutable domain data across features is forbidden.
- Reusable mutation logic SHOULD live on `<Feature>State` methods.
- Feature internals MUST NOT be mutated directly from `app.rs` or sibling features.
- State initialization MUST define explicit defaults and avoid hidden globals.

### Rationale

- Canonical ownership prevents split-brain state and non-local bugs.

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

### Rationale

- Predictable names reduce search friction and API drift.

## 10. Testing Matrix (Strict Profile)

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

### Rationale

- Boundary-focused tests catch most regressions with minimal noise.

## 11. Anti-Patterns

Forbidden:

- Business logic in `mod.rs`.
- Direct sibling internal imports.
- Direct mutation of another feature's state.
- `unwrap()` in production feature code.
- Unbounded cloning when borrowing is sufficient.
- Stringly-typed ad-hoc event channels replacing typed enums.
- Hidden side effects in constructors/getters.

### Rationale

- These patterns create non-local behavior and fragile contracts.

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
    pub(crate) id: u64,
}

// otty/src/features/example/state.rs
use super::model::ExampleItem;

/// Runtime state for the example feature.
#[derive(Debug, Default)]
pub(crate) struct ExampleState {
    pub(crate) items: Vec<ExampleItem>,
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
            state.example.items.push(super::model::ExampleItem { id });
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
