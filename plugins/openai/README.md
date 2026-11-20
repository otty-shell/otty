# otty-plugin-openai

The official OpenAI plugin of OTTY. OTTY has no code for any agent by
name: this plugin teaches it Codex. Its agent part `codex` finds Codex's
config folders (`~/.codex`, `CODEX_HOME` and folders such as
`~/.codex-work`), lists their sessions, reads the tokens of every answer
from the session files, plain or compressed, and the statistics of a
session or a project. It tells which ChatGPT login or API key a profile
holds and whom it is for, but the plugin never sees a key or a token:
OTTY hands it handles in their place. The limits come from `codex
app-server`, which it asks only for the limits and never for the list
of sessions, else from the last record of them in the session files.

Its provider part `openai` reads the numbers of OpenAI accounts with the
handles OTTY gives it: the limits and credits of a ChatGPT login from
`chatgpt.com`, whichever agent keeps the login (Codex or OpenCode), and
the costs of an organization with an admin key added by hand, from
`api.openai.com`. A token whose claims say it expired is not sent; a
login behind a gateway is for the gateway's host and is not read here.
While it runs, the files of the `codex` part are refused to it. OTTY
carries a built copy of this plugin in `assets/plugins/openai/`, made by
`plugins/embed.sh openai`.

It is its own cargo workspace, outside the OTTY build, and uses the SDK
by path while it lives in this repository. Its tests run natively over
the made-up files of `fixtures/`, which stand for the home folder:

```sh
cargo test
cargo build --release --target wasm32-wasip2
```

To try a build in OTTY, put
`target/wasm32-wasip2/release/otty_plugin_openai.wasm` as `openai.wasm`
in a folder with `otty-plugin.toml`, `codex.svg` and `openai.svg`, and
use **Add plugin from folder…** on the Plugins page.
