# 01 — Kit: ModeSwitcher, ModeMenu

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: —
Spec: [../spec.md](../spec.md)

## What

New components in `crates/ui/kit/src/components/`, one file each, styled only
with `UiPalette` and kit tokens (spec Decision 15). Visual reference: canvas
boards `DockerMode` and `DockerModesMenu`, classes `.mode-tab*` and `.mm-*` in
`project/docker-mode.css`.

| Component | Shape |
|---|---|
| `ModeSwitcher` | Raised group (`surface-raised`, `radius-md`, padding 3, gap 2) of 30×26 icon tabs. Active tab: `surface-hover`, icon in `accent`. Temporary tab: 1px `line` outline. A trailing "⋯" button that shows when its menu is open. Each tab has a tooltip with its name and shortcut. |
| `ModeMenu` | Floating menu, 300 wide, `surface-overlay`, `shadow-popover`. Section label with an optional right-aligned note ("1 of 4"). Rows: 22px icon tile, name, optional `Kbd`, pin toggle. Pin states: none (Terminal), pinned (accent), unpinned, disabled with a hint. An optional hint line for an empty section. |

- Callers pass icons as `svg::Handle`; the kit bundles none.
- Sizes without a token yet (tab 30×26, icon tile 22, menu width 300) go to `size.rs` with doc comments.
- Messages: tab pressed, "⋯" pressed, row pressed, pin pressed. A disabled pin emits nothing.
- Gallery: one section per component, both themes, including a temporary tab and a full pinned list.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test`: pressing a tab and the "⋯" button emit their messages; pressing a pin emits the pin message; a disabled pin emits nothing.
- Contrast tests (`contrast.rs`): active icon on `surface-hover`, muted icon on `surface-raised`, both themes.
