# 20 New launch under an open menu

Status: needs-triage

## What differs

New launch… in the New session menu and on the start card sends a different
message in each app:

- old app: `QuickLaunchIntent::HeaderCreateCommand`;
- new app: `AppEvent::Command(AppCommand::NewLaunch)`, which quick launch
  turns into `HeaderCreateCommand`.

The input checks treat the two messages differently when a menu is open:

- **New session menu.** In both apps the item closes the header menu and opens
  the launch editor. The old app did it as a header menu choice; the new app
  closes every open menu, as it does for any command. Same result.
- **Start card tile.** If a menu is open and the press reaches the tile, the
  old app dropped the press (`MenuGuard::Ignore`), and the menu stayed open.
  The new app closes the menus and opens the launch editor
  (`MenuGuard::DismissAndRun`). The New terminal tile next to it already
  closed the menus and ran in the old app.

So the new app makes the two start card tiles behave alike. The owner was
asked about it in issues 10 and 12, and nobody has decided yet.

## How to see it

It may not be reachable with the mouse. Every menu of the new app draws a
dismiss layer that fills its layer (the palette, the notification list, the
All modes menu, the header menus, the pane menu and the Launches context
menu), and a press on it only closes the menu. The tile gets the press only
if some menu leaves the start card uncovered. To check:

1. Close every tab, so the start card shows.
2. Open each menu in turn: the Launches context menu (right-click in the
   Launches section), the All modes menu, the notification list, the palette,
   the New session menu.
3. With the menu open, press New launch… on the start card.

If every press only closes the menu in both apps, the difference is in the
code only, and this issue can be closed. If a press reaches the tile: the old
app does nothing, the new app closes the menu and opens a "Create Quick
Launch" tab. Pressing New terminal there closes the menu and opens a
terminal in both apps.

## Where

- Old: `app/src/screens/terminal.rs:142` and `:399` (the tile and the menu
  item send `HeaderCreateCommand`), `app/src/guards.rs:162-171` (quick launch
  intents other than the context menu's own are `Ignore` under a menu),
  `app/src/update.rs:113-131`.
- New: `crates/app/otty/src/screens/terminal.rs:267` and `:298`
  (`AppCommand::NewLaunch`), `crates/app/otty/src/guards.rs:221`
  (`AppEvent::Command(_) => DismissAndRun`),
  `crates/app/otty/src/dispatch/command.rs:73-75`.

## To decide

- Keep the new behaviour (both tiles close the menus and run), or
- send `QuickLaunchIntent::HeaderCreateCommand` from the tile again so a press
  under a menu is dropped as before.

## Comments
