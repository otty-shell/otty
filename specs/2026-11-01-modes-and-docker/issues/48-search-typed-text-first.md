# 48 — The text you typed is the first suggestion

Status: resolved
Type: bug
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 47
Spec: [../spec.md](../spec.md)

## What

Owner report (2026-11-03). The search does not work as a filter: typing offers only
existing values, so a partial value cannot be applied to get everything containing
it. The first suggestion must be the text as typed. Rules (spec Decision 21 is
updated):

- `sta`: the first suggestion is `sta` itself, a name filter. Enter applies it: the
  text stays in the field, the table is filtered by name, the list closes. Below it
  come the matching keys, then the matching names.
- `status:ru`: the first suggestion is `status:ru` itself. Enter adds the tag
  `status:ru`, which keeps every row whose status contains `ru`, and clears the
  field. Below it come the matching values.
- `status:` with nothing after the colon, and an empty field: no typed-text row. The
  first real suggestion is highlighted, as today.
- When the typed text equals a suggestion, ignoring case, there is one row for it,
  not two.
- **Enter** applies the highlighted row. **Tab** completes to the first real
  suggestion below the typed-text row, and does what Enter does if there is none:
  `sta` Tab gives `status:`, `run` Tab gives the tag `status:Running`.
- ↓ moves from the typed-text row to the real ones.
- The typed-text row looks plainly different from a real suggestion, and works in
  `TagSearchField` without any widget code.

Update `regression-checklist.md` for the new Enter and Tab rules.

Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov`
with the merge base before the commit.

## Done when

- Model tests for each rule above, including no duplicate row, `redis:7` as a name
  filter, and an unknown key `foo:bar` as a name filter.
- `iced_test` on the real Docker flow: `status:ru` Enter adds the tag and shows only
  running containers; `otty` Enter filters by name with the text kept; `sta` Tab,
  `run` Tab adds `status:Running`.

## Answer

Fixed in the kit model (`tag_search.rs`), so every widget gets it.

- New `SuggestionKind::Typed`: the text as typed, always first. Plain text shows as typed; `key:text` shows with the schema spelling of the key. None for an empty field, a bare `key:`, or when the text equals another entry ignoring case (that entry stands for it).
- Enter (`Apply`) applies the highlighted row. On the typed row, plain text stays in the field as a name filter and the list closes; `key:text` becomes a substring tag and the field clears.
- Tab is a new action, `TagSearchAction::Complete`: applies the highlighted row if it is real, else the first real row below the typed row, else does what Enter does. `TagSearchField` sends it for Tab.
- Down moves from the typed row to the real rows (the typed row is index 0, so the highlight starts on it).
- `TagSearchField` draws the typed row in the muted text color with no match marks.

Tests changed for the new rule (the typed row shifts positions, and Enter on `sta` or `web` no longer completes): in `tag_search.rs`, `applying_a_key_puts_it_in_the_field_and_shows_its_values`, `applying_a_value_adds_the_tag_and_clears_the_field`, `applying_a_name_fills_the_field_closes_the_list_and_filters` and `a_value_with_a_colon_is_a_whole_value` now press Down first; `typing_suggests_matching_keys_in_column_order_then_matching_names`, `names_starting_with_the_text_come_first_then_the_given_order`, `names_come_from_the_rows_the_tags_leave`, `after_a_key_the_values_of_that_key_are_suggested_and_filtered`, `the_matched_part_ignores_case_and_is_a_byte_range_of_the_shown_text`, `a_repeated_name_or_value_is_suggested_once` and `up_and_down_move_the_highlight_and_wrap_at_both_ends` expect the typed row; `with_no_suggestion_enter_does_nothing_otherwise` keeps only `compose:`. In `tag_search_field.rs`: the Enter key test for a key uses Down then Enter, the name test uses Tab, `each_key_of_an_open_list_sends_its_action` and the closed-list test expect `Complete` for Tab. In the app: `typing_in_the_search_goes_to_the_search_of_the_open_table` (Tab), `values_of_a_key_come_from_each_table`, `the_schema_lists_the_rows_in_the_order_of_the_table`, `a_row_press_after_filtering_hits_the_row_that_is_shown` (one more Down), and the Docker flow test now uses Tab. New tests cover each rule, `redis:7` and `foo:bar` as name filters, no duplicate row, `status:ru` Enter (two running containers) and `otty` Enter (text kept). `regression-checklist.md` steps 25, 26 and 33 describe the new Enter and Tab rules.
