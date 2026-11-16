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
2. Sees, for each of them, how much of the limits is used and how many
   tokens were spent.
3. Sees the projects the agents worked in and the sessions of each project,
   continues a session or starts a new one. Both open a terminal tab with
   the agent running in the project folder.

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

The default profile sets no variable, so what the user has in their own
environment keeps working.

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
- `Limits`: a list of windows (name, percent used, when it resets), credits
  (balance or "unlimited"), the plan name. Every part may be missing: a
  Codex account that pays per use has no windows at all.
- `Spend`: tokens in and out, today and for the last 30 days. Tokens only,
  no money.

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

- **Left panel**: the profiles (icon, name, state, the limit bars), an "Add
  profile" button, and under them the projects.
- **Main area**: for a selected project, its sessions (title, agent and
  profile, time, branch) and "New session", which asks for the profile when
  there is more than one. For a selected profile, its details: account and
  plan, each limit with its reset time, tokens spent, the source of the
  numbers, the config folder, edit, hide and remove.
- **Status bar**: the counts of projects and sessions.
- The right panel is not used.

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

## Out of scope

- Providers that only have an API key (DeepSeek, Kimi, Alibaba) and keeping
  such keys. The profile dialog and the traits are built so that they are
  another agent in the list.
- Money: prices and cost of the tokens.
- Login tokens kept in the macOS Keychain: on macOS the first Claude source
  works only when the token file exists, else the next source is used.
- Browser cookies as a source of limits.
- Reading a session's content and deleting sessions.
- Search in sessions. It comes later; this is why every session file,
  compressed or not, is already readable in the first version.
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

## Issues

One file each under `issues/`, in this order:

1. `otty-agents`: shared types, the traits, the walk over sources.
2. Claude Code: detect, sessions from files, launch command.
3. Codex: detect, sessions from plain and compressed files, launch command.
4. Profiles: `agents.json`, default profiles, the folders offered in the
   dialog, creating a folder.
5. The mode: `Mode::Agents`, the feature crates, profiles and projects in
   the left panel, the sessions list, opening a tab.
6. The profile dialog: add an existing folder or create a new one, log in,
   edit, hide, remove.
7. Limits from the HTTPS requests, both agents, with the HTTP client.
8. Codex limits from `app-server` and from the session files.
9. Tokens spent, both agents.
10. Claude limits from the hidden terminal, following the logic of
    CodexBar.
