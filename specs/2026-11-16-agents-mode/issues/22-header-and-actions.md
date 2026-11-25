# 22 The dashboard header and actions, AI Profiles, no Hide, no program path

Status: ready-for-agent

## Why

The owner used the first build and asked for a quieter header and fewer
controls (spec Decision 10).

## What to do

1. The left panel section of profiles is called "AI Profiles"; the dialog
   is "Add AI profile" / "Edit AI profile"; the `+` tooltip says "Add AI
   profile". The mode keeps its name. All languages.
2. Dashboard header: the logo, the profile name, then Refresh, then "⋯". The
   status badge is gone. The plan and the account are a muted line next to
   the name; when the row is too narrow the line moves under the name while
   Refresh and "⋯" stay on the name's line, at the right. Adjust or replace
   the kit `HeadingRow` of issue 20 so that the text wraps, not the buttons
   (keep the kit's own tests and gallery in step).
3. "⋯": Edit; Log in while the profile is not logged in; Remove for an added
   profile (Remove still asks first). The Edit and Log in buttons are gone
   from the header. A profile that is not logged in shows its hint in the
   dashboard body with a "Log in" button there too, so the action stays one
   click away.
4. Hide is gone everywhere: the menu item, "Hidden profiles (N)", the
   intents, the `hidden` flag of `Profile`, `DefaultProfileChanges` and
   `agents.json`. An old file with `hidden` loads with the profile shown;
   saving drops the field.
5. The program path is gone: the Edit field, the `program` of `Profile` and
   `agents.json`, and its checks. Programs are found by name through the
   login shell, as today. An old file's `program` is ignored.

## Done when

- Tests, written first, cover: the header at a wide and a narrow width (the
  text moves, the buttons stay), the "⋯" items per profile state, Log in in
  the body, an old `agents.json` with `hidden` and `program` loading, the
  new names.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Built in `otty-agents`, the kit, `otty-app-agents`, `otty-app-agents-io`,
`otty-app-i18n` and the binary's window tests. Names: the left panel
section is "AI Profiles" (the key is now `agents_section_profiles`), the
`+` tooltip and the add dialog say "Add AI profile", the edit dialog "Edit
AI profile" (no longer "Edit {name}"); the mode is still "Agents". All ten
languages.

Header: the logo, the name, the plan and the account muted after the name,
then Refresh and "⋯". The kit `HeadingRow` changed: the actions always end
the first line at their size, the heading gets the room they leave, and a
new optional `note` follows the heading while all three fit, else goes to
a line of its own under it (kit tests and gallery entry updated). The
dashboard puts the logo outside the `HeadingRow`, so the moved line starts
under the name. The badge is gone, with `state`/`tone` of the view model
and its four state words. "⋯" lists Edit, Log in while the profile is not
logged in and its program was found (`AgentsState::can_log_in`, also used
by the dashboard), and Remove, after a line, for an added profile; Edit
and Log in now close the menu. A profile that is not logged in has Log in
under its hint in the body.

Hide is gone: the menu item, "Hidden profiles (N)", the three intents, the
state's hidden list, `Profile::is_hidden`, `DefaultProfileChanges::is_hidden`
and the field in `agents.json`. The program path is gone: the dialog
field, `Profile::program`, `DefaultProfileChanges::program`, the field in
`agents.json`, the "program path does not exist" checks of both agents'
`detect`, and the path branches of the io lookup, `app-server` start and
hidden terminal (their only production input was that path). serde skips
unknown fields, so an old file with `hidden` and `program` loads with every
profile shown and the program looked up by name; saving writes neither.
The dashboard's Profile list keeps "Program", the name the agent runs
(`claude`, `codex`).

Tests seen failing first: the two `saved_profiles` tests (old file, no
fields written), the kit `HeadingRow` tests (against a stub `note`), the
reducer test that Edit and Log in close the menu, the dashboard tests (no
Edit, Log in in the body, Refresh and "⋯" at the end of the name's line,
the plan under the name in the smallest area), the menu view and model
tests. Not seen failing: the io test of an old `agents.json` and the i18n
names test, which were written after `otty-agents` had already changed
(the first could not compile against the old code, the second used keys
that did not exist yet).
