# otty-plugin-manifest

Reads and checks the manifest of an OTTY plugin, `otty-plugin.toml`, and
the permissions each part of a plugin declares, and answers whether a part
may read a path, run a program, call a host or open a key place. A part is
an extension point of a domain, written under the domain's key, such as
`[[agents.agent]]` with `agents.agent = "0.1"` in `[api]`.

Two places check the same permissions: `otty-plugin-host`, which runs
plugins inside OTTY, and the test host of `otty-plugin-sdk`, which plugin
tests run against. It is a separate crate so that plugin tests do not build
wasmtime. It must not depend on wasmtime, `iced` or any crate under
`crates/app/`.
