# 13 OTTY's own logins live in their plugin's folder; the Plugins icon is a plug

Status: done
Blocked by: 12

## Why

Owner, 2026-11-24:

- OTTY's own login is made by a plugin, so its folder belongs to that
  plugin: `<plugins folder>/<plugin>/accounts/<id>/` (today
  `~/.local/share/otty/plugins/openai/accounts/<id>/`), not
  `<data dir>/accounts/<id>/`. It sits next to the plugin's version
  folders, so an update, which replaces only `<plugin>/<version>/`, keeps
  it. Removing the plugin removes it with the plugin.
- A profile may not be added inside an own login folder: "Remove" of an
  own login deletes its folder, which would delete a working profile.
- The Plugins mode icon (`assets/svg/mode-plugins.svg`) looks off next to
  the other mode icons. The owner chose variant B, a plug.

The owner does not want a "Use this folder" button when a new profile's
folder already has files: the current error stays.

## What to build

1. **Own login folder under the plugin.** `OwnLogin::folder_in` (and
   whatever builds the path, `crates/agents/src/own_login.rs`, the io
   crate, the host's own-login folder rules) uses
   `<plugins folder>/<plugin id>/accounts/<id>/`. The plugins folder is
   the one the installer uses (`otty-plugin-releases`
   `InstallPlaces::plugins_folder`). `agents.json` `own_logins` keeps
   `id`, `plugin`, `provider`, `agent`; the folder is still not stored.
   No backward compatibility: an own login folder at the old place is not
   read (nothing was released).
2. **Updates keep it, removal says so.** Tests over the installer: an
   update of a plugin keeps `<plugin>/accounts/` and what it holds; the
   installer never takes `accounts` for a version folder. Removing a
   plugin removes its own logins too, and the Remove plugin dialog of
   Plugins mode says how many of OTTY's own logins go with it ("OTTY's
   own logins of this plugin are removed too: 2"), only when there are
   any. The own login entries of a removed plugin leave `agents.json`.
3. **No profile inside an own login folder.** Adding or editing a profile
   whose folder is inside `<plugins folder>/*/accounts/` is refused in the
   profile dialog with a plain line ("This is a private login folder of
   OTTY, it can not be a profile."), as the dialog refuses a folder that
   another profile uses.
4. **The Plugins icon is a plug.** Replace `assets/svg/mode-plugins.svg`
   with:
   ```svg
   <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M9 3v4M15 3v4"/><path d="M6 7h12v3a6 6 0 0 1-12 0z"/><path d="M12 16v5"/></svg>
   ```
   Check every place that uses it (mode dock, Plugins hint card, tests
   that compare icons). If OTTY Lite's kit draws the mode icons from a
   copy, leave the website repo alone.
5. Docs: the Overview and Plugins pages say where OTTY's own logins live
   and that removing a plugin removes them.

## Done when

- Tests first: an own login's folder is under its plugin's folder; an
  update keeps it; removing the plugin removes it and its entries, and
  the Remove dialog names how many; a profile inside an own login folder
  is refused; the Plugins mode icon is the plug.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-24, done. OTTY's own login folder is now
  `<plugins folder>/<plugin>/accounts/<id>/`, such as
  `~/.local/share/otty/plugins/openai/accounts/<id>/`.

  Folder: `OwnLogin::folder_in(plugins_folder)` builds it from the plugin
  and the id. The plugins folder is the one of the installer
  (`otty_app_plugins_io::places(..).plugins_folder`), which the app now
  keeps as `App::plugins_folder` in place of the data folder and gives to
  every read and write of own logins. `agents.json` still keeps `id`,
  `plugin`, `provider` and `agent`, and now an entry whose plugin is no
  plain name is not read either, as for the id, so the folder can never
  point elsewhere. The old place is not read.

  Updates and removal: the installer only ever replaces
  `<plugin>/<version>/` and `<plugin>/.staging/`, and removes the whole
  `<plugin>/` on Remove, so it needed no change. Plugins mode tells the
  app `PluginsOutcome::Removed(plugin)` once a removal worked, and the
  app tells Agents mode `AgentsIntent::PluginRemoved(plugin)`, which drops
  the `own_logins` entries of that plugin and saves (nothing when it had
  none). The count of the Remove dialog comes from `agents.json`: the app
  gives Plugins mode `PluginsCtx::own_logins` (own logins by plugin, from
  `Agents::own_logins_by_plugin`), the dialog keeps the count when it
  opens, and shows "OTTY's own logins of this plugin are removed too: 2"
  only when there are any. Counting entries and not folders keeps the
  count right without a disk read, since entries and folders are made and
  removed together.

  Profiles: `otty_agents::is_own_login_folder(folder, plugins_folder)` is
  true for `<plugins folder>/<plugin>/accounts` and anything below it.
  The profile dialog refuses such a folder, added or edited, with "This
  is a private login folder of OTTY, it can not be a profile."
  (`FolderProblem::OwnLogin`), as it refuses a folder another profile
  uses. Agents mode gets the plugins folder with
  `Agents::with_plugins_folder`.

  Icon: `assets/svg/mode-plugins.svg` is the plug of variant B. It is
  used only through `icons::MODE_PLUGINS` (mode switcher, All modes menu,
  the Plugins hint card), and no copy of the old icon is left in this
  repository.

  Two texts in all ten locales (`agents_error_folder_is_own_login`,
  `tpl_plugins_remove_own_logins`). Docs: Overview (Add account, the
  folder, refused as a profile, removed with its plugin), Plugins
  (Remove, the table, where plugins are kept), What OTTY Reads and Sends
  (the folder in "What OTTY writes"), Write a Plugin (the folder), and the
  spec's mentions of the folder. No official plugin changed: they get the
  folder from the call.

  Tests first, each seen failing: the folder of an own login under its
  plugin and the own login folder check (`otty-agents`, failing to
  build), and the plug icon. Written before the code and failing to build
  until it was there: a plugin that is no plain name is not read,
  forgetting the own logins of a plugin, the Remove dialog with and
  without own logins and `Removed` told only after a removal that worked
  (Plugins mode), a removed plugin's entries leave `agents.json` and one
  with none saves nothing, a profile inside an own login folder refused
  in Add and Edit (Agents mode), and the io tests of the new folder (made,
  made once, checked with the real Anthropic plugin, removed alone, found
  by a read). These passed at once, since the installer already worked
  so: an update and a reload keep `<plugin>/accounts/`, and Remove
  removes it. Written with the code: the count by plugin of `Agents`, and
  the app test that the count reaches Plugins mode and that a removed
  plugin's own logins leave Agents mode.
