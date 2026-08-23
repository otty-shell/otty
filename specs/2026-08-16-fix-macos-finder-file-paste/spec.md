# Fix macOS Finder file paste path

Created: 2026-08-16
Agent: Codex
Status: COMPLETE
Approved: Yes
Iterations: 0
Worktree: No
Type: Bugfix

## Summary

Copying a file in macOS Finder and pressing `Cmd+V` in OTTY must paste the
file's decoded POSIX path. Finder publishes `public.file-url`, while iced's
clipboard path reads only its text representation.

## Behavior Contract

- A Finder file URL is converted to a decoded POSIX path before OTTY writes it
  to the PTY.
- Ordinary text paste keeps using the existing iced clipboard fallback.
- Malformed and non-file URLs are not converted into filesystem paths.
- Linux and Windows keep the existing text clipboard path.

## Implementation

- `crates/ui/terminal/src/clipboard.rs` owns the macOS pasteboard read and URL
  conversion.
- `crates/ui/terminal/src/input.rs` resolves paste text at that boundary before
  publishing `Event::Write`.
- `crates/ui/terminal/Cargo.toml` keeps the Objective-C dependencies macOS-only.
- `crates/ui/terminal/src/lib.rs` registers the private clipboard module.

## Verification

- RED: the focused input test failed because the PTY received the original
  `file://` URL instead of `/Users/example/My File テスト.txt`.
- GREEN: the focused input test passed, and all three clipboard adapter tests
  passed.
- `cargo +nightly fmt -- --check`: passed.
- `cargo test --workspace --all-features`: 530 passed, 2 ignored.
- `cargo deny check`: exited 0 with the existing yanked `core2 0.4.0` and
  `spin 0.9.8` warnings.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
  blocked by the three pre-existing macOS unused-symbol errors tracked in PR
  #87; the same failures reproduce on the unmodified base.
- `cargo llvm-cov --workspace --all-features --fail-under-lines 80`: exited 1
  at the repository's 67.81% line coverage. The changed terminal modules are
  above the requested level: `clipboard.rs` is 94.83% and `input.rs` is
  81.83%.
- Real macOS Finder verification: copying `otty Finder paste テスト file.txt`
  and pressing `Cmd+V` in the latest-`main` build inserted
  `/private/tmp/otty Finder paste テスト file.txt`, preserving spaces and the
  wide-character payload without exposing a `file://` URL.

## Not Verified

- Linux and Windows target builds were not run on this macOS host.
