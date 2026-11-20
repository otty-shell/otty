# otty-plugin-openai

The official OpenAI plugin of OTTY. OTTY has no code for any agent by
name: this plugin teaches it Codex. Its agent part `codex` finds Codex's
config folders (`~/.codex`, `CODEX_HOME` and folders such as
`~/.codex-work`), lists their sessions, reads the tokens of every answer
from the session files, plain or compressed, and the statistics of a
session or a project. It tells whether a profile is logged in, but the
plugin never sees a key or a token: OTTY hands it handles in their place.
It never asks `codex app-server` for the list of sessions.

The logins of a profile, the limits Codex shows and the provider part
`openai` (the numbers of a ChatGPT or OpenAI account) come in later
releases.

It is its own cargo workspace, outside the OTTY build, and uses the SDK
by path while it lives in this repository. Its tests run natively over
the made-up files of `fixtures/`, which stand for the home folder:

```sh
cargo test
cargo build --release --target wasm32-wasip2
```

To try a build in OTTY, put
`target/wasm32-wasip2/release/otty_plugin_openai.wasm` as `openai.wasm`
in a folder with `otty-plugin.toml` and `codex.svg`, and use **Add
plugin from folder…** on the Plugins page.
