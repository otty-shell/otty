# Agents mode

Status: decisions agreed with the owner on 2026-11-16

## Problem

Coding agents such as Claude Code and Codex run in OTTY terminal tabs, but
OTTY knows nothing about them. To continue yesterday's work the user has to
remember which agent they used, in which folder, open a tab, go to the
folder and pick the session in the agent's own picker. To see how much of a
limit is left they have to ask each agent separately.

## Goal

A new mode, Agents, next to Terminal and Containers. In it the user:

1. Sees the agents they use. An agent that is already installed and logged
   in shows up as connected with no setup. The user can add more, such as a
   second Claude Code account that keeps its data in another folder.
2. Sees, for each of them, what it costs in the way that account is paid
   for: how much of the usage windows of a subscription is used, how many
   credits are left, or how much money an API key spent; and how many tokens
   were spent.
3. Sees the projects the agents worked in, finds one by name, sees the
   sessions of a project in a table, continues a session or starts a new
   one. Both open a terminal tab with the agent running in the project
   folder.

The screens are drawn in the design canvas "Agents mode"
(https://claude.ai/artifact/SAcZcaMjkaE738p3GfEn5w, approved by the owner on
2026-11-16), built from the OTTY design system. Where this spec and the
canvas disagree on a detail of the look, the canvas wins; on behaviour, this
spec wins.

The first version covers Claude Code and Codex. Adding the next agent must
be: implement the traits, add the description and the icon, add one line to
the list of agents.

## Words

- **Agent**: a coding agent program, such as Claude Code or Codex.
- **Profile**: one agent together with one config folder and a name. The
  config folder is where the agent keeps its login, settings and sessions
  (`~/.claude`, `~/.codex`). Two profiles of the same agent are two
  accounts: "Claude Code" in `~/.claude` and "Claude Code · Semrush" in
  `~/.claude-semrush`.
- **Project**: a folder at least one session ran in.
- **Session**: one conversation with an agent, which the agent can continue.
- **Source**: one way to get a piece of data, such as "ask the agent's
  server" or "read the agent's files". Sources of the same data are tried in
  order (Decision 5).
- **Billing**: how the account of a profile is paid for: a subscription
  with usage windows, a pool of credits, or an API key that is charged per
  token (Decision 9).

## Decisions

### 1. Profiles, not agents, are what the user connects

Everything the mode shows belongs to a profile: its sessions, its limits,
its account.

- Each known agent has a **default profile** in its default config folder:
  `CLAUDE_CONFIG_DIR` or `~/.claude` for Claude Code, `CODEX_HOME` or
  `~/.codex` for Codex. It appears by itself when the folder exists or the
  program is found.
- The user can **add a profile**: pick the agent from the list of known
  agents, give it a name, and then either:
  - point at a config folder that already exists. The dialog offers the
    folders in the home folder that look like a config folder of that agent
    (for Claude Code: a folder with `projects/` and `.claude.json` inside);
  - or let OTTY **create a new one**. The dialog proposes a path from the
    name, such as `~/.claude-semrush` for "Semrush", and OTTY creates the
    folder. The new profile is "not logged in"; its "Log in" button opens a
    terminal tab where the agent runs its own login with this folder.
- The user can **edit** a profile, the default one too: its name, its config
  folder (for a setup where the default folder is somewhere else), and the
  path of the program when it is not found by name.
- The user can **hide** a profile they do not want to see, and remove one
  they added.

Profiles are saved in `agents.json` in OTTY's config folder, next to
`modes.json`. OTTY never keeps a secret in this file: it only knows where
the config folder is.

A profile is in one of these states: connected, not logged in, program not
found, config folder not found.

### 2. Projects are shared by all profiles

A project is a folder. The sessions of every profile that ran in this
folder are listed together, each with the icon of its agent and the name of
its profile. The `otty` project appears once, with Claude Code and Codex
sessions inside.

Projects are sorted by their latest session. A project whose folder no
longer exists stays in the list, marked as missing; its sessions can not be
opened.

### 3. Opening a session opens a terminal tab

"Continue" and "New session" switch to Terminal mode and open a command tab
(the same `TabsIntent::OpenCommandTab` Containers mode uses for a shell in a
container) with:

