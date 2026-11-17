# 34 Project screen fixes; Codex plan names as Codex shows them

Status: ready-for-agent
Blocked by: 33

## Why

The owner's fifth round of changes (spec Decision 14, 2026-11-17):

- The sessions table of a project has a Branch column the owner does not
  need.
- The menu of a session has "Open in terminal", which opens the project
  folder, not the session; the owner finds it useless there (the project
  title row already has that button). The menu should hold only what acts
  on the session.
- The folder path in the project title row is not centred vertically
  against the folder name, and it is the full path; the owner wants it
  short, as a terminal prompt writes it.
- The New session split button: when it is off (no profile can start, or
  the folder is gone), the plus is faded but the arrow next to it is not.
  The kit `SplitButton` draws its arrow at full strength and lights it
  under the pointer even without a message (the same iced issue fixed for
  icons in issue 32: the colour's alpha is ignored for an svg; use the
  svg opacity the kit uses since issue 32).
- Codex plan names: the owner chose the names Codex itself shows in
  `/status` (issue 33 had the code written as words).

## What to do

1. **No Branch column** in the sessions table of a project. Remove the
   column, its text keys if unused, and the branch tag of the search if it
   becomes useless (say what you did). Keep the branch in the data only if
   something else uses it.
2. **The session menu** lists only actions on the session: Continue,
   Continue in split, Copy resume command, Copy session ID. Remove "Open in
   terminal" from it (`SessionAction::OpenFolder` if nothing else uses it).
3. **The path in the title row:**
   - It sits on the same vertical centre as the folder name (or on its
     baseline, whichever the design system's heading rows use; check
     `HeadingRow` and the dashboard header).
   - It is written short, as a terminal prompt does: the home folder is
     `~`; when more than three folders follow `~` (or `/` outside the
     home), only the last two stay, after `…`: `/home/user/projects/main/ottyrun/otty` is `~/…/ottyrun/otty`,
     `/home/user/otty` is `~/otty`, `/home/user/work/otty` is
     `~/work/otty`, `/srv/data/a/b/c` is `/…/b/c`. Put the rule in one pure
     function with tests; reuse an existing one if OTTY already has it
     (look at the dashboard's `home_relative` and the terminal and status
     bar code).
   - The full path shows in a tooltip on the short one.
4. **The split button's arrow when off:** in the kit, the arrow of a
   `SplitButton` with no menu message is drawn at the disabled opacity
   (45 %, `DISABLED_OPACITY` in `components/mod.rs`), does not light up
   under the pointer, shows no hover background and its tooltip still
   shows. Check every other kit control that draws an svg for the same
   fault and list what you find; fix only the split button unless the fix
   is the same one line.
5. **Codex plan names** in `plan_name` of
   `crates/agents/src/codex/rate_limits.rs`, as Codex's `/status` shows
   them (`SubscriptionDisplay::label` with `Status` in
   `codex-rs/tui/src/subscription.rs`; the codes are the serde names of
   `PlanType` in `codex-rs/protocol/src/account.rs`, read on 2026-11-17):

   | Code | Name |
   |---|---|
   | `free` | Free |
   | `go` | Go |
   | `plus` | Plus |
   | `pro` | Pro 200 |
   | `prolite` | Pro 100 |
   | `promax` | Pro 500 |
   | `team` | Business |
   | `self_serve_business_usage_based` | Business |
   | `business` | Enterprise |
   | `self_serve_business_prolite` | Business Premium |
   | `enterprise_cbp_automation` | Enterprise (Automation) |
   | `enterprise`, `ent26`, `enterprise_cbp_usage_based` | Enterprise |
   | `edu` | Edu |
   | `edu_plus` | Edu Plus |
   | `edu_pro` | Edu Pro |
   | `sci` | Academic Researchers |
   | `sci_plus` | Academic Researchers Plus |

   A code not in the table stays the code in words, as today. The doc
   comment names where the table comes from and the date.

## Done when

- Tests, written first and seen failing, cover: no Branch column; the
  session menu items; the short path rule (home, short, long, outside the
  home, the home itself, a trailing `/`); the full path in the tooltip; the
  path and the name on one centre line; the split button's arrow faded and
  not lit when off (render it like issue 32's test did, comparing pixels,
  since a colour check missed the fault there); every Codex plan code of
  the table.
- Run by hand: open a project; no Branch column; right click a session:
  no Open in terminal; the path is short and centred, its tooltip is the
  full path; with no profile able to start, the plus and the arrow are both
  faded.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

### 2026-11-17

Built in `otty-agents`, `otty-app-domain`, `otty-app-agents`, the
terminal feature, `otty-app-i18n`, the kit and one binary test, tests
first.

