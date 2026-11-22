# otty-plugin-anthropic

The official Anthropic plugin of OTTY. OTTY has no code for any agent by
name: this plugin teaches it Claude Code. Its agent part `claude-code`
finds Claude Code's config folders (`~/.claude`, `CLAUDE_CONFIG_DIR` and
folders such as `~/.claude-work`), lists their sessions, reads the tokens
of every answer with its session and project, and tells
which logins and keys a profile holds and whom each is for. A key in
`settings.json` behind another server, such as DeepSeek's, is reported as
a key for that server's host. The plugin never sees a key or a token:
OTTY hands it handles in their place.

What only the user can do, the parts ask for themselves, in OTTY's
language, which they follow when the user picks another: `claude` for a
profile nobody is logged in with or whose login expired or a provider
refused (Claude Code logs in or renews the login), and a field for the
admin key (`sk-ant-admin…`) when the bill of an organization needs one or
Anthropic refused the one given.

Its provider part `anthropic` reads the numbers of Anthropic accounts
any way it can, with the handles OTTY gives it: the limits of a Claude
login (and its extra usage) from Anthropic's usage request, whichever
agent keeps the login, and the cost report of an organization with an
admin key added by hand, both from `api.anthropic.com`. When the
login's token is not at its place, as when Claude Code keeps it
elsewhere, or the server gives no answer, it reads the limits Claude
Code itself shows, in a Claude Code profile OTTY found the login in: it
starts `claude` there, unable to run anything, in a terminal nobody
sees, types `/usage` and reads the panel from the screen. It answers
Claude Code's question whether to trust a folder only for OTTY's own
probe folder, and afterwards deletes the session files that run left in
`projects/<probe folder>/`; its manifest allows it to delete nothing
else. OTTY runs this at most once in five minutes per profile. With no
way that works it reads nothing, which is no failure. It never sees
when a token expires and renews nothing: a refused token it only reports
as refused, and OTTY asks the agent part that keeps the login, such as
`claude-code`, what to do. While it runs, the files of the
`claude-code` part are refused to it, but for the session files of its
own `/usage` run.

It is its own cargo workspace, outside the OTTY build, and uses the SDK
by path while it lives in this repository. Its tests run natively over
the made-up files of `fixtures/`, which stand for the home folder:

```sh
cargo test
cargo build --release --target wasm32-wasip2
```

OTTY carries no plugin. `plugins/build.sh anthropic` from the repository
root builds this one into `assets/plugins/anthropic/`: `otty-plugin.toml`,
`anthropic.wasm` and the icons. Install it with **Add plugin from
folder…** on the Plugins page and pick that folder. After a change, run
the script again, commit the folder and press **Reload** on the
plugin's row.
