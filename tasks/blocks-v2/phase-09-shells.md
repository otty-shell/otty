# Phase 9: additional shells and complex environments

Status: **not started**.

Parent document: [Blocks v2 specification](spec.md). IDs: B2-090–B2-094.

## Goal

Extend protocol v2 beyond local Bash/Zsh only where integration can be verified by a real
shell-lifecycle test. An unsupported environment must honestly report `Unsupported` or
`Degraded`, while the terminal remains fully usable without hooks.

## Dependencies

This phase starts after protocol v2, the lifecycle reducer, output routing, and integration
status are stable. New test or runtime dependencies require prior approval. A missing shell
binary in CI produces a capability-gated skip rather than a false success.

## Scope

- [ ] **B2-090** Add Fish protocol-v2 hooks and real-shell tests for the full lifecycle.
- [ ] **B2-091** Add PowerShell protocol-v2 hooks and tests on available Linux, macOS, and
  Windows CI platforms, accounting for quoting and status differences.
- [ ] **B2-092** Add tmux/screen detection, self-test, and documented passthrough without
  automatically changing user configuration.
- [ ] **B2-093** Design explicit SSH/container bootstrap with threat model, capability
  negotiation, versioning, and cleanup review.
- [ ] **B2-094** Add Nushell or another shell only together with real-shell tests; report it as
  `Unsupported` until then.

## Shared lifecycle matrix for each shell

- source or import integration twice in one process;
- root and nested shell with unique IDs and a parent link;
- success, non-zero exit, pipeline status, command-not-found, signal, and Ctrl-C;
- multiline command, Unicode, quotes, and control-like text;
- preservation of existing prompt, preexec, precmd, and exit hooks;
- shell restart or `exec`, missing loader, and unsupported protocol version;
- absence of optional external tools;
- clean shell exit and recovery from a missing command-end.

## Environment security rules

- Never modify tmux, screen, SSH, or shell user configuration automatically.
- Remote bootstrap runs only through an explicit action and displays the installed version and
  target path.
- Session identity and trust in the local terminal are not transferred to the remote side
  without negotiation.
- Temporary remote assets use restricted permissions and documented cleanup.
- Shell command and output contents do not enter connection diagnostics.

## Automated verification

After implementing each adapter, add a separate capability-gated command, for example:

```bash
cargo test -p otty --test shell_integration -- --nocapture
```

The CI matrix must explicitly list detected `bash`, `zsh`, `fish`, `pwsh`, `tmux`, and
`screen` binaries. An adapter cannot merge when its binary is available but lifecycle tests
are skipped.

## Manual verification

1. Launch OTTY with Fish, wait for `Active v2`, and complete the shared lifecycle matrix.
2. Repeat in PowerShell on every supported platform, separately checking `$LASTEXITCODE`,
   native-process exit, and pipeline semantics.
3. Run Bash, Zsh, and Fish inside tmux and screen. Exercise commands, a nested shell, and an
   alternate-screen application; protocol events must not become visible garbage or disappear.
4. Disable passthrough or configuration. The UI must show a diagnosable degraded status and
   offer instructions without editing configuration.
5. Connect through SSH or to a container without remote bootstrap. The terminal must work
   normally and integration must honestly report unsupported or degraded.
6. Run explicit remote bootstrap after reviewing target and version. Verify lifecycle,
   reconnect, version mismatch, and cleanup.
7. Reject bootstrap or interrupt it halfway through. Remote user files must remain intact;
   partial temporary assets are removed or discoverable through documented cleanup.
8. Select an unknown shell. The application must not crash or imitate block lifecycle and must
   report `Unsupported(<shell>)`.

The phase is complete only for environments that pass a real lifecycle matrix. An unverified
script prototype does not constitute support.
