# 30 Plugins is its own mode

Status: done
Blocked by: 29

## What to build

Owner, 2026-11-22: the Plugins page opened over Settings looks awkward.
Plugins becomes its own mode, like Agents mode:

1. A new mode **Plugins** in the mode switcher and the All modes menu.
   It is not pinned by default (`DEFAULT_PINNED` stays Containers and
   Agents); the user can pin it like any mode.
2. Layout as in Agents mode: a left side panel and a main area.
   - Left: the list of installed plugins (icon, name, its mark such as
     Official / Community / Local, and a sign when it is off, waits for
     an update or failed), a search field that filters by name, and a
     button to add a plugin (from a repository or from a folder, the two
     ways of today).
   - Main: the selected plugin: what it is (name, version, author,
     repository, source), its parts with on/off, their permissions, its
     health and last failure, update / reload / remove. With no plugin
     selected, or none installed, a short line saying what to do.
3. The Plugins page over Settings goes away. Settings' **Manage
   plugins…** and Agents mode's "No agent plugin" button open Plugins
   mode with the right plugin selected when there is one.
4. Words in all ten locales. Docs (`docs/public/Agents/Plugins.mdx` and
   wherever the Plugins page is named) follow.

Reuse the side panel and list components of Agents mode and of
`otty-ui-kit` where they fit; keep the plugin logic in the existing
plugins feature crates.

## Done when

- Tests first for the model: search filters by name, the selection, the
  open-from-Settings/Agents outcome selects the plugin, the mode is not
  pinned on first start.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-22, issue 30, done. Plugins is `Mode::Plugins` (saved as
  `plugins`, last in `Mode::ALL`, no right panel), in the switcher and All
  modes with its own puzzle icon (`assets/svg/mode-plugins.svg`) and the
  palette entry "Switch to Plugins mode"; `DEFAULT_PINNED` stays
  Containers and Agents. `otty-app-plugins` is now the mode: its state
  keeps the selected plugin, the search and an open dialog in place of
  the page's open flag, its list screen, the highlighted row and "⋯";
  it fills the left panel (an "Installed" section as in Agents mode,
  with `+` for Add plugin…, a folder button for Add plugin from folder…,
  the search by name, letter case aside, and a `TreeRow` per plugin with
  its icon, name, mark and a sign "failed", "update" or "off"), the main
  area (the page of the selected plugin: version, description, who
  reviewed it, authors, repository link, source, its parts with switch,
  health, last failure and granted permissions, the news and Review,
  Check for updates or Reload, Show log, Remove; or a card that says
  what to do with both ways to add) and the mode overlays (Add plugin…,
  Add plugin from folder…, install, update, log, Remove). The Show
  permissions dialog, `ToggleActions`, `Close`, `Escape`, the outcomes
  `Opened` and `Closed`, and the keys `PluginsTitle`,
  `PluginsShowPermissions` and `TplPluginsPermissionsTitle` are gone;
  Esc and Cancel send `Back`. `PluginsPlace::List` shows the mode with
  nothing selected, `Plugin(id)` selects it, `Review(id)` selects it and
  opens its update dialog when one waits, `AddPlugin` opens the URL
  field. A removed plugin leaves nothing selected; an installed one is
  selected. The binary opens the mode through
  `dispatch::open_plugins_mode`: Settings' Manage plugins… now closes
  Settings (its unsaved edits go, as on Close) and shows the mode, and
  `AgentsOutcome::OpenPlugins` uses it too. The Plugins page layer over
  Settings is gone from `view.rs`; a dialog of the mode is
  `Overlay::PluginsDialog` and counts only in Plugins mode. Nineteen new
  words in all ten locales and a new line for the empty state. Tests:
  the search (case, names only, the note when nothing matches), the
  signs, the selection by a press, from Settings and from Agents, after a
  removal and after an install, the two empty-state lines, the page
  facts, parts, health and permissions, the views of the panel and the
  page, Plugins not pinned on first start and pinnable, the mode in the
  window with its dialog, Manage plugins… switching the mode, Esc in a
  dialog of the mode. Docs (Plugins, Write a Plugin, Overview), the spec
  (Decision 12, the UI list; the old conflict choice left the UI list
  too), the READMEs of the official plugins and the template, and the
  comments of the code say Plugins mode. No crate dependency changed;
  the earlier issues' comments are left as they were.

  Left open for the owner: Manage plugins… drops the unsaved edits of
  Settings, as Close does; "off" shows only when every part is off; a
  single failed call that did not pause the part gives no "failed" sign;
  the search looks at names only; Open repository is now the repository
  link on the page, and the permissions show on the page in place of the
  Show permissions dialog; the status bar of the mode has nothing of its
  own on the left; both new icons (puzzle, folder) are drawn for OTTY.
