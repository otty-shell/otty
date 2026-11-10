# 23 One delivery for every component

Status: needs-triage

## Why

Each of the 11 files in `crates/app/otty/src/dispatch/` (title bar, sidebar,
modes, notifications, explorer, palette, settings, tabs, terminal, quick
launch, containers) had its own copy of `pub(crate) fn deliver(app,
intent)`: build the component's ctx, call `reduce` once, read every outcome
with a private `read`, and batch the tasks. Only the ctx and `read`
differed. Eleven copies of the same steps can drift apart: one component
could, for example, read only its first outcome, and nothing would catch it.

## What to do

1. Add `dispatch/delivery.rs` with the `Delivery` trait (`Component`,
   `parts`, `read`) and the one generic `deliver(app, intent)`: take the
   component and its ctx from `parts`, reduce once, read every outcome in
   order, batch the tasks.
2. Turn each `dispatch/<x>.rs` into `impl Delivery for <X>Intent`: the ctx
   building goes into `parts`, `read` keeps its match and its comments, and
   the file keeps its tests. Helpers such as `terminal::base_settings` stay.
3. Replace every `<x>::deliver(app, ...)` in the binary with
   `deliver(app, ...)`. Commands and the window events stay plain
   functions.
4. Write a test of the shared behavior first: one intent whose `reduce`
   gives several outcomes has every outcome read in the same update.
5. Update the module doc of `dispatch/`, the doc of `Component` in
   `otty-app-slots` and the spec.

The feature crates do not change, and the behaviour of the app does not
change.

## Done when

- `dispatch/delivery.rs` holds the only `deliver` of a component, and
  `grep -rn "::deliver(" crates/app/otty/src` shows only `dispatch::deliver`
  and `command::deliver`.
- Each `dispatch/<x>.rs` holds only `parts`, `read` and its own helpers.
- The binary has every test it had before, plus the new one.
- All checks from AGENTS.md pass, and `app/` is unchanged.

## Comments

### 2026-11-11: one deliver, eleven `Delivery` impls

**What changed.**

- `dispatch/delivery.rs` holds `Delivery` and `deliver`. The trait is
  implemented on each intent type in the binary, so a call names only the
  intent: `deliver(app, TabsIntent::CloseActive)`. The feature crates know
  nothing about it.
- Each `dispatch/<x>.rs` is now `impl Delivery for <X>Intent` with `parts`
  and `read`; the `read` matches and their comments are as they were. The
  palette's `parts` returns the shared borrows of the tabs, the launches and
  the modes next to the mutable borrow of the palette, which the compiler
  allows because they are different fields of `App`.
- `command.rs` and the window events in `dispatch/mod.rs` stay plain
  functions and call `dispatch::deliver`. `command::deliver` keeps its name.
- Nothing outside `dispatch/` calls a component's file any more, so the
  submodules of `dispatch/` are now private; `dispatch` re-exports
  `Delivery` and `deliver`.
- New test `dispatch::delivery::tests::every_outcome_of_one_intent_is_read_in_the_same_update`:
  opening the launch editor in Containers mode gives three tab outcomes,
  and in one call quick launch shows the editor, the returned task scrolls
  the sessions list to its end and the modes switch to Terminal mode. It
  was written first and failed to build before `deliver` existed.
- Docs: the module doc of `dispatch/`, the doc of `Component` in
  `otty-app-slots` (it said nothing calls components through the trait
  generically, which is no longer true) and the spec (the binary's
  `dispatch/` entry, "Why traits", the flow, a new "Delivery" section and
  step 6 of "Moving a feature").

**Checks.**

- `otty-app` tests: 525 before and 526 after. The lists match except for
  the one new test; no test was lost or renamed.
- `cargo +nightly fmt --all`: clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
  clean.
- `cargo deny check`: advisories, bans, licenses and sources ok; only the
  three known yanked-crate warnings (`clipboard-win`, `core2`, `spin`).
- `cargo test --workspace --all-features`: 6323 passed, 0 failed, 5 ignored
  (the same ignored tests as before this issue).
- `grep -rn "::deliver(" crates/app/otty/src`: only `dispatch::deliver` and
  `command::deliver`.
- `git diff --stat 66c58fe -- app`: empty.
- `cargo llvm-cov` was not run for this issue.
