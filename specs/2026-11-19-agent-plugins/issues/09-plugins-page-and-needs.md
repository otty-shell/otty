# 09 Plugins page and the plugin needs in Agents mode

Status: ready-for-agent
Blocked by: 01, 07, 08

## Why

Spec Decision 12, "UI changes" and the table "What the UI shows". The
user needs one place to add, update, turn parts on and off, and remove
plugins, and Agents mode must say which plugin read each number. The page
opens from Settings, since plugins will not only be for agents.

## What to build

1. Feature crates `otty-app-plugins` and `otty-app-plugins-io` in
   `crates/app/features/plugins/` (each with a `README.md`), following
   the app's architecture checks (dependency limits and `clippy.toml`
   bans; the io crate does the network and disk through
   `otty-plugin-releases`). The page is opened from Settings.
2. The list: icon, name, version, "Official" (from the constant of issue
   08), "Community", "Built in" or "Local"; its parts, each with its kind
   and an on/off switch ("Codex (agent) [on] · OpenAI (provider) [on]");
   "⋯" (Check for updates, or **Reload** for a local plugin; Open
   repository; Show permissions; Show log; Remove). A paused part shows **Try again**. An empty state with **Add
   plugin…**.
3. **Add plugin…**: URL field, then the install dialog (spec "Install and
   update flow" step 4) with each part and its permissions in words, and
   for each id another plugin serves the choice "Anthropic is already
   served by Anthropic (official). Use: [official] / [this plugin]";
   **Install** / **Cancel**; errors in plain words, "Needs a newer OTTY"
   among them.
4. **Add plugin from folder…**: a folder picker, then the same install
   dialog; the plugin is marked "Local". **Reload** reads the folder
   again and shows the update dialog when the rebuilt copy asks for more
   permissions.
5. Updates: daily check and **Check for updates**; automatic updates show
   "Updated to <version>" in the plugin's row; an update that waits shows
   a badge and the update dialog with the permission diff per part
   ("new", "no longer", new parts); a release that needs a newer OTTY
   shows "<version> needs a newer OTTY".
6. Agents mode: Account info and Profile info "Read by: <plugin>
   <version>" (or "no plugin", or "(off)") linking to the plugin; each
   row of Where the numbers come from names its plugin; the needs "Key
   for <host> found in <agent>: no provider plugin [Add plugin…]" (button
   now opens Add plugin), "<Part> of the <Plugin> plugin is off [Turn
   on]", "Plugin update needs your OK: <name> <old> → <new> asks for new
   permissions [Review]", "<Plugin> <version> could not read <what> [Copy
   report]"; the "format changed" need gets **Open plugin repository**.
   They show where needs show today: the pages, All accounts / All
   profiles Needs you, Found by OTTY.
7. i18n for every new text in the app's languages.

## Done when

- Tests first (feature core, no I/O): the list model from installed
  plugins and their parts; turning one part off leaves the others on;
  the install dialog lists permissions per part and the conflict
  choices; the update dialog's diff; each need appears for its situation
  of the spec table and leaves when it is solved; Read by rows for
  plugin, built-in, no plugin, off; a local plugin shows "Local" and
  Reload instead of Check for updates.
- The app's architecture checks and the checks of `AGENTS.md` pass.

## Files

`crates/app/features/plugins/` (new), `crates/app/features/agents/core/`
(info rows, needs), `crates/app/otty/` (wiring, Settings entry),
`crates/app/i18n/`.

## Owner checks by hand

Add the template plugin with **Add plugin from folder…** from its build
folder, see the permissions of its two parts, install, turn one part off
and on, rebuild it and press **Reload**, remove; see "Read by" on an
account and a profile; see the five official plugins listed as "Official
· built in".

## Comments
- 2026-11-20, issue 08 (left for this issue): wire the real `Places` (XDG folders), the built-in readers list and the once-a-day update trigger into the app. When an update adds a part whose id another plugin serves, there is no dialog yet (the new part stays off) — show the same "Use: [A] / [B]" choice. When a plugin is removed, turn back on the other plugins' parts that had lost to it. Releases are read only from the first page (100); fine for now.
