# 37 A plugin in the list has a context menu

Status: done
Blocked by: 36

## Why

Owner, 2026-11-22: in the list of plugins in the left panel of Plugins
mode, the context menu (right click) should do what the plugin's page
does: show the log, remove, update.

## What to build

1. A right click on a plugin row in the left panel
   (`crates/app/features/plugins/core/src/view/side_panel.rs`) opens a
   context menu in the kit's style, as the context menus of Explorer and
   Quick Launch (`crates/app/features/explorer/core/src/view/context_menu.rs`,
   `crates/app/features/quick-launch/core/src/view/context_menu.rs`), with
   the actions of the page, in its order and words:
   - **Reload** for a local plugin, **Check for updates** for one from a
     repository;
   - **Review update…** while an update waits for the user's OK;
   - **Show log**;
   - **Remove…** (danger), which asks as the page's Remove does.
2. Each item sends the same intent as the page's button, so results come
   the same way (notifications, dialogs). While the plugin is busy the
   items that change it are disabled, as on the page.
3. The menu closes on Escape, a click outside, or after an item. A right
   click does not change which plugin is selected unless the page's own
   rules say so (follow Explorer).

## Done when

- Tests first: a right click on a row opens the menu with the items of
  that plugin (Reload for a local plugin, Check for updates for a
  repository one, Review update… only with a waiting update); each item
  sends the page's intent; Remove asks first.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-22, done. A right click on a plugin row of the left panel sends
`PluginsIntent::OpenMenu`, and the menu opens at the pointer in the kit's
`context_menu`, as the menus of Files and Quick Launch. Plugins mode now
takes a `PluginsCtx` (the pointer and the size of the main area), as
Explorer and Agents mode do. The items are Check for updates (Reload for
a local plugin), Review update… while an update waits, Show log, a
separator and Remove… (danger). Each sends the intent of the page's
button (`CheckForUpdates`, `Reload`, `Review`, `ShowLog`, `AskRemove`),
and the reducer closes the menu before it does what the page does, so
the notifications and dialogs come the same way and Remove… asks first.
While a change runs (`is_busy`) every item but Show log is disabled. The
right click does not change the selection (as the menus of the left
panel of Agents mode), and Review update… selects the plugin, as Review
of the page does. Esc, a press outside the menu and any other input of
the window close it: the binary knows it as `Overlay::PluginsMenu`, lets
its items pass under it and closes it with the other menus. A dialog
lies over it. The menu of a plugin removed elsewhere closes with it. New
words: "Review update…" and "Remove…" in ten locales. Docs: Plugins.mdx.
