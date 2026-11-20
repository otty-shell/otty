# otty-test-plugins

Small WebAssembly components for the tests of `otty-plugin-host`: a valid
plugin, one that never returns, one that grabs memory, and one that imports
`wasi:filesystem`. They let the tests check the engine's limits on real
components.

The built copies are kept in `../fixtures/`, so `cargo test` needs no
`wasm32-wasip2` target. After changing a source here, run
`scripts/build-test-plugins.sh` from the repository root (it needs `rustup
target add wasm32-wasip2`) and commit the new `.wasm` files. This crate is
its own cargo workspace and is never built by the OTTY workspace.
