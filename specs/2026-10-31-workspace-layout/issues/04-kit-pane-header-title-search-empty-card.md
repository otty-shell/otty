# 04 — Kit: PaneHeader, TitleSearch, ActionTile, FloatingNote, EmptyCard

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 01
Spec: [../spec.md](../spec.md)

## What

Reference: `pane-head`, `tb-search` in `V4Workspace`; `float` note in
`V4Focus`; `empty-card`, `big-action` in `V4Empty`.

| Component | Shape |
|---|---|
| `PaneHeader` | Thin bar above a pane: program name, mono muted folder, gap, optional `Chip`; focused variant on `surface-header`, other panes on `surface-canvas` |
| `TitleSearch` | Title-bar button that looks like a search field: search icon, the focused session title, `Kbd` with the palette shortcut; one press message |
| `ActionTile` | Large row button for the empty card: icon, label, optional `Kbd`; first one can use the accent icon |
| `FloatingNote` | Pill on `surface-overlay` with `shadow-popover`: text parts, muted parts and a `Kbd` |
| `EmptyCard` | Centered column: logo (`LOGO` size, never recolored), `title`, muted description, a stack of `ActionTile`s |

- Gallery sections in both themes.
- `iced_test`: `TitleSearch` and `ActionTile` presses emit their messages.

## Done when

- All five render in the gallery in both themes and match the V4 artboards in size and tokens.
