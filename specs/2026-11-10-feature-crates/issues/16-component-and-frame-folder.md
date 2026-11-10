# 16 Component trait and the frame folder

Status: needs-triage
Blocked by: 18

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
   `crates/app/otty/src/frame/{chrome,sidebar}` (modes became a feature in issue 18). `view.rs` and
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

### 2026-11-10: Component trait and the frame folder

**The trait.** `crates/app/slots/src/feature.rs` is now `component.rs`, and
the trait `Feature` is `Component`, with every `impl`, bound, import and doc
link under `crates/app/` (the features, the frame widgets, the binary's
`dispatch/`, `screens/`, tests and the doc examples of `otty-app-slots`).
The place traits keep their names and extend `Component`. The trait's doc
now says what it is: how a part of the app with its own state changes it
(`Intent` in, `reduce`, `Outcome`s out) and what it listens to
(`subscription`), for a feature and a frame widget alike; it has no view,
a component draws through the place traits or its own methods. The variant
`SectionMessage::Feature` keeps its name: it is the press on the feature's
own part of a left panel section, not the trait. The word "feature" stays
where it means a feature crate.

**The frame folder.** `crates/app/otty/src/chrome/` and `sidebar/` moved to
`crates/app/otty/src/frame/{chrome,sidebar}`; `frame/mod.rs` says the frame
is the part of the window that composes several components. `view.rs` and
`screens/` stay where they were. Only the `crate::chrome` and
`crate::sidebar` paths changed.

**Docs.** The spec's socket section names `Component`, says features and
the frame widgets implement it and why it has no view, and the binary
section lists `frame/`. AGENTS.md named neither the trait nor the frame
folder, so it did not change. The old `app` is unchanged.
