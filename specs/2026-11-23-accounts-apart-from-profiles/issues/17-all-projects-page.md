# 17 All projects: a page with a table of every project found

Status: ready
Blocked by: 16

## Why

Owner, 2026-11-24: the Projects section of Agents mode needs an "All
projects" screen with a table of all the projects found, as Accounts and
Profiles have All accounts and All profiles
(`crates/app/features/agents/core/src/model/overview.rs`).

## What to build

1. **Entry.** The Projects section of the left panel gets an "All
   projects" row or header action, the same way the Accounts and
   Profiles sections open All accounts and All profiles (same look,
   same intent shape, `AgentsScreen::AllProjects`, a `TableScreen` per
   issue 38/39 of spec 2026-11-19-agent-plugins).
2. **Table.** One row per project found (the same projects the Projects
   section lists, hidden ones too with a "hidden" mark when "show
   hidden" is on, as All profiles does), columns:
   - **Project**: its name (folder name), the full path in the tooltip;
   - **Profiles**: the profiles whose sessions ran in it, as names in
     parentheses form of issue 16 ("Codex (Work)"), shortened with "+N"
     when many;
   - **Sessions**: how many;
   - **Last active**: when its latest session was updated, as the
     sessions table writes times;
   - **Today**: tokens and money of today, as All profiles' Today column
     writes them (comma between, tooltips by kind from issue 16).
   Sorted by Last active, newest first; the column headers sort as other
   tables of Agents mode do. A press on a row opens that project's page.
   The table fits the smallest screen (`size::SCREEN_MIN`), check column
   minimums, and scrolls down on its own under a sticky header.
3. **Empty.** With no project found, a short line says so, as the other
   All pages do.
4. Words in all ten locales (commas, no semicolons, no middle dots).
   Docs (`docs/public/Agents/Overview.mdx`) follow.

## Done when

- Tests first: the Projects section opens All projects; the table lists
  every project with its profiles, sessions, last active and today's
  tokens and money, newest first; a row opens its project; hidden
  projects show only with "show hidden"; the table fits the smallest
  screen; no projects shows the empty line.
- The checks of `AGENTS.md` pass.

## Comments
