# 15 Parity check

Status: needs-triage
Blocked by: 19

## What to do

Compare the new app with the old one before the owner removes the old one.

- Write a manual check list that walks through every mode, overlay and shortcut
  in both apps side by side. The owner runs it.
- Check the overlay rules from the spec ("Overlays") with each pair of overlays
  that can be open together.
- Check startup: shell integration files, host name, saved theme, saved
  launches and settings, including broken files.
- Search the new app for any `Task::done` sent from one feature to another and
  for I/O outside `effects/` and `subscription.rs`.
- Each difference found becomes its own issue in this folder.

The release still ships the old app. The steps for removing it are in the spec,
"When the old app is removed".

## Done when

- The check list is written and the owner has run it.
- Every difference is fixed or has its own issue.

## Comments

### 2026-11-10: automated checks, the check list and the start-up files

Checked on top of `1bf9729`. The old `app/` is unchanged
(`git diff --stat 1bf9729 -- app` is empty).

**Automated checks.**

- `grep -rn "Task::done" crates/app`: no message from one component to
  another. Two uses outside test helpers stay: `effects/startup.rs` asks for
  the first tab after the shell integration and the host name (the startup
  order, from the binary itself), and `otty-app-containers-io`
  (`effects.rs:306`) hands a pull's abort handle back to the form that
  started the pull (one feature talking to itself). The rest are in
  `otty-app/src/testing/task_outputs.rs` and its tests.
- I/O in the feature core crates and the single-crate features:
  `grep -rnE "std::fs|std::process|std::env|notify::|\bnotify\b|bollard|Subscription::run"`
  over their `src` and `Cargo.toml` finds only `std::process::ExitStatus` in
  the terminal (a data type of a finished shell, no process is started) and
  comments in the `Cargo.toml` files. A wider search for `Task::perform`,
  `Task::run`, `tokio::`, `File::`, `read_dir`, `Command::new` and `env::var`
  finds nothing but `::new` of plain types and `::default()`.
- I/O in the binary outside `effects/` and `subscription.rs`: only
  `app.rs`, which reads `HOME` and `SHELL` once and finds the config folder
  (the spec puts this in `app.rs`), and test helpers. `app.rs` also calls the
  `read_at_start`/`load_at_start` functions of the settings, quick launch and
  modes `io` crates, and `dispatch/` calls the `io` crates, as the spec says.
- Dependencies: `cargo tree -p <pkg> -e normal | grep -E
  "bollard|notify|otty-app-[a-z-]+-io"` prints nothing for each of the nine
  feature core crates. No feature crate (core or io) has another feature in
  its `cargo tree -e normal`, and no feature `Cargo.toml` names another
  feature.
- Gap marks: `grep -rn "issue [0-9][0-9]" crates/app` and
  `grep -rn "expect(dead_code" crates/app` print nothing; there is no
  `dead_code`, `TODO` or `FIXME` left in `crates/app`.

**Unported old tests.** 218 test names of `app/src` do not appear in
`crates/app`. The 142 of them about the binary, the terminal, containers
subscriptions, palette, tabs, modes and palette effects were read one by
one against the new code: 113 are covered under new names, 4 tested hops
that no longer exist, and 25 are kept in the code but have no new test
(the Containers screen scrolling and the panel drags across the table and
the Run form, the palette over a Containers dialog, toasts not counting as
an overlay, an inline edit cancelled by a row press or a terminal shortcut,
the logs after a panel drag or after leaving focus mode, New folder, New
launch, `SelectModeAt`, tabs kept across a mode switch, a smaller window).
None of them lost or changed behaviour. The rest (old effects, storage and
reducer tests) moved into the feature reducers and the `io` crates.

**Start-up files.** Both apps use one folder, `$HOME/.config/otty` (the
system temp folder plus `otty` without `HOME`):

| File | Old app | New app |
|---|---|---|
| `settings.json` | read in `App::new` (`widgets/settings/storage.rs`), written atomically through `settings.json.tmp` on Save | `otty-app-settings-io` `read_at_start`/`write` (`file.rs`), same name and atomic write |
| `quick_launches.json` | read in `App::new` (`widgets/quick_launch/storage.rs`), written atomically | `otty-app-quick-launch-io` `load_at_start`/`save` (`launches_file.rs`), same |
| `modes.json` | read in `App::new` (`widgets/modes/storage.rs`), written atomically | `otty-app-modes-io` `load_at_start`/`save` (`pinned_file.rs`), same |
| `otty.zsh`, `.zshrc` / `otty.bash`, `otty.bashrc` | written at start, when the shell changes in Settings, and for a quick launch of a plain zsh or bash (`shell_integration.rs`) | `otty-app-shell` `integrated_shell_session` / `with_shell_integration`, called with the config folder at the same three moments; the scripts are the same files from `assets/shell-integrations` |

The host name is read the same way (`nix::unistd::gethostname`). Each app
creates the folder before a write. Tests: the settings file name is pinned
by `the_settings_file_lies_in_the_config_folder`, the folder by
`the_config_folder_is_otty_in_the_users_config_folder`; the launches and
modes file names have no test of their own (read in code:
`launches_file.rs:128`, `pinned_file.rs:154`). A broken, missing or
unreadable file gives the defaults and a toast in both apps.

Two differences were found:

- The start toasts came in another order: the old app told the settings,
  the launches, then the pinned modes; the new one the settings, the pinned
  modes, then the launches. Fixed in 736a32f, `fix(app): tell start problems in the
  old order` (test `start_problems_come_in_the_order_the_files_are_read`
  first, then `App::new` reads and tells the launches before the modes).
- `HOME` that is not valid Unicode (already noted in issue 18): known
  difference K5.

**The check list** is `specs/2026-11-10-feature-crates/parity-checklist.md`:
start-up with broken files, the window frame, modes, terminal, quick launch
with SSH passwords and passphrases, explorer, palette, settings,
notifications, containers, Esc for each overlay, pairs of overlays, theme
and language. Its first section lists the known differences from issues
05–18 (K1–K8) for the owner to accept. The owner has not run it yet.

**Issues created.** [20 New launch under an open menu](20-new-launch-under-an-open-menu.md):
the start card's New launch… is now an app command, so under an open menu
it would close the menus and run, where the old app dropped it. Every menu
draws a dismiss layer, so the press may never reach the tile; the issue
says how to check.

**Checks.** `cargo +nightly fmt --all -- --check`, `cargo clippy
--workspace --all-targets --all-features -- -D warnings`, `cargo deny
check` (the 3 known yanked-crate warnings) and `cargo test --workspace
--all-features` (6319 passed, 0 failed, 5 ignored; `otty-app` 529) pass.
`cargo llvm-cov` was not run, as asked. The windows were not opened (no
display for the agent).
