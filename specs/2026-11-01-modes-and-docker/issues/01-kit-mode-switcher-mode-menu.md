# 01 — Kit: ModeSwitcher, ModeMenu

Status: resolved
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

## Answer

Built `ModeSwitcher` and `ModeMenu` in `otty-ui-kit`.

### API

`components::mode_switcher` (one file):
- `ModeSwitcher::new(&palette).tab(ModeTab).more(label, is_open, msg)`; converts into `Element`.
- `ModeTab::new(icon: svg::Handle, label)` with `.shortcut(..)`, `.active(bool)`, `.temporary(bool)`, `.on_press(msg)`. The tooltip reads `Name (Shortcut)`.
- The caller passes tabs in display order (Terminal, pinned, temporary). The "name of the active mode next to the tabs" is left to the app.

`components::mode_menu` (one file):
- `ModeMenu::new(&palette).section(ModeMenuSection)`.
- `ModeMenuSection::new(label)` with `.note(..)`, `.row(ModeRow)`, `.hint(..)`.
- `ModeRow::new(icon, name)` with `.shortcut(..)`, `.pin(ModePin)`, `.on_press(msg)`.
- `ModePin<Message>`: `Hidden` (Terminal), `Pinned { label, on_press }`, `Unpinned { label, on_press }`, `Disabled { hint }`. `label`/`hint` are the tooltips. A disabled pin has no message, so it emits nothing.

Messages are caller-defined: tab, "⋯", row and pin each carry the message the caller gave. The kit has no text of its own, so all strings (names, tooltips, "Pinned", "1 of 4", hint) come from the caller via `i18n`.

### Tokens and files
- `size.rs`: `MODE_TAB_WIDTH` 30, `MODE_TAB_HEIGHT` 26, `MODE_ICON_TILE` 22, `MODE_MENU` 300.
- `glyph.rs` (private to the kit): `dots()` and `pin(is_pinned)` structural shapes, because the "⋯" button and the pin toggle are part of the components' own structure. Mode icons are still caller-supplied.
- `components/mod.rs`: two `pub mod` lines.
- `palette.rs` tests: contrast tests for accent on `surface-hover` and `text-muted` on `surface-raised` at 4.5:1, and accent and `text-faint` pins on `surface-overlay` at 3:1, both themes. They sit in `palette.rs` because that is where the contrast checks live (`contrast.rs` only tests the ratio function).
- Gallery: "ModeSwitcher" and "ModeMenu" sections in the `rows` group (both themes through the theme toggle): active tab, temporary tab with the menu open, full pinned list with a disabled pin, unpinned row with the empty-section hint.

### Tests (`iced_test`)
Tab, active tab and "⋯" presses send their messages; a tab without a message sends nothing; row press, pin and unpin presses send their messages; a disabled pin and the blank spot of a Terminal row send nothing; styles for active, idle, open and temporary tabs; note layout.

### Differences from the ticket
- The row hover background covers the name button only, not the pin, because iced has no shared hover across two buttons. The pin has its own hover background.
- Tab and pin icon hover turns `text-strong` when the pointer is over the icon itself (same approach as `IconButton`), not over the whole tab.
- Coverage (lines): 79.57% before, 79.79% after.
