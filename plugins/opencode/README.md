# otty-plugin-opencode

The official OpenCode plugin of OTTY. OTTY has no code for any agent by
name: this plugin teaches it OpenCode. Its one agent part, `opencode`,
finds OpenCode's data folder (`~/.local/share/opencode`, or the one
`XDG_DATA_HOME` names) and reads the sessions and the tokens and the
cost of every answer, with its session and project, from the database
(`opencode.db`, or the one `OPENCODE_DB` names). The cost OpenCode wrote
is given as it is, so the money is exact; an answer with no cost is
priced at the API price OTTY gives for its model.

It is an agent only: it has no provider part and no login code.
OpenCode works with no login, with its free models, so the part never
says a login is missing and asks the user for nothing. The logins
OpenCode keeps, in `auth.json` and in the `credential` table, are never
read; the manifest still declares them as keys, so OTTY never gives
them to the part as text.

OTTY opens the database read-only, so the plugin never writes to it.

OpenCode's installer puts the program at `~/.opencode/bin/opencode`, a
folder the login shell may not have on its `PATH`: the manifest lists it
in `program_places`, so OTTY finds the program there and starts it by
that path. When OpenCode is nowhere, the manifest's `install` gives
OpenCode's install script, `curl -fsSL https://opencode.ai/v2/install |
bash`, which OTTY runs in a terminal tab for **Install OpenCode**, and
OpenCode's page, `https://opencode.ai/docs`.

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
folder…** in Plugins mode and pick that folder. After a change, run
the script again, commit the folder and press **Reload** on the
plugin's row.
