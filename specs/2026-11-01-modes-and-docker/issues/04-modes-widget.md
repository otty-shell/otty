# 04 — `modes` widget: Mode, active mode, pin rules, menu state, `modes.json`

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: —
Spec: [../spec.md](../spec.md)

## What

New widget `app/src/widgets/modes/` in the usual shape (state, reducer,
model, event, types, storage) — spec Decisions 1–5. Tests first.

- `Mode` enum: `Terminal`, `Docker`. No trait, no registry (Decision 1).
- State: active mode (always `Terminal` at start), pinned modes in pin order (never `Terminal`, at most 4), whether the All modes menu is open.
- Rules: pinning a fifth mode is refused; unpinning the active mode keeps it active as a temporary tab; choosing a mode closes the menu.
- View model: the switcher tabs (Terminal, pinned, then the temporary one), the menu sections (Pinned with "N of 4", Other modes) with pin states including "disabled: unpin another mode first".
- Intents: select mode, pin, unpin, toggle menu, close menu. Effects: mode changed (the root uses it in ticket 06), save pinned.
- Storage: `~/.config/otty/modes.json`, `{"pinned": ["docker"]}`, written atomically like `widgets/settings/storage.rs`. Missing or broken file, or unknown names: defaults (`[Docker]`) for what cannot be read, with a log warning.
- Wire it into `Widgets`, `AppEvent::Modes` and `app/src/events/modes.rs`. The view does not use it yet.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer tests: each rule above, menu open/close, select closes the menu, save effect after pin and unpin only.
- Storage tests: missing file, broken JSON, unknown entry, more than 4 entries, round trip.
- View model tests: temporary tab appears for an unpinned active mode and goes away after switching.

## Answer

Commit `c20eef1` on branch `worktree-agent-acfe470988a8778e2`.

### State shape

`app/src/widgets/modes/` (`types`, `state`, `event`, `reducer`, `model`,
`storage`, `errors`, `mod`).

- `Mode` (`types.rs`): `Terminal`, `Docker`. `Mode::ALL` (menu order),
  `name_key()` (i18n `Key`), `storage_name()` / `from_storage_name()`
  (`"terminal"`, `"docker"`). `MAX_PINNED = 4`, `DEFAULT_PINNED = [Docker]`.
  No trait, no registry. Icons are not on `Mode` yet; ticket 06 adds them.
- `ModesState` (`state.rs`): `active: Mode` (always `Terminal` at start),
  `pinned: Vec<Mode>` (pin order), `pin_limit` (`MAX_PINNED`),
  `is_menu_open`. The pin rules live only here: `pin` refuses Terminal, an
  already pinned mode, and a full list; `unpin` never changes the active
  mode; `select` always closes the menu. `with_pinned(&[Mode])` builds the
  state by pinning each entry through the same rules, so storage cannot
  produce a state that breaks them.

### Intents and effects

| Intent | State change | Effect |
|---|---|---|
| `Select(mode)` | active = mode, menu closed | `ModeChanged(mode)` only when the active mode changed |
| `Pin(mode)` | appended to pinned (if the rules allow) | `SavePinned(pinned)` only when it changed |
| `Unpin(mode)` | removed from pinned; active stays | `SavePinned(pinned)` only when it changed |
| `ToggleMenu` | menu open ↔ closed | — |
| `CloseMenu` | menu closed | — |

Pin and Unpin keep the menu open. `app/src/events/modes.rs` routes
`AppEvent::Modes`: intents go to `app.widgets.modes.reduce`;
`SavePinned` runs `storage::save_pinned` in `Task::perform` and logs a
failure (the reducer does no I/O); `ModeChanged` does nothing yet.
`App::new` creates the widget with `ModesWidget::load()`.

### View model (`ModesWidget::vm()` → `ModesViewModel`)

- `active: Mode`: the name shown next to the tabs.
- `tabs: Vec<ModeTabViewModel { mode, is_active, is_temporary }>`:
  Terminal, the pinned modes in pin order, then one temporary tab
  (`is_temporary: true`) when the active mode is not pinned. It is derived
  from the state, so it goes away as soon as another mode is selected.
- `is_menu_open`.
- `pinned_rows` (Terminal, then the pinned modes) and `other_rows` (the
  rest, in `Mode::ALL` order): `ModeMenuRowViewModel { mode, is_active,
  pin: PinButton }`, with `PinButton::Hidden` (Terminal), `Unpin`, `Pin`,
  `PinDisabled` (every slot taken: "Unpin another mode first").
- `pinned_count` and `pin_limit`: the "N of 4" note. The vm holds numbers,
  not text, so tests do not depend on the active locale.

### Storage

`~/.config/otty/modes.json` (or `$TMP/otty/modes.json` without `$HOME`):
`{"pinned": ["docker"]}`, pretty-printed, written to `modes.json.tmp` and
renamed (same as `widgets/settings/storage.rs`). Loading:

