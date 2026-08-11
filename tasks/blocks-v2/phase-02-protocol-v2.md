# Phase 2: protocol v2 parser

Status: **partially complete**.

Parent document: [Blocks v2 specification](spec.md). IDs: B2-020–B2-027.

## Goal

Introduce a bounded, versioned wire protocol between shell hooks and the terminal backend. The
parser must survive fragmented, malformed, and oversized input plus foreign-session events
without panic, unbounded allocation, or block-lifecycle corruption.

## Current state

`event-v2;h`, bounded hex and JSON parsing, the typed envelope, all primary semantic events,
OSC 133 A/B, terminal-session validation, per-shell sequence diagnostics, and wire-format
documentation are implemented. Session IDs come from system `/dev/urandom`, so no new
dependency was required.

The v1 `block` parser, schema, and production action dispatch have been removed. Fragmented and
malformed legacy-v1 frames are safely ignored, after which the parser continues accepting v2.
The remaining work is a randomized arbitrary-DCS allocation and panic test.

## Scope

- [x] **B2-020** Add v2 framing, schema, recovery, and fragmented-stream tests.
- [x] **B2-021** After the failing legacy-ignore regression from B2-002, remove v1
  `dcs/block.rs`, `DcsMessageKind::Block`, and production dispatch to `Action::BlockEvent`.
  Legacy `otty-dcs;block` must be discarded as unsupported DCS without creating a semantic
  event.
- [x] **B2-022** Add bounded framing and hex decoding with a required major version.
- [x] **B2-023** Add semantic events `shell_hello`, `prompt_prepare`, `command_start`,
  `command_end`, `context_update`, `shell_exit`, and `integration_error`.
- [x] **B2-024** Add OSC 133 A/B and semantic prompt boundaries.
- [x] **B2-025** Register system-random sessions and reject foreign or missing sessions.
- [x] **B2-026** Validate per-shell sequences and emit payload-free diagnostics.
- [x] **B2-027** Document wire format, limits, and Bash/Zsh examples in
  `otty-escape/README.md`.
- [ ] Add a deterministic or randomized byte-stream test proving that the buffer remains
  bounded for arbitrary DCS input.

## Security contract

- decoded payload is at most 32 KiB and encoded payload at most 64 KiB;
- commands and identifiers have separate, smaller limits;
- an unknown major version is not applied to the model;
- malformed hex, JSON, or UTF-8 is discarded and the parser continues accepting later bytes;
- diagnostics contain only an error type and safe IDs or revisions, never command/output text;
- the terminal accepts events only for a registered `TerminalSessionId`.

## Automated verification

```bash
cargo test -p otty-escape dcs
cargo test -p otty-escape osc
cargo test -p otty-surface block::lifecycle
cargo clippy -p otty-escape --all-targets --all-features -- -D warnings
```

## Manual verification

1. Launch `cargo run -p otty` and wait for `Integration v2 active`.
2. Run several ordinary commands and confirm that prompt and command completion still create
   blocks.
3. Send malformed DCS with `printf '\033Pevent-v2;h;zz\033\\'`.
4. Immediately run `printf 'parser-alive\n'`. The terminal must continue working and the
   malformed sequence must not create or finish a block.
5. Send an encoded payload larger than 64 KiB, then run another ordinary command. The UI must
   neither hang nor retain noticeably more memory after discard.
6. Open a second terminal tab. Events and block IDs from the two tabs must not mix.
7. In Bash and Zsh, run commands containing Unicode, quotes, newline/heredoc, and ESC/BEL-like
   text. Block metadata must contain the original command without control-sequence injection.
8. Repeat fragmented and unsupported-version cases through the matching parser tests with
   `--nocapture`, confirming that diagnostics never print the payload.

The phase is complete after the production v1 parser is removed and legacy-ignore,
malformed, oversized, and foreign-session scenarios cannot corrupt adjacent blocks.
