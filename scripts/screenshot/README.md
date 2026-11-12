# README screenshot

Two scripts that make `assets/otty.png` the same way every time.

1. `demo.sh` starts OTTY in a clean demo home: a fresh clone of this
   repository, demo launches, a short prompt, and Helix and btop themes that
   use the OTTY colors. Your own settings, history and launches stay out of
   the picture. `demo.sh --clean` removes it afterwards.
2. `frame.sh <screenshot.png>` puts the window screenshot on the brand
   background and writes `assets/otty.png`.

## Shot list

Take the screenshot at 2x scale, with a window of about 1440×900 and a
font size of 15–16, so the text stays readable when GitHub shows the
picture at half size.

- Left sidebar: the `Servers` and `Project` launch folders open.
- Main pane: four command blocks. Press arrow up to get the commands in
  order: `git log --oneline --graph --decorate -8`,
  `cargo test -p otty-escape --quiet`, `docker ps` and `git push` (fails
  on purpose: the demo clone has no remote, which shows a failed block).
- Second pane on the right: `hx crates/escape/src/lib.rs`, or the
  `Open editor` launch.
- Explorer: open `crates/escape/src` so it matches the active session.
- Command palette closed, notifications empty.

For the Containers mode picture, start with `demo.sh --containers`. It
pulls and starts `nginx`, `redis` and `postgres` containers, which
`--clean` removes.
