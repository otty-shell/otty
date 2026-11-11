# 14 Containers: inspector, forms and actions

Status: needs-triage
Blocked by: 13

## What to do

Move the rest of `app/src/widgets/containers` into `otty-app-containers`: the
child modules `inspector`, `run`, `create` and `actions`. They become plain
inner modules of the crate.

This issue covers:

- the inspector in the right panel, with its tabs, inspect tree, links and log
  follow toggle; the parent keeps the open inspector tab;
- the Run container form in the main area;
- the create dialogs for volumes and networks; creating one still locks Prune;
- the row menu and the confirm dialogs;
- pulling images, with the pull handle kept in the run and create state;
- their Docker calls in `effects/containers.rs`.

## Done when

- Everything in Containers mode works as in the old app.
- `otty-app-containers` still has no `bollard` and no `tokio`.
- The steps and checks in "Moving a feature" are done.

## Comments

### 2026-11-10: a test left for this issue by issue 02

Bring the old i18n test
`given_a_confirm_dialog_switch_then_its_label_fits_the_dialog` from
`app/src/i18n/mod.rs`. It measures a kit `Switch`, so it needs iced and could
not live in `otty-app-i18n`. It belongs with the confirm dialogs here.

### 2026-11-10: the containers feature crate, second half

**`otty-app-containers`** now holds the whole old widget: the children
`actions`, `create`, `inspector` and `run` were copied (not moved) from
`app/src/widgets/containers/child/` as plain inner modules of the crate,
with `model/inspector.rs` and `view/inspector.rs` of the parent; the parent
pieces issue 13 left out (`state.rs` fields and ctx builders, `types.rs`,
`model/main_area.rs`, `model/mod.rs`, `view/main_area.rs`, the Run branch of
`model/status_bar.rs`) came back from the old files. The old app is unchanged
(`git diff --stat 3c2a65c -- app` is empty). Each child keeps the shape it
had: `event.rs` with its intent and its outcome, `ctx.rs` with what it reads
from the rest of the feature (`ActionsCtx`, `RunCtx`/`RunEngine`,
`InspectorCtx`; the feature's own `Ctx` stays `()`), `reducer.rs`, `state.rs`,
`model.rs`, `types.rs` and `view/`. The crate gained `serde_json` (already a
workspace dependency; the inspect tree reads the answer with it).

**Outcomes.** Each child's `reduce` returns `Vec<XOutcome>`; `XEffect`, the
`(Task, Option<Outcome>)` pair and every `Task::done` are gone:

- `ActionsOutcome`: `RunAction`, `OpenShell`, `TakeKeyboard` (the row menu
  takes the keyboard; it was `focus::nothing()`), `Done`, `Failed`,
  `RunRequested`, `PullRequested`;
- `CreateOutcome`: `StartPull`, `CancelPull`, `ActionRequested`, `Pulled`,
  `Failed`;
- `RunOutcome`: `StartPull`, `CancelPull`, `LoadImageDefaults`,
  `RunContainer`, `CheckFolder`, `WaitForTypingPause`, `WaitForLateAnswer`,
  `Started`, `Failed`;
- `InspectorOutcome`: `LoadInspect`, `CheckLogRun`, `CopyToClipboard`,
  `FollowLink`.

The parent reducer hands each child its intent and turns each outcome into
`ContainersOutcome`s in the same call (`on_*_outcome`): the decisions stay in
the reducers (a done action reloads its engine, a started Run shows its
container selected, Run and Pull again on an image open the form and the
dialog, a sent volume or network starts as an action and locks Prune, a link
is followed, a lost place closes the row menu, `Enter` asks the open form
about its folders, `Open` closes the Run form and its pull, a reconnect
checks the followed logs). The inspector follows the selection after every
intent, as before.

`ContainersIntent` gained `WindowResized(Size)`, `SelectInspectorTab`,
`OpenRowMenu`, `RowMenuPressed`, `OpenCreate`, `Create(CreateIntent)`,
`Run(RunIntent)`, `Actions(ActionsIntent)` and `Inspector(InspectorIntent)`.
`ContainersOutcome` gained `TakeKeyboard`, `RunAction`, `OpenShell`,
`PullImage` (the pull dialog's pull), `PullRunImage` (the Run form's pull),
`CancelPull`, `LoadImageDefaults`, `RunContainer`, `CheckFolder`,
`WaitForTypingPause`, `WaitForLateAnswer`, `LoadInspect` and `CheckLogRun`.
The child intents and the types the binary builds or reads are public and
re-exported: `ActionsIntent`, `RowAction`, `CreateIntent`, `FormField`,
`RunIntent`, `RunEdit`, `InspectorIntent`, `LogTarget`, `LogRead`,
`InspectTarget`, `LinkTarget`, `InspectorTab`, `ActionRequest`,
`EngineAction`, `PullHandle` (with `new`, `abort`, `is_aborted`), `LOG_TAIL`
and `MAX_LOG_LINES`.

**Places and getters.** `impl RightPanel` (the image panel while the Run form
is open, else the inspector with its header, buttons, tabs and tab body);
`impl Overlays` (the create dialog, then the confirmation, then the row menu:
dialogs are `Overlay::dialog`, the row menu `Overlay::menu`; they lie over the
whole window, as the table reports window coordinates);
`ContainersScreen::RunForm` in `MainArea`; new getters
`followed_logs() -> Option<&LogTarget>`, `folder_recheck_timer()`,
`has_create_dialog()`, `has_confirm()`, `has_row_menu()`. The toolbar has its
create and Prune buttons again, rows their "⋯" column and right press.

**How the binary wires it:**

- `dispatch/containers.rs`: one call per new outcome. `TakeKeyboard` →
  `take_keyboard()`; `OpenShell` → `tabs::deliver(OpenCommandTab)` with a
  local session of the engine's tool (`shell_tab_intent`); the others go to
  `effects/containers.rs`;
- `effects/containers.rs`: `run_action`, `pull_image`, `pull_run_image`
  (abortable `Task::run` plus the `PullStarted` handle), `cancel_pull`,
  `load_image_defaults`, `run_container`, `check_folder`,
  `wait_for_typing_pause`, `wait_for_late_answer`, `load_inspect`,
  `check_log_run`, copied from the old effects with their tests;
- `subscription.rs`: `subscription/followed_logs.rs` (the old inspector
  stream, keyed by `LogTarget`, with its tests) and the folder timer of
  `folder_recheck_timer()`; `engine_watch.rs` sends `ContainerStarted` to the
  inspector again;
- `screens/containers.rs`: the right panel and the overlays from the feature;
  `has_overlay` for the grip check; `screens::empty_place` is gone;
- `guards.rs`: `Overlay::{ContainersCreate, ContainersConfirm,
  ContainersRowMenu}` with their kind and Esc (Cancel, Cancel, close),
  `top_overlay` in Containers mode (create over confirm over row menu), the
  dialog answers in `is_dialog_answer` (the dialogs send nothing from their
  dimmed area), the row menu's items and dismiss in `context_menu_guard`;
- `update.rs`: `close_all_menus` closes the row menu;
- `view.rs`: the right panel and overlays of Containers mode; the grips are
  off while a Containers overlay is open in Containers mode;
- `dispatch/mod.rs`: a window resize also sends `WindowResized`;
  `App::with_saved` sends the first size.

The SSH prompt and the palette lie over the Containers overlays by the layer
order (they are frame layers above the mode overlays) and by `top_overlay`.

**Checklist (from issues 09, 11, 12 and 13):**

- children and parent state fields, getters, ctx builders, `InspectorTab`
  with `SelectInspectorTab` (and `open()` back to Overview), `EngineAction`,
  `ActionRequest`, `PullHandle`, `OpenList`, `ListRow`: done;
- reducer pieces (selection follow, `follow_link`, `open_create`,
  `on_*_outcome`, the intents, Run form closing on `Open`, `RecheckFolders` on
  `Enter`, logs on `EngineReconnected`, row menu closed with a lost place):
  done, the notes in `reducer.rs` are gone;
- views (create and Prune buttons, "⋯" column and right press, Run form text
  in the status bar, inspector, `ContainersScreen::RunForm`): done;
- the `dead_code` marks naming issue 14: all removed; `grep -rn "issue 14"
  crates/app` prints nothing;
- binary: right panel, overlays, guards, `close_all_menus`, grip check,
  children's effects, logs subscription, folder timer,
  `WatchEvent::ContainerStarted`: done;
- 09: Containers dialogs in `is_dialog_answer`, and the old test that a press
  on their dimmed area keeps them open: done (`view.rs`, `screens`);
- 11/12: the SSH prompt over a Containers dialog and row menu, the Containers
  overlays in `top_overlay`: done (`guards.rs`, `view.rs`);
- 02: `given_a_confirm_dialog_switch_then_its_label_fits_the_dialog`: in
  `actions/view/confirm_dialog.rs`.

**Tests.** The copied child reducer tests were rewritten to read the
returned outcomes (scripts turned `(effects, outcome)` pairs and
`ContainersEffect::X(XEffect::Y)` into outcome lists), and the parent
reducer tests that need a child came back from the old file (links, row
menu, create/run/actions outcomes, the Run form and confirmation kept
through searches) along with the two parent tests issue 13 had trimmed. The
first build against the copied reducers failed with 93 errors (the parent
reducer had no arms for the new intents and no `on_*_outcome`); then the
reducers changed. Three Run form tests that load the kit's fonts or switch
the language run alone, which a feature crate may not do (`Command::new` is
banned), so they moved to the binary's `effects/locale.rs` and draw the form
through the feature's main area; the read-only label test now checks that
the label is drawn whole in its column instead of measuring the switch
against the private column width. The old `update.rs` logs tests check that
the stream starts and stops (`followed_logs()`); the line counts they also
read belong to a test-only getter of the old widget, and the feature's own
test `the_logs_stop_while_the_right_panel_is_hidden_and_start_fresh_when_shown`
covers them. Ported to the binary: the effects tests, the logs stream tests,
the overlay tests of `guards.rs`, `update.rs` and `view.rs` (Esc, dialog
answers, dimmed area, the prompt and the frame overlays over Containers
overlays, terminal input kept out, grips), the screen tests (Run form,
inspector, row menu and dialogs, the "⋯" button position, the Run form's
scrolling), the failed-action toast and the shell tab.

Counts: `otty-app-containers` 1094 tests and 6 doc tests (429 and 3
before), `otty-app` 701 (636 before); the whole workspace runs 6290 tests,
all passing.

**Checks.** `cargo tree -p otty-app-containers -e normal | grep -E
"bollard|notify"` prints nothing; `tokio` comes only through `otty-ui-term` →
`otty-app-theme` (and → `otty-app-slots`), no direct `tokio`. A planted
`std::fs::read` in `run/reducer.rs` failed clippy with `use of a disallowed
method std::fs::read` (note: `a feature touches no files; the binary does it
in effects/`); it was removed. `cargo +nightly fmt --all -- --check`, `cargo
clippy --workspace --all-targets --all-features -- -D warnings`, `cargo deny
check` (only the 3 known yanked-crate warnings), `cargo test --workspace
--all-features` and `cargo build -p otty-app` pass. `cargo llvm-cov` was not
run, as asked. The window was not opened (no display for the agent).

**For the owner to decide.**

- The window size reaches the feature as `ContainersIntent::WindowResized`,
  so the row menu view draws from the feature's own state (the old app passed
  the window size to the view).
- Two outcomes for a pull (`PullImage`, `PullRunImage`) instead of one with a
  field naming its form; the binary turns each into the intents of its form.
- `LogTarget::reading` and `PullHandle::is_aborted` are public (the binary's
  tests build log targets and check a cancelled pull).
- The old screen tests of issue 13's half that need no child (the no-engine
  card's scrolling and drags) are still not in the new app.
