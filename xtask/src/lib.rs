//! Development tools for OTTY.
//!
//! The architecture check reports every place in the `app` sources that
//! breaks a rule of `docs/internal/architecture/app/`. It has no rules for
//! now: they are written again once the architecture change in
//! `specs/2026-11-09-split-router/spec.md` settles. The
//! known places are listed, each with its reason, in
//! [`architecture::allowlist::ALLOWLIST`]; the test in
//! `xtask/tests/architecture.rs` fails on anything else. `cargo xtask arch`
//! prints the same report for reading: every accepted place with its
//! reason, and what is new or stale.
//!
//! The widget generator, [`new_widget::create`], behind
//! `cargo xtask new-widget <name>`, creates a widget with the files that
//! the architecture asks for and wires it into the app. The test in
//! `xtask/tests/new_widget.rs` runs it on a copy of `app/src` and checks the
//! copy against the same allowlist.

pub mod architecture;
pub mod new_widget;