| | Claude Code | Codex |
| --- | --- | --- |
| New session | `claude` | `codex` |
| Continue | `claude --resume <id>` | `codex resume <id>` |
| Folder | the project folder | the project folder |
| Config folder of an added profile | `CLAUDE_CONFIG_DIR=<folder>` | `CODEX_HOME=<folder>` |

The variable is set whenever the profile's folder is not the agent's own
default folder (an added profile, or a default profile the user moved). A
default profile in its default folder sets no variable, so what the user
has in their own environment keeps working (issue 04).

The program runs through the user's login shell, because a program
installed by a version manager (Codex under `nvm`) is on `PATH` only there.

This part needs only the program and the folder. It must keep working when
limits or session lists can not be read.

### 4. One trait for an agent, small traits for its sources

All of this lives in a new library crate, `otty-agents`, which knows nothing
about the window.

```rust
/// A coding agent OTTY knows.
pub trait Agent {
    /// Id, name, icon.
    fn descriptor(&self) -> &AgentDescriptor;
    /// Config folder of the default profile.
    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf;
    /// Whether the profile is installed and logged in.
    fn detect(&self, profile: &Profile) -> ProfileStatus;
    /// Program, arguments and variables that start or continue a session.
    fn launch(&self, profile: &Profile, session: Option<&SessionId>) -> AgentCommand;
    /// Program, arguments and variables that log the profile in.
    fn login(&self, profile: &Profile) -> AgentCommand;
    /// Ways to list sessions, best first. Empty: not supported.
    fn session_sources(&self) -> Vec<Box<dyn SessionSource>>;
    /// Ways to read the limits, best first. Empty: not supported.
    fn limit_sources(&self) -> Vec<Box<dyn LimitSource>>;
    /// Counts the tokens spent from local files. `None`: not supported.
    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>>;
}
```

The traits are justified by the abstraction rules: there are two real
agents from the first day and several real sources for each, and each source
crosses an infrastructure boundary (files, a child process, the network).

What an agent can not do, it does not return, and the mode does not draw
that block for its profiles. No agent has to pretend.

The data all agents share:

- `AgentSession`: id, project folder, title, time of the last change, git
  branch when known.
- `Limits`: the plan name and the billing of Decision 9 with its numbers.
  Every number may be missing; the screen shows only what the source gave.
- `Spend`: tokens in, cached and out, today, for the last 7 and 30 days,
  per day for the last 30 days, per project and per model; and money where
  the billing is an API key (Decision 9).

### 5. Sources are tried in order, and a broken one never breaks the mode

The agents change their file formats and their servers without notice.
So every kind of data has several sources, the way CodexBar does it, and
one common function walks them:

- The sources are tried best first. The first one that answers wins.
- Any failure moves on to the next source: the source is not available, the
  login is missing or expired, the network is down, the answer is not what
  the code expects.
- When the answer came from a source that is not the first one, the profile
  shows which one, and why the earlier ones failed.
- When no source answers, the last good answer stays on screen with its
  time, greyed out. If there never was one, the block says "not available"
  with the reasons.
- Reading is tolerant: an unknown line or field is skipped, one unreadable
  session file never hides the others.
- A source never changes the agent's data. It reads files, asks a server,
  or starts the agent's program only for a request that is known not to
  write.

Sources of the first version:

| Data | Claude Code | Codex |
| --- | --- | --- |
| Sessions | 1. files in `<config>/projects/` | 1. files in `<config>/sessions/` and `<config>/archived_sessions/`, plain and compressed, with the names from `session_index.jsonl` |
| Limits | 1. `GET api.anthropic.com/api/oauth/usage` with the token from `<config>/.credentials.json`<br>2. `claude` in a hidden terminal, `/usage`, read from the screen | 1. `GET chatgpt.com/backend-api/wham/usage` with the token from `<config>/auth.json`<br>2. `codex app-server`, `account/rateLimits/read`<br>3. the last `rate_limits` record in the newest session file |
| Tokens spent | counted from the session files | counted from the session files |
| Money (API key billing) | 1. Anthropic Admin API cost report, with an admin key (Decision 9)<br>2. tokens from the session files at list prices | tokens from the session files at list prices |

Notes:

- Sessions have one source for each agent: the session files. The tolerant
  reading is their protection: a file with no readable folder falls back to
  what its path tells, a session with no title shows its first prompt or
  its id.
- The simple request comes first, the heavier ways after it. For limits
  this is the HTTPS request.
