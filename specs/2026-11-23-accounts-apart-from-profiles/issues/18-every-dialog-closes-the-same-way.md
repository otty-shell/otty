# 18 Every dialog closes the same way

Status: ready
Blocked by: 17

## Why

Owner, 2026-11-25: every popup must look and work the same, and the
statistics popups can not be closed by a press on the dimmed place around
them. "I thought it was one reusable component."

It is one component, the kit `Dialog`
(`crates/ui/kit/src/components/dialog.rs`), but closing on the scrim is
opt-in there (`on_backdrop_press`), and only the Settings dialog asks for
it; the other twenty or so dialogs of the app do not. So the same
component behaves differently per dialog.

## What to build

1. **One rule in the component.** A dialog that can be closed (it has
   `on_close`, the cross of its title) closes the same way everywhere:
   the cross, Esc, and a press on the scrim all send its close message.
   The kit `Dialog` does it by itself from `on_close`; `on_backdrop_press`
   goes (or stays only for a dialog that must not close on the scrim,
   if any is found, and then says why in its doc). A press on the card,
   or one that starts on the card and ends on the scrim, still sends
   nothing, as the kit tests pin today.
2. **Every dialog of the app.** Check each user of `Dialog::new` (Agents:
   stats, rename, remove, key, keyring, profile, new session, add
   account, found report; Plugins: forms, can access, install/update;
   Containers: create form, confirm; Quick Launch secret prompt;
   Explorer delete; Settings) has `on_close` with the message its Cancel
   or Close sends, so the cross, Esc and the scrim behave alike. Remove
   the per-dialog `on_backdrop_press` calls. If a dialog today ignores
   Esc or has no cross, give it the same close.
3. **Through the app.** The app's guards (`crates/app/otty/src/guards.rs`)
   let the scrim press of any dialog through, test it through the app's
   input path for at least the statistics popup and one Plugins dialog.

## Done when

- Tests first: a press on the scrim of a dialog with `on_close` sends its
  close message (kit); the statistics popup of a session closes on a scrim
  press through the app's input path; every dialog of the app closes on
  the scrim, by its cross and by Esc (one test per feature at least);
  a press on the card still closes nothing.
- The checks of `AGENTS.md` pass.

## Comments
