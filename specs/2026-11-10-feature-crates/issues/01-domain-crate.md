# 01 Domain crate

Status: needs-triage
Blocked by: —

## What to do

Create the `crates/app/` folder and the first helper crate, `otty-app-domain`,
in `crates/app/domain`. It has no iced and no I/O.

Copy into it from the old app (the old files stay where they are):

- `app/src/domain/` (`Mode`, `ExitBehavior`, `QuickLaunch`);
- `app/src/commands.rs` (`AppCommand`);
- the shortcut names and their labels from `app/src/shortcuts.rs`
  (`AppShortcut`), without the iced key matching, which goes to the binary in
  issue 05;
- `app/src/shell_session.rs`.

The types one feature hands to another (`TabLaunch`, `TabMachine`,
`WizardTabInit`) are added later, by the issue of the first feature that needs
each of them.

Add the workspace members for `crates/app/`. Do not touch the `app` member.

Prove how clippy finds its settings for the coming feature crates: put a
`clippy.toml` with the feature bans (see the spec, "Clippy bans") in
`crates/app/features/`, add a throwaway crate under it that calls
`std::fs::read`, and check that clippy fails on it. Then remove the throwaway
crate. Write in this issue's comments whether one shared file works or each
feature crate needs its own copy.

## Done when

- `otty-app-domain` builds without iced in its dependency tree.
- The copied tests pass in the new crate.
- The clippy check above is done and its result is written down.
- The old `app` is unchanged.
- fmt, clippy, `cargo deny check`, the tests and the coverage comparison from
  AGENTS.md are run, and their results are reported.

## Comments
