# otty-plugin-sdk

What an OTTY plugin is written with: the WIT files of every contract
version, the plain functions over the host operations, key handles,
needs, and the `export!` macro that names the plugin type and the
extension points it exports. The host's bindings read the same WIT files.

A part of a plugin is an extension point of one domain of OTTY, and each
domain has its own module and its own WIT packages, so two domains may
each have a "provider". The `agents` domain, Agents mode, is
`otty_plugin_sdk::agents`: the `Agent` and `Provider` traits and every
type only they use, with the WIT packages `otty:agents-agent` (whose
`types` interface holds the values both share, such as `ReadError` and
`Limits`) and `otty:agents-provider`. The host operations, keys and
needs are at the crate root and in `otty:host`, since no domain owns
them. The optional `Language` trait of the whole plugin belongs to no
domain (`otty:language`). A plugin names what it exports as
`export!(Example with agents::agent, agents::provider, language)`.

It also holds a test host, so a plugin's tests run natively with plain
`cargo test`: the host operations are served from the plugin's
`fixtures/` folder and canned HTTP answers, and every call is checked
against the permissions of the part being tested. The test host is never
built for `wasm32-wasip2`.

Where a number comes from is the plugin's business. An agent part that
reads the limits its agent shows implements `Agent::read_limits(profile,
login)`: it tries its own programs, files or servers in its own order and
returns the first limits it gets, or one `ReadError` that says why.
OTTY asks the provider part of the login first and the agent part only
when that gives nothing. The default says "not supported", and OTTY
shows no limits of the agent.

What the user must do for an agent or a provider is the part's to say,
as a `Need`: a stable id, a title, a text and one button, a `NeedAction`
(run a command in a terminal tab, ask for a key under the part's own
slot, open an `https` page, or copy a report). A part gives it in
`ReadError::NeedsUser` when it can not read until the user acts, and in
`ProfileStatus::NotLoggedIn` for a profile nobody is logged in with.
OTTY shows it as it is, with the plugin's name. A key the user gave for
an `ask-key` comes back to `Provider::numbers` and `Provider::read_bill`
as a `GivenKey` handle with its slot; `given_key(given, slot)` finds it.

A provider part that reads with a login an agent keeps, such as a
ChatGPT or a Claude login, does not know which agent keeps it. When the
provider refuses it or it expired, the part answers only
`ReadError::LoginRefused`; OTTY asks the agent part that found the login,
through `Agent::login_refused(profile, login)`, what the user must do,
such as run the agent in the profile's folder so it renews the login.
The default gives no need, and OTTY then says itself that the login was
refused. `ReadError::NeedsUser` stays for keys the provider part itself
asks for, such as an admin key.

The texts are in the language `env::language()` names, or in English;
`env::in_language` picks one of the part's own translations.
`env::language()` gives the language OTTY shows at that call, which may
change while the plugin is loaded. A plugin with translations implements
`Language`, names `language = "0.1"` in its manifest's `[api]` and
exports it: OTTY calls `Language::language_changed(tag)` when the user
picks another language, and reads the plugin's parts again at once when
it answers `true`, so its needs show in the new language. A plugin
without it is never called. In tests, `TestHost::set_language` switches
the language between two calls.

Plugin authors depend on it by a git tag of the OTTY repository; it goes
to crates.io with its first stable version. The plugins in this
repository depend on it by path. See `plugins/template/` for a plugin to
start from.

Do not use `std::collections::HashMap` or `HashSet` in a plugin: their
random seed makes the `.wasm` import `wasi:random`, and OTTY refuses a
plugin that imports anything but `wasi:cli`, `wasi:io` and `otty:*` at
install. Use `BTreeMap` and `BTreeSet`.
