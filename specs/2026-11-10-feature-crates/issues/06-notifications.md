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

### 2026-11-10: the notifications feature crate

**`otty-app-notifications`** (`crates/app/features/notifications`) is a copy
of `app/src/widgets/notifications`; the old widget is unchanged
(`git diff --stat 081ff00 -- app` is empty). It depends on `iced` (with
`svg`), `otty-app-i18n`, `otty-app-slots`, `otty-app-theme` and
`otty-ui-kit`; for tests on `iced_test` and `otty-app-testing`. Its public
API:

- `Notifications` with `new`, `unseen_count`, `is_list_open`,
  `toast_timer() -> Option<Duration>` and `toasts(theme) -> Option<Element>`;
- `impl Feature` with `Ctx<'a> = NotificationsCtx { pub now: Instant }`: the
  binary reads the clock, so `reduce` stays plain and the tests pick the time;
- `impl Overlays`: the open list as `Overlay::menu`.

```rust
pub enum NotificationsIntent {
    Push(String), DismissToast(u64), Tick, OpenList, CloseList, Clear, Remove(u64),
}

pub enum NotificationsOutcome {
    /// The list opened: it takes the keyboard, as any menu does.
    ListOpened,
    /// The list closed: in Terminal mode the active pane gets the keyboard back.
    ListClosed,
}
```

`NotificationsEffect`, the `NotificationsEvent` wrapper and the two
`Task::done` of the reducer are gone. The copied reducer tests were rewritten
to check the returned outcomes and run first against the copied `Task`
reducer (they failed to build: no `is_empty` and no `==` on a `Task`), then
`reduce` changed to return `Vec<NotificationsOutcome>`.

**The `Overlays` trait** in `otty-app-slots` (`src/overlays.rs`):

```rust
pub trait Overlays: Feature {
    fn overlay<'a>(&'a self, theme: ThemeProps<'a>)
        -> Option<Overlay<'a, Self::Intent>>;
}
```

The signature of the spec fits: a feature has at most one overlay open at a
time and gives it or `None`; the frame puts it into the feature's layer and
treats it by its kind. The toasts are no overlay (Esc leaves them, they take
no focus, a dialog lies over them), so they do not go through this trait: the
feature gives them with its own method `toasts(theme)`, and the binary draws
them in the window content, over the panels and the focus note, under every
overlay. A trait for the toasts would have one implementation, which
AGENTS.md does not allow. A feature whose overlays lie in two different
layers (quick launch: its context menu among the mode overlays, the SSH
prompt on top) is the first real case for more than one; issue 12 decides
then, for example with a second method.

**The toast timer runs in the binary.** `iced::time::every` exists only with
iced's `tokio` or `smol` feature, and turning that on in the feature crate
would pull `tokio` into it. So `Feature::subscription` stays `none`, and the
feature says what it needs with `toast_timer()`: `Some(250 ms)` while a toast
shows, `None` otherwise. The binary's `subscription.rs` turns that into
`iced::time::every(..)` and each tick into `NotificationsIntent::Tick`. The
decision (when and how often) stays in the feature; the binary only runs it.

**`otty-app-testing`** (`crates/app/testing`, a dev-dependency only) holds
`mouse::middle_click` and `mouse::right_click`. Six old widgets test a middle
press outside their menu, so the later features need them too. The binary's
`testing/mouse.rs` was removed and its mode menu tests use the crate. The
helpers that find the broom and a row cross in the kit list are only used by
the list's own view tests, so they stay there.

**How the binary wires it:**

- `AppEvent::Notifications(NotificationsIntent)` and `App::notifications`;
- `dispatch/notifications.rs`: builds the `Ctx` from `Instant::now()`, calls
  `reduce` and reads `ListOpened` → `take_keyboard()`, `ListClosed` → nothing
  until the terminal feature (issue 11) focuses the active pane;
