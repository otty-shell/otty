# UI Widgets Conventions

## 1. Scope

This document is a normative specification for `otty/src/ui/widgets`.

- A widget module MUST encapsulate one domain-facing UI surface.
- New widget modules MUST be created in `strict` profile (see section 2).
- Existing modules MAY temporarily remain in `legacy` profile while being migrated.
- The goal is deterministic, presentation-only composition with predictable boundaries and reusable behavior across the app UI.

This file applies to flat widgets and nested widget folders under `otty/src/ui/widgets/**`.

## 2. Compliance Profiles

### 2.1 Strict Profile (Default)

Use for all new widgets and fully migrated widgets.

- MUST satisfy all sections in this document.
- MUST use canonical layout from section 3.
- MUST keep render paths side-effect free.

### 2.2 Legacy Profile (Temporary)

Use only for pre-existing widgets during migration.

- MAY keep historical structure temporarily.
- MUST NOT add new violations.
- MUST include a migration task to reach `strict` profile.

## 3. Canonical Widget Layout

Each strict widget MAY use one of two canonical layouts.

Simple widget layout (single-file), for small widgets with one render surface and limited branching:

```text
<widget_name>.rs        # required: <Widget>Props + <Widget>Event + view + local pure helpers
```

Complex widget layout (module folder), for large widgets with multiple visual regions, overlays, or dense helper logic:

```text
<widget_name>/
  mod.rs                # required: public surface only
  view.rs               # required: primary render entrypoint
```

Optional files:

```text
event.rs               # optional: local widget event contract
props.rs               # optional: extracted props
style.rs               # optional: style factories only
layout.rs              # optional: pure geometry/layout helpers
overlay.rs             # optional: overlays/context menus only
helpers.rs             # optional: pure local helpers only
tests.rs               # optional: tests when not inline
```

- A widget MUST start as single-file unless complexity already justifies module-folder at creation time.
- A widget SHOULD be promoted to module-folder when at least one condition holds:
  - multiple independent render regions (for example: header + list + form + footer)
  - overlay/context-menu rendering with separate lifecycle logic
  - large amount of pure helper/style/layout code reducing readability
  - growing event mapping branches where separation improves navigation
- Files outside canonical/optional lists MUST NOT be added without updating this specification.

Simple widget (single-file) expected contents:

- `<Widget>Props`
- `<Widget>Event`
- `view`
- local pure helpers (if needed)

Complex widget (module-folder) expected files:

- `view.rs` (required)
- `event.rs` and/or `props.rs` when extraction improves clarity
- `style.rs` for reusable style factories
- `layout.rs` for geometry/placement calculations
- `overlay.rs` for context menu/popup layers
- `helpers.rs` for pure deterministic utility logic

Current tree analysis (informative): the current `ui/widgets` tree shows both complexity classes.

Simple/single-file fit (examples):

- `tab_content.rs`
- `quick_launches/error.rs`
- `sidebar_workspace/terminal.rs`

Complex/module-folder fit (examples):

- `settings.rs`
- `quick_launches/sidebar.rs`
- `quick_launches/editor.rs`
- `sidebar.rs`
- `tab_bar.rs`

Recommended migration for complex examples:

- keep `view.rs` as primary composition
- extract `props.rs` when props grow
- extract `style.rs` when style closures dominate view code
- extract `layout.rs` for geometry/clamping constants and helpers
- extract `overlay.rs` for context menus/popups
- keep only pure computation in `helpers.rs`

## 4. Module Responsibilities

- `mod.rs`: module declarations, curated re-exports, and zero business logic.
- `view.rs`: primary render composition and event mapping.
- `event.rs`: local widget event enum `<Widget>Event`.
- `props.rs`: borrowed render inputs and UI configuration.
- `style.rs`: style closures/factories only.
- `layout.rs`: deterministic geometry/layout calculations only.
- `overlay.rs`: layered overlays/context menus and dismiss behavior only.
- `helpers.rs`: pure reusable local helpers only.

## 5. Primary Contracts

### 5.1 Required Primary Types

Each widget MUST expose:

- Exactly one primary props type named `<Widget>Props<'a>`.
- Exactly one primary output message contract named `<Widget>Event`.
- Exactly one primary render entrypoint named `view`.

### 5.2 Render Entrypoint Rules

- The render entrypoint MUST be `pub(crate)` and re-exported from `mod.rs` (folder-based widgets).
- Widget render code MUST remain side-effect free.
- Widgets MUST NOT schedule async tasks or perform runtime I/O.

Recommended signature:

```rust
pub(crate) fn view<'a>(
    props: ExampleWidgetProps<'a>,
) -> Element<'a, ExampleWidgetEvent>
```

## 6. API Exposure

