# 22 The title bar and the status bar

Status: needs-triage

## Why

The folder `crates/app/otty/src/frame/chrome/` held two parts of the window
that have nothing to do with each other:

- the title bar at the top: a real component (`ChromeWidget` with its state,
  whether the left button is held on the title bar; `ChromeIntent`,
  `ChromeOutcome` and `reduce`) and its view `view/action_bar.rs`;
- the status bar at the bottom: only a picture. `StatusBarInput` →
  `StatusBarViewModel` and `view/status_bar.rs`, with no state, no intents
  and no `reduce`.

"Chrome" named neither of them well. A reader looking for the status bar had
to know that it lived inside the title bar's folder.

## What to do

1. Move the status bar to `frame/status_bar/` with `model.rs`
   (`StatusBarInput` → `StatusBarViewModel`) and `view.rs`, with their tests.
   It stays a view: no component, no empty state or intents, because an
   empty component is a placeholder the AGENTS.md rules forbid.
2. Rename `frame/chrome/` to `frame/title_bar/` and every name that means the
   title bar: `ChromeWidget` → `TitleBar`, `ChromeState` → `TitleBarState`,
   `ChromeIntent` / `ChromeOutcome` → `TitleBarIntent` / `TitleBarOutcome`,
   `AppEvent::Chrome` → `AppEvent::TitleBar`, the `App` field `chrome` →
   `title_bar`, `dispatch/chrome.rs` → `dispatch/title_bar.rs`, and the uses
   in `guards.rs`, `update.rs`, `view.rs` and the tests. The reducer keeps
   `#![forbid(clippy::disallowed_methods)]` at its top.
3. Update the module docs of `frame/`, the doc of `Component` in
   `otty-app-slots` and the spec: the frame is the title bar, the status bar
   and the sidebar, and the status bar is a view, not a component.

The behaviour of the app does not change.

## Done when

- `frame/` holds `title_bar/`, `status_bar/` and `sidebar/`, and
  `frame/chrome/` is gone.
- `grep -rni chrome crates/app specs/2026-11-10-feature-crates/spec.md`
  shows nothing about this part of the window.
- The binary has the same number of tests as before, and every moved test
  still runs.
- All checks from AGENTS.md pass, and `app/` is unchanged.

## Comments

### 2026-11-10: two places instead of the chrome

**Where things went.**

- `frame/status_bar/` holds `model.rs` (`StatusBarInput`,
  `StatusBarViewModel`) and `view.rs` (the status bar view), each with its
  tests. `mod.rs` only declares them and says why the status bar is no
  component. `screens/terminal.rs` builds the view model and draws it, as
  before.
- `frame/title_bar/` holds `TitleBar` (`mod.rs`), `TitleBarState`,
  `TitleBarIntent` / `TitleBarOutcome`, `reducer.rs` (still with
  `#![forbid(clippy::disallowed_methods)]` at its top) and `view.rs`. The
  view was `view/action_bar.rs`; with the status bar gone the `view/` folder
  would hold one file, so the view became `view.rs` and its props
  `ActionBarProps` became `TitleBarProps`.
- `AppEvent::TitleBar`, the `App` field `title_bar` and
  `dispatch/title_bar.rs` replace the chrome names. The update test
  `the_action_bar_button_stops_the_logs_and_starts_them_again` is now
  `the_title_bar_files_button_stops_the_logs_and_starts_them_again`; no test
  name said "chrome".
- The docs of `frame/mod.rs`, `Component` in `otty-app-slots` and the spec
  (the binary's `frame/` entry and the list of frame components) name the
  title bar, the status bar and the sidebar. The binary README never named
  the chrome.
- `grep -rni chrome crates/app specs/2026-11-10-feature-crates/spec.md`
  still shows `Surface::Chrome` in the title bar view and `surface_chrome()`
  in a theme test. Both are the name of a background color of the UI kit,
  not this part of the window, so they stay.

**Checks.**

- `otty-app` tests: 525 before and 525 after. The only changes in the list
  are the new module paths and the one renamed update test.
- `cargo +nightly fmt --all`: clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
  clean.
- `cargo deny check`: advisories, bans, licenses and sources ok; only the
  three known yanked-crate warnings (`clipboard-win`, `core2`, `spin`).
- `cargo test --workspace --all-features`: 6322 passed, 0 failed, 5 ignored
  (the same ignored tests as before this issue).
- `git diff --stat 016ee31 -- app`: empty.
- `cargo llvm-cov` was not run for this issue.
