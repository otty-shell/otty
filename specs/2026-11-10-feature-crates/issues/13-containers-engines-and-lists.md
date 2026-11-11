# 13 Containers: engines and lists

Status: needs-triage
Blocked by: 04, 12

## What to do

Create `otty-app-containers` and move the first half of
`app/src/widgets/containers`, following "Moving a feature" in the spec. It adds
the `StatusBar` trait to `otty-app-slots`.

This issue covers:

- the engine tree in the left panel;
- the main area: the open list with sorting and table search, the searching
  state and the no-engine cards;
- the status bar;
- the Docker calls in `effects/containers.rs` (finding engines, probing, lists)
  and the engine events stream in the binary's `subscription.rs`.

The crate uses `otty-containers` with `default-features = false` (issue 04), so
it has the types and cannot reach the client.

The child modules of the old widget (`actions`, `create`, `inspector`, `run`)
come in issue 14. Inside the crate they are plain modules with no layout rules.

## Done when

- Containers mode shows the engines, the lists of containers, images, volumes
  and networks, sorting, search and the no-engine cards as in the old app, and
  lists update on engine events.
- `otty-app-containers` has no `bollard` and no `tokio` in its dependency tree.
- The steps and checks in "Moving a feature" are done.

## Comments

### 2026-11-10: the containers feature crate, first half

**`otty-app-containers`** (`crates/app/features/containers`) is a copy of the
parent of `app/src/widgets/containers` without `child/`, `subscription.rs`,
`model/inspector.rs` and `view/inspector.rs`; the old widget is unchanged
(`git diff --stat 6235a0f -- app` is empty). It depends on `iced` (with
`svg`), `log`, `thiserror`, `otty-app-domain`, `otty-app-i18n`,
`otty-app-slots`, `otty-app-theme`, `otty-ui-kit` and `otty-containers` with
`default-features = false`; for tests on `iced_test`, `otty-app-testing` and
`otty-containers` with `fixtures` (the client is in the test build only). It
is an explicit workspace member. Its public API:

- `Containers` with `new`, `is_active`, `is_right_panel_hidden`,
  `engine_search_timer() -> Option<Duration>` (5 s while the no-engine card
  is on screen in Containers mode), `watched_engines() -> Vec<&EngineInfo>`
  (every reached engine, only in Containers mode) and
  `engine_tree(theme) -> Element`;
- `impl Feature` with `Ctx<'a> = ()`: `reduce` reads no data of another
  feature, so there is no `ctx.rs`. The clock of the tag search still moves
  after each intent, read inside `reduce` as the old widget did;
- `impl MainArea` with `Screen = ContainersScreen { Searching,
  NoEngine(NoEngineCase) }` (the old `ScreenId` values of this half); the
  table of an open list is `ScrollsOnItsOwn`;
- `impl StatusBar` (new trait, below);
- the types the binary builds from engine answers: `ContainersError`
  (`new(text)`; the conversion from `EngineError` moved to the binary),
  `LoadedLists`, `LoadedList`, `ResourceKind`, `Place`, `NoEngineCase`.

```rust
pub enum ContainersIntent {
    FromPage { page, intent }, ModeChanged(Mode), Enter, Leave,
    RightPanelHidden(bool), DiscoveryFinished(Discovery), EngineSearchTick,
    BackgroundSearchFinished(Discovery), ListsLoaded { engine, lists },
    Open(Place), ToggleEngine(EngineAddress), OpenComposeProject(usize),
    OpenPortLink { row, link }, SortBy(usize), SelectRow(usize),
    TagSearch(TagSearchAction), Refresh, CheckAgain, CopyCommand(String),
    OpenLink(String), LinkOpened(Result<(), String>),
    ListChanged { engine, resource }, EngineReconnected(EngineAddress),
    EngineUnreachable(EngineAddress), ListLoaded { engine, list },
}

pub enum ContainersOutcome {
    Discover, DiscoverInBackground, LoadLists(EngineInfo),
    LoadList { info, resource, containers },
    CopyToClipboard(String), OpenUrl(String),
    /// A row press selected a row: make the right panel visible.
    RevealInspector,
    /// Tell the user about an error; the text is ready to show.
    Failed(String),
}
```

