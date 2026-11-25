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

### 2026-11-17

Cause confirmed: while a menu of Agents mode is open, `context_menu_guard`
in `crates/app/otty/src/guards.rs` let through only `ProjectFilter`,
`NewSessionWith`, `SessionMenu` and `CloseMenu`. Edit, Log in and Remove of
the profile menu send `EditProfile`, `LogIn` and `RemoveProfile`; they fell
to the `_ => Dismiss` arm, so the frame closed the menu and dropped the
press. A test that sends Edit and Remove through `update_input` failed
before the fix ("Edit opens the profile dialog") and passes now. The three
intents are in the arm. One guard test per menu (funnel, New session,
session, profile) draws only the menu layer of the app, presses every item
and checks that the press is an item and that the guard lets it through.

Names: "Profiles", "Add profile", "Edit profile" in all ten languages. The
dialog's ways are "Type a path" (the old "Path") and "New folder". The
"Found on disk" way is gone with what only it used: `FolderChoice::Found`,
`FoundFolderPicked`, the found list and pick of the dialog,
`CandidatesLoaded` / `LoadCandidates`, `load_candidates` of the io crate,
`candidate_config_dirs` and `Agent::looks_like_config_dir` of
`otty-agents`, the texts `agents_folder_found` and `agents_tokens_source`,
and their tests. Opening the add dialog and picking another agent now read
nothing. A right click on a profile row in the left panel sends
`OpenProfileMenu`, so the same menu opens at the pointer; a right click on
another row while a menu is open only closes it, as for the session rows.
"Tokens, last 30 days" has no note; the note parameter of `blocks::title`,
used by nobody else, went with it.

Header: measured first. On one line the boxes of the name and the plan
already shared one centre (267.025 both), so the one-pixel test of the
dashboard passed before any change. A rendered picture of the header
showed the real fault in less room: the logo sat outside `HeadingRow`, in
a row centred on the whole block, so with the plan under the name the logo
was centred between the two lines, not by the name. The kit `HeadingRow`
got `lead`: a part before the heading on the first line's centre, with the
moved note starting where the heading starts; the dashboard passes the
logo as the lead. Kit tests and the gallery entry cover it.

Tests seen failing first: the five agents-crate tests (right click, two
ways, the dialog reads nothing on open and on agent pick, no tokens note),
the i18n names test, the profile-menu guard test and the Edit/Remove test.
Not seen failing: the dashboard one-pixel check (already true, see above);
the other three menu guard tests (those menus already passed); the kit
`lead` tests, which could only fail to compile before `lead` existed.
