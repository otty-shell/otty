# otty-plugin-deepseek

The official DeepSeek plugin of OTTY. OTTY has no code for any provider
by name: this plugin teaches it DeepSeek. Its provider part `deepseek`
reads the money left on a DeepSeek account (`GET
https://api.deepseek.com/user/balance`) with the account's key. It finds
no accounts yet: no agent of this plugin keeps a login, and a provider
part never reads another plugin's profiles, so a DeepSeek key in Claude
Code's `settings.json` is no account. Accounts come with "paste a key",
which will give the key under the slot `key`. The plugin never sees the
key: OTTY hands it a handle, which goes only into the request to
`api.deepseek.com`.

A refused key the part asks the user to replace where the agent keeps
it, with the keys page at hand. Its texts are in OTTY's language, which it follows when the user
picks another.

The icon is a letter icon ("D" on a rounded square) until a
single-color DeepSeek logo with a licence that allows shipping it is found.

It is its own cargo workspace, outside the OTTY build, and uses the SDK
by path while it lives in this repository. Its tests run natively with
made-up keys and made-up answers, never the network:

```sh
cargo test
cargo build --release --target wasm32-wasip2
```

OTTY carries no plugin. `plugins/build.sh deepseek` from the repository
root builds this one into `assets/plugins/deepseek/`: `otty-plugin.toml`,
`deepseek.wasm` and the icons. Install it with **Add plugin from
folder…** in Plugins mode and pick that folder. After a change, run
the script again, commit the folder and press **Reload** on the
plugin's row.
