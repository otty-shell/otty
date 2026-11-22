# 30 Plugins is its own mode

Status: open
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
