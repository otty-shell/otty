# 28 OTTY ships no plugins; the user installs them

Status: done
Blocked by: 21

## What to build

Owner, 2026-11-21: on first start OTTY has no plugins and Agents mode is
empty. The user installs plugins, and the installed plugins stay between
starts. Official plugins are installed the same way as any other plugin:
**Add plugin from folder…** now, **Add plugin…** from a repository with
releases once the official plugins have their own repositories. This
replaces "Embedded plugins" of Decision 10.

1. The binary carries no plugin. Remove the built-in copies and all that
   only serves them: `crates/plugins/releases/src/embedded.rs` and its
   `include_bytes!`, `Installer::install_embedded`, the start-up install
   (`otty_app_plugins_io::install_built_in` in
   `crates/app/otty/src/startup.rs`), the `embedded` source, the
   `built_in` flag, `removed_built_in`, the "Official · built in" mark
   and its words in every locale, the "damaged built-in copy" error, and
   "a newer OTTY brings its newer copy".
2. No migration: Agents mode and `plugins.json` were never released. An
   old `plugins.json` with `"source": "embedded"` need not be read; the
   owner moves it away by hand.
3. `assets/plugins/<id>/` stays: the built folders that the tests read
   and that authors and the owner install from with **Add plugin from
   folder…**. `plugins/embed.sh` no longer embeds anything; rename it to
   `plugins/build.sh` and update every reference.
4. Agents mode with no agent plugin installed (or all agent parts off)
   says so in one short line and has a button that opens the Plugins
   page.
5. The Found by OTTY report opens on the first start that has an agent
   part to read, not on a start with no plugins; it is not marked shown
   before it was really shown.
6. Spec and public docs follow: Decision 10 and every place that says the
   official plugins are embedded or installed on first start
   (`spec.md`, `docs/public/Agents/*.mdx`, the official plugin pages,
   `plugins/*/README.md`).

## Done when

- Tests first: a start installs no plugin; an installed plugin is there
  after the next start; Agents mode with no agent plugin shows the line
  and the button; Found by OTTY does not open with no agent part.
- No `include_bytes!` of a `.wasm` in non-test code.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-21, issue 28, done. OTTY carries no plugin and a start installs
  none. Gone: `crates/plugins/releases/src/embedded.rs` (with every
  `include_bytes!`), `Installer::install_embedded`, the `embedded` source,
  the `built_in` flag, `removed_built_in`, `UpdateOutcome::Installed`,
  `InstallError::DamagedBuiltIn`, `otty_app_plugins_io::install_built_in`
  and its step in `startup.rs` (its `plugins` argument went with it),
  its clippy ban, the "Official · built in" mark and the "damaged
  built-in copy" error with their words in all ten locales, and the
  `SHA256SUMS` files of `assets/plugins/<id>/`, which only the built-in
  check read. `plugins/embed.sh` is now `plugins/build.sh`: it builds a
  plugin into `assets/plugins/<id>/` (manifest, `.wasm`, icons) and
  writes no hashes. Agents mode with no agent part on shows the card "No
  agent plugin" with one line and **Manage plugins…** (the words of
  Settings), which opens the list of the Plugins page; it shows before
  the spinner, since nothing can be read. Found by OTTY opens only after
  a read with an agent part on, so a start with none neither opens it
  nor marks it shown. Tests: a first start has no plugin and a plugin
  installed from a folder is there on the next start (plugins io); every
  folder of `assets/plugins/` installs from its folder through the real
  engine with all parts on (replaces the built-in install tests of the
  releases crate and of plugins io); the start-up tasks hold no install;
  the card, its button and its outcome; Found by OTTY with no agent part.
  Spec (Decision 10, the "Embedded plugins" paragraph, now "No plugin
  ships with OTTY", the UI list, the crates list, out of scope, the
  settled question), the public docs (the Plugins page with a new
  section "Install the official plugins", Overview, Write a Plugin, What
  OTTY Reads and Sends, the five official plugin pages) and the READMEs
  of the official plugins say how to install them now. Left as they
  were: the comments of earlier issues and `docs/research/wasm-plugins.md`,
  which record what was true then, and the comments about the in-core
  "readers built into OTTY" (`app.rs`, `account_numbers.rs`,
  `profile_list.rs`, `plugin.rs` of the host), which are not about plugin
  copies.
