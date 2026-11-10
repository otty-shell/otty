# Feature crates

## Problem

The whole app is one crate, `app`. Its layers are folders, and inside one crate
any module can import any other, so the compiler does not hold the boundaries.
We held them with text rules, an architecture test and a widget generator. The
rules grew to 37, the test could be bypassed, and keeping the documents in step
with the code took more time than writing features.

Widgets also talk to each other through the iced message loop. A widget returns
an effect, the effects file turns it into a message for another widget, and
iced delivers that message on a later update. `app/src` has about 110 such hops
(`Task::done`) and only about 30 places with real I/O. Every hop adds types, and
between two hops iced can draw a frame or take a key press. Several focus and
ordering bugs came from that gap.

## Goal

Build the app again as a set of crates under `crates/app/`, the way Zed splits
its features:

- the compiler holds the boundaries through crate dependencies;
- a feature answers with a plain value instead of sending a message to another
  feature;
- every feature plugs into the window through the same socket, and how it is
  built inside is free.

The user sees no change in behaviour. One thing gets better: a chain of
reactions, for example a launch that opens a tab with a terminal, happens in one
update, so no frame shows a half-done state.

## The old app stays

The old `app` crate stays as it is while the new app is built:

- nothing in `app/` is changed or deleted by this work;
- it keeps building, its tests keep running, and the release still ships it;
- the new app copies code from it and rewrites it crate by crate, so the owner
  can compare how it was with how it is.

The owner removes `app/` when the new app is complete.

Both apps live in one workspace, and package names must be unique, so the new
binary is the package `otty-app` (binary `otty-app`) until the old one is gone.

## Layout

```
crates/
  app/
    otty/                   otty-app, the binary
    features/
      containers/           otty-app-containers
      quick-launch/         otty-app-quick-launch
      terminal/             otty-app-terminal
      tabs/                 otty-app-tabs
      explorer/             otty-app-explorer
      palette/              otty-app-palette
      settings/             otty-app-settings
      notifications/        otty-app-notifications
    domain/                 otty-app-domain
    i18n/                   otty-app-i18n
    theme/                  otty-app-theme
    slots/                  otty-app-slots
    testing/                otty-app-testing
  containers/  escape/  libterm/  pty/  surface/  vte/
  ui/  kit/  terminal/  tree/
```

## Crates

### Features

A feature is one thing the user works with. It can fill several places in the
window.

| Crate | Copied from | Places it fills |
|---|---|---|
| `otty-app-containers` | `app/src/widgets/containers`, with `child/*` as inner modules | left panel, main area, right panel, status bar, overlays |
| `otty-app-quick-launch` | `widgets/quick_launch`, without `services.rs` and `storage.rs` | left panel, main area (the launch editor tab), overlays |
| `otty-app-terminal` | `widgets/terminal_workspace` | main area, overlays |
| `otty-app-tabs` | `widgets/tabs` | left panel (the Sessions section) |
| `otty-app-explorer` | `widgets/explorer` | right panel |
| `otty-app-palette` | `widgets/palette` | overlays |
| `otty-app-settings` | `widgets/settings` | overlays |
| `otty-app-notifications` | `widgets/notifications` | overlays |

### Helpers

| Crate | What it holds | iced |
|---|---|---|
| `otty-app-domain` | `domain/` (`Mode`, `ExitBehavior`, `QuickLaunch`), `commands.rs` (`AppCommand`), shortcut names and labels (`AppShortcut`), `shell_session.rs`, and the types one feature hands to another (`TabLaunch`, `TabMachine`, `WizardTabInit`) | no |
| `otty-app-i18n` | `i18n/` | no |
| `otty-app-theme` | `theme.rs`, `icons.rs`, `fonts.rs`, `ThemeProps` | yes |
| `otty-app-slots` | the socket traits, `Overlay`, `MainContent`, focus ids from `focus.rs` | yes |
| `otty-app-testing` | test helpers the feature crates share; only a dev-dependency | yes |

`geometry.rs` (where a menu opens) goes to `otty-ui-kit`: it is plain interface
math and knows nothing about the app.

### Binary

`otty-app` is the window frame and everything that links features together:

- `main.rs`, `app.rs`, `Message`, `update.rs` with the input checks and the
  overlay rules from `guards.rs`;
- `view.rs`: the frame and the one layer order;
- `screens/`: which feature fills which place in each mode;
- `dispatch/`: reads each `Outcome` and calls the next feature;
- `effects/`: all I/O, including `services.rs`, `shell_integration.rs`, the
  SSH check from `quick_launch/services.rs`, the storage of quick launch and
  settings, and the Docker calls;
