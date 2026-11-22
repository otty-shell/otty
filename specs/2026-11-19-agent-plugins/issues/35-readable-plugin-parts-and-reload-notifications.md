# 35 The parts of a plugin page read easily; Reload says its result in a notification

Status: done
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

Owner, 2026-11-22, later changes (they win over the prototype and over
"What to build" below where they differ):

1. The Source fact on the page says only where the plugin comes from:
   the local folder, or the link of the repository it was installed
   from. No "Folder {folder}, changes only on Reload" and no other
   wording about updates. When Source is the repository, there is no
   separate Repository row with the same link; a Repository row stays
   only for a local plugin whose manifest names one.
2. The kind tag (Agent / Provider) on a part card is compact: small text,
   tight vertical padding. If the kit `tag` has big vertical padding, the
   kit gets a compact size rather than a padding hack in the feature.
3. The Logins group shows only the place (a file or a database), such as
   `~/.local/share/opencode/opencode.db`, without "column value of table
   credential".
4. No handle note at all: the card does not say "OTTY gives logins and
   keys to the plugin only as handles; the plugin never sees their
   values".
5. The "Can access" title with its muted line stays.

Owner, 2026-11-22, after the updated prototype (the rest of the prototype
is approved):

6. For a plugin from a repository, Source is only the repository link: no
   "GitHub releases, checked for updates once a day".
7. In the Logins group a key in an environment variable shows only the
   variable name, such as `OPENAI_API_KEY`. The keyring entry keeps its
   muted "once you allow the keyring".
8. The kind tag is small (16 px high, 11 px text) and centred on the line
   of the part's name. The install and update dialogs use the same tag and
   the same "Can access" title.
9. The muted line under "Can access" is "Only this is open to the part,
   OTTY refuses everything else." New or changed words of this issue use
   a comma, not a semicolon, inside a sentence.
10. In a notification, success or error, the icon and the text are
    centred vertically to each other.

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

- 2026-11-22, issue 35, done. A part on a plugin's page is a card
  (`view/details.rs`): its name, the kit `tag` of its kind (Agent or
  Provider, 16 px high, centred in a 20 px line), its health as a kit
  `StatusBadge` (Working, Paused with "Paused after three failures" as
  its tooltip, Off), Try again while paused, the switch at the right
  end, the last failure with Copy report, then "Can access" with its
  muted line and one row per group (`view/can_access.rs`, shared with the
  install and update dialogs). `model/permissions.rs` turns each
  permission into a place in its group. Logins show only the file, the
  database or the variable. The keyring entry keeps "once you allow the
  keyring". A provider's accounts and the sites of their keys are "From
  agents". A line `plugins.json` keeps that the manifest no longer has
  goes to "Other", without backticks. There is no handle note. The update
  dialog shows all permissions of a changed part, the new ones marked
  "new" in the warn color and the dropped ones "no longer". Source is the
  folder or the repository link, which opens the repository; Repository
  shows only for a local plugin whose manifest names one. Reload, Check
  for updates and an agreed update send `PluginsOutcome::Done` or
  `PluginsOutcome::Failed`, and the binary pushes them as the new
  `NotificationsIntent::PushDone` (a neutral toast with a green check:
  kit `NoticeTone::Success`, `NotificationList::success_row`, icon
  `assets/svg/notice-done.svg`) or as `Push`. The page keeps only the
  waiting update with Review and "needs a newer OTTY". A failed daily
  check also sends its error notification, and the row stays signed
  failed. The tag shows only the kind, not the domain that issue 21 put
  in the title ("Agents: agent"). Twenty words nothing uses left the ten
  locales and 23 came. The public docs (`Plugins.mdx`, `Write a
  Plugin.mdx`) follow.
