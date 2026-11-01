# 09 — Left panel: Launches section

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 03, 07
Spec: [../spec.md](../spec.md)

## What

The quick launch tree becomes the lower `PanelSection` "Launches" of the left
panel, as in `V4Workspace`.

- Header actions: New folder and New launch (existing intents `QuickLaunchHeaderCreateFolder`, `QuickLaunchHeaderCreateCommand`). Collapsed header shows the launch count.
- Rows: folders with caret and folder icon; launches with a `TargetDot` (`Ssh` for SSH launches, `Local` otherwise) and the muted word "running" while a launch is active (existing `launching` data).
- Drag and drop, inline rename and the context menu keep working unchanged.
- Section open/closed is view state in the sidebar widget (one bool per section, not persisted).
- English title "Launches"; update all 10 locales.

## Done when

- Quick launch flows of the regression checklist pass, including drag and drop; `iced_test`: header buttons emit the existing intents.

## Answer

The quick launch tree is now the "Launches" `PanelSection` of the left panel.

- `quick_launch/view/sidebar_panel.rs`: `view(SidebarPanelProps)` builds the section. It is generic over the message type: `on_toggle` is the message for the title (the toggle belongs to the sidebar) and `on_intent: fn(QuickLaunchIntent) -> Message` wraps the tree and the header buttons. The section is `grow(true)`, with New folder and New launch header buttons (existing intents `HeaderCreateFolder`, `HeaderCreateCommand`; tooltips reuse `create_folder` and `create_quick_launch`). While closed, the header shows the launch count.
- `sidebar_tree.rs`: folders have caret plus a folder icon; launches have a `TargetDot` (`Ssh` for SSH launches, `Local` otherwise) and the muted word "running" while the path is in `launching`. Drag and drop, inline rename, hover and the context menu code is unchanged. The blinking play icon (`is_indicator_highlighted`) is gone; the field is still kept by the reducer.
- `TreeRow` (kit) gained `.dot(TargetKind)` and `.state(word)`. Purely additive.
- Sidebar widget: `launches_open: bool` (default open, not saved), `SidebarIntent::ToggleLaunches`, `SidebarViewModel::is_launches_open`. No effects.
- Domain helpers: `QuickLaunch::is_ssh()`, `QuickLaunchFile::launch_count()`.
- i18n: `quick_launch_panel_title` replaced by `launches_panel_title`; new `launch_running`; all 10 locales.
- Assets: `folder-plus.svg`, `folder-closed.svg`.
- `view.rs`: `view_left_panel` is now a `column![launches]`.

### How ticket 10 adds the Sessions section

In `view_left_panel` (`app/src/view.rs`), build the Sessions `PanelSection` (not `grow`) and put it first: `column![sessions, launches]`. Add its open flag the same way as `launches_open` (state field, `ToggleSessions` intent, vm field, reducer test first). Launches is the only `grow` section, so it takes the height left under Sessions.

### Tests (written first)

- `sidebar/reducer.rs`: launches section starts open and toggles without effects.
- `quick_launch/types.rs`: `is_ssh`; `launch_count` for nested folders and an empty file.
- `quick_launch/view/sidebar_panel.rs` (`iced_test`): title sends toggle; New launch and New folder buttons send `HeaderCreateCommand` and `HeaderCreateFolder`; body only while open; count only while closed; "running" word only for an active launch.
- Existing quick launch reducer tests are untouched.

### Not verified

The visual look (the app was not run). The dot on a launch row has no test; drag and drop was not exercised by hand.

## Comments

Regression checklist steps to walk by hand: 8 (command launch, "running" word), 9 (SSH launch), 10 (create a launch: now the "+" in the Launches header; the add menu button is gone from the left panel, New tab remains the tab bar "+"), 11, 12 (rename in the tree), 13, 14, 15 (drag and drop into a folder and to the empty area), 16 (restart keeps the tree), 22 (hide and show the panel with Launches open and collapsed), 20 and 21 (light theme and a non-English language show the new title and "running").