- OTTY never asks `codex app-server` for the list of sessions
  (`thread/list`). That request is not read-only: Codex repairs its own
  index of sessions while it answers. On the owner's machine the command
  line `codex` 0.130.0 did not know that older session files are compressed
  (`.jsonl.zst`, 2038 of 2847 files), took their index records for stale
  and deleted 1622 of 2820. The request for limits does not touch that
  index, so `app-server` stays as the second source of limits.
- Codex itself lists sessions from that index (`state_5.sqlite`) and unpacks
  a file only when the session is opened. OTTY does not read the index: its
  layout is private to Codex and changes with its version. OTTY reads the
  first line of each session file, which holds the id and the folder, and
  remembers the result for a file that did not change.
- Claude Code does not compress sessions; it deletes the ones older than
  its `cleanupPeriodDays` setting.
- Only the sessions the user started are listed. The ones an agent starts
  for its helper agents are hidden.
- OTTY only reads a login token. It never writes or refreshes it. An expired
  token is a failure of that source, and the profile says "open the agent to
  log in again" when nothing else answered.
- The second Claude source is slow (it starts the program) and depends on
  the text of the screen. It runs with the profile's config folder, in a
  folder of its own, and removes the session files it leaves behind.

### 6. Where things are on screen

The canvas "Agents mode" draws every screen below. Everything is built from
`otty-ui-kit` components; a piece the kit lacks is added to the kit, not
drawn by hand in the feature.

**Left panel**, under the mode switcher: two `PanelSection`s that open and
close and share the panel height, the same way Sessions and Launches do in
Terminal mode.

- **Agents**: one row per profile: the agent's logo, the profile name, and
  one short number on the right that depends on the billing (Decision 9):
  the most used window of a subscription (`42 %`), the credits left
  (`3 760 left`), the money spent today (`$12.40 today`). A state that needs
  the user wins over the number: "needs login", "program not found". The
  header has Refresh and a `+` that opens the Add profile dialog. Hidden
  profiles sit under "Hidden profiles (N)" at the end.
- **Projects**: one row per project: folder name and the number of
  sessions; "folder gone" in the error tone for a missing folder. The
  header has a search button and a `SplitButton` "New session" (the plus
  starts a session in the selected project, the chevron chooses the agent).
- **Search and filter of projects.** The search button opens a field under
  the header. Typing filters the list at once by the folder name or any part
  of its path, case does not matter; a note under the field says "2 of 7
  projects match". Esc or the cross clears the field; Esc on an empty field
  closes it. The funnel next to the field opens a menu: the agent (all, or
  one), "Projects used in the last 30 days", "Projects whose folder is
  gone", "Sort by name" (else by the latest session), "Clear all filters".
  Each filter that is on shows as a chip with a cross under the field, and
  the funnel is in the accent color while any is on. The search text and the
  filters live while OTTY runs, not on disk.

**Main area** for a selected project (the screen "Project: sessions table"):

