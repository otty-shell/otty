# 16 Component trait and the frame folder

Status: needs-triage
Blocked by: 14

## What to do

Two renames the owner chose once the features were in place. They only move
and rename code; behaviour does not change.

1. **Rename the `Feature` trait to `Component`** in `otty-app-slots`, with
   every use. The trait says how a part of the app changes its state and what
   it listens to (an `Intent` in, `reduce`, `Outcome`s out, `subscription`),
   and the frame widgets implement it too, so the name "feature" promised more
   than it checks. Not `Widget`: in this repo that is iced's drawing trait
   (`otty-ui-kit` and `otty-ui-term` implement it). The trait has no `view`:
   a component draws through the place traits (`LeftPanel`, `MainArea`,
   `RightPanel`, `StatusBar`, `Overlays`), one per place it fills, because one
   component can fill several places and the frame views draw data of several
   components. The place traits keep their names and now extend `Component`.
   Update the spec and AGENTS.md wording that names the trait.
2. **Gather the frame widgets in one folder** of the binary:
   `crates/app/otty/src/frame/{chrome,sidebar,modes}`. `view.rs` and
   `screens/` stay where they are: they hold no state and only compose the
   window from the frame and the features.

## Done when

- No `Feature` trait is left; `grep -rn "Feature for\|: Feature\|slots::Feature" crates/app`
  prints nothing, and the file `crates/app/slots/src/feature.rs` is now
  `component.rs`.
- The frame widgets live under `crates/app/otty/src/frame/`.
- The old `app` is unchanged.
- fmt, clippy, `cargo deny check` and the tests pass, and their results are
  reported.

## Comments
