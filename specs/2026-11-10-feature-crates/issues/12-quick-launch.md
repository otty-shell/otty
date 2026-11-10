# 12 Quick launch

Status: needs-triage
Blocked by: 11

## What to do

Move `app/src/widgets/quick_launch` into `otty-app-quick-launch`, following
"Moving a feature" in the spec. It fills three places: the Launches section of
the left panel (under the Sessions section of tabs), the launch editor in the
main area, and overlays (the context menu, the inline edit and the SSH secret
prompt).

- The SSH check and building the terminal settings
  (`quick_launch/services.rs`) go to `effects/quick_launch.rs` in the binary.
- Reading and saving the launches (`storage.rs`) go there too. A failed save
  becomes a notification once per streak of failures.
- A prepared launch returns `LaunchReady(TabLaunch)`. `TabLaunch` moves to
  `otty-app-domain`, and the binary hands it to tabs.
- Which machine a launch runs on is decided in this feature's `reduce`, not in
  the binary.
- The SSH secret prompt is always the top layer.

The spec's SSH example is the main test of this issue: write it as a binary
test first.

## Done when

- Saved commands and SSH launches, including a password or passphrase prompt
  and a retry, open their tabs as in the old app.
- The launch editor creates and edits launches; broken launch files at start
  become a notification.
- `otty-app-quick-launch` runs no processes and reads no files itself.
- The steps and checks in "Moving a feature" are done.

## Comments
