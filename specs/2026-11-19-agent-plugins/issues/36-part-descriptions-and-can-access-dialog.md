# 36 A part says what it does; what it can access opens in a dialog

Status: ready
Blocked by: 35

## Why

Owner, 2026-11-22, after issue 35, about a plugin's page in Plugins mode:

- "Can access" should be a button that opens a dialog with the groups,
  and the line "Only this is open to the part, OTTY refuses everything
  else." belongs in that dialog, not on the card.
- Each part should have a short description that comes from the plugin.
- The plugin descriptions of the official plugins use semicolons: use
  commas.
- Whether a plugin is local or from a repository should be a tag, not
  the long trust note ("Local plugin: installed from a folder on this
  computer and never updated on its own").
- No Repository row: a local plugin usually has none, and a plugin from
  a repository already shows it as its Source.
- On the Anthropic page it is not clear what "From agents" holds. Today
  the provider part shows "Anthropic accounts" (the logins and keys of
  Anthropic accounts that agent plugins found, as handles) and the sites
  `anthropic.com`, `claude.ai` (the keys found for those hosts), with no
  word on what they are.

## Prototype and owner's choices

Prototype in OTTY Lite (website repo, read only):
`/home/kemokempo/projects/main/ottyrun/website/src/lite/prototype-plugins/`,
page `http://localhost:4321/lite/prototype-plugins/` (`?open=access`
opens Can access of the Anthropic provider). Owner, 2026-11-22: the main
page is approved as the prototype shows it:

- the button "Can access · N" (N places) on the card, next to the part's
  description; the dialog "<Part> can access" with the line and the
  groups, each group label with one muted line under it (Files "Read
  only", Changes files "May write and delete", Databases "SQLite, read
  only", Variables "Environment variables, read only", Programs "Runs
  them and reads the output", Hidden terminal "Runs without a visible
  tab and reads the output", Keyring "Entries of the system keyring,
  given as handles", Logins "Where your logins are kept, given as
  handles", Sites "Sends requests only to these", Found logins "Logins
  and keys of <Provider> accounts that agent plugins found, given as
  handles");
- "From agents" is "Found logins", its places "Anthropic accounts",
  "keys for `anthropic.com`" (host in monospace);
- the trust tag after the version, its note in the tooltip; facts
  Version, Authors, Source;
- short plugin descriptions with commas, e.g. "Claude Code and Anthropic
  accounts in OTTY.", and one sentence per part, e.g. Claude Code "Finds
  Claude Code profiles, sessions, tokens and logins.", Anthropic "Reads
  Claude limits and Anthropic organization bills.".

The install dialog (and the update dialog the same way) must read as
nicely: the line "Only this is open to the part, OTTY refuses everything
else." once above the parts, each part as a compact card (name, kind tag,
description), under it its groups inline under "Can access" with the
explanation lines. No button there: at install the permissions are
visible at once. The update dialog keeps its "new" / "no longer" marks.

## What to build

1. **A part's description.** A part of the manifest (`[[agents.agent]]`,
   `[[agents.provider]]`) may have `description`, one short sentence,
   read by `otty-plugin-manifest` like the plugin's own. The part card
   shows it under its header, muted; a part with none shows nothing. The
   official plugins (Anthropic, OpenAI, OpenCode, DeepSeek, OpenRouter,
   template) give one for each part. Write a Plugin and the manifest
   reference follow.
2. **Plugin descriptions with commas.** The official plugins' `description`
   lines use commas, no semicolons, and can be shorter now that each part
   says what it does. Bump each changed plugin's patch version and
   rebuild with `plugins/build.sh`.
3. **Trust tag.** In the page header, after the version, a small tag
   "Official", "Community" or "Local" (the kit `tag`, like the kind tag
   of a part) replaces the trust note line; the tag's tooltip holds the
   note.
4. **No Repository row.** Facts are Version, Authors and Source only.
5. **Can access is a button.** The card shows a small secondary button
   "Can access…" (with the number of places, if it fits well); it opens
   a dialog titled "<Part> can access" with the line "Only this is open
   to the part, OTTY refuses everything else." and the groups as today.
   A part with no permission shows the muted line "Asks for no
   permissions" and no button. The install and update dialogs keep the
   groups inline, as today.
6. **"From agents" says what it is.** In the dialog every group may carry
   one muted line saying what it means; "From agents" is renamed to say
   what it holds (for example "Found logins": "Logins and keys of
   Anthropic accounts that agent plugins found, given as handles"), and
   its places read as such ("Anthropic accounts", "keys for
   anthropic.com"). Final words come from the prototype.

## Done when

- The prototype in OTTY Lite shows it and the owner agrees.
- Tests first: a part's description is read and shown, none shows
  nothing; the trust tag and no Repository row; Can access opens the
  dialog with the line and the groups, a part with no permission has no
  button.
- The checks of `AGENTS.md` pass.

## Comments
