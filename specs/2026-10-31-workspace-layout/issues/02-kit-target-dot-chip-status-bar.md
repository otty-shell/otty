# 02 — Kit: TargetDot, Chip, StatusBar

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 01
Spec: [../spec.md](../spec.md)

## What

New components in `crates/ui/kit/src/components/`, one file each, styled only
with `UiPalette` and kit tokens. Visual reference: `ot-dot`, `ot-chip`,
`ot-statusbar` in `V4Workspace` and the design system cards `TargetChip` and
`StatusBar`.

| Component | Shape |
|---|---|
| `TargetDot` | Small round dot; kind `Local`, `Ssh`, `Prod`, `Staging` → `target-*` token; never shown without a word next to it (design system rule) |
| `Chip` | `surface-raised`, `radius-sm`, `small` text; optional leading `TargetDot`, optional muted note ("this computer"); mono variant for paths |
| `StatusBar` | `statusbar` (24) tall, `surface-chrome`, `caption` text; items on the left, a gap, items on the right; an item can be plain text, mono text, a target (dot + name) or a button |

If a size has no token yet (dot diameter), add it to `size.rs` with a doc comment instead of a local constant.

- Gallery: one section per component, both themes.
- `iced_test`: a status bar button item emits its message.

## Done when

- Components render in the gallery in OTTY Dark and OTTY Light; target dots reach 3:1 on `surface-chrome`, `surface-panel` and `surface-raised` (contrast tests reuse `contrast.rs`).

## Answer

Added three kit components, one file each in `crates/ui/kit/src/components/`, plus one new size token and gallery sections (one per component; the gallery switches between OTTY Dark and OTTY Light).

- `target_dot.rs`: `TargetKind { Local, Ssh, Prod, Staging }` with `color(&palette)`, and `target_dot(kind, &palette) -> Element`. Round dot in the `target-*` token.
- `chip.rs`: `Chip::new(label, &palette).target(TargetKind).note(&str).mono(bool)`, converts into `Element`. `surface-raised`, `radius-sm`, `small` text.
- `status_bar.rs`: `StatusBar::new(&palette).left(StatusItem).right(StatusItem)`, converts into `Element`; `StatusItem::{text, mono, target(kind, name), button(label, message)}`. 24px tall, `surface-chrome`, `caption` text.
- New token: `size::DOT` (8px dot diameter) in `size.rs`.

Tests: target dots reach 3:1 on `surface-chrome`, `surface-panel` and `surface-raised` in both themes (reusing `contrast_ratio`); each kind maps to its token; an `iced_test` check that a status bar button item sends its message.

Not verified: the visual look. The design CSS (`ot-dot`, `ot-chip`, `ot-statusbar`) was not available locally, so paddings and gaps (chip 2/8, bar 0/12, item gap 12) are best estimates from kit tokens, and the gallery was only compiled, not viewed.
