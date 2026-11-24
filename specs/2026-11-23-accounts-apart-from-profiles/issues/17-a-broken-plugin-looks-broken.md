# 17 A broken plugin looks broken; two small wording fixes

Status: ready
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
