#!/usr/bin/env bash
# Build the .deb package inside an old (Ubuntu 20.04) environment so that the
# resulting binary links against glibc 2.31 and libssl3, making it installable
# on Ubuntu 20.04, 22.04 and 24.04.
#
# See: https://github.com/otty-shell/otty/issues/53
set -euo pipefail

# Dependencies required by the iced GUI toolkit (gtk/webkit/dbus) and by the
# ssh2 crate (OpenSSL) on Ubuntu 20.04.
export DEBIAN_FRONTEND=noninteractive
export PKG_CONFIG_ALLOW_CROSS=1

apt-get update
apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    libdbus-1-dev \
    libwebkit2gtk-4.0-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

# Install Rust via rustup (matches rust-toolchain.toml pinned version).
if ! command -v cargo >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --default-toolchain stable --profile minimal
    . "${HOME}/.cargo/env"
fi

# Install cargo-deb if not already available.
if ! command -v cargo-deb >/dev/null 2>&1; then
    cargo install cargo-deb
fi

cargo build --release -p otty
cargo deb -p otty
