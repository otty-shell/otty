#!/usr/bin/env bash
# Builds the test plugins of otty-plugin-host and the template plugin for
# wasm32-wasip2 and copies them to crates/plugins/host/tests/fixtures/,
# where the tests read them. Run it after changing them or the WIT files
# of otty-plugin-sdk. Needs: rustup target add wasm32-wasip2
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
plugins="$root/crates/plugins/host/tests/plugins"
template="$root/plugins/template"
fixtures="$root/crates/plugins/host/tests/fixtures"

mkdir -p "$fixtures"

cd "$plugins"
for plugin in minimal misbehave read-files programs; do
    nice -n 15 cargo build --release --target wasm32-wasip2 -j 2 \
        --features "$plugin"
    cp target/wasm32-wasip2/release/otty_test_plugins.wasm \
        "$fixtures/$plugin.wasm"
    echo "built $fixtures/$plugin.wasm"
done

cd "$template"
nice -n 15 cargo build --release --target wasm32-wasip2 -j 2
cp target/wasm32-wasip2/release/otty_plugin_example.wasm \
    "$fixtures/template.wasm"
echo "built $fixtures/template.wasm"
