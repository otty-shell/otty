# otty-plugin-opencode

The official OpenCode plugin of OTTY. OTTY has no code for any agent by
name: this plugin teaches it OpenCode. Its one agent part, `opencode`,
finds OpenCode's data folder (`~/.local/share/opencode`, or the one
`XDG_DATA_HOME` names), reads its logins from `auth.json` and from the
`credential` table of its database, and reads the sessions, the tokens and
the cost of every answer and the statistics from the database
(`opencode.db`, or the one `OPENCODE_DB` names). It has no provider part:
the accounts of its logins are read by the plugins of their providers.

OTTY opens the database read-only, so the plugin never writes to it, and
the plugin never sees a key or a token: OTTY hands it handles in their
place.

It is its own cargo workspace, outside the OTTY build, and uses the SDK
by path while it lives in this repository. Its tests run natively over
made-up data folders: the files of `fixtures/`, which stand for the home
folder, and databases the tests write in a temporary folder.
`fixtures/.local/share/opencode/opencode.db` is made from
`fixture-database.sql` (the command is at its top).

```sh
cargo test
cargo build --release --target wasm32-wasip2
```

To try a build in OTTY, put
`target/wasm32-wasip2/release/otty_plugin_opencode.wasm` as
`opencode.wasm` in a folder with `otty-plugin.toml` and `opencode.svg`,
and use **Add plugin from folder…** on the Plugins page.
