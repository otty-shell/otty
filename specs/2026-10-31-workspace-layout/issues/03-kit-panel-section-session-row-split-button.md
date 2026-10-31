# 03 — Kit: PanelSection, SessionRow, SessionGroup, SplitButton

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 01
Spec: [../spec.md](../spec.md)

## What

Building blocks of the left panel. Reference: `sec`, `sec-head`, `ot-group`,
`ot-session` in `V4Workspace` and `V4Palette`, and the design system card
`SessionRow`.

| Component | Shape |
|---|---|
| `PanelSection` | Header: caret (open/closed), `heading` title, optional count or note, up to three `IconButton` (`sm`) actions on the right; body below when open; one section can be marked to take the remaining height. Emits a toggle message |
| `SessionGroup` | Group label row: `TargetDot`, machine name, muted kind ("this computer", "ssh") |
| `SessionRow` | `row` (30) tall, indented under its group; title (ellipsis is not available in iced 0.14, keep clipping as today), optional muted state word ("connecting", "editor"), selected = `surface-selected`, hover `surface-raised`; press, right press and close messages |
| `SplitButton` | Primary `IconButton` plus a narrow chevron button sharing one hover frame; two messages |

- Gallery sections in both themes, including a `PanelSection` holding `SessionGroup` + `SessionRow`s and a collapsed one.
- `iced_test`: section toggle, row press, split button parts each emit their own message.

## Done when

- Every state (idle, hover, selected, collapsed) is visible in the gallery in both themes.
