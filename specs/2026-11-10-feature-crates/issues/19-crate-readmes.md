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

### 2026-11-10: a README for every crate

**The READMEs.** 22 crates had no `README.md` and got one: every crate under
`crates/app/` (the binary `otty-app`, the helpers `domain`, `i18n`, `theme`,
`slots`, `shell`, `testing`, and each feature with its `core` and `io`
crates) and `crates/containers`. Each one says in a few plain sentences what
part of OTTY the crate is and why it is separate; a feature also says which
places of the window it fills, and a `core` crate says what it must not do
itself (read files, call an engine, start programs) and which `io` crate does
it. The crates that already had a README (`vte`, `escape`, `pty`, `surface`,
`libterm`, `ui/*`) and the old `app` crate are unchanged.

**Cargo.toml.** Each of the 22 crates has `readme = "README.md"`. The binary
pointed at the repository README (`../../../README.md`); it now points at its
own.

**AGENTS.md.** A rule in the Rules list: every new crate gets a short
`README.md` when it is created, saying in plain words why it exists, and its
`Cargo.toml` points at it.
