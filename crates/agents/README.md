# otty-agents

The generic agent logic of OTTY: how profiles, sessions, limits, tokens,
money estimates and accounts are found, joined and checked, with no agent
or provider named in its code. The agents and providers themselves are read
by the parts of plugins that OTTY runs through `otty-plugin-host`; this
crate turns what those parts answer into what the app shows (walking files,
adding up tokens, "format changed", joining accounts, the registry).

It is a separate crate so that this knowledge stays apart from the window.
It must not depend on `iced` or on any crate under `crates/app/`, and it
reads no environment variables itself: the app passes in what it read.