- `subscription.rs` and turning iced key presses into `AppCommand`;
- the frame widgets `chrome`, `sidebar` and `modes` as plain modules.

## Boundaries

### Crate dependencies (checked by Cargo)

- A feature crate depends on helper crates, `otty-ui-*` and libraries. It never
  depends on another feature or on the binary.
- A feature crate has no direct `tokio` or `notify` dependency. `tokio` may
  still reach it through `otty-ui-term` (for example via `otty-app-theme`);
  that is fine, because clippy bans starting background work in a feature.
  A feature that needs a timer says so through a getter, and the binary runs
  `iced::time::every`. It uses `otty-containers`
  only with `default-features = false`, which gives the data types without the
  Docker client.
- Helper crates never depend on a feature or on the binary. `otty-app-domain`
  and `otty-app-i18n` have no iced.
- `otty-ui-*` never depend on a crate under `crates/app/`.

### Clippy bans

- Feature crates ban `std::fs`, `std::process::Command`,
  `std::env::{var, var_os, vars}` and
  `iced::Task::{perform, future, run, stream}`.
- The bans live in one `clippy.toml` shared by the feature crates. Clippy reads
  the nearest `clippy.toml` from the crate folder upwards; issue 01 checks this
  by planting a violation. If it does not work that way, each feature crate gets
  its own copy.
- The binary may do I/O, and review keeps it in `effects/`.
- The root `clippy.toml` stays for the old app.

## The socket

The traits live in `otty-app-slots`:

```rust
/// What every feature gives: its events, its answer and how it changes.
pub trait Feature {
    type Intent;
    type Outcome;
    /// Outside data the feature reads while it reduces; `()` when none.
    type Ctx<'a>;

    fn reduce(&mut self, intent: Self::Intent, ctx: Self::Ctx<'_>) -> Vec<Self::Outcome>;

    fn subscription(&self) -> Subscription<Self::Intent> {
        Subscription::none()
    }
}

/// A feature that draws the left panel.
pub trait LeftPanel: Feature {
    fn left_panel<'a>(&'a self, theme: ThemeProps<'a>) -> Element<'a, Self::Intent>;
}

/// A feature that draws the main area.
pub trait MainArea: Feature {
    fn main_area<'a>(&'a self, theme: ThemeProps<'a>) -> MainContent<'a, Self::Intent>;
}

/// A feature that draws the right panel.
pub trait RightPanel: Feature {
    fn right_panel<'a>(&'a self, theme: ThemeProps<'a>) -> Element<'a, Self::Intent>;
}

/// A feature that draws the status bar.
pub trait StatusBar: Feature {
    fn status_bar<'a>(&'a self, theme: ThemeProps<'a>) -> Element<'a, Self::Intent>;
}

/// A feature that draws a menu or a dialog over the window.
pub trait Overlays: Feature {
    fn overlay<'a>(&'a self, theme: ThemeProps<'a>) -> Option<Overlay<'a, Self::Intent>>;
}
```

The exact signatures are settled when the first feature implements each trait.
Each trait is added when the first feature needs it, not all at once.

Why traits: AGENTS.md allows an abstraction with several real implementations,
and each trait here has two to eight. A trait writes the socket as code, so the
compiler checks it: a screen that puts a feature into a place it does not
implement fails to build. Nothing calls features through the traits
generically; every call names its feature.

Two rules come with the socket:

- **A view draws only from its own state.** Outside data reaches that state
  earlier, through an `Intent`. For example, the host name reaches `tabs` as
  `TabsIntent::HostNameRead(name)` instead of `tabs.vm(host_name)`.
- **`reduce` reads outside data through `Ctx`**, for data that is read at the
  moment of the decision, such as the terminal settings or which commands can
  run.

## How things move

### Types

| Type | Lives in | What it is | Made by |
|---|---|---|---|
| `Intent` | the feature crate | the feature's input: "do this" | its view (a click), its subscription (a tick), an I/O answer, the binary on behalf of another feature |
| `Outcome` | the feature crate | what `reduce` reports: "this happened, this is what I need from outside" | only `reduce` |
| `Message` | the binary | the only message that goes through iced | iced: input, window events, ticks, I/O answers |
| shared types | `otty-app-domain` | data one feature hands to another | — |

What goes away:

- `XEffect`: a request for I/O and news for another feature are now variants
  of `Outcome`;
- the `XEvent { Intent, Effect }` wrapper;
- `AppEvent`, which becomes the binary's `Message`: one variant per feature
  with its `Intent`, plus keyboard, window and commands;
