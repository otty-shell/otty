# 13 OTTY's own logins live in their plugin's folder; the Plugins icon is a plug

Status: ready
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
