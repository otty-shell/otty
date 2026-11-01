# 16 — New session split button

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 03, 10
Spec: [../spec.md](../spec.md)

## What

The Sessions header plus becomes a kit `SplitButton`, as in `V4Palette`:

- Plus → New terminal.
- Chevron → kit `Menu`: New terminal (shortcut), Split current pane right (shortcut; disabled without a terminal tab), separator, label "Launches" and each launch at the top level of the tree (runs it), separator, New launch….
- This replaces the old add menu (`view_add_menu_overlay` in `view.rs` and the `AddMenu*` sidebar intents): New folder is in the Launches header (ticket 09). Remove what is left unused.

## Done when

- Every menu item runs an existing intent (`iced_test`); the add menu code is gone; regression checklist passes.

## Answer

The Sessions header plus is now a kit `SplitButton`: the plus opens a terminal tab (as before), the chevron opens the New session menu. The old add menu is gone.

### Menu

`app/src/widgets/tabs/view/new_session_menu.rs` (generic over the message type, 280 wide, kit `Menu`):

- New terminal (shell name) with the shortcut label, sends `TabsIntent::OpenTerminalTab`.
- Split current pane right with the shortcut label, sends `TerminalWorkspaceIntent::SplitPane` (vertical axis) for the active terminal tab's focused pane. It has no press message, so it is disabled, when the active tab is not a terminal.
- Separator, label "Launches" and each launch at the top level of the tree (folders are skipped), separator. Without launches the label and its separator are left out.
- New launch... sends `QuickLaunchIntent::HeaderCreateCommand`.
- Kit `Menu` gained `width(f32)` (default unchanged).
- The overlay in `view.rs` (`view_new_session_menu_overlay`) keeps the same dismiss layer and cursor anchor as the old add menu.

### State

The sidebar's add menu state was renamed, not rewritten: `SidebarIntent::NewSessionMenuOpen` / `NewSessionMenuDismiss`, `SidebarViewModel::{has_new_session_menu_open, new_session_menu_cursor}`. The open flag also frames the chevron (`SplitButton::menu_open`).

### Choosing an item

The menu closes and the intent runs: `update.rs` checks `guards::is_new_session_menu_choice` (open terminal, split pane, launch a node, create launch). When the menu is open and the event is one of these, the app dismisses the menu and dispatches the event, instead of asking the context menu guard (which drops quick launch intents).

### One new intent

A tree click is two events (`NodePressed`, then `NodeReleased` on the same node), and a menu item can send only one message. So there is one new quick launch intent, `LaunchNode { path }`, that calls `handle_node_left_click`, the same function a tree release ends in. A launch started from the menu is therefore the same launch as a tree click (selection, launching state, setup).

### Removed

- `view_add_menu_overlay` in `view.rs` and its imports.
- `SidebarIntent::AddMenuCreateTab`, `AddMenuCreateCommand`, `AddMenuCreateFolder`, `DismissAddMenu` (the old `AddMenuOpen` / `AddMenuDismiss` became the new names).
- `SidebarEffect::OpenTerminalTab`, `QuickLaunchHeaderCreateCommand`, `QuickLaunchHeaderCreateFolder` and their handlers in `events/sidebar.rs`.
- `QuickLaunchIntent::HeaderAddButtonPressed`, its redirect in `events/quick_launch.rs`, and the now unreachable `_` arm of the quick launch reducer.
- i18n keys `menu_create_tab`, `menu_create_folder`, `menu_create_quick_launch` (all 10 locales). New keys: `menu_new_terminal`, `menu_split_current_pane`, `menu_new_launch`, `new_session_menu` (chevron tooltip), all 10 locales.

### Tests

Written before the code. Reducer, guard and view tests were seen to fail to compile against the old API; the view tests were written against `todo!()` bodies, not run red.

| Where | Tests | What they pin |
|---|---|---|
| `sidebar/reducer.rs` | 2 | menu opens at the cursor and dismisses with no effects; dismissing a closed menu changes nothing |
| `sidebar/state.rs` | 1 | renamed: opens at the cursor |
| `quick_launch/reducer.rs` | 1 | `LaunchNode` starts the command like a click |
| `quick_launch/types.rs` | 1 | top-level launch titles skip folders |
| `guards.rs` | 4 | open ignored, dismiss allowed, the four choices recognised, other events (close tab, node press, ready) are not choices |
| `tabs/view/new_session_menu.rs` (`iced_test`) | 7 | every item sends its intent; split disabled without a terminal; "Launches" label only with launches; height grows one row per launch |
| `tabs/view/session_list.rs` (`iced_test`) | 1 new, 1 moved | chevron sends the open message; plus still opens a terminal tab |

## Comments

Regression checklist steps for the owner (the agent cannot run the GUI):

- 2: second terminal from the Sessions plus (and Ctrl Shift T). Hover the plus and the chevron: two tooltips.
- 10: create a launch from the menu (New launch...) and from the Launches header plus.
- 8, 9: run a command launch and an SSH launch from the menu's Launches list; the row appears in Sessions and the tree shows "running", the same as a tree click. For an SSH launch with a passphrase, the prompt opens.
- New: chevron opens the menu under the cursor; the chevron keeps its frame while open. Esc is not wired (no add menu had it); a click outside, or a right click, closes it.
- New: Split current pane right splits the focused pane to the right, and the new pane gets focus. With the Settings tab active the item is dimmed and does nothing.
- New: with launches in folders only, the menu shows no "Launches" label. Launches at the top level show in tree order.
- 21: switch language; menu items and the chevron tooltip change.
- 22: toggle Sessions closed; the header has no split button then (count only), as before for the plus.

Not done, on purpose: "Terminal in a folder..." (out of scope), Alt N labels (do not exist), launch dots in menu rows (kit `MenuItem` has no dot). The menu opens at the cursor (right of the chevron, or flipped to its left near the window edge), not directly under the plus as in the design.
