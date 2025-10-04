## otty-vte

Lightweight, table-driven VT/ANSI virtual terminal parser.

otty-vte turns a byte stream from a pseudo terminal into high‑level terminal
actions: printing characters, executing C0/C1 controls, parsing ESC/CSI/DCS/OSC,
and dispatching the corresponding sequences.

Inspired by ECMA‑48 and DEC VT specs and by implementations such as
alacritty/vte, wezterm/vtparse, and GNOME VTE.

### Features
- Full state‑machine recognition of VT/ANSI families:
  - Ground, ESC, CSI, DCS (including passthrough), OSC, SOS/PM/APC
  - Supports both 7‑bit (`ESC [`) and 8‑bit C1 introducers (`0x90`, `0x9B`, `0x9D`, …)
- Correct UTF‑8 handling (collect and emit Unicode code points)
- Event‑driven interface via the `Actor` trait (`hook/put/unhook` for DCS,
  `osc_dispatch` for OSC, `csi_dispatch`/`esc_dispatch` for CSI/ESC)
- Bounds and safeguards on parameter/intermediate buffers

### Quick Start
Add the crate to your workspace (usually as a local member):

```toml
[dependencies]
otty-vte = "0.1.0"
```

Implement the `Actor` trait and feed bytes to the parser:

```rust
use otty_vte::{Actor, Parser, CsiParam};

#[derive(Default)]
struct MyActor;

impl Actor for MyActor {
    fn print(&mut self, c: char) { println!("print: {c}"); }
    fn execute(&mut self, byte: u8) { println!("exec: {byte:#04x}"); }
    fn hook(&mut self, byte: u8, params: &[i64], interms: &[u8], ignored: bool) {
        println!("DCS hook: final={byte:#04x} params={params:?} interms={interms:?} ignored={ignored}");
    }
    fn put(&mut self, byte: u8) { println!("DCS put: {byte:#04x}"); }
    fn unhook(&mut self) { println!("DCS unhook"); }
    fn osc_dispatch(&mut self, params: &[&[u8]]) { println!("OSC: {:?}", params); }
    fn csi_dispatch(&mut self, params: &[CsiParam], truncated: bool, byte: u8) {
        println!("CSI: params={params:?} truncated={truncated} final={byte as char:?}");
    }
    fn esc_dispatch(&mut self, params: &[i64], interms: &[u8], ignored: bool, byte: u8) {
        println!("ESC: params={params:?} interms={interms:?} ignored={ignored} final={byte as char:?}");
    }
}

fn main() {
    let mut parser = Parser::new();
    let mut actor = MyActor::default();

    // Example: "\x1b[31mhi\x1b[0m"
    parser.advance(b"\x1b[31mhi\x1b[0m", &mut actor);
}
```

### Architecture & API
- `Parser::advance(&mut self, bytes, &mut actor)` walks the byte stream and
  calls `Actor` methods according to the recognized sequences.
- `Actor` is the consumer contract. Implement it to mutate your terminal model
  or to forward events to a UI.
- `CsiParam` represents CSI parameters: either `Integer(i64)` or raw parameter
  bytes/markers (`P(u8)`), enabling extensions such as colon‑separated values.

### Supported Sequences (overview)
- Ground: print `0x20..=0x7E`, execute C0, transition to ESC/CSI/DCS/OSC
- ESC: plain ESC sequences and family introducers for CSI/OSC/DCS/SOS|PM|APC
- CSI: decimal parameters (`0..9`, `;`), private markers (`<=>?`), intermediates, finals
- DCS: `hook/put/unhook` passthrough for payloads (e.g. sixel)
- OSC: collect fields until `BEL` or `ST` and then `osc_dispatch`
- SOS/PM/APC: ignore content until `ST`

For detailed per‑state behavior and byte ranges, see `otty-vte/src/enums.rs`.
Each state is documented with a quick reference table and examples.

### UTF‑8 and C1
- UTF‑8: multibyte sequences are collected and emitted as `print` with the
  decoded Unicode scalar, not raw octets.
- C1 (0x80..=0x9F): may be interpreted as 8‑bit single‑byte introducers for
  CSI/DCS/OSC/SOS|PM|APC depending on terminal mode; the parser supports both
  models.

### Examples
- Plain text: `Hello\x07` → `print('H'..)` + `execute(0x07)`
- Colors (SGR): `ESC [ 31 m` → `csi_dispatch([Integer(31)], false, b'm')`
- Window title: `ESC ] 0 ; title BEL` → `osc_dispatch([b"0", b"title"])`
- Sixel: `ESC P q ... ST` → `hook('q')`, then a series of `put(..)`, then `unhook`

### Limits and Defaults
- Bounded buffers for CSI/OSC to prevent excessive memory usage. Excess data is
  flagged as truncated or ignored where applicable.
- Colon `:` in CSI parameters is supported as a raw parameter byte
  (`CsiParam::P(b':')`) to enable extensions (e.g., RGB in SGR).

### Tests
There is a test suite in `otty-vte/src/parser.rs` covering printing,
CSI/OSC/DCS, and parts of UTF‑8/C1 behavior.

```bash
cargo test -p otty-vte
```

### License
See `LICENSE` at the repository root.

### Contributing
- When changing `.rs` files, run `cargo fmt` and then `cargo clippy` and fix
  warnings where possible.
- Please follow the existing code structure and style.
