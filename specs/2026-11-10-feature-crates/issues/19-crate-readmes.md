# 19 A README for every crate

Status: needs-triage
Blocked by: 16

## What to do

Give every crate that has no `README.md` a short one that says, in plain
words, why the crate exists. This covers every crate under `crates/app/`
(features with their `core` and `io` crates, helpers, the binary) and
`crates/containers`. Crates that already have a README (`vte`, `escape`,
`pty`, `surface`, `libterm`, `ui/*`) keep theirs. The old `app` crate is not
touched.

Each README is a few lines a person reads in ten seconds:

- the crate name as the title;
- one or two sentences: what part of OTTY it is and why it is a separate
  crate (for example "the containers feature without I/O: its state,
  decisions and views; the Docker calls live in `otty-app-containers-io`");
- for a feature: which places of the window it fills;
- what it must not depend on, if that is the reason it exists.

No module lists, no type lists, no data flow: those go stale and the code
shows them.

Point `readme` in each crate's `Cargo.toml` at its own `README.md`.

Add a rule to AGENTS.md: every new crate gets such a README when it is
created.

## Done when

- Every workspace crate except the old `app` has a `README.md`, and its
  `Cargo.toml` points at it.
- AGENTS.md has the rule.
- `cargo deny check` and `cargo test --workspace --all-features` (doc tests
  included) pass.

## Comments
