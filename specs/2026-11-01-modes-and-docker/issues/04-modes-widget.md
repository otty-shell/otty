# 04 — `modes` widget: Mode, active mode, pin rules, menu state, `modes.json`

Status: ready-for-agent
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