- A title row: the folder name, its path in mono and muted, then two icon
  buttons on the right: "Open the folder in a terminal tab" (a terminal
  tab with the user's shell in that folder) and the "New session"
  `SplitButton`.
- A toolbar with a `TagSearchField` over the table (keys Title, Agent,
  Profile, Branch) and Refresh.
- A `DataTable` like the ones of Containers mode: Title, Agent, Profile,
  Branch, Messages, Updated; sorted by Updated, newest first, by default;
  every column sorts. Enter or a double click continues the selected
  session. The "⋯" of a row and a right click open a menu: Continue,
  Continue in a split, Copy resume command, Copy session ID, Open folder in
  terminal.

**Main area** for a selected profile: its dashboard ("Agent dashboard"
screens):

- A header: the logo, the profile name, a status badge, the plan and the
  account, then Refresh, "Edit" and "⋯" (Hide, and Remove for an added
  profile). "Log in" shows in the header instead of the badge's place when
  the profile is not logged in.
- On the left: the billing block of Decision 9, then "Tokens, last 30
  days": today, 7 days and 30 days, and a bar per day.
- On the right: "Profile" (account, plan, config folder, program, sessions,
  last used), then "Top projects, 30 days" for a subscription or credits, or
  "By model, this month" for an API key, then "Where the numbers come from":
  each source in order with its state (answered, not needed, login expired,
  no admin key, failed).
- A block that a profile's agent does not support is left out, not shown
  empty.

**Dialogs**: Add profile and Edit profile are one dialog (the screen "Add
agent profile"): the agent as a `Select` (a drop-down list, not tabs: the
list of known agents grows with every provider), the name, the way to give
the config folder as a segmented choice (Found on disk, Type a path, New
folder) and the folder field with a hint of what happens next. Edit adds the
program path and, for an API key, the budget and the admin key file
(Decision 9).

**Status bar**: on the left the counts of the open project or the state and
folder of the open profile; on the right the same items as in Containers
mode.

The right panel is not used; its title bar button is disabled in this mode.

The lists are read when the mode opens and on "Refresh". Limits are read
again every five minutes while the mode is on screen.

### 7. Crates

Same split as Containers mode:

- `crates/agents` (`otty-agents`): the traits, Claude Code, Codex, the
  function that walks the sources.
- `crates/app/features/agents/core` (`otty-app-agents`): state and views of
  the mode, no I/O.
- `crates/app/features/agents/io` (`otty-app-agents-io`): the background
  reads and `agents.json`.
- `Mode::Agents` is a new variant of the closed list of modes.

### 8. New dependencies

- `ureq`, an HTTP client (approved by the owner). The two HTTPS sources of
  limits need a client with TLS; the workspace has none (the Docker client
  only speaks to local sockets). `ureq` is a small blocking client, enough
  for two `GET` requests, run on a background thread.
- `ruzstd`, a reader of zstd files, for the compressed Codex sessions
  (approved by the owner). It only unpacks and is written in Rust, so
  nothing in C is built.

### 9. Three kinds of billing

An account is paid for in one of three ways, and the dashboard and the short
number in the left panel follow it:

| Billing | Example | Billing block of the dashboard | Number in the left panel |
| --- | --- | --- | --- |
| Usage windows | Claude Max, ChatGPT Plus | each window as a bar: percent used, when it resets; extra usage if the plan has it | the most used window, `42 %` |
| Credits | Codex on an enterprise plan with credits | credits used of the pool as a bar, how many are left, when the pool renews; credits spent today, in 7 days, this month | `3 760 left` |
| API key | Claude Code with `ANTHROPIC_API_KEY`, Codex with `OPENAI_API_KEY` | money today, in 7 days, this month, a bar per day, the user's budget as a bar when one is set | `$12.40 today` |

- The agent tells the billing of a profile from what it reads: Claude Code
  with an API key in its settings or `primaryApiKey` in `.claude.json` is an
  API key profile, else a subscription; Codex with `auth_mode` of an API key
  or a non-null `OPENAI_API_KEY` is an API key profile, else what its limits
  source says (windows or credits).
- A source gives only some numbers: the Codex credits answer has a balance
  and no total. Then the bar is left out and the block shows what is known
  ("3 760 credits left"). Nothing is guessed.
- **Money is an estimate unless a bill is read.** OTTY counts the tokens of
  the session files per model and multiplies them by a price table bundled
  with OTTY (list prices per million input, cached input and output tokens,
  with the date they were taken). A model with no price keeps its tokens and
  is marked "no price". The block says "an estimate".
- **The real bill** of an Anthropic organization comes from the Admin API
  cost report and needs an admin key. OTTY never stores the key: in Edit the
  user points at a file that holds it (or names an environment variable),
  and OTTY stores only that path or name in `agents.json`.
- **Budget**: the user can set a monthly budget for an API key profile in
  Edit. It is saved in `agents.json`, only drawn as a bar, and never stops
  anything.

### 10. Changes after the owner used the first build (2026-11-16)

These override Decision 6 and the canvas where they differ.

- **"AI Profiles".** The left panel section that lists the profiles is
  called "AI Profiles", and the dialog "Add AI profile" / "Edit AI
  profile". The mode keeps its name, Agents.
- **Dashboard header**: the logo and the profile name, then Refresh, then
  "⋯". No status badge. The plan and the account are a muted line next to
  the name; when the row is too narrow, that line moves under the name and
  the buttons stay on the name's line.
- **"⋯" of a profile**: Edit; Log in while the profile is not logged in;
  Remove for an added profile. There is no Edit button and no Hide: hiding
  is gone everywhere (the menu item, "Hidden profiles (N)", the `hidden`
  setting; an old `agents.json` that says `hidden` loads with the profile
  shown).
- **No program path.** The Edit dialog has no program field and
  `agents.json` keeps no program; the program is found by name through the
  user's login shell. An old file's `program` is ignored.
- **"Usage", not "Limits".** The billing block of the dashboard is called
  "Usage"; under its title only "Updated 2 min ago". Which source answered
  is said only in "Where the numbers come from".
- **Reset times are exact**: "Resets in 2 h 10 min, at 15:30", "Resets on
  Thursday at 09:00", or the date for more than six days ahead, in the
  user's time zone (the offset OTTY already reads, issue 09).
- **Source names are words**, translated: for Claude Code "Anthropic usage
  API" and "Claude Code /usage"; for Codex "ChatGPT usage API", "Codex
  app-server" and "Latest Codex session"; for money "Anthropic cost report"
  and "Session files at list prices". The sources list shows each name with
  a state label (answered, not needed, not available, login expired,
  failed); the reason of a failure is in the label's tooltip, not in a line
  under it.
- **Loading in the background.** OTTY starts reading the profiles, the
  sessions, the limits, the tokens and the money of Agents mode, and
  looking for the container engines of Containers mode, when it starts, not
  when the mode is first opened. While a mode is not on screen nothing is
  read again on a timer; opening the mode reads again what is older than
  the timer's period.
- **Notifications for real failures.** A failure that is not one of the
  expected states (not logged in, not available, a plan without numbers)
  shows a notification: `agents.json` that can not be read or written, a
  config or session folder that exists but can not be read, every limit
  source failing with at least one "unexpected answer" (the agent's format
  probably changed), a crash of the hidden terminal or of `app-server`. One
  notification per problem: it is not repeated while the same problem
  stays, and shows again only after it went away and came back.
- **Loading screen.** Until the first reading of the profiles is done, the
  main area of Agents mode shows a spinner with "Looking for agents…" (as
  Containers does while it looks for engines). When it is done and nothing
  is selected, the first AI profile of the list is selected and its
  dashboard opens.

## Out of scope

- Providers that only have an API key (DeepSeek, Kimi, Alibaba) and keeping
  such keys. The profile dialog and the traits are built so that they are
  another agent in the list.
- The real bill of an OpenAI organization (its usage API); Codex API key
  profiles show the estimate only.
- Login tokens kept in the macOS Keychain: on macOS the first Claude source
  works only when the token file exists, else the next source is used.
- Browser cookies as a source of limits.
- Reading a session's content and deleting sessions.
- Search inside the text of sessions. It comes later; this is why every
  session file, compressed or not, is already readable in the first version.
  Searching projects by name and the sessions table by its columns are in
  scope (Decision 6).
- Agents on other machines over SSH.
- Analytics.

## Rules for the work

- Tests and experiments never touch the real `~/.claude` or `~/.codex`. They
  use a temporary config folder with made-up files.
- Test data is made up. Never copy the owner's session files, account names
  or tokens into the repository.
- Tests make no network requests and start no real agent program.
- Never call `thread/list` on `codex app-server` (Decision 5).
- Parts of CodexBar (MIT licence, Swift) may be rewritten in Rust. A file
  that follows its code closely says so in a comment with a link.
- Agent logos are the real ones from Simple Icons (CC0), drawn in one color
  like the other OTTY icons. A new agent gets its logo the same way; OTTY
  does not draw its own.

## Issues

One file each under `issues/`. Done: 01–06. The order of the rest:

7. Limits from the HTTPS requests, both agents, with the HTTP client
   (started before the design; its drawing is replaced by 14).
11. Three kinds of billing in `otty-agents`.
8. Codex limits from `app-server` and from the session files.
9. Tokens spent, both agents, with the breakdowns the dashboard needs.
12. Money for API key profiles: the price table and the estimate.
13. The kit pieces and the agent logos.
14. The left panel: Agents and Projects sections, search and filter.
15. The project screen: the sessions table.
16. The profile dashboard and the profile dialog in the new look.
17. The Anthropic Admin API cost report and the budget.
18. Follow-ups of the money work.
10. Claude limits from the hidden terminal, following the logic of
    CodexBar.
19. Credits that come without numbers.
20. Agents mode at any window width.
21. Files follows the folder of an agent tab.
22. The dashboard header and actions, AI Profiles, no Hide, no program path.
23. The Usage block and the sources: exact times, names, state labels.
24. Loading in the background and notifications for real failures.
25. The loading screen and the first profile.
