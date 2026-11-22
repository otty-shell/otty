# otty-plugin-openai

The official OpenAI plugin of OTTY. OTTY has no code for any agent by
name: this plugin teaches it Codex. Its agent part `codex` finds Codex's
config folders (`~/.codex`, `CODEX_HOME` and folders such as
`~/.codex-work`), lists their sessions, and reads the tokens of every
answer, with its session and project, from the session files, plain or
compressed. It tells which ChatGPT login or API key a profile
holds and whom it is for, but the plugin never sees a key or a token:
OTTY hands it handles in their place.

Its provider part `openai` reads the numbers of OpenAI accounts any way
it can, with the handles OTTY gives it: the limits and credits of a
ChatGPT login from `chatgpt.com`, whichever agent keeps the login (Codex
or OpenCode), and the costs of an organization with an admin key, from
`api.openai.com`. When the login's token is not at its place or the
server gives no answer, it reads the limits Codex knows in a Codex
profile OTTY found the login in: `codex app-server`, which it asks only
for the limits and never for the list of sessions, else the last record
of them in the session files. With no way that works it reads nothing,
which is no failure.
A token whose claims say it expired is not sent; a login behind a
gateway is for the gateway's host and is not read here. A ChatGPT login
ChatGPT refused, or that expired, the provider part only reports as
refused: it does not know which agent keeps it, so OTTY asks that
agent's part (Codex's here, OpenCode's in its own plugin) what to do.

What only the user can do, the parts ask for themselves, in OTTY's
language, which they follow when the user picks another: `codex login`
for a profile nobody is logged in with, `codex` in the profile's folder
for an expired or refused ChatGPT login of Codex (Codex renews it), and
a field for the admin key (`sk-admin-…`) when the bill of an
organization needs one or OpenAI refused the one given.
While it runs, the files of the `codex` part are refused to it, but for
the session files, where Codex writes the limits.

It is its own cargo workspace, outside the OTTY build, and uses the SDK
by path while it lives in this repository. Its tests run natively over
the made-up files of `fixtures/`, which stand for the home folder:

```sh
cargo test
cargo build --release --target wasm32-wasip2
```

OTTY carries no plugin. `plugins/build.sh openai` from the repository
root builds this one into `assets/plugins/openai/`: `otty-plugin.toml`,
`openai.wasm` and the icons. Install it with **Add plugin from
folder…** in Plugins mode and pick that folder. After a change, run
the script again, commit the folder and press **Reload** on the
plugin's row.
