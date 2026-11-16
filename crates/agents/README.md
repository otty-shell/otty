# otty-agents

Everything OTTY knows about coding agents such as Claude Code and Codex: how
to find their profiles, list their sessions, read their limits and start
them. Adding the next agent means implementing its traits here and adding it
to the list of known agents.

It is a separate crate so that this knowledge stays apart from the window.
It must not depend on `iced` or on any crate under `crates/app/`, and it
reads no environment variables itself: the app passes in what it read.
