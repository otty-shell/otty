# UI Components Conventions

## 1. Scope

This document is a normative specification for `otty/src/ui/components`.

- A component module MUST encapsulate one reusable UI primitive.
- New component modules MUST be created in `strict` profile (see section 2).
- Existing modules MAY temporarily remain in `legacy` profile while being migrated.
- The goal is deterministic, human-readable, presentation-only primitives with predictable runtime cost.

## 2. Compliance Profiles

### 2.1 Strict Profile (Default)

Use for all new components and fully migrated components.

- MUST satisfy all sections in this document.
- MUST use canonical layout from section 3.
- MUST stay side-effect free in render paths.

### 2.2 Legacy Profile (Temporary)

Use only for pre-existing components during migration.

- MAY keep historical structure temporarily.
- MUST NOT add new violations.
- MUST include a migration task to reach `strict` profile.

## 3. Canonical Component Layout

Each strict component MAY use one of two canonical layouts.

Simple component layout (single-file), for small reusable primitives with limited branching:

```text
<component_name>.rs     # required: props + event + view + local pure helpers
```

Complex component layout (module folder), for larger primitives with dense style/helper logic:

```text
<component_name>/
  mod.rs                # required: public surface only
  view.rs               # required: primary render implementation
  event.rs              # optional: local event contract
  props.rs              # optional: extracted props
  style.rs              # optional: style factories only
  helpers.rs            # optional: local pure helpers only
  tests.rs              # optional: tests when not inline
```

- A component MUST start as single-file unless complexity already justifies module-folder at creation time.
- A component SHOULD be promoted to module-folder when at least one condition holds:
  - substantial style logic that reduces readability of single-file layout
  - multiple helper blocks (formatting, style derivation, layout calculations) requiring separation
  - growing local event/props contracts where extraction improves navigation
- Files outside canonical/optional lists MUST NOT be added without updating this specification.

Simple component (single-file) expected contents:

- `<Component>Props`
- `<Component>Event`
- `view`
- local pure helpers (if needed)

Complex component (module-folder) expected files:

- `view.rs` (required)
- `event.rs` and/or `props.rs` when extraction improves clarity
- `style.rs` for reusable style factories
- `helpers.rs` for pure deterministic utility logic

Current tree analysis (informative): the current `ui/components` tree fits the simple/single-file layout:

- `icon_button.rs`
- `menu_item.rs`

## 4. Module Responsibilities

- Single-file component: keep props, event contract, view, and local pure helpers together.
- `mod.rs` (folder-based): module declarations, curated re-exports, zero rendering logic.
- `view.rs`: layout and interaction-to-message mapping only.
- `event.rs`: local component event enum only.
- `props.rs`: borrowed render inputs only.
- `style.rs`: style factories/closures only.
- `helpers.rs`: deterministic pure helpers only.

## 5. Primary Contracts

### 5.1 Required Primary Types

Each component MUST expose:

- Exactly one primary props type named `<Component>Props<'a>`.
- Exactly one local event enum named `<Component>Event`.
- Exactly one primary render entrypoint named `view`.

### 5.2 Render Entrypoint Rules

- The render entrypoint MUST be `pub(crate)`.
- Side effects MUST NOT be hidden in `view`.
- Component render code MUST NOT allocate async tasks or perform runtime I/O.

Recommended signature:

```rust
pub(crate) fn view<'a>(
    props: ComponentProps<'a>,
) -> Element<'a, ComponentEvent>
```

## 6. API Exposure

- `ui/components/mod.rs` MUST register components as `pub(crate) mod <component>;`.
- A component folder's `mod.rs` MUST be the only import surface for its internal files.
- Folder-internal declarations MUST use private `mod ...;`.
- `mod.rs` MUST re-export only stable API items:
  - primary props type
  - primary event enum (if exists)
  - primary render entrypoint
- Wildcard re-exports (`pub use ...::*`) MUST NOT be used.

## 7. Dependency Graph Rules

### 7.1 Allowed Intra-Component Dependencies

- `view.rs` MAY depend on `event.rs`, `props.rs`, `style.rs`, `helpers.rs`, and shared crate UI utilities.
- `style.rs` MAY depend on theme and pure helper functions.
- `helpers.rs` MUST remain pure and deterministic.

### 7.2 Cross-Layer Rules

- Components MAY depend on `iced`, `crate::theme`, `crate::icons`, `crate::fonts`, and other components via public APIs.
- Components MUST NOT depend on:
  - `crate::features::*`
  - `crate::state::*`
  - `crate::app::*`
  - `crate::ui::widgets::*`

### 7.3 Forbidden Behaviors

- Filesystem/network/process I/O in component modules.
- Async task scheduling from components (`Task::perform`, runtime spawning).
- Hidden side effects in constructors/render helpers.

## 8. State Ownership And Mutation

- Components MUST treat all incoming data as read-only render inputs.
- Components MUST NOT mutate app or feature state.
- Mutation logic MUST remain in reducers/state layers, not in component modules.

## 9. Naming Rules

- Directory names MUST be `snake_case`.
- File names MUST be lower `snake_case`.
- Props type: `<Component>Props`.
- Event enum: `<Component>Event` (when present).
- Render function: `view`.
- Constants: `UPPER_SNAKE_CASE`.
- Boolean fields/functions: `is_`, `has_`, or `can_` prefix.

## 10. Testing Matrix (Strict Profile)

Each strict component SHOULD include deterministic tests only for:

- Pure helper calculations and formatting.
- Pure style/state derivation helpers (when exposed as pure functions).

Component `view` functions MUST NOT be unit-tested.

Test naming MUST use:
`given_<context>_when_<action>_then_<outcome>`

Tests MUST NOT require network or user-specific environment state.

## 11. Anti-Patterns

Forbidden:

- Business logic in component modules.
- Direct coupling to feature internals or app orchestrator logic.
- `unwrap()` in production component code.
- Hidden side effects in view/builders/helpers.
- Wildcard re-exports.

## 12. Canonical Strict Template

```rust
// otty/src/ui/components/example_button.rs
use iced::Element;

/// Props for rendering example button.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExampleButtonProps<'a> {
    pub(crate) label: &'a str,
}

/// Events emitted by example button.
#[derive(Debug, Clone)]
pub(crate) enum ExampleButtonEvent {
    Pressed,
}

/// Render example button.
pub(crate) fn view<'a>(
    props: ExampleButtonProps<'a>,
) -> Element<'a, ExampleButtonEvent> {
    let _ = props;
    iced::widget::text("example").into()
}
```

## 13. Compliance Checklist

A component is compliant only if all checks pass:

- Canonical file layout is satisfied.
- `mod.rs` is thin and exports only stable API (folder-based components).
- Exactly one primary render entrypoint exists.
- Props/event/view naming matches required patterns.
- No forbidden dependency edges exist.
- No side effects or runtime I/O in render paths.
- Tests (if present) cover helper/computation logic only.
