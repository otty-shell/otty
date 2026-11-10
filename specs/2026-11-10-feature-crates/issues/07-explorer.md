# 07 Explorer

Status: needs-triage
Blocked by: 06

## What to do

Move `app/src/widgets/explorer` into `otty-app-explorer`, following "Moving a
feature" in the spec. It adds the `RightPanel` trait to `otty-app-slots`.

- Reading folders goes to `effects/explorer.rs` in the binary.
- Watching files for changes uses `notify`, which a feature may not depend on,
  so the watcher lives in the binary's `subscription.rs` and sends explorer
  intents.
- Following the folder of the active terminal (`SyncRoot`) is wired in issue 11,
  when the terminal moves.

## Done when

- The Files section in the right panel of Terminal mode shows and opens folders
  and files as in the old app, and updates when files change.
- `otty-app-explorer` has no `notify` and no `tokio`.
- The steps and checks in "Moving a feature" are done.

## Comments
