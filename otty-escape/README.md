# otty-escape

`otty-escape` turns raw terminal bytes into high–level semantic actions that a
terminal renderer can understand. It wraps the low-level [`otty-vte`] parser and
maps the decoded escape sequences onto the strongly typed [`Action`] enum, so
terminal frontends can focus on rendering and state management instead of
decoding ECMA-48/xterm control flows.

This crate is one piece of the **OTTY** terminal stack:

```
┌─────────────┐   raw bytes    ┌─────────────┐   high level actions   ┌──────────────┐
│ PTY / shell │ ─────────────▶│ otty-escape │ ─────────────────────▶│ terminal UI  │
└─────────────┘                └─────────────┘                        └──────────────┘
```

## Quick start

```rust
use otty_escape::{Action, Actor, Parser};

#[derive(Default)]
struct Logger;

impl Actor for Logger {
    fn handle(&mut self, action: Action) {
        println!("{action:?}");
    }
}

fn main() {
    let mut parser = Parser::new();
    let mut actor = Logger::default();
    let bytes = b"Hello\x1b[1m world\x1b[0m!\n";

    parser.advance(bytes, &mut actor);
}
```

Run it with:

```
cargo run
```

You will see the individual `Print`, `SGR`, and `LineFeed` actions emitted by
the parser.

## Examples

- [log_actions](./examples/log_actions.rs) – prints every action produced for a byte stream; great for
  exploring the parser output.

Run any example with `cargo run --example <name>`.

## References

- [kitty](https://sw.kovidgoyal.net/kitty)
- [xterm](https://invisible-island.net/xterm)
- [xterm.js](https://xtermjs.org/docs/api/vtfeatures)
- [wezterm vtparse](https://github.com/wezterm/wezterm/tree/main/vtparse)
- [Alacritty VTE](https://github.com/alacritty/vte)

Many thanks to **wezterm**, **alacritty** and **xterm.js** for implementation examples