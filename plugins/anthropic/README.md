# otty-plugin-anthropic

The official Anthropic plugin of OTTY. OTTY has no code for any agent by
name: this plugin teaches it Claude Code. Its agent part `claude-code`
finds Claude Code's config folders (`~/.claude`, `CLAUDE_CONFIG_DIR` and
folders such as `~/.claude-work`), lists their sessions, reads the tokens
of every answer and the statistics of a session or a project, and tells
which logins and keys a profile holds and whom each is for. A key in
`settings.json` behind another server, such as DeepSeek's, is reported as
a key for that server's host. The plugin never sees a key or a token:
OTTY hands it handles in their place.

The part also reads the limits Claude Code itself shows: it starts
`claude`, unable to run anything, in a terminal nobody sees, types
`/usage` and reads the panel from the screen. It answers Claude Code's
question whether to trust a folder only for OTTY's own probe folder, and
afterwards deletes the session files that run left in
`projects/<probe folder>/`; its manifest allows it to delete nothing else.
OTTY runs this at most once in five minutes per profile.

The provider part `anthropic` (the limits of a Claude login read from
Anthropic's server, and the bill of an organization) comes in a later
release.

It is its own cargo workspace, outside the OTTY build, and uses the SDK
by path while it lives in this repository. Its tests run natively over
the made-up files of `fixtures/`, which stand for the home folder:

```sh
cargo test
cargo build --release --target wasm32-wasip2
```

OTTY carries a built copy of this plugin in `assets/plugins/anthropic/`.
After a change, `plugins/embed.sh anthropic` from the repository root
builds it again and replaces that copy with its `SHA256SUMS`.

To try a build in OTTY without rebuilding OTTY, put
`target/wasm32-wasip2/release/otty_plugin_anthropic.wasm` as
`anthropic.wasm` in a folder with `otty-plugin.toml` and `claude-code.svg`,
and use **Add plugin from folder…** on the Plugins page.
