//! Development tools for OTTY.
//!
//! The architecture check reads the `app` sources as plain text and reports
//! every place that breaks a rule of `docs/internal/architecture.md`. The
//! known places are listed, each with its reason, in
//! [`architecture::allowlist::ALLOWLIST`]; the test in
//! `xtask/tests/architecture.rs` fails on anything else. `cargo xtask arch`
//! prints the same report for reading: every accepted place with its
//! reason, and what is new or stale.

pub mod architecture;
