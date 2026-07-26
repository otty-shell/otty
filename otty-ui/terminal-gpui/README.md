# OTTY GPUI terminal widget

`otty-ui-term-gpui` is an embeddable terminal `Entity<Terminal>` for GPUI. It
owns an `otty-libterm` runtime without owning the host window, emits semantic
GPUI events, and supports replacing local, SSH, or custom backends without
recreating the entity.

## Embed a terminal

```rust,no_run
use gpui::{Context, Entity, IntoElement, Render, Window, div, prelude::*};
use otty_ui_term_gpui::{
    LocalBackend, LocalOptions, Terminal, TerminalConfig,
};

struct Workspace {
    terminal: Entity<Terminal>,
}

impl Workspace {
    fn new(cx: &mut Context<Self>) -> Self {
        let terminal = cx.new(|cx| {
            Terminal::new(
                TerminalConfig::default(),
                LocalBackend::new(LocalOptions::default()),
                cx,
            )
        });

        Self { terminal }
    }
}

impl Render for Workspace {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().size_full().child(self.terminal.clone())
    }
}
```

Keep the `Subscription` returned by `cx.subscribe` or `cx.subscribe_in` to
receive `TerminalEvent`. Large frames remain available through
`Terminal::snapshot_arc()` and are not copied into emitted events.

Runtime configuration uses ordinary entity updates:

```rust,ignore
terminal.update(cx, |terminal, cx| {
    terminal.set_theme(new_theme, cx);
    terminal.replace_backend(LocalBackend::new(new_options), cx);
});
```

Validated configuration parts can also be assembled selectively:

```rust
use otty_ui_term_gpui::{TerminalConfig, TerminalFont};

let config = TerminalConfig::builder()
    .font(TerminalFont::monospace(15.0)?)
    .build()?;
# Ok::<(), otty_ui_term_gpui::ConfigError>(())
```

The crate exports typed GPUI actions for copy, paste, selection, and scrolling.
Host keymaps can bind them with normal `gpui::KeyBinding` values. Printable
text is committed through GPUI's input handler, so IME input is sent exactly
once; terminal control/navigation keys use `TerminalBindings`.

## Examples

- `full_screen`: local shell, title events, copy/paste actions;
- `split_view`: two independent terminal entities and focus handles;
- `themes`: runtime palette replacement;
- `fonts`: runtime font metrics and PTY resize;
- `bindings`: terminal-local mode-aware key binding;
- `blocks_overlay`: host-owned block overlay;
- `backend_switching`: backend replacement in the same entity.

Run one with:

```text
cargo run -p otty-ui-term-gpui --example full_screen
```

## Platform notes

GPUI 0.2.2 enables Linux X11/Wayland support by default. Linux build hosts need
the XCB and xkbcommon development packages (for example `libxcb-devel` and
`libxkbcommon-devel` on Fedora). macOS uses GPUI's native platform backend.
Windows and WASM are outside this widget's current release matrix.
