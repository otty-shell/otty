# otty-plugin-openrouter

The official OpenRouter plugin of OTTY. OTTY has no code for any provider
by name: this plugin teaches it OpenRouter. Its provider part
`openrouter` reads the limit of an OpenRouter key and how much of it was
used (`GET https://openrouter.ai/api/v1/key`) with the key itself: a key
the user added by hand, or one an agent part found for `openrouter.ai`.
The plugin never sees the key: OTTY hands it a handle, which goes only
into the request to `openrouter.ai`. With no OpenRouter key found it does
nothing.

The icon is a letter icon ("O" on a rounded square) until a
single-color OpenRouter logo with a licence that allows shipping it is found.

It is its own cargo workspace, outside the OTTY build, and uses the SDK
by path while it lives in this repository. Its tests run natively with
made-up keys and made-up answers, never the network:

```sh
cargo test
cargo build --release --target wasm32-wasip2
```

To try a build in OTTY, put
`target/wasm32-wasip2/release/otty_plugin_openrouter.wasm` as
`openrouter.wasm` in a folder with `otty-plugin.toml` and
`openrouter.svg`, and use **Add plugin from folder…** on the Plugins page.
