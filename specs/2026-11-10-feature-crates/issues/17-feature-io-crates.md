# 17 I/O crates for features and the shell helper

Status: needs-triage
Blocked by: 14

## Why

Until now each feature's I/O lived in the binary (`effects/<feature>.rs`,
`subscription/*`). Most of it belongs to exactly one feature: the SSH check
and the launches file to quick launch, the settings file to settings, folder
reads and file watching to explorer, the Docker calls and event streams to
containers. So a feature was spread over two crates, and the binary knew about
Docker, SSH and `notify`, although its job is only to compose the window and
link the parts.

## What to do

1. **One I/O crate per feature that talks to the outside world**, next to the
   feature in the same folder:

   ```
   crates/app/features/containers/
     core/   otty-app-containers      state, reduce, views (the crate as it is now)
     io/     otty-app-containers-io   Docker calls, event streams, logs
   ```

   The same for `quick-launch`, `settings` and `explorer`. Notifications,
   palette and tabs have no I/O and stay one crate. The terminal stays one
   crate too: it owns its live sessions (spec, "Exceptions").

   - `io` depends on `core` (for the `Intent` type) and on what the I/O needs
     (`otty-containers` with the client, `tokio`, `notify`, `otty-libterm`).
     It turns each answer into the feature's `Intent`.
   - `core` never depends on `io`. An `io` crate never depends on another
     feature.
   - Feature `core` crates keep the bans of `crates/app/features/clippy.toml`.
     `io` crates need their own `clippy.toml` (a nearer file replaces the
     shared one), with the root thresholds and no I/O bans.
   - The binary keeps only the calls: in `dispatch/<feature>.rs` an outcome
     arm calls the `io` crate and wraps the answer into a `Task`; in
     `subscription.rs` it runs the `io` crate's streams.
2. **The shell helper crate** `otty-app-shell` in `crates/app/shell/`. Two
   features need the shell integration (new shell tabs of the terminal, and
   quick launch commands that start a plain zsh or bash), so it is a helper,
   not part of one feature. It holds the integration scripts and their writing
   to disk, `ShellSession`, and adding the integration to the options of a
   local shell. The binary writes the files at start and when the shell
   changes in Settings; the terminal gets a ready `ShellSession`;
   `otty-app-quick-launch-io` adds the integration to a launch.
