# otty-test-plugin-example

The example test plugin of `otty-plugin-host`: the plugin template of the
`otty-plugin-template` repository, a made-up agent part `example-agent`
and a made-up provider part `example-cloud`, with its tests left out and
its texts in English and Russian only. Its provider part also reads a
login with no key with `example-agent usage` in a hidden terminal, which
its manifest leaves out, so the tests that need it add it. The
tests of OTTY that need a whole plugin, from the host to Agents mode and
Plugins mode, run it, so no test of OTTY needs an official plugin.

`home/` is the made-up home folder of those tests. The built copy is kept
in `../fixtures/example/` as an installed plugin folder holds it:
`otty-plugin.toml`, `example.wasm` and the icons. After changing a file
here, or the WIT files of `otty-plugin-sdk`, run
`scripts/build-test-plugins.sh` from the repository root and commit the
new copy. This crate is its own cargo workspace and is never built by the
OTTY workspace.