1. **No Branch column.** The table has Title, Agent, Profile, Updated;
   `UPDATED_COLUMN` is now 3. The search lost its branch key (it would
   find nothing to filter). Nothing else used the branch, so it left the
   data too: `AgentSession::branch` and `with_branch`, the `gitBranch` of
   Claude Code records and the `git.branch` of the Codex `session_meta`
   are no longer read. The keys `AgentsColumnBranch` and
   `AgentsOpenInTerminal` are gone from all ten languages. Without the
   column the table fits even the smallest main area, so its width test
   now checks that it fits there and scrolls sideways only below it.
   Spec Decision 4 still lists "git branch when known" in the shared
   session data; it is out of date now.
2. **Session menu:** Continue, Continue in split, Copy resume command,
   Copy session ID. `SessionAction::OpenFolder` and the menu's
   `can_open_folder` are removed; `AgentsOutcome::OpenFolder` stays for
   the title row button.
3. **Path in the title row.** The terminal pane header already had the
   rule (`short_folder_path` in the terminal feature). A feature may not
   use another feature, so the function moved, unchanged, to
   `otty-app-domain` (`folder_path::short_folder_path`), and both the
   terminal and Agents use it with the home folder of the agents'
   environment. The view model has `path` (short) and `full_path`. The
   whole path shows in a tooltip: the kit's bubble was private, so the kit
   got a small public `components::tooltip::tooltip(content, label,
   palette)` that puts it under any content. The path is now as wide as
   its text (`shrink(0.0)`), so the tooltip shows only over the text; it
   still gives way first in a narrow window.
   Centre or baseline: the boxes of the name and the path already shared
   one centre, and a picture drawn with the kit's fonts showed the path
   about 3.4 px above the name's baseline, the same look the owner
   corrected in issue 31 for the plan next to the name ("looked higher
   than the name, as the two texts shared one middle"). `HeadingRow` now
   only centres (its baseline note left with the plan in issue 33). So
   the path sits on the name's baseline: the kit's
   `fonts::ui_baseline_below_middle` came back and got
   `fonts::mono_baseline_below_middle` (JetBrains Mono: 1020 and -300 of
   1000), and the path gets a padding above of twice the gap between the
   two baselines, which moves its centred box down by the gap. The test
   checks the baseline, not the centre, as the "Done when" asked.
4. **Split button arrow.** With no menu message the chevron keeps
   `text-muted`, fades through the svg opacity (`icon_opacity`), does not
   light up and, being a disabled iced button, has no hover background;
   its tooltip still shows.
   Other kit controls that draw an svg, checked for the same fault (none
   use a faded colour on an svg any more since issue 32; these draw an
   svg at full strength when they get no message):
   - `ModeSwitcher` tab (`ModeTab` with no `on_press`): icon full strength
     and lit under the pointer through `IconHover`, the same fault as the
     arrow. The app always gives a message.
   - `ActionTile`, `TitleSearch` and `NoteSelect` (no `on_toggle`): with
     no message the icon or caret and the text stay full strength (no
     hover, iced marks the button disabled). They have no disabled look
     at all, not only the svg; the app always gives a message.
   - `PanelSection` (no `on_toggle`) and `TreeRow` (no `on_press`): no
     message means "does not fold" or "a row", not "disabled"; fine.
   - The row "⋯" of `DataTable`, the toolbar's "⋯" (`OverflowMenu`) and
     the cross of `Notice` are always active; fine.
   None of these fixes is the same one line, so only the split button
   changed.
5. **Codex plan names** follow the table of this issue; the doc comment
   of `plan_name` names `SubscriptionDisplay::label` with `Status`,
   `PlanType` and the date. Old expectations in Codex tests changed with
   it: `prolite` is "Pro 100", `business` is "Enterprise".

Tests seen failing first: the short path rule (6 of 7; the neighbour of
the home passed against the stub), the Codex names, the columns, the
rows without a branch cell, the short and full path of the view model,
the search keys, Updated as the last column, the session menu items, the
tooltip over the path (pixels, the whole path changes the picture only
under the pointer), the path on the name's baseline, the arrow at 45 %
and not lit (pixels against a disabled plus and a bare arrow at 45 %,
both themes), the kit tooltip (pixels) and the two font baselines. Not
seen failing: the weights of each font sharing a baseline (passed
against the stubs) and the binary's guard test (one item fewer).

Checks: fmt, clippy, deny and one workspace test run (5657 passed) pass.
`otty-app-testing` became a dev dependency of `otty-app-agents` (for
`TempDir`); it is an app crate, no new outside dependency. Coverage was
not measured.
