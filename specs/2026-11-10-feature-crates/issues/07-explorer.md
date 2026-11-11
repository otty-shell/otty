# 07 Explorer

Status: needs-triage
Blocked by: 06

## What to do

Move `app/src/widgets/explorer` into `otty-app-explorer`, following "Moving a
feature" in the spec. It adds the `RightPanel` trait to `otty-app-slots`.

- Reading folders goes to `effects/explorer.rs` in the binary.
- Watching files for changes uses `notify`, which a feature may not depend on,
  so the watcher lives in the binary's `subscription.rs` and sends explorer
  intents.
- Following the folder of the active terminal (`SyncRoot`) is wired in issue 11,
  when the terminal moves.

## Done when

- The Files section in the right panel of Terminal mode shows and opens folders
  and files as in the old app, and updates when files change.
- `otty-app-explorer` has no `notify` and no `tokio`.
- The steps and checks in "Moving a feature" are done.

## Comments

### 2026-11-10: the explorer feature crate

**`otty-app-explorer`** (`crates/app/features/explorer`) is a copy of
`app/src/widgets/explorer` without `services.rs`, `errors.rs`,
`subscription.rs` and `watcher.rs`, which are I/O and went to the binary.
The old widget is unchanged (`git diff --stat a32f980 -- app` is empty). It
depends on `iced` (with `svg`), `log`, `otty-app-i18n`, `otty-app-slots`,
`otty-app-theme`, `otty-ui-kit` and `otty-ui-tree`; for tests on
`iced_test`. Its public API:

- `Explorer` with `new` and `watched_folders() -> Vec<PathBuf>`: the
  followed folder and every folder of the tree that was read, sorted. The
  feature decides what to watch; the binary runs the watch, as with
  `toast_timer()` of notifications;
- `FileNode` (with `new`, the getters and the folders-first `Ord`) and
  `TreePath`, because the binary builds the nodes from a folder read;
- `impl Feature` with `Ctx<'a> = ()`: `reduce` reads no outside data, so
  there is no `ctx.rs`;
- `impl RightPanel`: the Files section (title, the folder line, the tree).

```rust
pub enum ExplorerIntent {
    NodePressed { path }, NodeHovered { path }, SyncRoot { cwd },
    RootLoaded { root, nodes }, FolderLoaded { path, nodes },
    DirectoryChanged { directory }, DirectoryReloaded { directory, nodes },
    LoadFailed { message }, FolderLoadFailed { path, message },
}

pub enum ExplorerOutcome {
    /// Read the followed folder; answer RootLoaded or LoadFailed.
    LoadRoot { root: PathBuf },
    /// Read a folder the user pressed open; answer FolderLoaded or
    /// FolderLoadFailed.
    LoadFolder { path: TreePath, directory: PathBuf },
    /// Read a watched folder that changed again; answer DirectoryReloaded or
    /// LoadFailed.
    ReloadFolder { directory: PathBuf },
    /// Open a pressed file in a command tab with the saved editor.
    OpenFile { file_path: PathBuf },
    /// Tell the user about a failure; the text is ready to show.
    Failed(String),
}
```

`ExplorerEffect`, the `ExplorerEvent` wrapper and the five `Task::done` of the
reducer are gone. The copied reducer tests were rewritten to check the
returned outcomes and run first against the copied `Task` reducer (14 build
errors: no `==` and no `is_empty` on a `Task`), then `reduce` changed to
return `Vec<ExplorerOutcome>`. Two reducer tests are new: a pressed open
folder closes and reads nothing, a pressed file asks to open it. The
`FileNode` order tests moved from `model.rs` to `types.rs`, next to the
type.

**The `RightPanel` trait** in `otty-app-slots` (`src/right_panel.rs`):

```rust
pub trait RightPanel: Feature {
    fn right_panel<'a>(&'a self, theme: ThemeProps<'a>)
        -> Element<'a, Self::Intent>;
}
```

The signature of the spec fits: the frame owns the panel (open or closed,
its width, the resize edge), and the feature gives only what fills it, drawn
from its own state. The containers inspector (issue 13) is the second
implementation.

**How the binary wires it:**