- missing file, unreadable file, broken JSON, no `pinned` array, or an
  entry that is not a string → defaults (`[Docker]`) and a log warning;
- unknown names (e.g. `"kubernetes"` from a newer version) are skipped with
  a warning and the rest is kept; a list of only unknown names leaves
  nothing pinned;
- `"terminal"`, repeats and entries past 4 are left out by the pin rules,
  with one warning;
- `{"pinned": []}` stays empty (the user unpinned everything), it does not
  fall back to the defaults.

### i18n

New keys in all ten catalogs: `mode_terminal`, `mode_docker`,
`modes_all_modes` ("⋯" tooltip), `modes_pinned_section`,
`tpl_modes_pinned_note` (`{count} of {limit}`), `modes_other_section`,
`modes_pin`, `modes_unpin`, `modes_unpin_another_first`. A test checks every
locale fills both placeholders. No formatting function for the note yet:
ticket 06 adds `i18n::modes_pinned_note(count, limit)` with `fill_in`.

### How tickets 05 and 06 use it

- **05 (shortcuts, palette):** send
  `AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(mode)))`. "Pinned
  mode N" = `vm().tabs` / `pinned_rows` position N (index 0 is Terminal);
  Ctrl Shift M → `ToggleMenu`. Palette entries "Switch to … mode" → `Select`.
  Shortcut labels for the menu rows come from the row position in
  `pinned_rows`.
- **06 (root view):** read `app.widgets.modes.vm()` and pass `tabs`,
  `pinned_rows`, `other_rows`, `pinned_count` / `pin_limit` and
  `is_menu_open` to the kit `ModeSwitcher` / `ModeMenu`; map tab and row
  clicks to `Select`, pin toggles to `Pin` / `Unpin`, the "⋯" button to
  `ToggleMenu`, Esc and click outside to `CloseMenu` (likely through
  `update.rs` guards like the other menus). Pick the slot views from
  `vm().active`. Handle `ModeChanged` in `events/modes.rs` if the switch
  needs more (e.g. focusing the terminal again).
- **Remove the dead-code expectation** on `pub(crate) mod modes;` in
  `app/src/widgets/mod.rs` once the view and shortcuts use the widget. It is
  `#[cfg_attr(not(test), expect(dead_code, ...))]`, so the build warns
  ("unfulfilled lint expectation") as soon as nothing in the module is dead.

### Deviations and notes

- **Pin limit in tests.** There is only one mode to pin, so "a fifth mode is
  refused" cannot be reached with real modes. `ModesState` keeps the limit
  as a field (`pin_limit`, always 4 in the app) and a `#[cfg(test)]`
  constructor `with_pin_limit(pinned, limit)` lets tests use limit 0 to
  check the full-slots rules (refused pin, `PinDisabled`). Test-only `Mode`
  variants were rejected: every `match` on `Mode` in later tickets would
  need extra arms. The storage test "more than 4 entries" can only check
  that repeats and unknown names never go past the limit.
- **Missing file logs a warning** too, as Decision 4 says, so the first
  start logs one line.
- **Save race:** two saves started at nearly the same moment write the same
  `modes.json.tmp`; the second rename wins. Pin clicks are far slower than a
  small file write, so this was left as is.

### Tests (written first, seen failing, then implemented)

| Where | Tests | What they pin |
|---|---|---|
| `widgets/modes/reducer.rs` | 18 | first start; select changes and reports, same mode reports nothing, select closes the menu; toggle and close menu without effects; pin appends and saves, pinned again / Terminal / full refused; limit 4; unpin removes and saves, active mode stays active, Terminal / not pinned refused; pin and unpin keep the menu open; only pin and unpin save; `with_pinned` drops Terminal, repeats and entries past the limit |
| `widgets/modes/model.rs` | 7 | first start vm; active pinned mode is not temporary; unpinned mode under Other modes with Pin; temporary tab for an unpinned active mode; temporary tab goes away after switching; full slots → `PinDisabled`; open menu |
| `widgets/modes/storage.rs` | 10 | missing file; broken JSON; wrong shapes; empty list stays empty; unknown names skipped; only unknown names; Terminal and repeats skipped; more than 4 entries; round trip; file format and no leftover `.tmp` |
| `widgets/modes/types.rs` | 2 | storage names round trip; name keys |
| `events/modes.rs` | 3 | intent reduced and effect routed back; unpin asks to save; `ModeChanged` does nothing yet |
| `i18n/mod.rs` | 1 | every locale fills the pinned note placeholders |

## Comments

- `cargo +nightly fmt`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo test --workspace --all-features`
  (1144 passed, 0 failed) and `cargo deny check` (advisories, bans,
  licenses, sources ok) pass. No new dependencies.
- Coverage (`cargo llvm-cov --workspace --all-features`, lines): base
  `46ec6c4` 79.57% → this ticket 79.85%.
