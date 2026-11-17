# 39 Folder suggestions: no flicker, arrows and Enter

Status: ready-for-agent
Blocked by: 38

## Why

The owner tried the folder suggestions of issues 36 and 38 (2026-11-17):

- "the suggestions twitch when I type characters; it must not be so";
- "I want to move through them with the arrows and pick with Enter".

Cause of the twitch, as read by the coordinator:
`FolderSuggestions::suggestions` (`crates/app/features/agents/core/src/folder_suggestions.rs`)
shows only an answer whose query is the text typed now. After each letter
the old answer no longer counts, so the list closes, and it opens again
when the new answer comes a moment later. Check whether anything else also
moves (the list's width or place following its rows, the error line under
the field coming and going while the answer is pending) and fix that too.

## What to do

1. **No flicker.** While the answer about the new text has not come, the
   list keeps showing what it showed, narrowed by what was typed since
   when that is possible (the folders of the same parent whose name starts
   with the new text, case aside, hidden ones only with a dot, as the
   look-up does). When the new answer comes it replaces them. The list
   closes only when the new answer has no folders, when the text asks for
   nothing (empty, not a full path), on a pick, on Esc or on a press
   away. The list does not jump in place or width while typing; the error
   or hint line under the field does not blink while an answer is pending
   (keep the last state until the answer).
2. **Keyboard.**
   - Down opens the highlight on the first row, then moves down; Up moves
     up; at the ends it stops (no wrap), and Up from the first row leaves
     the list (no row highlighted).
   - The highlighted row is kept in view (the `FloatingList` already
     scrolls to a highlighted row; the tag search field uses it).
   - Enter on a highlighted row picks it, exactly as a press on it does
     (the field gets the folder with `/` and the next level lists). With
     no row highlighted, Enter does what it does today (Start / Save).
   - Tab with a highlighted row picks it too (as in the tag search), else
     moves the focus as today.
   - Typing a letter clears the highlight. The pointer over a row shows
     the hover look as today; it does not move the highlight.
   - Esc closes the list first, then cancels the dialog (as today).
   - The field keeps the keyboard through all of it (commit 269e6f4).
3. **Where it lives.** Reuse what the kit already has for the tag search
   (`tag_search_field.rs`: the highlighted row, the keys on the input
   through `InputKeys`, `FloatingList::list(list, highlighted, ...)`). The
   keyboard handling belongs in the kit `TextField` suggestions so every
   field with suggestions gets it; the feature keeps only what it must
   (which row is highlighted, if the kit can not keep it). One code path
   for both dialogs (New session and profile), as issue 38 set.

4. **One suggestion mechanism in the whole app** (owner, 2026-11-17:
   "suggestions are the same everywhere as a mechanism, only the data
   differs, so the code must be reused everywhere"). The field with a
   list under it — what is typed, the rows, the highlighted row, the keys
   (Down, Up, Enter, Tab, Esc), the press on a row, the press away, the
   scroll to the highlight, the look — is one kit piece. The tag search
   field (`tag_search_field.rs`) and the folder field (`TextField`
   suggestions) both use it, and only give their rows and what a pick
   does. Look for every other place that shows suggestions under an input
   (the command palette and its list, `note_select`, the containers and
   other features) and move it onto the same piece when it is the same
   mechanism; list what you moved and what you left, with why. The tag
   search tests and the command palette tests stay green.

## Done when

- Tests, written first and seen failing, cover: the list stays open and
  narrowed while an answer is pending, and is replaced when it comes; it
  closes when the answer has none; Down/Up move and stop at the ends; Up
  from the first row leaves the list; Enter and Tab pick the highlighted
  row; Enter with none highlighted submits; typing clears the highlight;
  Esc closes the list first. Kit tests drive the field frame by frame,
  keeping iced's state between frames (as the focus test of
  `text_field.rs` does), so a lost focus or a reset highlight shows.
- The binary: in the New session dialog through the whole window, type,
  press Down and Enter: the field holds the picked folder.
- Run by hand: type a path letter by letter in both dialogs: the list
  does not blink or jump; Down/Up/Enter work.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

