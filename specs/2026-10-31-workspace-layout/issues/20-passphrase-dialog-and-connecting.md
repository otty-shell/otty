# 20 — SSH passphrase dialog and the connecting row state

Status: ready-for-agent
Type: task
Class: B
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 10
Spec: [../spec.md](../spec.md)

## What

- `secret_prompt.rs` in the `V4Passphrase` look: kit `Dialog`; title "Passphrase for <key path>" or "Password for <user@host>" by `SshSecretKind`; muted line "<launch> needs this key to connect as <user@host>."; password `TextField`; the existing error text under the field; Cancel and primary Connect.
- While an SSH launch is waiting for a secret or connecting, its session row shows the muted state word "connecting" (from existing quick launch launch state; view model only).

## Done when

- SSH flow of the regression checklist passes (right and wrong passphrase, cancel); view model test for the state word.
