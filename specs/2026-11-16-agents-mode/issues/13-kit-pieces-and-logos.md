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
