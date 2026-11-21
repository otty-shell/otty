#!/usr/bin/env bash
# Builds the official plugin in plugins/<folder>/ for wasm32-wasip2 and
# copies what OTTY installs into assets/plugins/<id>/: otty-plugin.toml,
# <id>.wasm and every icon the manifest names. The copy is replaced whole.
# OTTY carries no plugin: the user installs this folder with
# "Add plugin from folder…" on the Plugins page, and the tests read it.
#
#   plugins/build.sh anthropic      # -> assets/plugins/anthropic/
#   plugins/build.sh template       # -> assets/plugins/example/
#
# Run it after changing a plugin, then commit the copy.
# Needs: rustup target add wasm32-wasip2
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: plugins/build.sh <folder in plugins/>" >&2
    exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
folder="$root/plugins/$1"
manifest="$folder/otty-plugin.toml"

if [ ! -f "$manifest" ]; then
    echo "no plugin in $folder: otty-plugin.toml is missing" >&2
    exit 1
fi

# A value of the [plugin] table of the manifest, such as id or wasm.
plugin_value() {
    awk -F'"' -v key="$1" '
        /^\[/ { table = $0 }
        table == "[plugin]" && $0 ~ "^" key " *=" { print $2; exit }
    ' "$manifest"
}

id="$(plugin_value id)"
wasm="$(plugin_value wasm)"
if [ -z "$id" ] || [ "$wasm" != "$id.wasm" ]; then
    echo "wasm = \"$wasm\" of $manifest must be \"$id.wasm\"" >&2
    exit 1
fi

# The cargo package name, as the name of the built .wasm.
crate="$(awk -F'"' '
    /^\[/ { table = $0 }
    table == "[package]" && /^name *=/ { print $2; exit }
' "$folder/Cargo.toml" | tr '-' '_')"

# The plugin builds into its own folder, never into a shared target.
target="$folder/target"
nice -n 15 cargo build --release --locked --target wasm32-wasip2 -j 2 \
    --manifest-path "$folder/Cargo.toml" --target-dir "$target"

copy="$root/assets/plugins/$id"
rm -rf "$copy"
mkdir -p "$copy"

cp "$manifest" "$copy/otty-plugin.toml"
cp "$target/wasm32-wasip2/release/$crate.wasm" "$copy/$wasm"
awk -F'"' '/^icon *=/ { print $2 }' "$manifest" | sort -u | while read -r icon; do
    cp "$folder/$icon" "$copy/$icon"
done

echo "built $id into $copy:"
ls -1 "$copy"
