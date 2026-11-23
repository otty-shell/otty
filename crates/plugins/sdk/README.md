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
`types` interface holds the values both share, such as `ReadError`) and
`otty:agents-provider`. The host operations, keys and
needs are at the crate root and in `otty:host`, since no domain owns
them. The optional `Language` trait of the whole plugin belongs to no
domain (`otty:language`). A plugin names what it exports as
`export!(Example with agents::agent, agents::provider, language)`.

It also holds a test host, so a plugin's tests run natively with plain
`cargo test`: the host operations are served from the plugin's
`fixtures/` folder and canned HTTP answers, and every call is checked
against the permissions of the part being tested. The test host is never
built for `wasm32-wasip2`.

An agent part does agent things only: profiles, sessions, tokens and
their money, and the commands that start the agent. It knows nothing of
accounts: `Agent::detect` says a profile is found, not logged in (with
the need that fixes it, the part's own call) or gone, never whose login
it is. Each answer carries its money: the cost the agent wrote, as it
is, else its tokens at the API price of the model that OTTY's price list
gives (`agents::price_of`, the host call `prices.price-of`), marked "at
API prices". `agents::answer_cost` does both; OTTY only adds the money
up. In tests, `TestHost::with_price` gives the price list.

A provider part owns its accounts. OTTY calls `Provider::accounts` with
the profile folders of the agent parts its manifest names in `logins`
(`LoginFolder::Profile`), and the part reads the logins there with its
own permissions, keys as handles. Each account has an id the part keeps
stable, also after its tokens are renewed, a name, a kind, the folders it
was found in and its needs. Where a number comes from is the plugin's
business: `numbers` reads an account any way it can, only in the folders
of `Account::found_in`, from the provider's server with the login there,
or by asking the agent itself; the provider world imports files,
programs and hidden terminals for that. With no way that works the part
answers numbers with nothing in them, not an error. In tests,
`TestHost::as_provider_in` runs a call with its folders.

What the user must do for an agent or a provider is the part's to say,
as a `Need`: a stable id, a title, a text and one button, a `NeedAction`
(run a command in a terminal tab, ask for a key under the part's own
slot, open an `https` page, or copy a report). A part gives it in
`ReadError::NeedsUser` when it can not read until the user acts, and in
`ProfileStatus::NotLoggedIn` for a profile the agent can not log in
with.
OTTY shows it as it is, with the plugin's name. A key the user gave for
an `ask-key` comes back to `Provider::numbers` and `Provider::read_bill`
as a `GivenKey` handle with its slot; `given_key(given, slot)` finds it.

A provider part that sees a login refused or expired says so with a need
of the account, in `Account::needs` or as `ReadError::NeedsUser` of a
read, such as one that runs the agent's login command in the folder the
account was found in. Whether a login expired is the plugin's call.

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
