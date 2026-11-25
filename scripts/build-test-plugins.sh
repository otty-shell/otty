#!/usr/bin/env bash
# Builds the test plugins of otty-plugin-host for wasm32-wasip2 and copies
# them to crates/plugins/host/tests/fixtures/, where the tests read them:
# minimal, misbehave, read-files and programs from tests/plugins/, one
# .wasm each, and the example plugin from tests/example/ as the folder
# fixtures/example/ an installed plugin has: otty-plugin.toml, example.wasm
# and the icons. Run it after changing them or the WIT files of
# otty-plugin-sdk. It builds into CARGO_TARGET_DIR when that is set.
# Needs: rustup target add wasm32-wasip2
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
host_tests="$root/crates/plugins/host/tests"
fixtures="$host_tests/fixtures"

mkdir -p "$fixtures"

# The folder cargo builds the crate in the folder $1 into.
target_of() {
    echo "${CARGO_TARGET_DIR:-$1/target}"
}

plugins="$host_tests/plugins"
target="$(target_of "$plugins")"
cd "$plugins"
for plugin in minimal misbehave read-files programs; do
    nice -n 15 cargo build --release --target wasm32-wasip2 -j 2 \
        --features "$plugin"
    cp "$target/wasm32-wasip2/release/otty_test_plugins.wasm" \
        "$fixtures/$plugin.wasm"
    echo "built $fixtures/$plugin.wasm"
done

example="$host_tests/example"
target="$(target_of "$example")"
copy="$fixtures/example"
cd "$example"
nice -n 15 cargo build --release --target wasm32-wasip2 -j 2
rm -rf "$copy"
mkdir -p "$copy"
cp otty-plugin.toml example-agent.svg example-cloud.svg "$copy/"
cp "$target/wasm32-wasip2/release/otty_test_plugin_example.wasm" \
    "$copy/example.wasm"
echo "built $copy"
