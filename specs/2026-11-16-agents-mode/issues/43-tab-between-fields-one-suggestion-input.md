# 43 Tab moves between fields; one input with suggestions everywhere

Status: ready-for-agent
Blocked by: 42

## Why

The owner (2026-11-18, spec Decision 18):

- Tab must move to the next field in every form and dialog of OTTY. Today
  neither iced 0.14 nor the app moves the focus on Tab, so Tab with no
  suggestion lit does nothing (issue 41).
- "The search and the work of every input with suggestions must be the
  same, so the code must not be duplicated."
- Down does not open a closed list of suggestions (keep as it is).

## What to do

1. **Tab and Shift+Tab** move the keyboard to the next and the previous
   field (iced's focus operations, `focus_next` / `focus_previous`, or
   what iced 0.14 has for it; read its source first).
   - In a modal dialog (kit `Dialog`) the focus moves only among the
     dialog's fields and wraps around inside it; it never reaches what is
     behind the dialog.
   - In a form page (the containers Create/Run forms, settings, and any
     other form OTTY has; list them) it moves among that form's fields.
   - A terminal pane keeps its Tab: Tab in a focused terminal still goes
     to the shell, and no shortcut takes it. Check every place that takes
     keys before the terminal does.
   - In a field with suggestions, Tab with a row lit picks it (as now);
     with none lit it moves the focus. Esc and Enter do not change.
   - One place in the code does this for the whole app (the kit or the
     binary's key handling, whichever fits the architecture checks); no
     form gets its own Tab code.
2. **One input with suggestions.** Every input that shows a list of
   suggestions uses the kit's `SuggestionInput` for the typing, the list,
   the lit row, the keys, the press on a row and away, and the look; the
   field gives only its rows and what a pick does.
   - Find every input with suggestions in the whole workspace (kit, every
     feature, the binary) and list it in the comment with what it uses.
   - The command palette was left out in issue 39 because its list is the
     palette's body, not a list floating under the field. Check its code
     for any rule it duplicates (moving the lit row, Enter on the lit row,
     Esc, the row look) and take those from the shared piece; keep only
     what really differs (its groups and footer), and say what and why.
   - No key rule, list look or pick logic may stay copied anywhere.
3. Down on a closed list does nothing (owner); keep a test for it.

## Done when

- Tests, written first and seen failing, cover: Tab and Shift+Tab move
  between the fields of a dialog and wrap inside it; they never reach a
  field behind the dialog; Tab in a form page; Tab in a focused terminal
  goes to the terminal; Tab with a lit suggestion picks, with none moves
  on; Down on a closed list does nothing. The binary tests press Tab
  through the whole window in the profile dialog and the New session
  dialog (the guard allow-list, issue 26).
- The comment lists every input with suggestions and what it uses, and
  every rule that was copied and is now shared.
- Run by hand: the profile dialog, the New session dialog and a
  containers form: Tab and Shift+Tab walk the fields; a terminal still
  completes with Tab.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

### 2026-11-18 — built

**What was built**

- **Tab and Shift+Tab walk the fields, from one place in the kit**: the
  private widget `FieldFocus` (`crates/ui/kit/src/components/field_focus.rs`).
  The kit wraps every `Dialog` and every `FormPage` in it, so no form or
  dialog has Tab code of its own and the binary needs none. After the
  widgets inside had the key, a Tab that none of them took moves the
  keyboard to the next field inside it, Shift+Tab to the one before, round
  from the last field to the first and back. Ctrl, Alt or the logo key with
  Tab are not moves.
  - In a dialog (`FieldFocus::modal`) Tab always stays in the dialog: with
    no field of it holding the keyboard, Tab gives it to the first field,
    Shift+Tab to the last; a dialog with no field keeps the press too. So
    nothing behind a dialog ever gets Tab.
  - In a form page (`FieldFocus::form`) Tab walks only from a field of the
    form; a Tab in a field outside the page is left alone.
  - iced 0.14's `focus_next` / `focus_previous` were read and not used:
    they walk every field of the window and do not go round (from the last
    field the keyboard goes nowhere). `FieldFocus` counts the fields with
    iced's own `focusable::count` and gives the keyboard to one of them with
    a small operation over its own content only.
  - A field is what takes the keyboard in iced 0.14: a text input (also the
    inputs inside a token, key-value and mapping field). Buttons, switches,
    drop-downs and segmented choices can not take the keyboard in iced
    0.14, so Tab passes over them.
- **A terminal keeps its Tab.** The terminal widget takes Tab itself (it
  writes `\t`, and `ESC [ Z` for Shift+Tab) and marks the key as used, so
  nothing after it sees it, and no terminal lies inside a dialog or a form
  page. No shortcut uses Tab (`keys.rs` matches arrows and characters
  only). Under a dialog the dialog takes Tab before the window behind it.
- **A field with suggestions**: Tab with a row lit picks it, as before, and
  the field keeps the keyboard. With none lit Tab is not taken, so it moves
  to the next field. Shift+Tab never picks: a Tab binding of `InputKeys`
  is for Tab alone. A field that loses the keyboard closes its list (new
  `InputKeys::on_blur`, sent by `SuggestionInput` with its close message),
  so no list stays open under a field that was left. Esc and Enter did not
  change.
- **Down on a closed list does nothing**, as the owner answered; a test
  keeps it (`down_on_a_closed_list_does_nothing`).

**Forms and dialogs Tab now walks**

- Dialogs (kit `Dialog`): the profile dialog (name, config folder, admin
  key, budget), New session (folder), the containers create dialogs (Pull
  image: image and tag; Create volume and Create network: name and
  driver), Settings (shell, editor), the SSH password prompt. The dialogs
  with no field (Files delete, containers confirm, agents remove,
  statistics) keep Tab and do nothing with it.
- Form pages (kit `FormPage`): the containers Run form, the launch editor
  of the Launches tree.
- Not forms, left as they were: the search fields on a screen (containers
  table search, the Agents project and sessions searches, the title bar
  search) and the name fields typed in the trees. Tab in them still does
  nothing.

**Every input with suggestions, and what it uses**

1. New session dialog, folder (`agents/core/src/new_session_dialog/view.rs`):
   kit `TextField::suggestions` → `SuggestionInput`.
2. Profile dialog, config folder (`agents/core/src/profile_dialog/view.rs`):
   the same.
3. Containers table search (`containers/core/src/view/main_area.rs`): kit
   `TagSearchField` → `SuggestionInput`.
4. Agents project screen, sessions search (`agents/core/src/view/project.rs`):
   the same.
5. Command palette (`palette/src/view/command_palette.rs`): kit
   `CommandPalette`, which now takes the keys of its list (`ListKeys`) and
   the look of an entry row (`list_row`) from `suggestion_input.rs`.

Not inputs with suggestions: `NoteSelect` (the agent drop-down: nothing is
typed), `SearchField` (the projects search filters the list itself),
`TitleSearch`, `TokenField`, `KeyValueField`, `MappingField` and the tree
name fields.

**Rules that were copied and are now shared**

All of them were in the command palette; the folder fields and the tag
search already shared them (issues 39 and 41).

- Moving the lit row: `PaletteList::move_up` / `move_down` (first entry lit
  at once, wrap at both ends). Now `ListKeys` decides every move: no entry
  lit until ↓, ↓ and ↑ stop at the ends, ↑ from the first entry leaves the
  list. `PaletteList` only keeps the row (`highlight`, `highlighted`);
  typing clears it. `PaletteAction::MoveUp`, `MoveDown` and `RunActive`
  are gone, `Highlight(Option<usize>)` and `RunFirst` are new.
- Enter on the lit row: was the input's submit on the active entry. Now
  `ListKeys`: Enter and Tab with an entry lit run it (`RunItem`, the same
  as a press on it).
- Esc: was a binding of its own. Now `ListKeys`: Esc closes an open list,
  then is the field's own; for the palette both are Close.
- The row look: the palette entry built its own menu-item button. Now
  `list_row`, the same function `SuggestionInput` draws its rows with
  (full width, `padding: 0 10px`, the menu item colours, lit or not).
- `ListKeys` is the old `SuggestionInput::list_keys`, taken out of the
  struct so the palette can bind the same keys on its own input.

What stays the palette's own, and why: its list is the body of the panel,
not a list floating under the input (so not `FloatingList`); the group
labels between the entries and the "No matches" line; the footer of key
hints; its entry height (`size::PALETTE_ITEM`, the design's) and its
twelve visible entries; the look of its input; what Enter does with no
entry lit (see below).

**Choices and deviations**

- **Enter with no entry lit runs the first match** of the palette
  (`RunFirst`). The shared rule says Enter with none lit "does what the
  field does without a list"; for the palette that is taken to be running
  the best match, so typing `split` and Enter still works. Say if Enter
  should do nothing until ↓ instead.
- **The palette search still differs** from the tag search: the palette
  finds the typed text inside a title or a detail (case aside), the tag
  search ranks fuzzy matches and forgives one typo (`frizbee`), the
  folder fields ask the disk for names that start with the text. The
  issue's list of what must be shared did not name the matching, so it was
  left; the owner's "the search ... must be the same" may mean it. Say if
  the palette should rank like the tag search (one matcher for both).
- **The lit palette entry is not scrolled into view** past the twelve
  shown, as before; `FloatingList` keeps a lit row in view only for a list
  with rows of one height, and the palette list has group labels.
- **Tab walks only dialogs and form pages.** A search field on a screen
  is no form, so Tab there does nothing, as before.
- **Leaving a field closes its list a frame later.** iced tells a field
  that it lost the keyboard only on the next event; the walk asks for a
  new frame, so the close comes at once to the eye. A press away from a
  field can now send its close twice (the press away and the lost focus);
  closing a closed list does nothing.

**Tests seen failing first**

- Kit `dialog.rs`: `tab_moves_to_the_next_field_and_wraps_inside_the_dialog`,
  `shift_tab_moves_to_the_field_before_and_wraps_inside_the_dialog`,
  `tab_never_reaches_a_field_behind_the_dialog`,
  `tab_in_a_dialog_with_no_field_stays_in_the_dialog`. The first run of the
  first two failed for a wrong reason (the test window gave the dialog no
  room); after that fix they were seen failing on their asserts again with
  the dialog not wrapped in `FieldFocus`.
- Kit `form_page.rs`: `tab_moves_among_the_fields_of_the_form` failed on its
  assert. `tab_in_a_field_outside_the_form_does_not_enter_it` passed from
  the start: it guards the field outside.
- Kit `text_field.rs` (a folder field and a name field in a dialog):
  `tab_with_no_row_lit_moves_to_the_next_field_and_closes_the_list` and
  `shift_tab_with_a_row_lit_moves_back_and_picks_nothing` failed.
  `tab_with_a_row_lit_picks_it_and_stays_in_the_field` and
  `down_on_a_closed_list_does_nothing` keep what was already so and passed;
  the second was seen failing by binding ↓ on a closed list.
- Kit `command_palette.rs` and `palette_list.rs`: first they did not
  compile (no `Highlight`, `RunFirst`, `highlight`); with empty stand-ins
  `with_no_row_lit_each_key_sends_the_shared_action`,
  `with_a_row_lit_the_keys_move_and_pick_by_the_shared_rules`,
  `a_highlight_lights_that_match_only`,
  `a_highlight_past_the_matches_lights_nothing`,
  `a_new_query_clears_the_highlight` and the first-listing test (none lit)
  failed on their asserts.
- Palette feature (reducer and view): written before the code against the
  new actions; they could not compile against the old kit. Changed:
  `arrows_move_the_active_entry` became
  `a_highlight_lights_the_entry_and_typing_clears_it`;
  `enter_switches_to_the_active_session_and_closes_the_palette` became
  `enter_with_none_lit_runs_the_first_match_and_closes_the_palette`; the
  first listing has none lit; the containers test lights entry 7 and runs
  it; the view flow presses ↓ twice (the first ↓ now lights the first
  match); new `typing_then_enter_with_none_lit_runs_the_first_match`. The
  binary tests that typed and pressed Enter send `RunFirst` (the same
  meaning); the guard test sends `Highlight(Some(0))`.
- Binary, through the whole window and the input checks:
  `tab_walks_the_fields_of_the_profile_dialog` (name → folder → name, then
  Shift+Tab → folder; each letter reaches the field the keyboard is in) and
  `tab_in_the_new_session_dialog_never_leaves_it` (no field focused, the
  projects search showing behind: Tab and Shift+Tab give the keyboard only
  to the folder field) failed on their asserts with the walk off.
  `tab_in_a_focused_terminal_goes_to_its_shell` (Tab writes `\t`,
  Shift+Tab `ESC [ Z`) passed from the start: it guards the terminal's Tab.

**Check by hand**

1. Agents mode → Add profile → "Type a path": click the name, Tab: the
   config folder field has the keyboard; Tab again: back to the name;
   Shift+Tab: the folder. An API key profile adds the admin key and the
   budget to the walk.
2. In the config folder type `~/.c`: the list opens. Tab with no row lit
   goes to the next field and the list closes. ↓ then Tab picks the row and
   stays in the field. ↓ then Shift+Tab goes back and picks nothing.
3. Agents mode → "+" (New session): Tab keeps the keyboard in the folder
   field, also with the projects search open behind the dialog.
4. Containers → Pull image: Tab walks image and tag, round. The Run form:
   Tab walks its fields (image, name, command, ports, environment,
   volumes), round, and skips the switches, drop-downs and buttons.
5. Settings: Tab walks shell and editor. The launch editor of the Launches
   tree: Tab walks its fields.
6. A terminal: Tab still completes in the shell; Shift+Tab goes to the
   shell too.
7. The command palette: no entry is lit when it opens; ↓ lights the first,
   ↓ stops at the last, ↑ from the first leaves the list; Enter or Tab runs
   the lit entry; with none lit Enter runs the first match; Esc closes.
8. With a list closed by Esc, ↓ does not open it, in every field.
