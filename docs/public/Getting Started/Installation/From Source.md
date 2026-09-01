---
title: From Source
description: Build and run OTTY from its Rust source code.
sidebar:
  order: 2
---

Build OTTY from source when you want to test the latest code or contribute to the project.

## Prerequisites

Install the following tools before building OTTY:

- [Git](https://git-scm.com/)
- [Rust](https://www.rust-lang.org/tools/install) through `rustup`
- A native build toolchain for your operating system, such as Xcode Command Line Tools on macOS or
  a C/C++ compiler, `make`, `perl`, and `pkg-config` on Linux

The repository pins Rust 1.96.0 in `rust-toolchain.toml`. Rustup selects and installs that toolchain
when you run a Cargo command in the repository.

## Build OTTY

Clone the repository and create an optimized build:

```sh
git clone https://github.com/otty-shell/otty.git
cd otty
cargo build --release -p otty
```

The executable is written to `target/release/otty`.

## Run OTTY

Run the compiled executable directly:

```sh
./target/release/otty
```

For development, build and run OTTY in one command:

```sh
cargo run -p otty
```
