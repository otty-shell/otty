# otty-plugin-opencode

The official OpenCode plugin of OTTY. OTTY has no code for any agent by
name: this plugin teaches it OpenCode. Its one agent part, `opencode`,
finds OpenCode's data folder (`~/.local/share/opencode`, or the one
`XDG_DATA_HOME` names), reads its logins from `auth.json` and from the
`credential` table of its database, and reads the sessions and the tokens
and the cost of every answer, with its session and project, from the database
(`opencode.db`, or the one `OPENCODE_DB` names). It has no provider part:
the accounts of its logins are read by the plugins of their providers.

OTTY opens the database read-only, so the plugin never writes to it, and
the plugin never sees a key or a token: OTTY hands it handles in their
place.

What only the user can do, the part asks for itself, in OTTY's language,
which it follows when the user picks another: `opencode auth login` for
a profile nobody is logged in with, and again when the provider part of
another plugin refused a login OpenCode keeps, such as a ChatGPT login
the `openai` part read.

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

OTTY carries no plugin. `plugins/build.sh opencode` from the repository
root builds this one into `assets/plugins/opencode/`: `otty-plugin.toml`,
`opencode.wasm` and the icons. Install it with **Add plugin from
folder…** on the Plugins page and pick that folder. After a change, run
the script again, commit the folder and press **Reload** on the
plugin's row.
