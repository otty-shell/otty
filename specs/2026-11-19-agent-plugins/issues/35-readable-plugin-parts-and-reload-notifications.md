# 35 The parts of a plugin page read easily; Reload says its result in a notification

Status: ready
Blocked by: 34

## Why

Owner, 2026-11-22, about a plugin's page in Plugins mode
(`crates/app/features/plugins/core/src/view/details.rs`):

- The first part of the page (icon, name, description, facts) is good.
  The second part, **Parts**, is hard to read: each part is a switch, a
  health line and a long list of bullets such as
  "• Reads your login in the column `value` of `credential` in
  `~/.local/share/opencode/opencode.db` (OTTY gives it only as a handle)",
  with the backticks shown as they are.
- After **Reload** the result is drawn on the page ("Updated to 0.1.3"),
  and it says "updated to" a version even when nothing changed. The
  result should be a notification: "Plugin OpenCode reloaded" when the
  version is the same, "Plugin OpenCode updated to 0.1.4" when it
  changed.

## Prototype and owner's choices

Prototype in OTTY Lite (website repo):
`/home/kemokempo/projects/main/ottyrun/website/src/lite/prototype-plugins/`
(`PartCard.tsx`, `Notifications.tsx`, `PluginsPrototype.tsx`, `data.ts`,
`prototype.css`), page `http://localhost:4321/lite/prototype-plugins/`.
Owner, 2026-11-22:

- **Variant A**: the label of a group in a column on the left, its places
  on the right, as the facts list at the top of the page.
- **A success notification** is new: neutral, with a green check (the app
  has only red error toasts today). Build it.
- Over the groups a small title "Can access" with one muted line: "Only
  this is open to the part; OTTY refuses everything else." — the owner
  did not see at once that the groups are permissions.
- The group of provider logins found by other plugins is "From agents".

## What to build

1. **A part is a card.** Each part is a card on the page's surface, in the
   kit's style (border and radius of other cards):
   - a header row: the part's name, a small tag with its kind (Agent /
     Provider), its health as a status badge (Working, Paused, Off; the
     kit has `status_badge`, `tag`), the switch at the right end;
   - while paused, Try again next to the badge; the last failed call and
     Copy report as a muted row under the header, as today;
   - its permissions grouped by what they reach, one row per group: a
     short label on the left (Files, Changes files, Databases, Variables,
     Programs, Hidden terminal, Keyring, Logins, Sites, Accounts of other
     agents — final words chosen to be short in all ten locales) and on
     the right its places, one per line, in the monospace font, without
     backticks. A group with nothing is not shown. A login or key place
     says where (file, or column of table in database), with no repeated
     "(OTTY gives it only as a handle)": one muted line at the end of the
     card, shown only when the part reads logins or keys, says it once
     ("OTTY gives logins and keys to the plugin only as handles; the
     plugin never sees their values").
   - a part with no permission says so in one muted line, as today.
2. **The same groups in the install and update dialogs**
   (`view/permissions.rs`), so a permission reads the same way
   everywhere; the update dialog still marks what is new.
3. **Results of Reload and Check for updates are notifications**, in the
   app's notification list, as other features send theirs:
   - Reload, same version: "Plugin {name} reloaded";
   - Reload or update, new version: "Plugin {name} updated to {version}";
   - Check for updates with nothing new: "Plugin {name} is up to date";
   - a failure: an error notification "Plugin {name} could not be
     reloaded: {reason}" (or "…could not check for updates: {reason}").
   The page no longer shows these lines. What stays on the page: an
   update that waits for the user's OK with Review, and "a newer version
   needs a newer OTTY".
   Words in all ten locales; drop the keys nothing uses any more.

## Done when

- Tests first: a part card shows its kind, its health and its
  permissions grouped, places without backticks, the handle note once
  and only with logins or keys; Reload with the same version sends
  "Plugin X reloaded", with a new version "Plugin X updated to V", a
  failure an error notification; the page shows none of them; the install
  dialog shows the same groups.
- The checks of `AGENTS.md` pass.

## Comments
