# 02 — Kit: TargetDot, Chip, StatusBar

Status: ready-for-agent
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
