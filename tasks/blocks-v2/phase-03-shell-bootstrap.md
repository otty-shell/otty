# Phase 3: Bash/Zsh lifecycle and bootstrap

Status: **partially complete**.

Parent document: [Blocks v2 specification](spec.md). IDs: B2-030–B2-039.

## Goal

Provide a complete root and nested Bash/Zsh lifecycle: unique shell and block IDs, exact exit
and pipeline status, prompt boundaries, preservation of user hooks, and explicit integration
status. Bootstrap failure must not break the ordinary terminal.

## Current state

Bash and Zsh assets use protocol v2. PID-scoped guards, parent links, dependency-free JSON and
hex encoding, OSC 133, exact completion events, atomic asset writes with `0600` permissions,
system-random session IDs, concurrent bootstrap tests, and `Pending`, `Active`, `Degraded`, and
`Unsupported` UI statuses are implemented. Bash OSC markers use invisible `PS1` syntax; the
Fedora/Bash 5.3 `${PROMPT_START@P}` regression does not expose service text in the prompt.

The phase remains incomplete because assets are still prepared inside the shared `services.rs`,
and there is no handshake timeout, explicit persistent-loader install/uninstall, or complete
PTY matrix for signals, Ctrl-C, heredoc, hook reload, and `exec shell`.

## Scope

- [ ] **B2-030** Extend the real-shell harness to the complete PTY lifecycle matrix. Current
  process tests cover source-twice, nested IDs, exit and pipeline status, and some existing
  hooks.
- [x] **B2-031** Implement Bash v2 with early status capture.
- [x] **B2-032** Implement Zsh v2 with `$?`, `pipestatus`, and chaining of existing hooks.
- [x] **B2-033** Emit OSC 133 A/B without changing the visible prompt, including the
  Fedora/Bash 5.3 `${PROMPT_START@P}` regression.
- [x] **B2-034** Add PID-scoped idempotency and parent/current shell-context IDs.
- [x] **B2-035** Encode without per-prompt `jq`, Python, or Perl.
- [ ] **B2-036** Move versioned asset and bootstrap logic into focused
  `terminal_workspace/shell_integration/`; atomic writing is already implemented.
- [ ] **B2-037** Add a handshake timeout and `Pending` → `Degraded` transition; the other
  status variants already exist in the model and UI.
- [ ] **B2-038** Implement only an explicit persistent-loader install/uninstall flow with
  preview and confirmation; never edit `.bashrc` or `.zshrc` automatically.
- [x] **B2-039** Test concurrent bootstrap for multiple terminal sessions.

## Automated verification

```bash
bash -n assets/shell-integrations/otty.bash
zsh -n assets/shell-integrations/otty.zsh
cargo test -p otty --test shell_integration -- --nocapture
cargo test -p otty terminal_workspace::services
```

Capability-gated Zsh tests must explicitly report a skip when the binary is unavailable.
Primary Bash tests must not silently skip.

## Manual verification

1. Launch `cargo run -p otty` with Bash and confirm that status changes from `Pending` to
   `Integration v2 active`.
2. On Fedora/Bash 5.3, confirm that the prompt starts with the ordinary `user@host` text and
   does not expose `PROMPT_START@P}` or `PROMPT_END@P}`. OSC markers must not affect prompt
   width.
3. Run `false`, `false | true`, an unknown command, and a command interrupted with Ctrl-C.
   Each command block outcome must match the actual result.
4. Source the integration asset twice and run one command. Each lifecycle phase must produce
   one event and one block.
5. Start nested interactive Bash, run two commands, exit it, and continue in the parent. The
   child must have a different shell instance and must not change parent blocks.
6. Repeat the Bash lifecycle in Zsh and additionally verify pipeline status.
7. Open several terminal tabs concurrently. Each must receive a distinct session ID and become
   `Active` without races while creating integration files.
8. Launch a profile with an unsupported shell. The terminal must work and the UI must show
   `Unsupported` rather than remaining in `Pending` forever.
9. Simulate failure while writing the bootstrap directory. The terminal must start as
   `Degraded(bootstrap_failed)` without changing user rc files.
10. Once the loader exists, test preview, rejected confirmation, install, repeated install,
    and uninstall using a temporary HOME. The original rc file must be restored exactly.

The phase is complete when root and nested Bash/Zsh pass the full PTY matrix, timeout handling
distinguishes inactive integration, and the persistent loader is controlled only by explicit
user action.
