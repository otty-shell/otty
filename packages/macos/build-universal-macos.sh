#!/usr/bin/env bash
set -euo pipefail

# Build a universal (arm64 + x86_64) macOS binary for OTTY.
# Usage (from any directory inside the repo on macOS):
#   bash packages/macos/build-universal-macos.sh

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ "$(uname)" != "Darwin" ]]; then
  echo "This script must be run on macOS (Darwin)." >&2
  exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required but not found. Install Rust via rustup and try again." >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required but not found in PATH." >&2
  exit 1
fi

if ! command -v lipo >/dev/null 2>&1; then
  echo "Xcode command line tools (lipo) are required. Install via 'xcode-select --install'." >&2
  exit 1
fi

cd "${ROOT_DIR}"

echo "Adding macOS targets (idempotent)…"
rustup target add aarch64-apple-darwin x86_64-apple-darwin

echo "Building OTTY for arm64 (aarch64-apple-darwin)…"
cargo build -p otty --release --target aarch64-apple-darwin

echo "Building OTTY for x86_64 (x86_64-apple-darwin)…"
cargo build -p otty --release --target x86_64-apple-darwin

UNIVERSAL_DIR="target/universal-macos"
mkdir -p "${UNIVERSAL_DIR}"

ARM_BIN="target/aarch64-apple-darwin/release/otty"
X64_BIN="target/x86_64-apple-darwin/release/otty"
OUT_BIN="${UNIVERSAL_DIR}/otty"

if [[ ! -f "${ARM_BIN}" ]]; then
  echo "Expected arm64 binary not found at ${ARM_BIN}" >&2
  exit 1
fi

if [[ ! -f "${X64_BIN}" ]]; then
  echo "Expected x86_64 binary not found at ${X64_BIN}" >&2
  exit 1
fi

echo "Creating universal binary at ${OUT_BIN}…"
lipo -create -output "${OUT_BIN}" "${ARM_BIN}" "${X64_BIN}"

chmod +x "${OUT_BIN}"

echo
echo "Done."
echo "Universal macOS binary is available at:"
echo "  ${OUT_BIN}"
echo
echo "You can test it, for example, with:"
echo "  ${OUT_BIN}"

