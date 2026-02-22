# Post-Migration Compliance Report

**Reference:** `otty/src/features/CONVENTIONS.md`
**Date:** 2026-02-22
**Basis:** `00-compliance-audit.md` migration plan

---

## 1. Score Summary

| Feature | Before | After |
|---------|--------|-------|
| explorer | 86% (12/14) | 100% (14/14) |
| quick_launches | 86% (12/14) | 100% (14/14) |
| settings | 86% (12/14) | 100% (14/14) |
| tab | 93% (13/14) | 100% (14/14) |
| terminal | 71% (10/14) | 100% (14/14) |
| **Overall** | **84% (59/70)** | **100% (70/70)** |

---

## 2. Compliance Matrix (Post-Migration)

| Criterion | explorer | quick_launches | settings | tab | terminal |
|-----------|----------|----------------|----------|-----|----------|
| C1 file layout | ✓ | ✓ | ✓ | ✓ | ✓ |
| C2 declaration order | ✓ | ✓ | ✓ | ✓ | ✓ |
| C3 mod.rs thin | ✓ | ✓ | ✓ | ✓ | ✓ |
| C4 one state/event/reducer | ✓ | ✓ | ✓ | ✓ | ✓ |
| C5 reducer signature | ✓ | ✓ | ✓ | ✓ | ✓ |
| C6 re-exports stable API only | ✓ | ✓ | ✓ | ✓ | ✓ |
| C7 private modules | ✓ | ✓ | ✓ | ✓ | ✓ |
| C8 no wildcard re-exports | ✓ | ✓ | ✓ | ✓ | ✓ |
| C9 intra-feature deps | ✓ | ✓ | ✓ | ✓ | ✓ |
| C10 cross-feature deps | ✓ | ✓ | ✓ | ✓ | ✓ |
| C11 model.rs agnostic | ✓ | ✓ | ✓ | ✓ | ✓ |
| C12 naming | ✓ | ✓ | ✓ | ✓ | ✓ |
| C13 no anti-patterns | ✓ | ✓ | ✓ | ✓ | ✓ |
| C14 tests exist | ✓ | ✓ | ✓ | ✓ | ✓ |

---

## 3. Changes Made Per Feature

### 3.1 explorer

| Violation | Fix |
|-----------|-----|
| V-EXP-1 (C2): `mod services` before `mod state` | Reordered to `errors, event, model, state, services` |
| V-EXP-2 (C6): `ExplorerLoadTarget` leaked in re-exports | Removed from `explorer/mod.rs` re-exports |

### 3.2 quick_launches

| Violation | Fix |
|-----------|-----|
| V-QL-1 (C6): `#[rustfmt::skip]` on all module declarations | Removed all occurrences; corrected declaration order to `errors, event, model, state, storage, services, editor` |
| V-QL-2 (C9): `state.rs` imported from `storage.rs` | Removed `QuickLaunchState::load()`; added `bootstrap_quick_launches()` in `event.rs`; state.rs now only imports model |

### 3.3 settings

| Violation | Fix |
|-----------|-----|
| V-SET-1 (C6): `palette_label` and `is_valid_hex_color` sourced from `model` in `mod.rs` | `palette_label` definition moved to `services.rs`; `is_valid_hex_color` re-exported through `services.rs`; `mod.rs` now sources both via `use services::` |
| V-SET-2 (C9): `state.rs` imported from `storage.rs` | Removed `SettingsState::load()`; added `bootstrap_settings()` in `event.rs`; state.rs is now storage-free |

### 3.4 tab

| Violation | Fix |
|-----------|-----|
| V-TAB-1 (C6): re-exports beyond four primary categories | No code changes — all extra re-exports have verified external consumers. Convention updated to recognize deps structs and extended stable API as valid categories |

### 3.5 terminal

