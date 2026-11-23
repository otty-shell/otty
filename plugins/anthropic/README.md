# otty-plugin-anthropic

The official Anthropic plugin of OTTY. OTTY has no code for any agent by
name: this plugin teaches it Claude Code. Its agent part `claude-code`
names Claude Code's default config folder (`~/.claude`, or the one
`CLAUDE_CONFIG_DIR` names; the user adds other folders, such as
`~/.claude-work`, as profiles), tells whether each profile is logged in,
lists its sessions and reads the tokens of every answer with its session
and project. A profile whose `settings.json` sends Claude Code to another
server, such as DeepSeek's, has its tokens counted for that server's
host. The plugin never sees a key or a token: OTTY hands it handles in
their place.

Its provider part `anthropic` finds the Anthropic accounts itself, in the
Claude Code profile folders OTTY gives it, and reads only there while
OTTY asks. A Claude login is a subscription, known by its account and
named by its email (and its organization, unless it is the personal one
Claude makes for every account), with its plan; it stays the same account
when Claude Code renews its tokens, and one account logged in to two
profiles is one account found in two places. A login to an Anthropic
Console organization that pays per token is that organization. A Console
key and an Anthropic key of `settings.json` are keys, named by the end of
their fingerprint; an admin key is an organization. A key that
`settings.json` sends to another server, such as DeepSeek's, is no
Anthropic account.

It reads the numbers of an account any way it can, only in the folders
the account was found in: the limits of a Claude login (and its extra
usage) from Anthropic's usage request with the login's token, and the
cost report of an organization with its own admin key or the one the
user gives for it, both from `api.anthropic.com`. When the folder holds
no token, as when Claude Code keeps it elsewhere, or the server gives no
answer, it reads the limits Claude Code itself shows: it starts `claude`
pointed at that folder, unable to run anything, in a terminal nobody
sees, types `/usage` and reads the panel from the screen. It answers
Claude Code's question whether to trust a folder only for OTTY's own
probe folder, and afterwards deletes the session files that run left in
`projects/<probe folder>/` of that folder; its manifest allows it to
delete nothing else (`write` of its `logins` entry, inside the folders
of the call only). OTTY runs this at most once in five minutes per
account. With no way that works it reads nothing, which is no failure.
It renews no token itself: Claude Code renews it when it runs. OTTY's
own login (Add account → "Log in with Claude Code") is read first, and
when its token is past its time or Anthropic refused it, the `/usage`
probe runs there, so Claude Code keeps that login fresh; when even that
fails, the account needs "Log in again".

What only the user can do, the parts ask for themselves, in OTTY's
language, which they follow when the user picks another: `claude` in a
profile nobody is logged in with, and in the folder of a login that
expired with no way to renew it or that Anthropic refused (Claude Code
logs in or renews the login), and a field for the admin key
(`sk-ant-admin…`) when the bill of an organization needs one or Anthropic
refused the one given.

The native installer of Claude Code puts the program at
`~/.local/bin/claude`, a folder the login shell may not have on its
`PATH`: the manifest lists it in `program_places`, so OTTY finds the
program there and starts it by that path, also for `/usage`. When
Claude Code is nowhere, the manifest's `install` gives its native
installer, `curl -fsSL https://claude.ai/install.sh | bash`, which OTTY
runs in a terminal tab for **Install Claude Code**, and its setup page,
`https://code.claude.com/docs/en/setup`.

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
folder…** in Plugins mode and pick that folder. After a change, run
the script again, commit the folder and press **Reload** on the
plugin's row.
