# 03 — Kit: KeyValueList, InspectorHeader, InspectorTabs, LogView

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: —
Spec: [../spec.md](../spec.md)

## What

New kit components (spec Decision 15). Visual reference: `ot-inspector`,
`ot-insp-*` in the design system bundle, `.logv` in `project/docker-mode.css`
and the inspector on the `DockerMode` board.

| Component | Shape |
|---|---|
| `KeyValueList` | Two columns (96 / rest), keys muted, values mono, long values wrap, 8px gap. |
| `InspectorHeader` | Mono title (14), a `StatusBadge` line, a wrapping row of small buttons, an optional hint line under the buttons. |
| `InspectorTabs` | Underlined text tabs: selected tab `text-strong` with a 2px `accent` line. |
| `LogView` | Mono 11.5 / 18, `surface-chrome`, `radius-sm`, selectable text, wraps long lines, scrolls; while following it stays at the bottom when lines are added. |

- Messages: tab selected. Buttons are passed in as elements (kit `Button`).
- Gallery sections in both themes; `LogView` shows a follow toggle in the gallery only.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test`: a tab press emits its message; header buttons emit theirs.
- `LogView` keeps the bottom in view after lines are appended while following, and does not jump when not following.

## Answer

Four components, one file each in `crates/ui/kit/src/components/`.

- `key_value_list.rs`: `KeyValueList::new(&palette).entry(key, value)`; 96px muted key column, mono values that break at any character, 8px gap, 12.5px text.
- `inspector_header.rs`: `InspectorHeader::new(title, &palette).status(element).action(element).hint(&str)`; mono 14 title, a wrapping row of buttons, an optional muted hint. The status line is an `Element` the caller builds, because `StatusBadge` (ticket 02) did not exist yet. Ticket 09 passes `.status(StatusBadge::new(...))` (anything that converts into an `Element`) and small kit `Button`s (`ButtonSize::Small`) as `.action(...)`.
- `inspector_tabs.rs`: `inspector_tabs(&tabs, &selected, on_select, &palette)`, same shape as `segmented`; 30px tabs, selected one `text-strong` with a 2px accent line, 1px `line` rule under the row.
- `log_view.rs`: `LogViewState` (`new`, `push_lines`, `set_following`, `is_following`, `pending_count`, `text`, `line_count`, `perform(Action)`) and `LogView::new(&state, &palette).height(..).on_action(Message::Log)`. Built on `iced`'s `text_editor` because it is the only selectable text in iced 0.14; edit actions are dropped so the log is read-only.
- New tokens in `size.rs`: `KEY_VALUE_KEY` (96), `INSPECTOR_TAB` (30).

Following: while following, new lines are added and the cursor moves to the end, which brings the bottom into view. While not following, new lines are held back (`pending_count`) and the view does not move; switching following on adds them. Moving the cursor while the user has scrolled up would pull the view back to the cursor, so holding lines back is how "does not jump" is guaranteed.

Gallery: new `inspector` argument and sections in the full gallery, both themes. `LogView` has a follow toggle and an "Add line" button in the gallery only.

Tests (`iced_test`, written with the code in the same files): tab press sends its message (also for the selected tab); header buttons send only their own message and wrap when the row is full; key column width and long-value wrapping; following keeps the cursor on the last line; not following leaves text and cursor unchanged; edits are ignored.

Coverage (lines): 79.57% before, 79.71% after.

Not verified: the visual look (the gallery was compiled, not viewed), and the real scroll position of `LogView` (the cursor is checked, not pixels). `text_editor` draws no scrollbar, so the log scrolls with the wheel only. Component text comes from the caller, so no i18n keys were added in the kit.