| Violation | Fix |
|-----------|-----|
| V-TERM-1 (C2): `mod services` before `mod state` | Reordered to `errors, event, model, state, services` |
| V-TERM-2 (C6): `shell_cwd_for_active_tab` in `event.rs`, re-exported from `event` | Function moved to `services.rs`; `mod.rs` re-exports from `services` |
| V-TERM-3 (C11): `TerminalEntry` carried `pane: pane_grid::Pane` (Iced UI type) | Removed `pane` field from `TerminalEntry`; added `pane_for_terminal()` reverse lookup on `TerminalState` |
| V-TERM-4 (C9): `state.rs` imported `Task<AppEvent>`, `AppEvent`, `TabEvent` | Introduced `TerminalCommand` enum in `state.rs`; all state mutators now return `TerminalCommand`; `execute_command()` in `event.rs` maps it to `Task<AppEvent>` |

---

## 4. Convention Updates (CONVENTIONS.md)

Three clarifications were added:

**Section 6 — Deps structs as allowed re-export category:**
Added `FeatureDeps`-style reducer dependency structs as a 5th allowed re-export category, consistent with
the reducer signature already shown in Section 5.2.

**Section 6 — Extended stable API language:**
Added explicit language that re-exports with verified external consumers in `app.rs`, `ui/widgets/`, or sibling
features qualify as compliant extended stable API. `ui/widgets/` is a first-class consumer.

**Section 7.1 — C9 scope clarification:**
Added note that C9 governs **intra-feature module dependencies** only. External crate imports (`iced`, `std`,
`otty_ui_term`) are allowed in all modules. The restriction is: `state.rs` must not import from `storage.rs`,
`services.rs`, or sibling features within the same feature graph.

---

## 5. Acceptance Criteria Verification

All 22 criteria from Section 8 of the audit document are satisfied:

| ID | Result |
|----|--------|
| A-EXP-1 | ✓ `mod state` before `mod services` in explorer/mod.rs |
| A-EXP-2 | ✓ `ExplorerLoadTarget` absent from explorer/mod.rs |
| A-QL-1 | ✓ No `#[rustfmt::skip]` in quick_launches/mod.rs |
| A-QL-2 | ✓ No `storage` import in quick_launches/state.rs |
| A-QL-3 | ✓ `bootstrap_quick_launches()` present in quick_launches/event.rs |
| A-SET-1 | ✓ `fn palette_label` defined in settings/services.rs |
| A-SET-2 | ✓ `fn palette_label` absent from settings/model.rs |
| A-SET-3 | ✓ `fn is_valid_hex_color` remains in settings/model.rs |
| A-SET-4 | ✓ settings/mod.rs uses `use services::{is_valid_hex_color, palette_label}` |
| A-SET-5 | ✓ settings/mod.rs does not source those functions from model |
| A-SET-6 | ✓ No `storage` import in settings/state.rs |
| A-SET-7 | ✓ `bootstrap_settings()` present in settings/event.rs |
| A-TERM-1 | ✓ `mod state` before `mod services` in terminal/mod.rs |
| A-TERM-2 | ✓ `shell_cwd_for_active_tab` present in terminal/services.rs |
| A-TERM-3 | ✓ `shell_cwd_for_active_tab` absent from terminal/event.rs |
| A-TERM-4 | ✓ terminal/model.rs has no `use iced` import |
| A-TERM-5 | ✓ `TerminalEntry` has no `pane` field |
| A-TERM-6 | ✓ terminal/state.rs has no `Task`, `AppEvent`, or `TabEvent` imports |
| A-TERM-7 | ✓ `TerminalCommand` enum defined in terminal/state.rs |
| A-CONV-1 | ✓ CONVENTIONS.md Section 7.1 clarifies C9 scope |
| A-CONV-2 | ✓ CONVENTIONS.md Section 6 includes deps structs |
| A-CONV-3 | ✓ CONVENTIONS.md Section 6 includes extended stable API language |

---

## 6. Build Health

```
cargo build -p otty        → Finished (0 errors, 0 warnings)
cargo clippy --workspace   → Finished (0 warnings)
cargo test --workspace     → 359 tests: 0 failed
```