3. **Subscriptions.** A component gives a subscription that does no I/O
   (timers, window frames, its own terminals' events) itself through
   `subscription()`. For timers, turn on iced's `tokio` feature in the crates
   that need `iced::time::every` (notifications, quick launch, containers);
   it adds no new crate to `Cargo.lock`. Replace their timer getters
   (`toast_timer()`, `tick_timer()`, `engine_search_timer()`) with
   `subscription()`. A subscription that does I/O (file watching, Docker
   events, logs) lives in the feature's `io` crate; the feature says what to
   watch through a getter (`watched_folders()`, `watched_engines()`). Ban
   `iced::Subscription::run` and `run_with` in
   `crates/app/features/clippy.toml`; plant a call first and see clippy fail.
4. **The system language goes back to `otty-app-i18n`.** Which language the
   system wants is a question of the translations; the old app kept
   `detect_system_locale` in i18n, and issue 02 moved it out only to keep the
   crate free of I/O. A helper crate may hold I/O that belongs to it by
   meaning, as `otty-app-shell` does. Add `system_locale()` (through
   `sys-locale`, already in `Cargo.lock`) next to the pure
   `locale_from_system_preference`; the binary calls
   `set_locale(system_locale())` at start and when Settings follows the
   system.
5. Update the spec ("Crates", "Binary", "Boundaries") so it describes the
   `io` crates, the shell helper and the subscription rule.

After this the binary's own I/O is only what belongs to the whole app: the
host name and the pinned modes file of the frame.

## Done when

- `effects/` and `subscription/` of the binary hold no feature-specific I/O;
  each such piece lives in its feature's `io` crate or in `otty-app-shell`.
- No `core` feature crate depends on `bollard`, `notify` or an `io` crate.
- The timers run from the components' own `subscription()`.
- Behaviour does not change; the old `app` is unchanged.
- fmt, clippy, `cargo deny check` and the tests pass, and their results are
  reported.

## Comments

### 2026-11-10: io crates, the shell helper and the feature timers

**The shell helper.** `otty-app-shell` (`crates/app/shell`) holds
`ShellSession`, `shell_session_options`, the integration scripts and
writing them (`with_shell_integration`, `integrated_shell_session`,
`shell_integration_dir`), moved out of the binary with their tests. Its own
`clippy.toml` has the root thresholds and no I/O bans. The binary keeps
only `effects/shell_integration.rs::integrate`, which writes the files in the
background at start and when the shell changes in Settings;
`otty-app-quick-launch-io` adds the integration to a plain zsh or bash
launch. `shell_name` stays in `otty-app-domain`.

**The system language.** `otty_app_i18n::system_locale()` (through
`sys-locale`) sits next to `locale_from_system_preference`; the binary's
`effects/locale.rs` calls `set_locale` with the saved language or
`system_locale()`. The i18n crate falls under the root `clippy.toml`, whose
bans do not cover `sys-locale`, so it needs no file of its own. The binary
no longer depends on `sys-locale`.

**The io crates.** Explorer, settings, quick launch and containers moved to
`crates/app/features/<f>/core` (package names unchanged), and each got
`crates/app/features/<f>/io` as `otty-app-<f>-io` with its own
`clippy.toml` (root thresholds, no I/O bans):

- `otty-app-explorer-io`: the folder reads and the file watch
  (`watch_folders`), from `effects/explorer.rs`,
  `subscription/explorer_watch.rs` and `subscription/changed_folders.rs`;
  depends on the core, `notify`, `log`, `thiserror`, iced (`tokio`);
- `otty-app-settings-io`: the settings file (`read_at_start`, `read`,
  `write`), from `effects/settings.rs`; depends on the core, `serde_json`,
  `log`, `thiserror`, iced (`tokio`);
- `otty-app-quick-launch-io`: the launches file (`load_at_start`, `save`)
  and the check before a launch runs (`prepare`), from
  `effects/quick_launch.rs`, split into `launches_file.rs` and
  `launch_check.rs`; depends on the core, `otty-app-domain`,
  `otty-app-i18n`, `otty-app-shell`, `otty-libterm`, `otty-ui-term`,
  `serde_json`, `log`, `thiserror`, iced (`tokio`);
- `otty-app-containers-io`: the engine search and calls, the pulls, the Run
  form's folder checks and waits, the browser and the clipboard, the events
  watch (`watch_engines`) and the followed logs (`follow_logs`), from
  `effects/containers.rs`, `subscription/engine_watch.rs` and
  `subscription/followed_logs.rs`; depends on the core, `otty-containers`
  with its client, `log`, iced (`tokio`).

Each returns `Task`s and `Subscription`s that end in the feature's `Intent`;
`dispatch/<f>.rs` and `subscription.rs` only call them and `.map` the intent
into `AppEvent`. The binary lost `notify`, `sys-locale` and the engine
client (`otty-containers` and `otty-libterm` are now only its
dev-dependencies). Left in the binary's `effects/`: `host_name.rs`,
`modes.rs` (the pinned modes file, issue 18), `locale.rs`,
`shell_integration.rs` and `startup.rs`; `subscription/` is gone and
`subscription.rs` no longer lifts the clippy bans.

**Subscriptions.** Notifications, quick launch and containers give their
timers through their own `Feature::subscription()`, with iced's `tokio`
feature on for `iced::time::every`; `Cargo.lock` gained no package (only the
workspace's own new crates). `toast_timer()`, `tick_timer()`,
`engine_search_timer()` and `folder_recheck_timer()` are gone with the
binary's timer functions; each is now a private period (`toast_period`,
`tick_period`, `engine_search_period`, `folder_recheck_period`) whose tests
stayed, and the tests check `subscription().units()` too. The binary's tests
that asked `tick_timer().is_some()` ask the subscription. The getters
`watched_folders()`, `watched_engines()` and `followed_logs()` stay.

`crates/app/features/clippy.toml` now bans `iced::Subscription::run` and
`run_with`, and its comment says it covers the core crates only (an io
crate's nearer file replaces it). A planted `Subscription::run` and
`run_with` in `otty-app-notifications`, and a `run_with` in
`otty-app-containers`, failed clippy with `use of a disallowed method
iced::Subscription::run_with` and the note `a feature's I/O stream lives
in its io crate`; they were removed. The io crates, which use `run_with`,
pass. The ban reasons of that file now name the io crate.

**Tests ported from issues 13 and 14.** The old screen tests of the
no-engine card,
`dragging_the_panel_split_across_the_card_does_not_scroll_it` and
`dragging_a_bar_of_the_card_area_only_scrolls_it`, are in
`screens/containers.rs` with the helpers `app_with_card` and
`denied_podman_search` in `testing/window.rs`.

**Behaviour.** No change is meant. Two details differ in how it is built:

- The `cfg!(test)` guards of the settings and launches file paths hold only
  for the tests of their io crates now. One binary test,
  `an_opened_dialog_takes_the_keyboard_and_reads_the_file`, polls the read
  task of the settings file, so it reads (never writes) the settings file of
  the user running it; the answer is dropped.
- `otty-app-quick-launch-io` builds the settings of a launch with a private
  `settings_for_session`, the same three lines as the binary's
  `terminal_settings::for_session`, because an io crate cannot call the
  binary.

**Checks.** `cargo tree -p otty-app-<f> -e normal | grep -E
"bollard|notify|otty-app-.*-io"` prints nothing for each feature core
crate. `grep -rln "std::fs\|notify::\|otty_containers::Engine\|bollard"
crates/app/otty/src/effects crates/app/otty/src/subscription.rs` prints only
`effects/modes.rs`. `git diff --stat ee8c9de -- app` is empty. `cargo
+nightly fmt --all -- --check`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo deny check` (the 3 known
yanked-crate warnings), `cargo test --workspace --all-features` (6309
passed, 5 ignored; `otty-app` 599, `otty-app-containers` 1094,
`otty-app-containers-io` 30, `otty-app-explorer-io` 21,
`otty-app-quick-launch-io` 33, `otty-app-settings-io` 12, `otty-app-shell`
8) and `cargo build -p otty-app` pass. `cargo llvm-cov` was not run, as
asked.