- `Task::done` between features.

`Task` stays only in the binary: `Task::perform` for I/O and iced operations
such as focus and scrolling.

### Flow

```
  click, key, tick, Docker answer
                │  Message, the only message through iced
                ▼
  update(app, message)
    input checks: an open menu or dialog takes the input first
                │
                ▼
  feature.reduce(intent, ctx)  ──►  Vec<Outcome>
                │
                ▼
  dispatch/<feature>.rs reads each Outcome. There are three kinds:
    1. news for another feature ─► other.reduce(...) right away; its outcomes
                                   are read the same way, in the same update
    2. I/O is needed            ─► effects/<feature>.rs ─► Task::perform(...);
                                   the answer comes back later as a Message
                                   with an Intent of the same feature
    3. a window operation       ─► an iced Task (focus, scrolling)
                │
                ▼
  Task<Message> goes back to iced
```

Rules:

- `dispatch/` makes no decisions. Each outcome arm is one call. When the data
  for a call is missing, it sends nothing.
- A decision about a feature's state lives in that feature's `reduce`. For
  example, whether to focus a terminal after a tab closes is decided by the
  terminal feature, which gets `TabClosed { tab_id, new_active_id }`.
- `effects/` changes no state. It turns an I/O answer into a `Message`.
- The binary builds each `Ctx` from other features' getters before `reduce`.
- A feature's `subscription()` gives `Subscription<Intent>`, and the binary
  wraps it into `Message`.
- A type both sides read the same way lives in `otty-app-domain`. Otherwise the
  binary converts field by field.

### Commands and shortcuts

- A shortcut: the binary turns the key press into an `AppCommand`, and
  `dispatch/command.rs` calls the right feature.
- The palette: `palette.reduce` returns `CommandPicked(AppCommand)`, and
  `dispatch` takes the same path.

### Overlays

A feature returns `Overlay { kind, content }`, where `kind` is a menu or a
dialog. The frame owns everything else:

- the layer order, bottom to top: content; mode overlays; header menus and the
  modes menu; the palette; the SSH secret prompt;
- a menu closes on Esc, an outside click or a shortcut, and keys under it never
  reach the terminal behind;
- a dialog ignores shortcuts and outside clicks, and Esc means Cancel;
  Settings also closes on a click outside, which drops unsaved edits like Esc;
- focus after an overlay closes: in Terminal mode the active pane, in
  Containers mode nothing.

### Example: opening an SSH launch that asks for a password

| # | What arrives | Who handles it | What comes out |
|---|---|---|---|
| 1 | `Message::QuickLaunch(LaunchNode)`, a click | `quick_launch.reduce` | `[PrepareLaunch(request)]` |
| 2 | — | `dispatch/quick_launch.rs`, kind 2 | `effects::quick_launch::prepare` → `Task::perform` |
| 3 | `Message::QuickLaunch(SetupCompleted(SecretRequired))` | `quick_launch.reduce` opens the password prompt | `[]`; the frame draws it through `Overlays` and focuses it |
| 4 | `Message::QuickLaunch(SecretPromptSubmit)` | `quick_launch.reduce` | `[PrepareLaunch(request with the password)]`, then step 2 again |
| 5 | `Message::QuickLaunch(SetupCompleted(Prepared))` | `quick_launch.reduce` | `[LaunchReady(TabLaunch)]` |
| 6 | — | `dispatch`, kind 1: `tabs.reduce(OpenCommandTab(launch))` | `[CommandTabOpened { tab_id, … }]` |
| 7 | — | `dispatch`, kind 1: `terminal.reduce(OpenTab { tab_id, … })` | `[TerminalStarted]` |
| 8 | — | `dispatch`, kind 1: `modes.reduce(TabShown)` | `[]` |

Four messages go through iced: the click, two answers of the SSH check and the
password. Steps 5 to 8 happen in one update.

## Moving a feature

A view that gathers buttons of several features is drawn by the frame, not by
one of them, and each button sends an `Intent` to its own feature. The New
session menu (New terminal, Split, the launches, New launch) and the start
card are such views in the binary.

Issues 06 to 14 each move one feature, and each follows the same steps:

1. Create the crate in `crates/app/features/<name>` and copy the old widget's
   code into it. The old widget stays where it is.
2. Rewrite the copied `reduce` tests so they check the returned outcomes, and
   see them fail. Then change `reduce` to return `Vec<Outcome>`. `XEffect`,
   the `XEvent` wrapper and every `Task::done` to another widget go away.
