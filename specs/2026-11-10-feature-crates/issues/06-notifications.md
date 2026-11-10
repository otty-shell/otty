# 06 Notifications

Status: needs-triage
Blocked by: 05

## What to do

Move `app/src/widgets/notifications` into `otty-app-notifications`, following
"Moving a feature" in the spec. It is the first feature, so it also:

- adds the `Overlays` trait to `otty-app-slots`;
- sets the pattern every later feature copies: the `Message` variant,
  `dispatch/notifications.rs`, reading outcomes;
- creates `otty-app-testing` if its tests need helpers that later features will
  share. Helpers that need the whole app stay in the binary's tests.

Behaviour to keep (from the old app):

- an error shows as a toast at the top for 5 seconds, with a close button;
- toasts stay under dialogs;
- the bell in the header opens the list as a popup menu, with a badge that
  counts unseen errors;
- the list is kept in memory only, holds the last 100, rows highlight on hover,
  each row has an always visible remove cross, and Clear all is the broom;
- ages are shown as relative times.

Other features report an error through their own `Outcome`, and the binary turns
it into a notifications `Intent`. So far the frame is the only source: a failed
save of the pinned modes.

## Done when

- The behaviour above works in the new app.
- `otty-app-notifications` depends on no other feature and has no `tokio`.
- The steps and checks in "Moving a feature" are done.

## Comments
