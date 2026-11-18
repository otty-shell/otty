# 13 The kit pieces and the agent logos

Status: ready-for-agent

## Why

The approved design (canvas "Agents mode") is built from the OTTY UI kit,
and two pieces it needs are missing from the kit. The agent icons drawn in
issue 05 (monograms "CC" and "Cx") are unclear; the owner wants the real
logos in OTTY's single color style (spec "Rules for the work").

## What to do

1. **A row with an icon.** The Agents rows show the agent's logo before the
   name. Add an optional leading icon to the kit row the left panel uses
   for such rows (`TreeRow`, or `SessionRow` if that fits the layout better;
   say which and why), tinted `text-muted`, `text-strong` on the selected
   row. Keep the wrapping rules of the README: the label keeps its room
   first, the right text gives way first.
2. **The project search field** of the left panel: a small text field with
   a search icon and a clear cross, 28 high, on `surface-raised`, with the
   focus ring (canvas: "Project search and filter"). If an existing kit field
   can do this with a small option, add the option instead of a new
   component.
3. **Agent logos.** Replace `assets/svg/agent-claude-code.svg` and
   `assets/svg/agent-codex.svg` with the Claude and OpenAI logos from Simple
   Icons (CC0; take the files of a pinned release and note the version in a
   comment in the SVG), as single-color SVGs that the app tints like its
   other icons. Keep the robot of `mode-agents.svg` for the mode.
4. Show both in the kit gallery (`cargo run -p otty-ui-kit --example
   gallery`) like the other components, and add tests where the kit tests
   its components.

## Done when

- The gallery shows the row with an icon and the search field in both
  themes; the logos load in the app.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-16: Done. The row with an icon is `TreeRow`: the canvas draws the
Agents rows as tree rows (`.ot-session.ot-tree`: 8px padding, a 14px mark
before the label, the state word on the right), and `TreeRow` already had
an optional icon that the left panel uses. Its icon was always `text-muted`;
now an icon without its own color turns `text-strong` on the selected row.
The wrapping rules did not change; a test checks them with an icon. The
search field is a new kit component, `SearchField` (search icon, text, a
clear cross while there is text; the cross and Esc send the clear message;
28 high on `surface-raised`, the focus ring of the form fields). `TextField`
was not extended: it has a label row, a note, a mono font and is 34 high,
so the search field would have needed four options. The cross is the kit's
tiny icon button (14px with a 10px cross), not the 18px button of the
canvas, so the kit keeps one small size. The logos are `claude.svg` and
`openai.svg` of Simple Icons 13.21.0 (CC0-1.0), same path data, black fill,
with a comment naming the source. The gallery shows both pieces with the
real logos.