3. Move the feature's I/O to `effects/<name>.rs` in the binary, and its I/O
   subscriptions (file watching, Docker events) to the binary's
   `subscription.rs`. Their answers come back as the feature's `Intent`.
4. Implement `Feature` and the place traits for the places it fills. The first
   feature that fills a place adds that trait to `otty-app-slots`.
5. Make the views draw only from the feature's own state. Outside data comes in
   through an `Intent`.
6. Wire it into the binary: its `Message` variant, `dispatch/<name>.rs`,
   `effects/<name>.rs`, the screens, its subscription and its `Ctx`.
7. Add each type it hands to another feature to `otty-app-domain`, when both
   sides read it the same way.

The issue is done when the feature behaves in the new app as in the old one,
the crate breaks none of the boundaries above, and all checks from AGENTS.md
pass and are reported. A feature that is not moved yet is simply missing from
the new app; links to it are wired by the issue that moves it.

## Exceptions

- **The terminal feature owns live terminal sessions.** Creating a terminal
  starts a shell in a PTY (a pseudo-terminal), which is I/O, and `otty-ui-term`
  does it when the terminal is created. The terminal feature keeps the live
  sessions in its state. Zed's `terminal` crate does the same.

## Inside a feature

Free, with one rule: a feature's `Ctx` type lives in its own `ctx.rs`, not
next to the events, so it is easy to find what the feature reads from
outside. There are no other rules about which files a feature has or how its
types are named, and no generator. Review keeps it readable. If a trap shows up more than
once, it becomes a short rule in AGENTS.md, written as a trap to avoid, not as a
map of files.

## Tests

- **A feature:** call `reduce` and check the returned outcomes and the state. No
  iced runtime, no messages.
- **The binary:** give a `Message`, run `update`, and check the state of several
  features, for example "a ready launch opens a tab with a terminal".
- **I/O:** check that a Docker or SSH answer becomes the right `Intent`.

Tests are copied with the code from the old app and rewritten to check outcomes
first, then the code changes.

## Out of scope

- Splitting `otty-app-containers` further. Its sorting, table search, resource
  rows and inspect tree are about resource tables in general; they move to a
  shared crate only when a Kubernetes viewer needs them.
- Any change in behaviour or design, and new features.
- Plugins.
- Removing the old app and changing the release, see below.

## Open questions

1. **Coverage.** AGENTS.md asks for strictly higher line coverage than the merge
   base after every Rust change. Copied code comes with its tests, and binary
   wiring code is mostly untested. Each issue reports the coverage numbers. If a
   step cannot raise coverage, the owner decides.

## Issues

1. [01 Domain crate](issues/01-domain-crate.md)
2. [02 Translations crate](issues/02-i18n-crate.md)
3. [03 Theme crate](issues/03-theme-crate.md)
4. [04 Docker types without the client](issues/04-containers-client-feature.md)
5. [05 Binary with the window frame](issues/05-binary-frame.md)
6. [06 Notifications](issues/06-notifications.md)
7. [07 Explorer](issues/07-explorer.md)
8. [08 Palette](issues/08-palette.md)
9. [09 Settings](issues/09-settings.md)
10. [10 Tabs](issues/10-tabs.md)
11. [11 Terminal](issues/11-terminal.md)
12. [12 Quick launch](issues/12-quick-launch.md)
13. [13 Containers: engines and lists](issues/13-containers-engines-and-lists.md)
14. [14 Containers: inspector, forms and actions](issues/14-containers-inspector-forms-actions.md)
15. [15 Parity check](issues/15-parity-check.md)
16. [16 Component trait and the frame folder](issues/16-component-and-frame-folder.md)
17. [17 I/O crates for features and the shell helper](issues/17-feature-io-crates.md)

Order after 14: 17, then 16, then 15.

Features go one by one, from simple to complex, so every step builds and every
test passes.

## When the old app is removed

The owner does this after issue 15:

- delete `app/` and its workspace entry;
- rename the package `otty-app` to `otty`, because the release and the scripts
  use `-p otty`;
- in `.github/workflows/release-app.yml`, change `cargo generate-rpm -p app`
  (a path) to `-p crates/app/otty`;
- in the binary's `Cargo.toml`, change the deb assets from `../assets/...` to
  `../../../assets/...` and `readme` to `../../../README.md`;
- in `docs/internal/version-control.md`, change the version source from
  `app/Cargo.toml` to `crates/app/otty/Cargo.toml`;
- remove the old-app section of the root `clippy.toml` and the AGENTS.md line
  about the old app.