`ContainersEffect`, the `ContainersEvent` wrapper and every `Task::done` of
the reducer are gone. The copied reducer and view model tests were rewritten
to check the returned outcomes and run first against the copied reducer
returning a `Task` (56 build errors: no `==` and no `is_empty` on a `Task`),
then `reduce` changed to return `Vec<ContainersOutcome>`. Two decisions
moved into `reduce`: `ModeChanged(Mode)` enters or leaves Containers mode
(so `dispatch/modes.rs` makes no choice), and a browser that could not be
started comes back as `LinkOpened(Err(..))` and becomes `Failed(text)`, as
the pinned modes save does (the old effect built the toast text itself).
`Failed` is also the outcome the children report in issue 14.

**The `StatusBar` trait** in `otty-app-slots` (`src/status_bar.rs`):

```rust
pub trait StatusBar: Feature {
    fn status_bar(&self) -> Vec<StatusItem<Self::Intent>>;
}
```

Not the spec's `Element`: the Containers bar holds items of three owners,
the engine and counts (containers), the sessions button (the tabs count; a
press switches the mode) and the OTTY version (the frame). A bar of several
features is drawn by the frame, as the spec says for such views, so the
feature gives only its own left items and `screens/containers.rs` builds
the kit `StatusBar` and adds the right side. No theme argument: the items
carry no colours. `otty-ui-kit` got `StatusItem::map` (with a doc test) so
the frame turns the feature's items into `AppEvent`. The Terminal mode bar
stays a frame view (`chrome/view/status_bar.rs`), so the trait has one
implementation for now; it is the socket the spec plans for the place.

**The left panel.** The engine tree is a method of its own,
`Containers::engine_tree(theme)`, not `LeftPanel`. `LeftPanel` draws one
stacked section of Terminal mode: the frame owns its open state and header
menu (`SectionState`), sends `Toggle`/`OpenMenu` and shares the panel height
by `content_height`. The tree fills the whole panel and has no frame part,
so implementing the trait would mean an ignored `SectionState`, a
meaningless height and two messages that are never sent. It is like
`toasts()` of notifications: one implementation, no trait.

**How the binary wires it:**

- `AppEvent::Containers(ContainersIntent)`, `App::containers`;
- `dispatch/containers.rs`: one call per outcome. `Discover`,
  `DiscoverInBackground`, `LoadLists`, `LoadList`, `CopyToClipboard` and
  `OpenUrl` go to `effects/containers.rs`; `RevealInspector` →
  `sidebar::deliver(RevealRight)` and `Failed(text)` → notifications `Push`,
  both in the same update;
- `effects/containers.rs`: `discover_engines`, `discover_in_background`,
  `load_lists`, `load_list` (the engine calls copied from the old effects,
  the `EngineError` → `ContainersError` conversion with its test),
  `copy_to_clipboard`, `open_url` (the old `services::open_url`, run in
  `Task::perform`, its answer `LinkOpened`);
- `subscription.rs`: `subscription/engine_watch.rs` runs one events watch
  per `watched_engines()` entry (keyed by `EngineInfo`, copied from the old
  widget subscription with its mapping tests); `engine_search_timer()` runs
  as `iced::time::every` → `EngineSearchTick`;
- `dispatch/modes.rs`: `ModesOutcome::ModeChanged(mode)` tells the terminal
  and `ContainersIntent::ModeChanged(mode)`;
- `update.rs`: `finish_update` after every input and internal update sends
  `RightPanelHidden` when the sidebar's right panel and the feature differ
  (the old `tell_containers_about_right_panel`);
- `screens/containers.rs`: the engine tree, the main area
  (`MainContent::into_element`) and the status bar; the right panel and
  the overlays stay empty for issue 14;
- `sidebar/event.rs`: the `dead_code` mark on `RevealRight` is gone.

**Checklist of the items left for 13:**

- places of Containers mode: left panel, main area and status bar filled;
  right panel and overlays left for 14 (the inspector, row menu, dialogs);
- `ModeChanged` → enter or leave: done, in the feature's `reduce`;
- the right panel state after every update: done (`finish_update`);
- `SidebarIntent::RevealRight`: wired, mark removed;
- Docker events in `subscription.rs`: done; the logs stream is the
  inspector's, left for 14;
- the Containers overlays in `top_overlay`, `close_all_menus` and the grip
  check: none exists in this half, left for 14 (the notes in `guards.rs`
  and `update.rs` name issue 14);
- the old overlay tests about Containers: the press in the Containers views
  that closes menus and cancels an inline edit is ported (without the
  inspector tab and Prune presses); the prompt over a Containers dialog and
  row menu, the press on a dialog's dimmed area and the dialog checks wait
  for 14.

`grep -rn "issue 13" crates/app` prints nothing.

