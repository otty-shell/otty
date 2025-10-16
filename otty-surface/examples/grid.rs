//! Minimal example of using GridSurface.
//!
//! Run with:
//!   cargo run -p otty-surface --example grid

use otty_surface::GridSurface;

// Complex demo bytes resembling shell output. Use valid escapes only.
const SHELL_DEMO_BYTES: &[u8] = b"\x1b]0;otty-surface demo\x07\
\x1b[2J\x1b[H\
\x1b[1;34m=== otty-surface demo ===\x1b[0m\n\
Welcome to the demo of grid surface.\n\n\
\x1b[1;32muser\x1b[0m@\x1b[1;36mhost\x1b[0m:\x1b[1;34m~/project\x1b[0m$ ls -la\n\
total 24\n\
drwxr-xr-x  5 user staff    160 Oct 16 12:00 \x1b[34m.\x1b[0m\n\
drwxr-xr-x 10 user staff    320 Oct 16 11:00 \x1b[34m..\x1b[0m\n\
-rw-r--r--  1 user staff   4096 Oct 16 12:00 README.md\n\
drwxr-xr-x  3 user staff     96 Oct 16 12:00 \x1b[34msrc\x1b[0m\n\
-rwxr-xr-x  1 user staff   1024 Oct 16 12:00 \x1b[32mrun.sh\x1b[0m\n\
lrwxr-xr-x  1 user staff      7 Oct 16 12:00 \x1b[36mlink\x1b[0m -> target\n\n\
\x1b[1;32muser\x1b[0m@\x1b[1;36mhost\x1b[0m:\x1b[1;34m~/project\x1b[0m$ echo 'columns'\n\
name\tversion\tstatus\n\
alpha\t1.0.0\tOK\n\
beta\t0.9.1\tWARN\n\
gamma\t2.0.0\tFAIL\n\n\
\x1b[1;32muser\x1b[0m@\x1b[1;36mhost\x1b[0m:\x1b[1;34m~/project\x1b[0m$ \n\
Downloading: [          ] 0%\r\
Downloading: [##        ] 20%\x1b[K\r\
Downloading: [####      ] 40%\x1b[K\r\
Downloading: [######    ] 60%\x1b[K\r\
Downloading: [########  ] 80%\x1b[K\r\
Downloading: [##########] 100%\x1b[K\n\n\
Colors: default \x1b[1mbold\x1b[22m, \x1b[3mitalic\x1b[23m, \x1b[4munderline\x1b[24m, \x1b[7minverse\x1b[27m.\n\
Indexed: \x1b[38;5;196mred\x1b[0m \x1b[38;5;46mgreen\x1b[0m \x1b[38;5;21mblue\x1b[0m\n\
Truecolor FG: \x1b[38;2;255;128;0morange\x1b[0m BG: \x1b[48;2;30;30;30m\x1b[38;2;200;200;200mon dark bg\x1b[0m\n\n\
Cursor moves demo:\n\
start->\x1b[5Cright5\x1b[3Dleft3\n\
Line clear demo: begin... \x1b[Kend\n\n\
Scroll up by 2:\n\
line A\n\
line B\n\
line C\n\
\x1b[2Safter scroll\n";

fn render_ascii(surface: &GridSurface) -> String {
    let mut out = String::new();
    for y in 0..surface.height() {
        for x in 0..surface.width() {
            let cell = surface.cells()[y * surface.width() + x];
            let ch = match cell.ch {
                '\u{0000}' => ' ',
                c => c,
            };
            out.push(ch);
        }
        if y + 1 < surface.height() {
            out.push('\n');
        }
    }
    out
}

fn main() {
    // Create a larger grid surface for the demo.
    let mut surf = GridSurface::new(80, 25);

    // Feed complex shell-like demo.
    surf.feed(SHELL_DEMO_BYTES);

    // Render to ASCII for demo purposes.
    let screen = render_ascii(&surf);
    println!("Grid ({}x{}):", surf.width(), surf.height());
    println!("{}", screen);

    let cur = surf.cursor();
    println!("Cursor at: row={}, col={}", cur.y + 1, cur.x + 1);
}
