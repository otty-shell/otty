# AGENTS Instructions

Enhydra is a terminal-centric development environment.
It turns your familiar shell into a full-featured workspace, available anywhere - on a local machine or a remote server. All you need is to install enhydra and start working.

## Architecture

- enhydra based on client server architecture.
- `enhydra-agent` is a agent that must be installed either to server or localhost and provided the data for client manipulations.
- `enhydra-client` is a UI application based on [iced fraemwork](https://github.com/iced-rs/iced) that getting data from agent and render it.

## RUST

- Crate names are prefixed with `enhydra-`. For example, the core crate could be named as `enhydra-core`
- When using format! and you can inline variables into {}, always do that.
- Run `cargo fmt` and after than run `cargo clippy` when you make changes in `.rs` files. If you receive the errors or warnings from `cargo clippy` try to solve them.

## Terminal Emulation

VTE parser have to support the full xterm like sequences. 

For working with VTE parser use the next documentation:

- https://vt100.net/emu/dec_ansi_parser
- https://mitchellh.com/
- https://gitlab.gnome.org/GNOME/vte
- https://github.com/alacritty/vte
- https://github.com/wezterm/wezterm/tree/main/vtparse
- https://github.com/TragicWarrior/libvterm
- https://docs.google.com/spreadsheets/d/19W-lXWS9jYwqCK-LwgYo31GucPPxYVld_hVEcfpNpXg/edit?gid=1724051764#gid=1724051764
