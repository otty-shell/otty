# 01 The `otty-agents` crate: shared types, traits and the walk over sources

Status: ready-for-agent

## Why

Everything Agents mode knows about a coding agent goes through one library
crate, so that the next agent is "implement the traits and add a line"
(spec Decisions 4, 5 and 7). This issue makes the crate with no agent in it
yet.

## What to do

1. Create `crates/agents`, package `otty-agents`, with a short `README.md`
   (`AGENTS.md` rule for new crates). It must not depend on `iced` or on any
   crate under `crates/app/`.
2. The shared data, with doc comments and private fields:
   - `AgentId`, `AgentDescriptor` (id, name shown to the user, icon name).
   - `Profile`: agent id, name, config folder, optional program path,
     whether it is the default profile, whether it is hidden.
   - `ProfileStatus`: connected (with account and plan when known), not
     logged in, program not found, config folder not found.
   - `SessionId`, `AgentSession`: id, project folder, title, time of the
     last change, git branch when known.
   - `Limits`: windows (name, percent used, reset time), credits (balance
     or unlimited), plan name. Every part may be missing.
   - `Spend`: tokens in and out, today and for the last 30 days.
   - `AgentCommand`: program, arguments, environment variables.
   - `AgentEnv`: what the caller read from the environment (home folder,
     `CLAUDE_CONFIG_DIR`, `CODEX_HOME`). The crate reads no environment
     itself.
   - `SourceError` (`thiserror`): not available, not logged in, network,
     unexpected answer; each with a short text for the user.
3. The traits of spec Decision 4: `Agent`, `SessionSource`, `LimitSource`,
   `SpendCounter`. Each source has a `name()`. Methods are plain blocking
   functions; the app runs them on a background thread.
4. The walk over sources (spec Decision 5): try the sources in order,
   return the first answer together with the name of the source that gave
   it and the failures of the earlier ones; when none answered, return all
   failures. One function for sessions and limits if it reads well, two if
   not.
5. Projects (spec Decision 2): a function that takes the sessions of all
   profiles and returns the projects, one for each folder, sorted by their
   latest session, each with its sessions newest first.
6. `known_agents()`: the list of agents. Empty in this issue.

## Done when

- Tests, written first with `mockall` sources, cover: the first source
  answers; the first fails and the second answers, and the failure is
  reported; all fail; an agent with no sources; sessions of two profiles in
  one folder become one project; projects are sorted by the latest session.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Created `crates/agents` (`otty-agents`) with its `README.md`. It depends
only on `thiserror` (and `mockall` for tests), not on `iced` or any app
crate. It holds the shared data types with private fields, the four traits
of Decision 4, the walk over sources as two functions, `read_sessions` and
`read_limits`, that share one private loop, the `projects` function, and an
empty `known_agents()`. Tests were written first and cover the six cases
of "Done when", plus a profile with no sessions.

Choices the issue left open: a source's `name()` is `&'static str`, so an
answer can outlive its sources. When no source answered, the walk returns
the failures in order; an empty list means the agent has no such sources.
`SourceError` cases carry a short detail text where one helps (`NotAvailable`,
`Network`, `UnexpectedAnswer`). A limit window's reset time is optional. A
project keeps a copy of each session's profile, so the app can show the
agent and the profile name and launch the session. `Agent` is `Send + Sync`
and the sources are `Send`, because the app runs them on a background
thread. `MockAgent` is declared with `mock!` in `src/testing.rs` instead of
`automock`: mockall needs a named lifetime for `Option<&SessionId>` in
`launch`, and clippy rejects that lifetime on the trait itself.
