# otty-agents

Everything OTTY knows about coding agents such as Claude Code and Codex: how
to find their profiles, list their sessions, read their limits, count the
tokens they spent and start them. An agent or a provider is read either by
a reader built in here, or by a part of a plugin that OTTY runs through
`otty-plugin-host`; the generic work (walking files, adding up tokens,
"format changed", joining accounts) stays here for both.

It is a separate crate so that this knowledge stays apart from the window.
It must not depend on `iced` or on any crate under `crates/app/`, and it
reads no environment variables itself: the app passes in what it read.
