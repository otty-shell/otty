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
