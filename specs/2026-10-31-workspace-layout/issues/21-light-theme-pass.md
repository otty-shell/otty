# 21 — OTTY Light pass and screenshots

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 08, 09, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20
Spec: [../spec.md](../spec.md)

## What

- Walk every screen in OTTY Light next to `V4WorkspaceLight` and `V4SettingsLight`; fix any token used wrongly (overlays need `shadow-popover`; status colors never on `surface-hover`).
- Walk the whole regression checklist once in each theme.
- Take the screenshots for spec AC11 (every V4 screen in scope, both themes) and list them in Comments.

## Done when

- Checklist passes in both themes; screenshots ready for the PR.

## Answer

### Token audit (read the code of every view and kit component, both themes)

One rule break found and fixed: a danger menu item (Remove, Delete) drew
`status-error` text on `surface-hover` when lit. Now it uses `text-danger` on
`surface-danger` (`crates/ui/kit/src/components/menu.rs:218-250`, doc line 63).

Checked and fine: overlays with `shadow-popover` (menu `menu.rs:165`, command
palette `command_palette.rs:244`, dialog `dialog.rs:83`, floating note
`floating_note.rs:100`, select `select.rs:95`, tooltips `icon_button.rs:159`);
selected rows use `surface-selected` (`session_row.rs:185`, `tree_row.rs:226`);
session state words use status colors only on panel or selected rows; text
tokens per surface; app views only compose kit parts. The pane exit banner
uses `surface-overlay` without a shadow on purpose (it sits inside a pane).
The palette's active entry uses `surface-hover` like the design's menu item.

### Screenshots

`iced_test` renders headless with the software renderer here (no GPU, no
window). The whole root window cannot be rendered without a production
change: `App::new()` reads `~/.config/otty` and opens a real shell. So each
screen is composed in `app/src/screenshots.rs` (one `#[cfg(test)]` module,
nine `#[ignore]` tests) from the real view functions with fixed data, laid out
like `view.rs` (title bar, left panel, main area, right panel, status bar,
overlay layers) at 1440x900, written at 2x. Run:
`OTTY_SCREENSHOT_DIR=<dir> cargo test -p otty -- --ignored screenshots`
(default folder: `otty-screens` in the temp folder).

Visibility changes needed by the fixtures (no behavior change):
`QuickLaunchTreeViewModel` and `ExplorerTreeViewModel` fields, `SecretPromptState::new`
and `ContextMenuState::with_target / with_cursor` are `pub(crate)` now.

PNG files (each in `-dark` and `-light`), folder `/tmp/claude-1000/-home-kemokempo-projects-main-ottyrun-otty/901c711b-99ab-4ffe-9031-c7cc34df6eeb/scratchpad/screens`:
workspace, focus, empty, palette, new-session-menu, launch-menu, pane-menu,
passphrase, launch-editor, settings.

Not rendered: the real terminal text (the panes show drawn sample text, the
terminal widget is not used), the launch error tab, drag and drop states,
running-launch blink, tooltips.

## Owner bug fixes (2026-11-01)

- Sessions and Launches share the left panel height (`panel_height::share_panel_height` in the kit, `stack_left_panel` in `app/src/view.rs`): a closed section takes its header, open sections their content while it fits, else equal shares where a smaller section keeps its content height; Sessions scrolls inside its share. Content heights are counted from rows. Screenshot `many-sessions`.
- Title bar search: the icon, title and shortcut are centered in the 26px pill (the row now fills the button height).
- Session close cross: no background on hover, only the glyph turns brighter (`IconButton::hover_background(false)`); the old surface-raised square showed as a dark shape on the selected row. Screenshots `session-hover`, `session-close-hover`, `session-close-hover-selected`.
- Launches header: the New launch button keeps the plus icon (`add-tab.svg`). The plus is now centered in its 24px grid, so it lines up with `folder-plus.svg`.
- Launches header uses a split button (New launch / New folder) like Sessions: the plus runs New launch, the chevron opens the Launches menu (`new_launch_menu.rs`). One `HeaderMenu` state in the sidebar (`Sessions` or `Launches`, opened at the cursor) replaces the New session menu state. `folder-plus.svg` removed. Screenshot `launches-menu`.
