# 42 A disabled menu item says why on hover; no notes inside menus; no semicolons

Status: done
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

4. **Owner, 2026-11-23: the open-menu guard keeps no list of items.**
   `context_menu_guard` (`crates/app/otty/src/guards.rs`) no longer lists
   the intents each menu may send. What comes from the open menu itself
   (its items, a hover or a scroll on it) reaches it unlisted; everything
   else is input outside it, and a press outside closes it. Esc, the
   items of every menu and dialogs over menus work as before. A new item
   of a menu works without a change to the guard.
5. **Owner, 2026-11-23: one press switches menus.** With a menu open, a
   press on the trigger of another menu (on a profile page the period
   drop-down is open and the user presses the Tokens drop-down) closes
   the first and opens the second with that one press, for every menu and
   drop-down of the app that opens from a trigger. Tested through the
   app's input path.
6. **Owner, 2026-11-23, item 2 confirmed:** when money can not be shown,
   the Money item is disabled with its reason as a tooltip: no note in
   the usage block, no heading in the menu.
7. **Owner, 2026-11-23: By account rows as chips.** In the By account tab
   of the usage block (`model/usage_groups.rs` and its view) each row
   shows the provider's icon and the short account name, as the account
   chips do, with the whole name as its tooltip, built by the shared
   pieces of `model/account_chip.rs`.

## Done when

- Tests first: a disabled item with a reason shows the tooltip on hover
  and sends nothing on press; the account menu of a key has Rename… and
  Hide disabled and no note; the Money item, when money can not be
  shown, is disabled with its reason on hover and there is no heading;
  no locale value contains ";" (a test over the locale files).
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-23, done

- The kit `MenuItem::disabled_reason` shows the reason as the kit tooltip
  while the pointer is over a disabled item; an item that can be pressed
  or has no reason shows none. `NoteSelectOption::disabled_reason` does
  the same in the list of a drop-down (the floating list now draws the
  overlays of its rows, such as that tooltip).
- The account menu of a key has Rename… and Hide disabled with "A key can
  not be renamed or hidden yet, OTTY keeps nothing taken from a key." on
  hover and no note. Money of the measure menu and of the list of a
  statistics popup is disabled with its reason on hover; the heading of
  the menu and the note of the block are gone, and so is the key
  `tpl_agents_money_off` that nothing uses now. No other menu of the app
  had such a note: the headings left title groups.
- No locale value holds ";" or "；" (a test over every key and locale).
  The list separator `agents_failure_separator` is " · ", since a reason
  may hold commas. The words of the plugins Anthropic, OpenAI and
  OpenCode (now 0.2.5, 0.2.5 and 0.1.6) and of the template have no
  semicolons either; the template stays 0.1.0, the start version a new
  plugin copies and the tests of the releases build on.
- The guard: the view marks every message of an open menu
  (`AppEvent::FromMenu`, by `screens::overlay_layer` for every overlay of
  the kind Menu and for the header menus), and the input checks let it
  through unlisted; a choice of a header menu closes it and runs. Menus
  have no dismiss layer of their own any more (the kit `context_menu`,
  the All modes menu, the notifications list and the palette): a press
  away reaches what lies under it, the trigger of another menu too. A
  click on the window that sends nothing (the kit `press_away` around the
  window content) is `AppEvent::PressedAway`, which closes the menus.
  `menu_opened_by` names the triggers of the menus: one of another menu
  closes the open menus and opens its own; one of the menu on top goes to
  its feature, which opens its other menu there, or closes the same menu
  (Agents mode and the header menus toggle it); the bell and the palette
  button close their own menu. A press on a pane, in the Launches tree
  and the cross of a session row only close a menu or do nothing, as
  before.
- Known limit: a press on a widget that takes the press and sends nothing
  closes the menu on its release, so a text field outside a menu gets the
  focus as the menu closes.
- By account rows show the provider's logo and the short name with the
  whole name in the tooltip (kit `TableCell::icon_text`), from
  `account_label`, which the account chips use too.
