# otty-vte

Lightweight, table-driven VT/ANSI virtual terminal parser.

otty-vte turns a byte stream from a pseudo terminal into high‑level terminal
actions: printing characters, executing C0/C1 controls, parsing ESC/CSI/DCS/OSC,
and dispatching the corresponding sequences.

Inspired by ECMA‑48 and DEC VT specs and by implementations such as
[alacritty/vte](https://github.com/alacritty/vte), [wezterm/vtparse](https://github.com/wezterm/wezterm/tree/main/vtparse), and [GNOME VTE](https://gitlab.gnome.org/GNOME/vte).

### Features

- Full state‑machine recognition of VT/ANSI families:
- Supports both 7‑bit (`ESC [`) and 8‑bit C1 introducers (`0x90`, `0x9B`, `0x9D`, …)
- Correct UTF‑8 handling
- Event‑driven interface via the `Actor` trait

### Quick Start

Add the crate to your workspace (usually as a local member):

```toml
[dependencies]
otty-vte = "0.1.0"
```

Implement the `Actor` trait and feed bytes to the parser:

```rust
use otty_vte::{Actor, CsiParam, Parser};

#[derive(Default)]
struct MyActor;

impl Actor for MyActor {
    fn print(&mut self, c: char) {
        println!("print: {c}");
    }

    fn execute(&mut self, byte: u8) {
        println!("exec: {byte:#04x}");
    }

    fn hook(
        &mut self,
        params: &[i64],
        interms: &[u8],
        ignored: bool,
        byte: u8,
    ) {
        println!(
            "DCS hook: params: {params:?}, interms: {interms:?}, ignored: {ignored}, final: {byte:#04x}"
        );
    }

    fn put(&mut self, byte: u8) {
        println!("DCS put: {byte:#04x}");
    }

    fn unhook(&mut self) {
        println!("DCS unhook");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], byte: u8) {
        println!("OSC: params: {:?}, final: {:02X}", params, byte);
    }

    fn csi_dispatch(
        &mut self,
        params: &[CsiParam],
        intermediates: &[u8],
        truncated: bool,
        byte: u8,
    ) {
        println!(
            "CSI: params: {params:?}, interms: {intermediates:?}, truncated: {truncated}, final: {byte:#04x}"
        );
    }

    fn esc_dispatch(
        &mut self,
        params: &[i64],
        intermediates: &[u8],
        ignored: bool,
        byte: u8,
    ) {
        println!(
            "ESC: params: {params:?}, interms: {intermediates:?}, ignored: {ignored}, final: {byte:#04x}"
        );
    }
}

fn main() {
    let mut parser = Parser::new();
    let mut actor = MyActor::default();
    parser.advance(b"\x1b[31mhi\x1b[0m", &mut actor);
}
```

You can run this example for check the work by running `cargo run --example printable`

### Supported Sequences

For detailed per‑state behavior and byte ranges, see `otty-vte/src/enums.rs`.
Each state is documented with a quick reference table and examples.

### References

- [vt100 ansi parser state machine description](https://vt100.net/emu/dec_ansi_parser)
- [wezterm ecma-48 standard description](https://wezfurlong.org/ecma48)
- [official ecma48 standard](https://ecma-international.org/publications-and-standards/standards/ecma-48/)
- [Ghostty author blog](https://mitchellh.com/)
- [GNOME VTE](https://gitlab.gnome.org/GNOME/vte)
- [Alacritty VTE](https://github.com/alacritty/vte)
- [wezterm vtparse](https://github.com/wezterm/wezterm/tree/main/vtparse)
- [libvterm](https://github.com/TragicWarrior/libvterm)
- [terminal sequences spreadsheet](https://docs.google.com/spreadsheets/d/19W-lXWS9jYwqCK-LwgYo31GucPPxYVld_hVEcfpNpXg/edit?gid=1724051764#gid=1724051764)
- [wezterm documentation](https://wezterm.org/escape-sequences.html#operating-system-command-sequences)

### License

See [LICENSE](../LICENSE) at the repository root.