- no `effects/notifications.rs`: the feature has no I/O;
- the sources: `ModesOutcome::PinnedSaveFailed(text)` →
  `notifications::deliver(Push(text))` in the same update;
  `AppCommand::OpenNotifications` (the bell; the palette entry comes with
  issue 08) → `OpenList`;
- start problems: `effects::settings::read_saved` now returns
  `(SavedSettings, Option<SettingsProblem>)` (`Unreadable` or `Invalid`, as
  the old settings storage told them), and `App::new` gives the settings and
  modes problems to `App::tell_start_problems`, which pushes each one through
  `dispatch/notifications.rs`. The texts are built after the language is set.
  The old app sent them with `Task::done` after the first update; now they are
  pushed while the app is built, so no message goes through iced for them;
- `view.rs`: the toasts in the window content, the list as its own layer
  right over the modes menu (the palette, issue 08, goes between them), the
  bell's open state and unseen count in the header, and the list in the
  resize grip check;
- `guards.rs`: `Overlay::NotificationsList` (a menu, Esc → `CloseList`) on
  top in `top_overlay` until Settings and the SSH prompt come; the broom, the
  row crosses and the dismiss layer pass under a menu, a toast cross only
  closes the menus; `update.rs`: `close_all_menus` closes the list.

**Gaps of issue 05 resolved:** the toasts layer, the list layer, the bell
state and unseen count, `OpenNotifications`, `PinnedSaveFailed`, the modes
file load problem, the settings read problems, the list in `close_all_menus`,
`top_overlay` and the resize grip check. None of them had a `dead_code` mark
(they were comments and `false`/`0` values), so no mark was removed. No
"issue 06" note is left in `crates/app/`.

**Left for later issues:** focusing the active pane when the list closes in
Terminal mode (11, from `ListClosed`); the dialog check over the toasts and
the bell under a dialog (09 and 12 bring the first dialogs; the old test
`a_dialog_lies_over_the_toasts` comes with them); the palette entry that
opens the list (08); the error sources of the other features (each issue
turns its own outcome into `Push`).

**Tests.** Copied and ported: the state, view model, reducer and view tests
of the widget; from the old binary the toast and list view tests, the bell
tests, the list in the overlay rules, Esc, `close_all_menus` and the grip
check. New: the start problems, the settings problems, the failed pin save
as a toast, the open list as a menu overlay, `toast_timer`.
`otty-app-notifications` has 52 tests and 2 doc tests, `otty-app-slots` 6
doc tests, `otty-app-testing` 2 doc tests, `otty-app` 321 tests (302
before). The whole workspace runs 4275 tests, all passing.

**Checks.** `cargo tree -p otty-app-notifications -e normal | grep -E
"tokio|notify"` prints only `tokio` and `tokio-macros` under
`otty-ui-term`, which `otty-app-theme` depends on (`cargo tree -i tokio`:
`otty-ui-term` → `otty-app-theme` → `otty-app-notifications`); no `notify`,
no direct `tokio`. A planted `std::fs::read` in the crate failed clippy with
`use of a disallowed method std::fs::read` and the note `a feature touches no
files; the binary does it in effects/`, so the shared
`crates/app/features/clippy.toml` is read; the call was removed.
`cargo +nightly fmt --all -- --check`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo deny check` (only the 3
known yanked-crate warnings), `cargo test --workspace --all-features` and
`cargo build -p otty-app` pass. `cargo llvm-cov` was not run, as asked. The
window was not opened (no display for the agent).

**For the owner to decide.**

- `tokio` reaches every app crate through `otty-app-theme` → `otty-ui-term`.
  If "a feature has no tokio" should hold for the whole tree, the terminal
  settings types the theme builds need to move out of `otty-ui-term`, or the
  theme has to stop depending on it.
- A feature timer runs in the binary (`toast_timer()` plus
  `iced::time::every` in `subscription.rs`). The other way is to turn on
  iced's `smol` feature in the feature crates, which keeps `tokio` out but
  adds a second async runtime.
- The start problems are pushed while `App::new` builds the app, so their
  five seconds start a moment before the first frame.
