# 14 The left panel: Agents and Projects sections, search and filter

Status: ready-for-agent
Blocked by: 11, 13

## Why

The owner wants the left panel of Agents mode to work like Terminal mode's:
two sections that open and close, and a way to find a project among many
(spec Decision 6, canvas screens "Part: left panel" and "Project search and
filter").

## What to do

1. Two `PanelSection`s under the mode switcher, Agents and Projects, that
   open and close and share the panel height with `share_panel_height`,
   exactly as Sessions and Launches do in Terminal mode (look at
   `crates/app/otty/src/frame/sidebar/view/left_panel.rs` and the tabs and
   quick launch sections). Their open state is kept like theirs.
2. **Agents**: the row with the agent's logo (issue 13), the profile name
   and the short number of spec Decision 6 and 9 (`42 %`, `3 760 left`,
   `$12.40 today`), or the state that needs the user ("needs login",
   "program not found") in the error tone. Header: Refresh and `+` (opens
   the Add profile dialog). "Hidden profiles (N)" stays at the end.
   Selecting a row opens the profile dashboard.
3. **Projects**: folder name and session count; "folder gone" in the error
   tone. Header: the search button and the "New session" `SplitButton` (the
   plus starts a session in the selected project with the only profile, or
   asks for the profile; the chevron opens the list of profiles).
4. **Search and filter**, as spec Decision 6 says: the field opens under the
   header, filters at once by folder name or any part of the path, case
   does not matter, with the "N of M projects match" note; Esc and the cross
   clear, Esc on an empty field closes. The funnel menu: agent (all or one),
   last 30 days, folder gone, sort by name, clear all. Each filter that is
   on is a chip with a cross; the funnel is accent while any is on. The
   state lives while OTTY runs, not on disk. The filtering and sorting is a
   pure function of the core crate, tested on its own.
5. Remove what this replaces from the issue 05 and 06 drawing.

## Done when

- Tests, written first, cover the filter function (name, part of the path,
  case, each menu filter, sort, clear) and the reducer (open and close the
  field, Esc twice, chips, selecting a row).
- Run by hand: the steps the agent writes in its report.
- The checks of `AGENTS.md` pass.

## Comments
