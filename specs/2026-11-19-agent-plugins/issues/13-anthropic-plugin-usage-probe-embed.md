# 13 Anthropic plugin, Claude Code part: the `/usage` probe, embedded; core Claude Code deleted

Status: ready-for-agent
Blocked by: 06, 08, 12

## Why

Spec Decisions 6, 7 and 10. Claude Code shows some limits only on its
`/usage` screen. Today OTTY starts `claude` in a hidden terminal, answers
the trust question, reads the screen (`usage_probe.rs`,
`usage_screen.rs`, `trust_question.rs`, `window_id.rs`) and then deletes
the session files the probe left (`probe_sessions.rs`). In a plugin this
is a hidden terminal plus a declared write of the `claude-code` part.

## What to build

1. A limit source in the `claude-code` part through
   `terminal.hidden-terminal`: start `claude` with the arguments of today
   (declared in the part's `[[agent.permissions.terminal]]`), wait for
   the screen, answer the trust question, send `/usage`, parse the screen
   text into limit windows; all timing and text in the plugin.
2. Probe cleanup: a `write` permission of the part for only the `*.jsonl`
   files of the project folder of OTTY's probe working folder (spec
   "Permissions" wording "Deletes files in …"); remove them with
   `files.remove`, then the folder when empty, as `probe_sessions.rs`
   does.
3. Pacing stays in the host (issue 06): at most once in five minutes.
4. Embed the built copy of the Anthropic plugin in
   `assets/plugins/anthropic/` (issue 08); built from `plugins/anthropic/`
   by the script of issue 11 while the plugin lives in this repository.
5. Delete `crates/agents/src/claude_code/` except `usage.rs`,
   `usage_answer.rs` and `cost_report.rs`, which move to
   `account_numbers/` until issue 15; delete the built-in entry. The
   owner checks tokens, money and limits by hand.

## Done when

- Tests first: screen parsing on made-up screen texts (from today's
  tests of `usage_screen.rs`); the trust question answered; the probe
  sequence with a mock terminal; cleanup removes only the probe's files
  and refuses any other path.
- The core folder is gone except the moved
  files; the app shows Claude Code from the embedded Anthropic plugin.
- The checks of `AGENTS.md` pass; the plugin's own tests pass.

## Owner checks by hand

Limits of the Claude login appear on the account page; no probe session
is left in `~/.claude/projects/`; Profile info says "Read by: Anthropic
<version>".

## Comments
- 2026-11-20, issue 06: a plugin cannot learn the working folder OTTY starts a hidden terminal in, but Claude Code names its probe session files by that folder and the plugin must delete them (and its `write` glob must spell the folder out). Add a way for the plugin to know it (e.g. the host reports the terminal's folder, or the plugin passes the folder when it starts the terminal and the host checks it) — a small contract change, rebuild the test fixtures.
- 2026-11-20, issue 12: the probe also needs a `write` glob and an `[[agent.permissions.terminal]]` rule for `claude` in `plugins/anthropic/otty-plugin.toml`. Until this issue deletes the core `claude_code` reader, the plugin's `claude-code` id conflicts with the built-in entry. The host's `read-json-with-keys` stops at 1 MiB, and `.claude.json` grows with project history (the owner's are ~190 KB today): decide whether to raise the cap for key files or read only the needed fields.
- 2026-11-20, issue 13: done. The `claude-code` part reads limits from one source, `usage-screen`: `claude --allowed-tools "" --strict-mcp-config --settings <no hooks, no remote control>` with `DISABLE_AUTOUPDATER=1`, 160 by 200, in a hidden terminal (`usage_probe.rs`, `usage_screen.rs`, `trust_question.rs`, `probe_sessions.rs` of the plugin); the timing and every screen text live in the plugin. Errors come back as `read-error` texts that name what Claude Code asks, never screen text. Pacing stays in the host (issue 06).
- 2026-11-20, issue 13, the working folder (comment of issue 06): new host operation `terminal.working-folders() -> list<string>`, the folder as OTTY names it and, when links lead elsewhere, as the system resolves it; only for a part that declares a terminal. The host's `StartTerminal` gives it (`working_folder()`); the SDK has `terminal::working_folders()` and the test host `with_terminal_folder` (default `~/.cache/otty/agent-probe`). Test fixtures rebuilt with `scripts/build-test-plugins.sh`.
- 2026-11-20, issue 13, permissions (comment of issue 12): `write` allows only `~/.claude*/projects/*--cache-otty-agent-probe` and its `*.jsonl` (and the same under `{CLAUDE_CONFIG_DIR}`): OTTY's probe folder is always `~/.cache/otty/agent-probe`. `[[agent.permissions.terminal]]` declares `claude` with the five arguments above; the arguments are globs, so the braces of the settings JSON are escaped (`\{…\}`). `DISABLE_AUTOUPDATER` is listed in `env` so it may be passed.
- 2026-11-20, issue 13, `.claude.json` (comment of issue 12): `read-json-with-keys` now reads key files up to 4 MiB, as much as `files.read`, instead of 1 MiB; the plugin parses only the fields it needs, so its memory stays small.
- 2026-11-20, issue 13, embedding (used by issues 11, 14 and 16): `plugins/embed.sh <folder>` builds `plugins/<folder>/` for `wasm32-wasip2` (release, `--locked`, its own `target/`) and replaces `assets/plugins/<id>/` with `otty-plugin.toml`, `<id>.wasm`, every icon the manifest names and `SHA256SUMS` (as `sha256sum` writes it). The id and the `.wasm` name come from `[plugin]` of the manifest (`wasm` must be `<id>.wasm`). Then add one entry to `embedded_plugins()` in `crates/plugins/releases/src/embedded.rs`: `EmbeddedPlugin::new(id, include_str!(".../SHA256SUMS"))` with one `with_file` per file, and add the id to the test `every_built_in_copy_matches_its_hashes`. Run it again after every change of the plugin and commit the copy; a stale copy fails that test. `plugins/embed.sh template` makes the test copy `assets/plugins/example/`.
- 2026-11-20, issue 13, deleted: `crates/agents/src/claude_code/`, `ClaudeCode` and its built-in registry entry, `json_file::is_filled_text`. `cost_report.rs` is `account_numbers/anthropic_costs.rs`; `usage.rs` and `usage_answer.rs` are `account_numbers/claude_usage.rs` and `claude_usage_answer.rs`, the request now given a token and a plan, unused (`expect(dead_code)`) until issue 15. `CLAUDE_ICON` stays for the built-in Anthropic provider entry. The app tests use `otty_agents::fixtures::claude_code_stand_in()` (feature `fixtures`) where they only need Claude Code's name, commands and a bill, and the embedded Anthropic plugin (`agents_with_anthropic` in agents-io tests) where they read real files.
- 2026-11-20, issue 13, away: while Agents mode is not on screen a plugin's limit read starts no program or hidden terminal (`StartProgram::starts_now`, `CallBatch::without_programs`), and is not paced, as the core probe did. Such a read fails as "could not read" of the plugin, not with "OTTY starts it only while Agents mode is on screen".
- 2026-11-20, issue 13, left: the profile-level admin key bill of Claude Code profiles is gone with `ClaudeCode::bill_source` (an organization added by hand still reads its bill); issue 15 brings it back. The Claude-only error variants (`ClaudeQuestion`, `Unavailable::ClaudeAsks`, `ClaudeRateLimited`, `NoLoginToken`, `LoginMayNotReadUsage`, `Unexpected::UsagePanelUnreadable`, `SourceId::AnthropicUsageApi`/`ClaudeUsageCommand`) and their texts are left for issue 17.
