# 02 Claude Code: detect, sessions from files, launch command

Status: ready-for-agent
Blocked by: 01

## Why

Claude Code is the first of the two agents of the first version (spec
Decisions 3 and 5).

## What Claude Code keeps on disk

Checked with Claude Code 2.1.325 on Linux:

- The config folder is `CLAUDE_CONFIG_DIR`, else `~/.claude`.
- `<config>/.credentials.json` holds the login: `claudeAiOauth` with
  `accessToken`, `expiresAt`, `subscriptionType`, `rateLimitTier`. The file
  can be an empty object when the profile is not logged in.
- `<config>/projects/<folder name>/<session id>.jsonl` is one session, one
  JSON record on each line. The folder name is the project path with `/`
  and `.` replaced by `-`, so the path can not be rebuilt from it safely.
  Records of type `user` and `assistant` carry `cwd`, `sessionId`,
  `gitBranch` and `timestamp`. A record of type `ai-title` carries
  `aiTitle`; a record of type `last-prompt` carries `lastPrompt`.
- `<config>/projects/<folder name>/<session id>/subagents/` holds the
  sessions of helper agents. They are not listed.

## What to do

1. `ClaudeCode` implements `Agent`: descriptor, default config folder,
   `detect`, `launch` (`claude`, or `claude --resume <id>`; an added profile
   sets `CLAUDE_CONFIG_DIR`, the default one sets nothing), `login`
   (`claude`, which asks to log in on its first start, with the same
   variable). No limit sources and no spend counter yet.
2. One session source that reads the files:
   - only `*.jsonl` directly inside a project folder;
   - folder from the first `cwd` found, title from `aiTitle`, else the
     first prompt, else the id; time of the last change from the newest
     `timestamp`, else the file time; branch from `gitBranch`;
   - a session file can be large: read its start and its end, not all of
     it;
   - tolerant reading: an unknown or broken line is skipped; a file with
     nothing readable still gives a session with its id and the decoded
     folder name; one bad file never hides the others.
3. Add `ClaudeCode` to `known_agents()`.

## Done when

- Tests, written first on a temporary config folder with made-up files,
  cover: a normal session; a session with no title; broken lines in the
  middle; an empty file; helper sessions are not listed; a missing
  `projects` folder gives an empty list, not a failure; `detect` for a
  logged-in profile, an empty credentials file, a missing folder; the launch
  commands of the default and of an added profile.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Added `ClaudeCode` in `crates/agents/src/claude_code/`: the agent itself
(`agent.rs`), the login check (`detect.rs`), the walk over the projects
folder (`sessions.rs`) and the reading of one session file
(`session_file.rs`). It is the first entry of `known_agents()`. The crate
now depends on `serde` and `serde_json` from the workspace. The times of the
records are read by a small RFC 3339 reader in `src/timestamp.rs`, so no
time crate was added. Tests were written first, on made-up files in a
temporary folder.

Deviation in `detect`: `.credentials.json` can be an empty object while the
profile is logged in, because the token is kept somewhere else. So a
profile is connected when `claudeAiOauth.accessToken` is in that file, or
when the global config file has an `oauthAccount` object. The global config
file is `<config>/.claude.json`, and for a folder named `.claude` also the
`.claude.json` next to it (`~/.claude.json` of the default profile). The
account is the `emailAddress` of `oauthAccount` whenever it is there, also
with a token. The plan is `subscriptionType` with the multiplier of
`rateLimitTier`, such as "Max 20x". The token is skipped while reading and
never kept. The organization name is not shown, because `ProfileStatus`
has no place for it. "Program not found" means only an explicit program
path that does not exist; looking for `claude` on `PATH` is left to the app
(issue 05), since the crate reads no environment.

Choices the issue left open: a session file larger than 256 KiB is read as
its first and last 128 KiB. The folder is the first `cwd`; the title is the
newest `aiTitle`, else the first prompt (meta records, tool results and
Claude Code's `<command-...>` wrappers are not prompts), else the newest
`lastPrompt`, else the id; a title is its first line, cut at 100
characters. The branch is the newest non-empty `gitBranch`. A file that can
not be opened is skipped; any other file gives a session.
