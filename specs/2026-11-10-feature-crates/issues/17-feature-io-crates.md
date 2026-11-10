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
4. Update the spec ("Crates", "Binary", "Boundaries") so it describes the
   `io` crates, the shell helper and the subscription rule.

After this the binary's own I/O is only what belongs to the whole app: the
system language, the host name and the pinned modes file.

## Done when

- `effects/` and `subscription/` of the binary hold no feature-specific I/O;
  each such piece lives in its feature's `io` crate or in `otty-app-shell`.
- No `core` feature crate depends on `bollard`, `notify` or an `io` crate.
- The timers run from the components' own `subscription()`.
- Behaviour does not change; the old `app` is unchanged.
- fmt, clippy, `cargo deny check` and the tests pass, and their results are
  reported.

## Comments