- `ui/widgets/mod.rs` MUST register widgets as `pub(crate) mod <widget>;`.
- A widget folder's `mod.rs` MUST be the only import surface for its internal files.
- Widget-internal module declarations MUST use private `mod ...;`.
- `mod.rs` MUST re-export only stable API items:
  - primary props type
  - primary event contract
  - primary render entrypoint
- Wildcard re-exports (`pub use ...::*`) MUST NOT be used.

## 7. Dependency Graph Rules

### 7.1 Allowed Intra-Widget Dependencies

- `view.rs` MAY depend on `event.rs`, `props.rs`, `style.rs`, `layout.rs`, `overlay.rs`, `helpers.rs`, components, and shared UI utilities.
- `layout.rs`/`helpers.rs` MUST remain pure.
- `overlay.rs` MUST remain presentation-only.

### 7.2 Cross-Layer Rules

- Widgets MAY depend on:
  - `crate::ui::components` public APIs
  - `crate::theme`, `crate::icons`, `crate::fonts`
  - `crate::features::<feature>` re-exports only
- Widgets MUST NOT import feature internals directly, including:
  - `crate::features::<feature>::event::...`
  - `crate::features::<feature>::state::...`
  - `crate::features::<feature>::model::...`
  - `crate::features::<feature>::storage::...`
  - `crate::features::<feature>::errors::...`

### 7.3 Forbidden Behaviors

- Runtime I/O in widget modules.
- Async task scheduling from widgets.
- Reducer dispatch or orchestration logic inside widget modules.
- Direct coupling of leaf widgets to `crate::app::Event`.

## 8. State Ownership And Mutation

- Widgets MUST treat input state as read-only render data.
- Widgets MUST NOT mutate feature/app state directly.
- Domain decisions and mutations MUST remain in reducers/state layers.
- Widgets SHOULD receive precomputed/normalized data where possible.

## 9. Naming Rules

- Directory names MUST be `snake_case`.
- File names MUST be lower `snake_case`.
- Primary props type: `<Widget>Props`.
- Primary local event enum: `<Widget>Event`.
- Render function: `view`.
- Constants: `UPPER_SNAKE_CASE`.
- Boolean fields/functions: `is_`, `has_`, or `can_` prefix.

## 10. Testing Matrix (Strict Profile)

Each strict widget SHOULD include deterministic tests only for:

- Pure helper/layout calculations.
- Overlay positioning and clamping helpers.
- Other deterministic pure computation helpers.

Widget `view` functions SHOULD NOT be unit-tested.

Test naming MUST use:
`given_<context>_when_<action>_then_<outcome>`

Tests MUST NOT require network or user-specific environment state.

## 11. Anti-Patterns

Forbidden:

- Business logic in `mod.rs` or render modules.
- Direct feature internal imports.
- Direct state mutation from widgets.
- `unwrap()` in production widget code.
- Hidden side effects in render helpers.
- Wildcard re-exports.

## 12. Canonical Strict Template

```rust
// otty/src/ui/widgets/example_simple.rs
use iced::Element;

/// Props for rendering example widget.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExampleWidgetProps<'a> {
    pub(crate) title: &'a str,
}

/// Events emitted by example widget.
#[derive(Debug, Clone)]
pub(crate) enum ExampleWidgetEvent {
    Clicked,
}

/// Render example widget.
pub(crate) fn view<'a>(
    props: ExampleWidgetProps<'a>,
) -> Element<'a, ExampleWidgetEvent> {
    let _ = props;
    iced::widget::text("example").into()
}

// otty/src/ui/widgets/example_complex/mod.rs
mod view;

pub(crate) use view::{ExampleWidgetEvent, ExampleWidgetProps, view};

// otty/src/ui/widgets/example_complex/view.rs
use iced::Element;

/// Props for rendering example widget.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExampleWidgetProps<'a> {
    pub(crate) title: &'a str,
}

/// Events emitted by example widget.
#[derive(Debug, Clone)]
pub(crate) enum ExampleWidgetEvent {
    Clicked,
}

/// Render example widget.
pub(crate) fn view<'a>(
    props: ExampleWidgetProps<'a>,
) -> Element<'a, ExampleWidgetEvent> {
    let _ = props;
    iced::widget::text("example").into()
}
```

## 13. Compliance Checklist

A widget is compliant only if all checks pass:

- Canonical file layout is satisfied (single-file or module-folder).
- `mod.rs` is thin and exports only stable API (folder-based widgets).
- Exactly one primary render entrypoint exists.
- Props/event/view naming matches required patterns.
- No forbidden dependency edges exist.
- No side effects or runtime I/O in render paths.
- Tests (if present) cover helper/computation logic only.
