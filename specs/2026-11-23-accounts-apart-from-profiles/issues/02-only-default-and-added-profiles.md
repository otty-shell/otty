# 02 OTTY shows only the default profiles and the ones the user added

Status: ready
Blocked by: none

## Why

Decision 4: when an agent part is on, OTTY shows its default profile
(the folder the agent uses with no variable set, or the one its variable
names, such as `~/.codex` or `~/.claude`). The user adds other profiles
with Add profile, as today. OTTY no longer scans the home folder for
`~/.codex*` or `~/.claude*`: a found folder was often an old copy or
another tool's, and the scan existed mostly to find more logins.

## What to build

1. **No scan in OTTY.** `Agent::found_config_dirs` and `found_in_home`
   (`crates/agents/src/agent.rs`, `crates/agents/src/profile_list.rs`) go.
   `profiles` (`profile_list.rs`) lists the default profile of each agent
   part that is on, then the profiles of `SavedProfiles` the user added,
   less the ones the user hid. `Agent::has_sessions` goes if only the
   extra-folder check used it.
2. **The contract says "default only".** The doc comment of `profiles` in
   `crates/plugins/sdk/wit/agents-agent/0.1.0/agent.wit` says it gives the
   agent's default config folder only (Decision 12: no version bump).
   The shape stays. OTTY uses the place marked `is-default` and ignores
   any other the part gives (`crates/agents/src/plugin_agent.rs`).
3. **Official plugins.** `profiles.rs` of `plugins/openai`,
   `plugins/anthropic` and the profile listing of `plugins/opencode` and
   `plugins/template` give only the default folder. Bump each changed
   plugin's patch version and rebuild with `plugins/build.sh`.
4. Found by OTTY counts only these profiles. Add profile still proposes
   a new folder (`proposed_config_dir`) and adds any folder the user
   picks.
5. The module docs (`crates/agents/src/lib.rs`, `profile_list.rs`) follow.
   The public docs are issue 10.

## Done when

- Tests first: with `~/.codex` and `~/.codex-work` on disk and nothing
  added, only `~/.codex` is a profile, with the agent's variable set its
  folder is the default profile, an added `~/.codex-work` is listed, a
  hidden default profile is not, and the template plugin gives one place.
- The checks of `AGENTS.md` pass.

## Comments