**Left for 14** (the old code to copy back is in `app/src/widgets/containers`):

- the children `actions`, `create`, `inspector`, `run` as inner modules, and
  in the parent: the state fields `inspector_tab`, `inspector`, `actions`,
  `create`, `run`, `is_selinux_enabled`, `home_folder`, `lists_version` with
  their getters and ctx builders; `InspectorTab` with
  `SelectInspectorTab` (and `open()` going back to Overview); the types
  `EngineAction`, `ActionRequest`, `PullHandle`, `OpenList`, `ListRow`;
- in the reducer: the inspector following the selection after every
  intent, `follow_link`, `open_create`, the `on_*_outcome` arms, the intents
  `OpenRowMenu`, `RowMenuPressed`, `OpenCreate`, `Create`, `Run`, `Actions`,
  `Inspector`; the Run form closing on `Open`, `RecheckFolders` on `Enter`,
  the logs on `EngineReconnected`, the row menu closing when a search loses
  the open place (notes in `reducer.rs`);
- in the views: the create and Prune buttons of the toolbar, the row "⋯"
  column and the right press (`rows_without_actions`, `prune`, `can_create`
  of `TableViewModel`), the Run form status bar text, `model/inspector.rs`
  and `view/inspector.rs`, the Run form as `ContainersScreen::RunForm`;
- the items kept with a `dead_code` expectation naming issue 14 (they fail
  the build once used): `overview.rs` and `links.rs` (module level),
  `pull_amount`, the four `*_status` of `resource_rows.rs`,
  `ContainersState::show_item`, `ResourceList::selected`,
  `ResourceKind::create_key`, `split_reference`;
- the binary: the right panel and overlays in `screens/containers.rs`, the
  overlays in `guards.rs` (`top_overlay`, Esc, dialog check,
  `context_menu_guard` for the row menu), `close_all_menus`, the grip check,
  the effects of the children (inspect, log run, actions, Run form, pulls,
  the shell tab), the followed logs subscription, the Run form folder
  timer, and `WatchEvent::ContainerStarted` → the inspector (now dropped in
  `engine_watch.rs`);
- the old tests that need a child: the inspector link tests of the reducer
  (item, image, container, volume, network, port and Compose links, the
  tags and text they clear, the closed engine they open), the row menu tests
  (button, right press, among tagged rows, lost with its engine), the
  create/run/actions outcome tests, the Run form and confirm dialog kept
  through searches, the Run form status bar tests, the create, Prune and row
  menu view tests, `every_list_has_a_create_button`, the logs and folder
  timer subscription tests, the logs tests of the old `update.rs`, the
  effects tests of the children, and the action/request/pull handle/open
  list tests of `types.rs`.

**Tests.** `otty-app-containers` has 429 tests and 3 doc tests (the
copied state, reducer, model, view, sort, table search, resource rows,
format, links, overview and no-engine tests; new: the getters, the
places, the mode change, the link answer, `show_item` directly);
`otty-app` 636 (the dispatch of the outcomes, the row press that opens the
right panel, the link toast, the mode switch, the right panel tracking, the
watch and timer subscriptions, the screen and status bar, the effects);
`otty-app-slots` 11 doc tests (10 before); the kit gained one doc test. The
whole workspace runs 5557 tests, all passing.

**Checks.** `cargo tree -p otty-app-containers -e normal | grep -E
"bollard|notify"` prints nothing. `tokio` comes only through `otty-ui-term`
→ `otty-app-theme` (and → `otty-app-slots`), no direct `tokio`. A planted
`std::fs::read` in the reducer failed clippy with `use of a disallowed
method std::fs::read` and the note `a feature touches no files; the binary
does it in effects/`; it was removed. `cargo +nightly fmt --all -- --check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo deny check` (only the 3 known yanked-crate warnings), `cargo test
--workspace --all-features` and `cargo build -p otty-app` pass, on top of
a8d2aa2. `cargo llvm-cov` was not run, as asked. The window was not opened
(no display for the agent).

**For the owner to decide.**

- The status bar trait gives items, not an element (see above). Its right
  side (sessions button and version) is drawn by the frame.
- Until issue 14 a row press opens the right panel, which is empty in
  Containers mode; the toolbar has only the search and Refresh, and a row
  has no "⋯" button or right-press menu.
- `ContainersError::new` is public now: the binary builds it from the
  engine's error. The feature cannot see `EngineError` without the client.
- The engine's `ContainerStarted` event is dropped by the binary until the
  inspector comes.
