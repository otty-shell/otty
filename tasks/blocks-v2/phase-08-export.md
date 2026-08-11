# Phase 8: export and save

Status: **not started**.

Parent document: [Blocks v2 specification](spec.md). IDs: B2-080–B2-085.

## Goal

Create one backend pipeline that exports any section of an on-screen or off-screen block as
Plain, ANSI, Markdown, or versioned JSON. Clipboard and Save must use the same result;
filesystem I/O runs outside the render thread and returns an explicit success or error.

## Current state

The initial slice can copy some semantic content from a snapshot, but this is a temporary UI
helper. There is no unified `ExportBlock` query, formatter set, golden test suite, atomic save,
or `otty.block` schema, so no B2-080–B2-085 item is complete.

## Scope

- [ ] **B2-080** First add golden tests for Plain, ANSI, Markdown, and JSON; soft wrap, wide
  characters, hyperlinks, trailing spaces, and truncation markers.
- [ ] **B2-081** Implement one `ExportBlock` pipeline in
  `otty-surface/src/block/export.rs` with Prompt, Command, Output, and Whole selection.
- [ ] **B2-082** Move every clipboard entry point to the backend export result and remove
  viewport or cached-text fallback.
- [ ] **B2-083** Implement atomic save in the application layer with an explicit result or
  error event, a temporary file in the destination directory, flush/sync, and rename.
- [ ] **B2-084** Define a versioned `otty.block` JSON schema and round-trip tests without
  serializing internal `Surface` state.
- [ ] **B2-085** Before session persistence, agree on secret and redaction policy and request
  separate approval for any storage or compression dependencies.

## Format contract

- Plain removes terminal fill spaces, preserves meaningful internal spaces, and does not turn
  soft wrap into a hard newline.
- Wide-character spacer cells are not exported twice.
- ANSI reconstructs only supported SGR and hyperlink semantics and always has valid reset and
  termination.
- Markdown contains separate command/output fences and a stable metadata section.
- JSON has its own schema name and version independent of the live protocol version.
- `Whole` always orders prompt → command → output; `Output` never contains the prompt.
- Truncated content includes a marker and the available count of lost lines.

## Automated verification

```bash
cargo test -p otty-surface block::export
cargo test -p otty-ui-term --all-features
cargo test -p otty terminal_workspace
```

Golden fixtures must be small and readable and verify exact bytes. The JSON round-trip test
must reject an unsupported major schema and preserve unknown safe optional fields according to
the selected migration policy.

## Manual verification

1. Launch `cargo run -p otty` and create a block with Unicode and wide characters, colored ANSI
   output, an OSC 8 hyperlink, a long soft-wrapped line, and meaningful internal spaces.
2. Resize so wrapping changes and run Copy Output. Plain text must not acquire new hard
   newlines from reflow.
3. Copy Prompt, Command, Output, and Whole in every format and compare them with the contract
   above.
4. Scroll the block outside the viewport, then collapse or hide it and repeat export. The
   result must be byte-identical to the visible expanded block.
5. Save the same export and compare file bytes with the clipboard/backend result. Copy and Save
   must not use different formatters.
6. Open ANSI output in a compatible terminal, Markdown in a preview, and JSON in a parser;
   control sequences, fences, and schema must be valid.
7. Exceed the output budget and verify an explicit truncation marker in every format.
8. Save over an existing file, then simulate permission denied and interruption before rename.
   Success atomically replaces the file; failure leaves the original intact and shows the
   reason in the UI.
9. Inspect export diagnostics and logs; command and output contents must not appear.

The phase is complete when every UI action uses one backend pipeline, off-screen export needs
no scroll or render, and clipboard and atomic save produce identical bytes.
