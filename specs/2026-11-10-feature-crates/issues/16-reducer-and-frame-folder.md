# 16 Reducer trait and the frame folder

Status: needs-triage
Blocked by: 14

## What to do

Two renames the owner chose once the features were in place. They only move
and rename code; behaviour does not change.

1. **Rename the `Feature` trait to `Reducer`** in `otty-app-slots`, with every
   use. The trait only says how a part of the app changes its state (an
   `Intent` in, `reduce`, `Outcome`s out), and the frame widgets implement it
   too, so the name "feature" promised more than it checks. The place traits
   (`LeftPanel`, `MainArea`, `RightPanel`, `StatusBar`, `Overlays`) keep their
   names and now extend `Reducer`. Update the spec and AGENTS.md wording that
   names the trait.
2. **Gather the frame widgets in one folder** of the binary:
   `crates/app/otty/src/frame/{chrome,sidebar,modes}`. `view.rs` and
   `screens/` stay where they are: they hold no state and only compose the
   window from the frame and the features.

## Done when

- No `Feature` trait is left; `grep -rn "Feature for\|: Feature\|slots::Feature" crates/app`
  prints nothing.
- The frame widgets live under `crates/app/otty/src/frame/`.
- The old `app` is unchanged.
- fmt, clippy, `cargo deny check` and the tests pass, and their results are
  reported.

## Comments
