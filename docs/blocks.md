# Block-Oriented Terminal API

OTTY treats everything that happens in the shell as a **block**. The engine
splits the scrollback into semantic fragments — prompts, commands, and
fullscreen blocks — and keeps metadata for each fragment so that UI layers and
external tools can build overlays on top of the terminal output without guessing
where one command ends and another begins.

## Block kinds

Blocks are described by [`BlockSnapshot`](../otty-surface/src/block.rs) and have
an attached [`BlockMeta`] record. Three kinds are currently emitted:

- `BlockKind::Prompt` — the shell prompt before user input.
- `BlockKind::Command` — user input + the produced output.
- `BlockKind::Fullscreen` — commands that temporarily enter the alt-screen.

Prompt blocks never expose textual contents because overlay components usually
skip them.

## Accessing block data

`otty_ui_term::Terminal` mirrors the engine state and exposes convenience
helpers for block traversal:

- `terminal.blocks()` returns a cloned `Vec<BlockSnapshot>` for the latest
  frame so that callers do not have to depend on `SnapshotArc` directly.
- `terminal.block_text(block_id)` aggregates the characters for the supplied
  block and returns a `String` (excluding prompts or empty blocks).
- `terminal.block_prompt_text(block_id)` returns only the prompt/input line for
  a command block, if available.
- `terminal.snapshot_arc()` gives read-only access to the underlying
  `SnapshotArc` when more advanced operations are required.

Geometry for block overlays is computed with `otty_ui_term::block_rects`. The
function accepts the current `SnapshotView`, layout origin/size, and the cell
height, returning a list of [`BlockRect`] items that describe where every block
sits inside the rendered viewport.

```rust
let snapshot = terminal.snapshot_arc();
let view = snapshot.view();
let rects = otty_ui_term::block_rects(
    &view,
    layout_origin,
    layout_size,
    cell_height,
);
```

## Command & event flow

`TerminalView::command(widget_id, BlockCommand)` is the unified way to trigger
UI actions against a block. The following commands are available:

- `BlockCommand::Select(block_id)` — highlights a block in the widget.
- `BlockCommand::SelectHovered` — highlights the block currently hovered by
  the pointer, if any.
- `BlockCommand::CopySelection` — copies the current grid selection (if any)
  without changing block focus.
- `BlockCommand::ScrollTo(block_id)` — scrolls the viewport until the block is
  visible.
- `BlockCommand::Copy(block_id)` — copies block text to the clipboard (and
  implicitly selects it).
- `BlockCommand::CopyContent(block_id)` — copies block contents without the
  prompt line.
- `BlockCommand::CopyPrompt(block_id)` — copies only the prompt/input line.
- `BlockCommand::CopyCommand(block_id)` — copies the parsed command line
  (`BlockMeta::cmd`) without prompt decoration.
- `BlockCommand::PasteClipboard` — writes the standard clipboard contents into
  the terminal.

`TerminalView` emits block-specific events through `otty_ui_term::Event` so that
applications can react to user interactions:

- `Event::BlockSelected { id, block_id }` — a block became the active
  selection, either through clicks or `BlockCommand::Select`.
- `Event::BlockCopied { id, block_id }` — block contents were copied to the
  clipboard, either through UI affordances or `BlockCommand::Copy`.

Applications that want to draw their own overlay should switch terminals into
`BlockUiMode::ExternalOverlay` (via `Terminal::set_block_ui_mode`) and paint the
additional chrome themselves using the geometry helpers above. The
`otty-ui/terminal/examples/blocks_overlay.rs` example demonstrates how to:

1. Collect block rectangles on every frame.
2. Draw custom buttons aligned to each block.
3. Dispatch `BlockCommand`s when a button is pressed.
4. Listen for `Event::BlockSelected`/`Event::BlockCopied` to stay in sync with
   the terminal engine.

[BlockMeta]: ../otty-surface/src/block.rs
[`BlockRect`]: ../otty-ui/terminal/src/block_layout.rs