- `AppEvent::Explorer(ExplorerIntent)` and `App::explorer`;
- `dispatch/explorer.rs`: one call per outcome. `LoadRoot`, `LoadFolder`
  and `ReloadFolder` go to `effects/explorer.rs`; `Failed(text)` →
  `notifications::deliver(Push(text))` in the same update; `OpenFile` does
  nothing yet (see below);
- `effects/explorer.rs`: `load_root`, `load_pressed_folder`,
  `reload_folder` (each a `Task::perform` around `read_dir_nodes`, copied
  with `ExplorerError` from the old `services.rs` and `errors.rs`); only the
  read of a pressed folder answers with its own failure
  (`FolderLoadFailed`), the others with the log-only `LoadFailed`, as before;
- `subscription.rs` runs the watch over `app.explorer.watched_folders()`
  with `notify` (new dependency of the binary only, `notify = "8.2.0"`,
  already in `Cargo.lock` through the old app). The stream is in
  `subscription/explorer_watch.rs` and the mapping of a `notify` event to
  the changed folders in `subscription/changed_folders.rs`, both copied with
  their tests; each change comes back as `ExplorerIntent::DirectoryChanged`;
- `screens/terminal.rs`: the right panel of Terminal mode is
  `app.explorer.right_panel(theme)`;
- `guards.rs`: a hover over a Files row (`NodeHovered`) passes under an open
  menu; a press on a row closes the menus, as before.

**Gap of issue 05 resolved:** the right panel of Terminal mode and the file
watching in `subscription.rs`. They were comments, not `dead_code` marks, so
no mark was removed. No "issue 07" note is left in `crates/app/`.

**Left for later issues:**

- 11 terminal: following the folder of the active terminal. `SyncRoot`
  stays in the feature; the binary sends it from the terminal and tabs
  outcomes (old `TabsEffect::Activated`/`Closed` and
  `TerminalWorkspaceEffect::SyncExplorer`). The note is on
  `dispatch::explorer::deliver`. Until then the section says "No active
  folder" and the explorer reads and watches nothing.
- 09 to 11: opening a pressed file. The old app built the terminal settings
  of the saved editor (`services::editor_terminal_settings`) and opened a
  command tab. It needs the saved editor (09), `OpenCommandTab` (10) and the
  terminal settings (11); the note is on the `OpenFile` arm of
  `dispatch/explorer.rs`, and the old test
  `a_file_opens_in_the_saved_editor_not_an_unsaved_one` comes with it.
- 09 and 12: the dialog check and the inline edit check do not exist yet.
  In the old app a Files hover passed under every dialog and kept an inline
  edit; the notes are at those checks in `update.rs`.

**Tests.** Copied and ported: the state, reducer, view and node order tests
of the widget, the folder read and failure tests of the old
`effects/explorer.rs`, the watch and changed-folder tests, the Files hover
and press under a menu from `guards.rs` and `update.rs`. New: the watched
folders and the right panel of the feature, the dispatch of each outcome,
the toast for a pressed folder that can not be read, the right panel of
Terminal mode, the watch starting once a folder is followed, a read root
and a reloaded folder giving their files. `otty-app-explorer` has 31 tests
and 3 doc tests, `otty-app-slots` 7 doc tests (6 before), `otty-app` 353
tests (321 before). The whole workspace runs 4342 tests, all passing.

**Checks.** `cargo tree -p otty-app-explorer -e normal | grep notify` prints
nothing. `tokio` comes only through `otty-ui-term` → `otty-app-theme`
(`cargo tree -i tokio`), as for notifications; no direct `tokio`. A planted
`std::fs::read_dir` in the reducer failed clippy with `use of a disallowed
method std::fs::read_dir` and the note `a feature touches no files; the
binary does it in effects/`; it was removed. `cargo +nightly fmt --all --
--check`, `cargo clippy --workspace --all-targets --all-features -- -D
warnings`, `cargo deny check` (only the 3 known yanked-crate warnings),
`cargo test --workspace --all-features` and `cargo build -p otty-app` pass.
`cargo llvm-cov` was not run, as asked. The window was not opened (no
display for the agent).

**For the owner to decide.**

- `FileNode` and `TreePath` are public in the feature crate, because the
  binary builds the nodes when it reads a folder. The other way is a plain
  entry type in `otty-app-domain`, but only the explorer reads it.
- Until issue 11 the Files section can not show a folder in the new app:
  nothing sends `SyncRoot` yet.
