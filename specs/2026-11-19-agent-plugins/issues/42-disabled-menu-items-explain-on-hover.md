# 42 A disabled menu item says why on hover; no notes inside menus; no semicolons

Status: ready
Blocked by: 40

## Why

Owner, 2026-11-23: the context menu of an account shows an extra note
under a line, such as "A key can not be renamed or hidden yet: OTTY keeps
nothing taken from a key." (`Key::AgentsKeyNotSaved`,
`crates/app/features/agents/core/src/view/menus.rs`). The owner does not
want such notes in the menu: leave the items that can not be used
disabled, and show why only as a tooltip when the pointer is over the
disabled item.

The owner also asked (2026-11-22) for commas, not semicolons, inside
sentences of the UI words.

## What to build

1. **Disabled items explain on hover.** The kit menu
   (`crates/ui/kit/src/components/menu.rs`, `context_menu.rs`) lets a
   disabled `MenuItem` carry a reason, shown as the kit tooltip while the
   pointer is over it (the same tooltip look as elsewhere). A disabled
   item with no reason shows no tooltip. A press on a disabled item still
   does nothing and keeps the menu open, as today.
2. **No explanation notes inside menus.** Every menu of Agents mode that
   adds a note under a separator to say why an item is off uses the
   item's reason instead:
   - the account menu: Rename… and Hide disabled for a key, reason "A key
     can not be renamed or hidden yet, OTTY keeps nothing taken from a
     key.";
   - the Money / Tokens menu: Money disabled with its reason (today a
     heading with `money_reason` under a separator, `view/menus.rs`
     around the measure menu);
   - any other such note found in the app's menus (grep `.heading(` next
     to `.separator()` in menu views); headings that title a group of
     items stay.
3. **No semicolons in UI words.** In all ten locales
   (`crates/app/i18n/src/locales/*.json`), sentences use commas or a new
   sentence instead of semicolons (today about ten keys in English, such
   as `agents_rename_account_text`, `containers_built_in_network_hint`,
   `tpl_agents_found_intro`); a list separator such as
   `agents_failure_separator` becomes ", " (or " · " if items may hold
   commas, decide by its use). Plugins' own words (`plugins/*/src`) get
   the same, with a patch version bump and a rebuild of each changed
   plugin.

## Done when

- Tests first: a disabled item with a reason shows the tooltip on hover
  and sends nothing on press; the account menu of a key has Rename… and
  Hide disabled and no note; the Money item, when money can not be
  shown, is disabled with its reason on hover and there is no heading;
  no locale value contains ";" (a test over the locale files).
- The checks of `AGENTS.md` pass.

## Comments
