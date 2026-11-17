# 41 The tag search follows the keys of the folder suggestions

Status: ready-for-agent
Blocked by: 40

## Why

Issue 39 made one kit piece for suggestions under an input
(`crates/ui/kit/src/components/suggestion_input.rs`) and left the tag
search field with its own key rules: its first row is always
highlighted and Up/Down wrap at the ends. The folder fields start with no
row and stop at the ends. The owner wants one set of keys everywhere
(spec Decision 18, 2026-11-18).

## What to do

1. The tag search field (`tag_search_field.rs`, `tag_search.rs`; used by
   the containers table search and the agents project search) follows the
   folder fields' rules:
   - no row highlighted until Down; Down moves down and stops at the last
     row; Up moves up, and from the first row leaves the list;
   - Enter and Tab pick the highlighted row; with none highlighted, Enter
     does what the field does without a list (today: apply the typed
     text as a search, or the field's own Enter) and Tab moves the focus;
   - typing clears the highlight; Esc closes the list first.
2. Move the key rules into the shared piece so both fields take them from
   one place; the fields keep only their rows and what a pick does. No
   rule stays duplicated in the tag search.
3. Update the tag search tests that expected the old rules (first row
   lit, wrap, Tab with no highlight completing the key) to the new rules,
   and say which ones changed and why in the comment. The command palette
   keeps its own rules (it is not a list under a field; issue 39).

## Done when

- Tests, written first and seen failing, cover the new rules in the tag
  search field (frame by frame, with iced's state kept between frames, as
  the kit tests of issue 39 do): no row lit at first, Down lights the
  first, stop at the last, Up from the first leaves, Enter and Tab pick,
  Enter with no row applies the text.
- The binary: in the containers search through the whole window, type
  `sta`, Down, Enter: the field holds `status:`.
- Run by hand: Containers search and the Projects search in Agents mode
  behave like the folder fields.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

### 2026-11-18 — built

**What was built**

- **The key rules live in the shared piece now.** `SuggestionInput`
  (`crates/ui/kit/src/components/suggestion_input.rs`) decides every key
  of a list under a field: ↓ lights the first row, then the next one, and
  stops at the last; ↑ goes up, and from the first row leaves the list;
  Enter and Tab pick the lit row; with no row lit Enter goes to the input
  (its submit) and Tab is not taken; Esc closes an open list, then is the
  field's own Esc. A field gives only its rows, the lit row, what a pick
  sends (`on_pick`) and what a move of the light sends (`on_highlight`).
  A press on a row is the same pick. `row_below` and `row_above` moved
  here from `text_field.rs`.
- **`TextField`** (the folder fields) only hands its rows and messages to
  the piece; it no longer binds keys itself.
- **`TagSearchField`** hands its rows and two messages to the piece, plus
  its own keys: Backspace in an empty field removes the last tag, and
  Enter with no row lit is the input's submit (`Apply`). It binds no ↓,
  ↑, Enter, Tab or Esc rule of its own.
- **The `TagSearch` model** keeps the lit row as `Option<usize>`, none at
  first. `TagSearchAction::MoveUp`, `MoveDown` and `Complete` are gone;
  the new `Highlight(Option<usize>)` stores the row the kit chose.
  Typing, a pick, Esc and a press away clear it. `Apply` is now "Enter
  with no row lit": plain text stays as a name filter and the list
  closes; `key:text` becomes a tag and the field clears; a bare `key:`
  does nothing.
- **The row "the text as typed" is gone** (`SuggestionKind::Typed`). It
  was the first row and always lit, so Enter applied the text. Now Enter
  with no row lit does that, and the row would only stand in the way: ↓
  would light it first, and `sta`, ↓, Enter would keep `sta` instead of
  giving `Status:`, as this issue asks. With it went its muted look.
- The containers search field id is public (`TABLE_SEARCH_ID`), so the
  whole-window test can press it, as the agents dialogs do.

**Tests changed, and why**

- Kit `tag_search.rs` (model):
  - The first row is no longer lit: `the_first_suggestion_is_highlighted`
    became `at_first_no_suggestion_is_highlighted`.
  - ↓/↑ wrap is not the model's any more: `up_and_down_move_the_highlight_and_wrap_at_both_ends`,
    `down_moves_from_the_typed_row_to_the_real_ones`,
    `moving_in_a_closed_list_opens_it_without_moving_the_highlight`,
    `moving_with_no_suggestions_changes_nothing` and
    `typing_brings_the_highlight_back_to_the_first_suggestion` were
    replaced by tests of `Highlight`: it lights that row only, `None`
    leaves the list, typing and closing clear it, it does nothing with a
    closed list, and a list that shrank past it lights nothing (before,
    the last row stayed lit).
  - Tab no longer completes: `tab_completes_to_the_first_real_suggestion_below_the_typed_row`,
    `tab_applies_a_real_row_the_user_moved_to` and
    `tab_does_what_enter_does_when_no_real_suggestion_is_below` were
    removed; the "applying_*" tests that did ↓ then Enter became
    "picking_*" tests with `Pick`, and `a_pick_clears_the_highlight` was
    added.
  - No typed row: the tests of its place, trim, spelling and "no second
    row" were removed or now check the real rows only
    (`plain_text_suggests_keys_then_names`,
    `a_key_and_text_suggests_the_values`,
    `text_that_matches_nothing_suggests_nothing`, the list tests without
    their first `Typed` entry, the Russian tests likewise). New:
    `enter_in_an_empty_field_takes_no_key` (before, Enter there put the
    first key in the field).
  - `closing_the_suggestions_keeps_the_text_and_the_tags` now expects the
    tag `status:running` as typed: Enter with no row lit takes the text,
    not the matching value.
- Kit `tag_search_field.rs` (frame by frame with iced's state kept): new
  `at_first_no_row_is_lit`, `down_lights_the_first_row_then_the_next_and_stops_at_the_last`,
  `up_moves_up_and_from_the_first_row_leaves_the_list`,
  `enter_picks_the_lit_row`, `tab_picks_the_lit_row`,
  `enter_with_no_row_lit_applies_the_text`,
  `tab_with_no_row_lit_is_left_to_move_the_focus`,
  `typing_clears_the_lit_row`, `esc_closes_the_list_first`. Changed:
  the key-by-key test now expects `Highlight(Some(0))` for ↓, nothing
  for ↑ and Tab with no row lit; with a closed list Enter still applies
  and ↓ and Tab send nothing (before: Tab completed); the Tab flows press
  ↓ first; `the_typed_text_is_listed_first_and_drawn_muted` was removed.
- Containers: the reducer, `table_search.rs` and model tests that used
  `Complete` pick with `Pick(0)`; the view flow presses ↓ then Tab; two
  tests lost the typed first row in their expected lists.
- Agents: `a_search_that_lets_nothing_through_says_so_and_suggests_while_open`
  typed text that only the typed row matched; it types `profile` now (the
  Profile key).
- The binary: `down_and_enter_in_the_table_search_pick_the_first_key`
  (`crates/app/otty/src/screens/containers.rs`): through the whole window
  and the input checks, type `sta`, ↓, Enter: the field holds `Status:`
  (the shown name of the key).

**Tests seen failing first.** With the tests written, the kit failed to
compile (no `Highlight`). With a `Highlight` that did nothing, 38 kit
tests failed on their asserts, among them every new field test above
except `enter_with_no_row_lit_applies_the_text` and
`esc_closes_the_list_first`, which the old rules also passed (the typed
row was lit, and Esc already closed first). The binary test passed with
the old rules too (↓ moved from the typed row to the key), so it was
seen failing by taking the ↓ binding out of the shared piece: the field
then kept `sta`.

**Choices and deviations**

- **↓ in a closed list does not open it any more.** Before, ↓ opened the
  tag list after Esc. The folder fields never did, so the tag search
  follows them; typing or focusing the field opens it. Say if ↓ should
  open a closed list in every field.
- **Tab with no row lit does nothing** in the app today: neither iced
  0.14 nor the app moves the focus on Tab, so "Tab moves the focus" is
  what the folder fields do now, which is nothing. Moving the focus
  between fields would be a separate change for all fields.
- `TagSearchField` keeps its action function in an `Rc`, since the
  input, the pick and the move of the light each need it.
- The command palette keeps its own rules (issue 39).

**Check by hand**

1. Containers → the search over a table: type `sta`. No row is lit.
   ↓ lights `Status`, ↓ moves down and stops at the last row, ↑ moves up,
   ↑ on the first row leaves the list.
2. ↓ then Enter: the field holds `Status:` and lists its values; ↓ Enter
   on a value adds the tag and empties the field. Tab does the same as
   Enter on a lit row.
3. Type `Status:ru`, Enter with no row lit: the tag `Status:ru`. Type a
   name part, Enter: the table filters by it and the list closes.
4. Type a letter after ↓: the light goes. Esc closes the list; the text
   stays.
5. Backspace in the empty field removes the last tag.
6. Agents mode → a project: the sessions search does the same.
7. The New session and profile folder fields still behave as in
   issue 39.
