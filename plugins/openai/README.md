# otty-plugin-openai

The official OpenAI plugin of OTTY. OTTY has no code for any agent by
name: this plugin teaches it Codex. Its agent part `codex` names Codex's
default config folder (`~/.codex`, or the one `CODEX_HOME` names; the
user adds other folders, such as `~/.codex-work`, as profiles), tells
whether each profile is logged in, lists the sessions of each profile,
and reads the tokens of every answer, with its session and project, from
the session files, plain or compressed. The plugin never sees a key or a
token: OTTY hands it handles in their place.

Its provider part `openai` finds the OpenAI accounts itself, in the
Codex profile folders OTTY gives it, and reads nothing outside them: the
ChatGPT login of `auth.json` is a subscription known by its account id
(`chatgpt:<account id>`, the same after Codex renews the tokens) and
named by its email, with its plan; an API key is a key known by its
fingerprint (`key:<fingerprint>`, named such as `key ••••a1f3`), and an
admin key an organization. One account logged in to two folders is one
account found in both. A login Codex sends to a gateway is for the
gateway's host and is no OpenAI account.

It reads the numbers of an account any way it can, only in the folders
it was found in: the limits and credits of a ChatGPT login from
`chatgpt.com` with its token; when the server gives no answer, the
limits Codex knows there, from `codex app-server` started with
`CODEX_HOME` of that folder, which it asks only for the limits and never
for the list of sessions, else from the last record of them in the
session files; and the costs of an organization with an admin key, from
`api.openai.com`. With no way that works it reads nothing, which is no
failure. A token whose claims say it expired is not sent.

What only the user can do, the parts ask for themselves, in OTTY's
language, which they follow when the user picks another: `codex login`
for a profile nobody is logged in with, `codex login` in the folder of
an account whose ChatGPT login expired or was refused (whether it
expired is the plugin's call, from the token's claims), and a field for
the admin key (`sk-admin-…`) when the bill of an organization needs one
or OpenAI refused the one given.

When Codex is nowhere on the `PATH` of the login shell, the manifest's
`install` gives its install script, `curl -fsSL
https://chatgpt.com/codex/install.sh | sh`, which OTTY runs in a
terminal tab for **Install Codex**, and its repository,
`https://github.com/openai/codex`. The repository names no folder the
script puts the program in, so the part has no program place.

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
