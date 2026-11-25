# 17 A broken plugin looks broken; two small wording fixes

Status: done
Blocked by: 16

## Why

Owner, 2026-11-25, with a screenshot of Plugins mode: the installed
plugins (OpenCode 0.1.9, OpenAI and Anthropic 0.2.13, installed before
issue 15) showed letter icons ("A", "O"), the part named by its id
("opencode") and no descriptions, while the part said "Working". Cause:
their installed manifests still hold `[api]`, which the manifest parser
refuses since issue 15, so `read_manifest` in
`crates/app/features/plugins/io/src/installed.rs` returns `None` and the
list falls back to a letter icon and raw ids with no word about why. The
owner asked for a special icon for such plugins.

Also agreed the same day:
- the full name of an Anthropic organization login nests parentheses,
  "Anthropic (you@example.com (Example Org))": the plugin's own name
  uses a comma inside, "you@example.com, Example Org", so the full name
  is "Anthropic (you@example.com, Example Org)";
- after a comma a word starts in lower case: "3 760 left, resets …", not
  "Resets".

## What to build

1. **A broken plugin icon.** A kit/theme icon "broken plugin": the plug
   of the Plugins mode icon (`assets/svg/mode-plugins.svg`) with a small
   warning mark in a corner, in the same stroke style, drawn in the warn
   colour where it shows.
2. **When it shows.** A plugin is broken when OTTY can not read the
   manifest of its installed version, or the plugin failed to load (the
   host's load error), or its `.wasm` does not match its manifest
   (issue 15's contract checks). Then:
   - Plugins mode, the list row and the page header use the broken
     icon, the row's mark says "failed" (as the update check failure
     does), and its parts are not shown as "Working";
   - the page shows the reason in the error colour, such as "The
     manifest of version 0.1.9 can not be read: unknown field `api`",
     with **Reload** for a local plugin and **Reinstall** (install the
     same source again) for one from a repository, which replace the
     broken version with the current one;
   - Agents mode: a profile or account whose plugin is broken (if any
     still shows) and any place that would draw the letter icon for a
     part of a broken plugin uses the broken icon, with the tooltip
     "Plugin <name> failed, see Plugins".
   A plugin that reads and loads fine keeps its own icon. A part with no
   icon of its own in a healthy plugin keeps the letter icon.
3. **Words.** Anthropic plugin: an organization login's name is
   "<email>, <organization>" (bump, rebuild). In all ten locales, a word
   after a comma in a built text starts in lower case ("resets"), check
   every two-value text made by issue 16.
4. Docs (`docs/public/Agents/Plugins.mdx`) say what a broken plugin
   looks like and what Reload or Reinstall does.

## Done when

- Tests first: an installed plugin whose manifest holds `[api]` is shown
  broken with the reason and Reload, not "Working" with a letter icon; a
  plugin that failed to load is shown broken; a healthy plugin keeps its
  icon; Reload of a local broken plugin installs the folder's current
  version; the Anthropic organization's full name has no nested
  parentheses; "resets" is lower case after a comma.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-25: Done. A new icon `assets/svg/plugin-broken.svg`
(`icons::PLUGIN_BROKEN`): the same three paths of the plug of Plugins
mode, moved up and left, and a small filled warning triangle with the
exclamation mark cut out in the lower right corner; it is drawn in the
warn colour.

When a plugin is broken: `ListedPlugin::manifest` is now
`Result<Manifest, String>`, and `otty-app-plugins-io` keeps the reason
the installed manifest can not be read (for a shape error the parser's
own words, such as "unknown field `api`, expected ..."). A plugin whose
manifest reads but whose `.wasm` the engine does not load (a load error,
which holds issue 15's `LoadError::Contracts`) is named in the new
`Readers::failed` / `LoadedReaders::failed` of `otty-app-agents-io`
(`FailedPlugin`), and the binary passes them to Plugins mode with the
running parts: `PluginsIntent::Running` became `Running { running,
failed }` (`PluginLoadFailure`). `model::broken_reason` gives "The
manifest of version 0.1.9 can not be read: ..." or "The plugin failed to
load: ...". Then the row has the broken icon and is signed failed, the
page header has the broken icon, the parts that are on are "Failed"
(error tone, an off part stays "Off"), and the page shows the reason in
the error colour with Reload (local) or Reinstall (repository) on its
line; the first action button is left out then, and the menu of the
plugin has Reinstall in place of Check for updates.

Reinstall: `Installer::reinstall` takes the newest release of the
repository with no "newer than installed" limit, so the same version
replaces the broken copy too, goes through the same diff (asks for more
-> waits for Review), keeps the parts on or off; for a local plugin it is
Reload. New `PluginsIntent::Reinstall`, `PluginsOutcome::Reinstall`,
`PluginsIntent::Reinstalled`, `otty_app_plugins_io::reinstall`;
notifications "Plugin X reinstalled", "updated to", "could not be
reinstalled: ...".

Agents mode: the plugin notes carry `broken_parts` (`BrokenPart`, from
`plugins.json`, so they are known even when the manifest is not), sent
also after every load. `agent_icon` and `account_icon` give
`EntryIcon::Broken(<plugin name>)` when no registry entry serves the id
and a broken part would; a healthy entry keeps its own icon, an unknown
id keeps the letter. It is drawn in the warn colour with the tooltip
"Plugin <name> failed, see Plugins" (on the icon of a page, on the whole
row in the left panel), and as the broken icon where only an SVG can be
drawn.

Words: the Anthropic plugin names an organization login "<email>,
<organization>" (anthropic 0.2.15 -> 0.2.16, rebuilt into
`assets/plugins/anthropic`, its tests and clippy pass), so the full name
is "Anthropic (you@example.com, Example Org)". New
`i18n::after_comma`: a phrase of OTTY's own words after a comma starts
in lower case, except in German, whose phrases there start with a noun
(the German "resets now" became "Zurücksetzung jetzt" so all four start
with one). Checked every two-value text of issue 16: the credits line
"3 760 left, resets ..." and the need heading ("Codex, needs login", own
titles and the titles plugins give) use it; a need title that starts with
a name (plugin failed, format changed), "{amount}, at API prices" (lower
already), "{length}, {model}" (a model name) and the plugin row signs
(lower already, German nouns) stay. Seven new keys in all ten locales.
Docs: Plugins.mdx has "A broken plugin" (what it looks like, Reload and
Reinstall), the Anthropic page its version and the name with a comma.

Tests first, then code: the failing tests were committed first and did
not build (no `PLUGIN_BROKEN`, `after_comma`, `reinstall`,
`broken_parts`, `EntryIcon::Broken`, `PluginLoadFailure`, new keys and
fields) and the Anthropic tests failed on the names.
