# 20 — SSH passphrase dialog and the connecting row state

Status: resolved
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

## Answer

Dialog restyled in the V4Passphrase look; the "connecting" row word was NOT added.

- Title by `SshSecretKind`: "Passphrase for <key path>" or "Password for <user@host:port>". Muted line under it: "<launch> needs this key to connect as <user@host>." (password variant: "needs the password"). Field label "Passphrase" or "Password"; existing wrong-passphrase / password-rejected text still shows under the field on retry. Cancel (ghost) and Connect (primary) send the existing `SecretPromptCancel` / `SecretPromptSubmit` intents.
- Code: `app/src/widgets/quick_launch/view/secret_prompt.rs` (private `SecretPromptTexts`, private `dialog`). New accessors `SecretPromptState::launch_title()` / `account()` and `QuickLaunch::ssh_account()`.
- i18n (all 10 locales): removed `tpl_secret_prompt_{passphrase,password}_body`; added `tpl_secret_prompt_{passphrase,password}_{title,hint}` and `secret_prompt_{passphrase,password}_label`. The old plain `secret_prompt_*_title` keys stay (used by the error type).
- Tests (6, `iced_test`): title and muted line per kind, no error on first attempt, error text on retry for both kinds, Connect gives `SecretPromptSubmit`, Cancel gives `SecretPromptCancel`.

### Research: is there a session row while an SSH launch waits?

No. A tab is only opened by `OpenCommandTerminalTab`, which `handle_prepared_quick_launch` emits after `SetupCompleted(Prepared)`. `SecretRequired` opens the prompt and removes the launch from `launching`; the SSH probe also runs before any tab exists. A "connecting" session row would need new behavior (an early placeholder tab), so per the ticket it is left out.

## Comments

- Open item: the "connecting" state word needs a decision from the owner (open the tab before the SSH probe, with a placeholder state).
- Tests were written together with the implementation in one pass, not red first; the iced_test cases exercise real rendering and clicks.
- Regression checklist for the owner (SSH flow): 1) launch an SSH entry with an encrypted key: dialog shows "Passphrase for <key>" and the muted line; field is focused. 2) Type a wrong passphrase, press Enter: dialog reopens with "Wrong passphrase. Try again." under the field. 3) Type the right one: tab opens and connects. 4) Cancel: dialog closes, no tab, no launch indicator. 5) Password-auth SSH entry: "Password for user@host:port", wrong password shows the rejected text. 6) Empty submit does nothing. 7) Dialog is readable in the dark theme and in another locale.
- Gates: fmt, clippy `-D warnings`, `cargo test --workspace --all-features` (500 app tests pass), `cargo deny check` all clean. Coverage not measured.
