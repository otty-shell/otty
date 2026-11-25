# 26 The plugins move to their own repositories

Status: done
Blocked by: 25

## Why

Owner, 2026-11-25: keep three plugins, OpenAI, Anthropic and OpenCode,
and move every plugin out of the OTTY repository into its own folder next
to it, from where each is pushed to its own repository; the owner will
then try to install them from the repositories. The OTTY repository keeps
no plugin at all and no `assets/plugins/`.

Owner's answers, 2026-11-25:

1. Folders next to the OTTY checkout, each its own git repository
   (`git init`, a first commit, `origin` set, nothing pushed):
   `../otty-plugin-anthropic`, `../otty-plugin-openai`,
   `../otty-plugin-opencode` (remotes
   `git@github.com:otty-shell/otty-plugin-<id>.git`, matching the
   manifests' `repository`), and `../otty-plugin-template` for the
   template (`otty-shell/otty-plugin-template`).
2. DeepSeek and OpenRouter are deleted.
3. `plugins/` and `assets/plugins/` leave the OTTY repository. The host
   tests that use the built template (`template.wasm`) use a small test
   plugin of the host tests instead, beside `minimal`, `programs` and the
   others; the OTTY tests that ran the real plugins use it too. Each
   plugin's own tests move with it.
4. The plugins take the SDK from git: `otty-plugin-sdk = { git =
   "https://github.com/otty-shell/otty", branch = "agents" }`, so the
   `agents` branch is pushed before them. A tag comes at the release.
5. Each repository gets a GitHub Actions workflow that, on a tag
   `v<version>`, builds the `.wasm` for `wasm32-wasip2` and attaches to
   the GitHub release exactly the files OTTY's installer expects
   (`otty-plugin.toml`, the `.wasm`, the icons; check
   `crates/plugins/releases` for the asset names and the checks it does,
   such as the tag matching the version).
6. Nothing is pushed for now, and coverage is not measured now.

## What to build

1. **Plugin repositories.** Copy each plugin's sources (`Cargo.toml`,
   `Cargo.lock`, `otty-plugin.toml`, `src/`, icons, fixtures, README,
   tests) into its folder, with a `.gitignore` (`target/`), a README that
   says how to build (`cargo build --release --target wasm32-wasip2`) and
   release (push a tag), the workflow of answer 5, and the SDK from git
   (answer 4). Check each builds and its tests pass locally by pointing
   the git SDK at the local checkout for the check only, without
   committing that (for example `cargo --config
   'patch."https://github.com/otty-shell/otty".otty-plugin-sdk.path="<otty>/crates/plugins/sdk"'`).
   The plugin's docs page (now in `docs/public/Agents/Official Plugins/`)
   may also go into its README; the OTTY docs keep a short page per
   official plugin that links its repository.
2. **The OTTY repository.** Remove `plugins/` and `assets/plugins/`
   (with `plugins/build.sh`), DeepSeek and OpenRouter wherever they are
   named, and every test, fixture, script and doc that builds or reads
   them. Replace the template fixture with a host test plugin built by
   `scripts/build-test-plugins.sh` from `crates/plugins/host/tests/`
   sources, and move the OTTY tests that ran real plugins to it (keep
   what they check). Docs: Write a Plugin points at the template
   repository; the official plugin pages say where each lives and how to
   install it from its repository.
3. Nothing in the OTTY repository refers to `assets/plugins` or `plugins/`
   any more (grep), and the workspace builds and tests with no plugin.

## Done when

- The four folders are git repositories with one first commit each,
  `origin` set, nothing pushed; each builds for `wasm32-wasip2` and passes
  its own tests against the local SDK.
- The OTTY repository has no `plugins/` and no `assets/plugins/`, the
  tests that used them pass on the host test plugin, and the checks of
  `AGENTS.md` pass.

## Comments

- 2026-11-25, done. The four folders next to the OTTY checkout are git
  repositories with one first commit each (`otty-plugin-anthropic`
  d142657, `otty-plugin-openai` bb22f5d, `otty-plugin-opencode` 1e1ad13,
  `otty-plugin-template` 7d03c89), `origin` set to
  `git@github.com:otty-shell/otty-plugin-<id>.git`, nothing pushed. Each
  takes the SDK from the `agents` branch, has a `.gitignore`, a README
  (build, the local SDK for one command, install, release by tag) and
  `.github/workflows/release.yml`: on a tag `v<version>` it checks the
  tag is semver and equals `version` of the manifest, runs the tests,
  builds for `wasm32-wasip2` and attaches `otty-plugin.toml`,
  `<id>.wasm` (the manifest's `wasm`, which must be `<id>.wasm`) and the
  icons, the files `crates/plugins/releases` downloads. Each built for
  `wasm32-wasip2` and passed its tests against the local SDK (Anthropic
  202, OpenAI 213, OpenCode 36, template 34). Their `Cargo.lock` pins no
  SDK commit yet: the first build without the local SDK pins the head of
  `agents`, so after `agents` is pushed run `cargo update -p
  otty-plugin-sdk` in each and commit the lock. The template's manifest
  names `otty-plugin-template` as its repository.

  The OTTY repository: `plugins/` (with `build.sh`) and `assets/plugins/`
  are gone, with DeepSeek, OpenRouter, their docs pages and the tests that
  installed every built plugin. The host test plugin is
  `crates/plugins/host/tests/example/` (`otty-test-plugin-example`): the
  template with its tests left out, texts in English and Russian only,
  and a `/usage` read of a login with no key in a hidden terminal.
  `scripts/build-test-plugins.sh` builds it into
  `tests/fixtures/example/` as an installed folder (manifest,
  `example.wasm`, icons); its manifest is the template's, so the tests of
  Plugins mode keep their groups, and the two tests that need the hidden
  terminal add its permission to a copy. `template.wasm` is gone. The
  host, releases, otty-agents, Agents io and Plugins io tests use it; the
  Agents io tests that ran the Anthropic, OpenAI and OpenCode plugins now
  run the example agent and provider and check the same things, but for
  the OpenCode "works with no login" case (the plugin's own behaviour).
  The ignored measures of the real `~/.codex`, `~/.claude` and OpenCode
  data were removed: they ran the official plugins on the user's files.
  The made-up DeepSeek and OpenRouter providers of OTTY's tests are now
  Tealcloud and Hopway. Left on purpose: the LiteLLM table data and docs
  in `litellm.rs` and `price_table.rs`, which name LiteLLM's real
  providers, and `docs/research/`.

  Tests first (`wip: issue 26 tests`): the moved tests did not build, the
  example plugin was not built yet. Docs: Write a Plugin starts from the
  template repository; Plugins and each official plugin page name its
  repository and install it with **Add plugin…**.
