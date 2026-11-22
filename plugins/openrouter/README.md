# otty-plugin-openrouter

The official OpenRouter plugin of OTTY. OTTY has no code for any provider
by name: this plugin teaches it OpenRouter. Its provider part
`openrouter` reads the limit of an OpenRouter key and how much of it was
used (`GET https://openrouter.ai/api/v1/key`) with the key itself: a key
the user added by hand, or one an agent part found for `openrouter.ai`.
The plugin never sees the key: OTTY hands it a handle, which goes only
into the request to `openrouter.ai`. With no OpenRouter key found it does
nothing.

A refused key the part asks the user to replace where it sits: in a
field of OTTY for a key added there, on the keys page for a key an agent
keeps. Its texts are in OTTY's language, which it follows when the user
picks another.

The icon is a letter icon ("O" on a rounded square) until a
single-color OpenRouter logo with a licence that allows shipping it is found.

It is its own cargo workspace, outside the OTTY build, and uses the SDK
by path while it lives in this repository. Its tests run natively with
made-up keys and made-up answers, never the network:

```sh
cargo test
cargo build --release --target wasm32-wasip2
```

OTTY carries no plugin. `plugins/build.sh openrouter` from the repository
root builds this one into `assets/plugins/openrouter/`: `otty-plugin.toml`,
`openrouter.wasm` and the icons. Install it with **Add plugin from
folder…** in Plugins mode and pick that folder. After a change, run
the script again, commit the folder and press **Reload** on the
plugin's row.