### 2026-11-17 — built

**Why the list twitched** (three causes, all fixed)

1. **The list closed after each letter.** `FolderSuggestions::suggestions`
   showed only an answer whose query was the text typed now, so after a
   letter there were no rows until the new answer came (as the
   coordinator read it).
2. **The dialog jumped up and down.** While the answer was on its way the
   New session field said nothing ("checking"), so a "There is no such
   folder" line under it went away and came back after each letter. The
   dialog is centred in the window, so the field and the list moved by half
   a line each time.
3. **The list could jump to the other side of the field.** The kit chose
   the side (under or over the field) for the list's height now. As rows
   came and went in a smaller window, a list that no longer fit under the
   field flipped over it. The width never followed the rows (it is the
   field's width, at least 260).

**What was built**

- **One suggestion piece in the kit** (item 4): the private module
  `suggestion_input.rs`, `SuggestionInput`. It is the input, the list
  floating under it, the highlighted row lit and kept in view, the keys
  bound on the focused input, a press on a row, a press away, the cursor
  moved to the end when the app replaces the text (after a pick), and the
  look of the list and its rows (a menu). A field gives only its rows (what
  each shows and what a press on it sends), the highlighted row and what
  each key sends. `TagSearchField` and the `TextField` suggestions both
  use it now. Before, each had its own list, rows and look.
- **Keys in the kit `TextField`.** A new `highlighted_suggestion(row,
  on_highlight)`. While the list shows: ↓ highlights the first row, then
  the next, and stops at the last; ↑ goes up, and from the first row
  leaves the list; Enter and Tab pick the highlighted row (the same message
  as a press); with none highlighted, Enter submits and Tab does what it
  did before; Esc closes the list, then is the field's own Esc (Cancel).
  The views no longer switch the Esc message themselves.
- **The list does not blink.** `FolderSuggestions` keeps the shown rows
  apart from the answers. On typing, the rows of the last answer that the
  new text also stands for stay (same parent folder; the name starts with
  the new text, case aside; hidden ones only with a dot). When nothing is
  left, or the text now lists another folder (a `/` typed), the rows stay
  as they were. The new answer replaces them; an answer with no folders
  closes the list; an empty or not-full path closes it. The narrowing uses
  the look-up's own rule, now one function, `FolderQuery::suggests`, which
  the io look-up also uses (it was written inline there).
- **The line under the New session field keeps what it said** while the
  answer is on its way (`NewSessionDialog::shown_folder_state`); Start
  still waits for the answer.
- **The side of the list** is chosen for the list at its tallest (eight
  rows): under the field when that fits, else over it, else where there is
  more room (`FloatingList::tallest`). The tag search gets the same.
- **The highlight** lives in `FolderSuggestions` (`highlight`,
  `highlighted`). Typing, closing and picking clear it. An answer that
  comes while a row is highlighted keeps the highlight on the same folder,
  or drops it if that folder is gone. Each dialog has a new intent
  `SuggestionHighlighted(Option<usize>)`. Both dialogs go through the same
  code (`FolderSuggestions` and the kit field).

**Other places that show a list (item 4), and what was done**

- **Moved:** `TagSearchField` (and so every feature that uses it: the
  containers table search, the agents project search) and the `TextField`
  folder suggestions (New session and profile dialogs).
- **Left: the command palette.** Its list is not a list floating under a
  field. It is the body of the palette panel, with group labels, its own
  row height and a footer. It already shares `InputKeys` and the menu row
  look. Putting it on the floating piece would change the palette's design.
- **Left: `NoteSelect`** (the agent field of the profile dialog). It is a
  drop-down button, not an input: nothing is typed, there is no
  highlight to move. It already uses `FloatingList`, the layer under the
  new piece.
- No other input in the app shows suggestions (`TokenField`,
  `SearchField`, `TitleSearch` have none).

**Choices and deviations**

- **The app keeps the highlighted row, not the kit.** The rows are built
  from the app's data in each frame, so the kit must know the highlight
  when the view is built. iced's widget state can not be read then. The
  kit decides every move (where ↓ and ↑ go, what Enter and Tab pick); the
  app only stores the number it gets back.
- **The tag search keeps its own rules**: its first row is always
  highlighted, ↑ and ↓ wrap at the ends, Enter applies and Tab completes
  (its own actions), Backspace in an empty field removes the last tag. Its tests stay green as the issue asks. The piece
  carries the keys; what each key does stays with each field.
- **A pick now lists the next level.** Before, a pick closed the list
  until the next letter. Now the rows close on the pick and the answer
  about the picked folder opens its folders. So ↓ Enter ↓ Enter walks down
  the tree from the keyboard. This is what "the next level lists" reads
  as; say if the list should stay closed instead.
- **Start blinks for a moment.** While the answer is on its way Start is
  off, so on a found folder it can go off and on as you type (the look-up
  reads one folder, so this is about one frame). It stays off because a
  Start or Enter pressed then would use a folder not yet checked.
- **Up with no row highlighted** is not taken by the field, and neither
  is ↓ while the list is closed.

**Tests seen failing first** (each failed to compile, or with a wrong
value, before the code):

- Kit `TextField`, frame by frame with iced's state kept between frames
  (the focus test now uses the same harness): ↓ highlights the first row,
  then the next, and stops at the last; ↑ moves up and from the first row
  leaves the list; Enter picks the highlighted row and the next letter
  goes after it; Tab picks it; Enter with none highlighted submits; typing
  after a move clears the highlight (Enter then submits); Esc closes the
  list first, then is the field's Esc.
- Kit `FloatingList`: the side is chosen for the tallest list; with room
  on neither side, the side with more room.
- `FolderQuery::suggests`: prefix, case, hidden folders.
- `FolderSuggestions`: rows narrowed while the answer is on its way; the
  answer replaces them; an answer with none closes the list; rows that can
  not be narrowed stay; an empty or relative text closes the list; a shown
  row is highlighted, a missing one is not; typing, closing and picking
  clear the highlight; an answer keeps the highlight on its folder; the
  answer about a picked folder lists the next level.
- New session dialog: while the answer is on its way the field says what
  it said (a missing folder, an empty path), and the new answer replaces
  it; reducer: a highlighted row is picked, typing clears the highlight;
  view: ↓, ↑, Enter, Tab, Enter with none highlighted (Start), Esc with
  and without the list.
- Profile dialog: reducer: the highlight is kept until typing and the list
  stays narrowed; view: ↓, ↑, Enter, Tab.
- Binary, through the whole window and the input checks: in the New
  session dialog type `/pro`, give the made-up answer, press ↓, then
  Enter: the field holds `~/projects/` and the list of `~/pro` is gone.
  This test was written after the feature code. With the view's
  `highlighted_suggestion` taken out it was seen failing on its assert.
  The guard lists name the two new intents.

The tag search, command palette and all other tests stay green.

**Check by hand**

1. Agents mode → "+" (New session): type `~/pro` letter by letter. The
   list does not close and open, does not jump, and the line under the
   field does not blink.
2. Press ↓: the first row lights; ↓ again moves down and stops at the last
   row; ↑ moves up; ↑ on the first row leaves the list. The pointer over a
   row shows the hover look and does not move the highlight.
3. Enter on a highlighted row: the folder goes in the field with `/`, its
   folders list, and the next letter goes at the end. Tab does the same.
   With no row lit, Enter starts the session (when the folder is found).
4. Esc closes the list; Esc again cancels the dialog.
5. Add profile → "Type a path": the same with `~/.c` (hidden config
   folders).
6. Make the window low, so the folder field sits near the bottom: the list
   opens over the field and stays there while typing.
7. The tag search of a project screen and of Containers still works as
   before: ↑/↓ wrap, Enter applies, Tab completes, Backspace in an empty
   field removes the last tag.

### 2026-11-18 — coordinator: the owner's answers

1. A pick lists the folders in the picked folder: yes, keep.
2. Start off for a moment while the typed folder is checked: yes, fine.
3. The tag search follows the same keys as the folder fields: yes
   (issue 41).
