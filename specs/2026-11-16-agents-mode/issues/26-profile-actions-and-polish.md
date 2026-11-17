# 26 Profile actions that work, Profiles, right click, header alignment

Status: ready-for-agent

## Why

The owner tried the build of issue 25 (spec Decision 11):

- "Edit" in the "⋯" menu of a profile does nothing, and "Remove" does
  nothing either. Cause (read in the code on 2026-11-17): while a menu of
  Agents mode is open, the frame lets through only the intents listed in
  `crates/app/otty/src/guards.rs` (the arm with the comment "The items and
  the dismiss layer of the menus of Agents mode"): `ProjectFilter`,
  `NewSessionWith`, `SessionMenu` and `CloseMenu`. The items of the profile
  menu added in issues 16 and 22 send other intents, so the guard drops them.
- The header: the muted "plan · account" line is not centred vertically
  against the logo and the name.

## What to do

1. Let the profile menu's items through the guard (Edit, Log in, Remove, and
   whatever intents they send). Add one guard test per menu of Agents mode
   that presses every item of that menu and checks the intent arrives, so a
   new item can not be forgotten again.
2. "AI Profiles" becomes "Profiles"; "Add AI profile" / "Edit AI profile"
   become "Add profile" / "Edit profile"; all languages.
3. The dialog's config folder ways are "Type a path" and "New folder"; the
   "Found on disk" way, its candidate list and the code that only served it
   go (`candidate_config_dirs` and its use, if nothing else needs them).
4. A right click on a profile row in the left panel opens the same profile
   menu as the "⋯" of its dashboard, at the pointer (as the session rows of
   issue 15 do).
5. "Tokens, last 30 days" loses its "Counted from the session files" note.
6. The header: the logo, the name and the plan · account line share one
   vertical centre on one line; when the line moves under the name it stays
   left-aligned with the name. Fix it in the kit `HeadingRow` if the cause is
   there, with its test.

## Done when

- Tests, written first, cover: each menu item of each Agents menu passes the
  guard; the right click opens the profile menu; the dialog has two ways;
  the names; the header alignment (the note's centre equals the name's
  centre within a pixel on one line).
- Run by hand: Edit and Remove from "⋯" and from the right click work.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments
