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

The provider part `anthropic` (the limits of a Claude login and the bill
of an organization) and the `/usage` screen of Claude Code come in later
releases.

It is its own cargo workspace, outside the OTTY build, and uses the SDK
by path while it lives in this repository. Its tests run natively over
the made-up files of `fixtures/`, which stand for the home folder:

```sh
cargo test
cargo build --release --target wasm32-wasip2
```

To try a build in OTTY, put
`target/wasm32-wasip2/release/otty_plugin_anthropic.wasm` as
`anthropic.wasm` in a folder with `otty-plugin.toml` and `claude-code.svg`,
and use **Add plugin from folder…** on the Plugins page.
