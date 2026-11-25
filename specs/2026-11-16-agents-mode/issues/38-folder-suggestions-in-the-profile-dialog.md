# 38 Folder suggestions in the profile dialog

Status: ready-for-agent
Blocked by: 37

## Why

Issue 36 gave the folder field of the New session dialog suggestions: as
the path is typed, the folders it may stand for float under the field. The
config folder field of the profile dialog (Add profile, Edit profile) has
none. The owner wants the same there, by the same code (spec Decision 17,
2026-11-17): "if it exists in one place, reuse it".

## What to do

1. **The config folder field of the profile dialog** shows folder
   suggestions exactly as the New session dialog does: the same look-up
   (`AgentsOutcome::LookUpFolder` / `AgentsIntent::FolderLookedUp`, the
   io crate's `folder_lookup`), the same rules (`~`, a trailing `/` lists
   the folders in it, hidden folders only when the dot is typed, at most
   50), the same pick (puts the folder with a `/` in the field and lists
   the next level), Esc closes the list first, a press away closes it, an
   answer for an older text is dropped. Config folders are usually hidden
   (`~/.claude-work`), so check that typing `~/.c` lists them.
2. **One piece of code for both dialogs.** Today the suggestion state
   (the query, the answer, stale answers, the pick) lives in
   `new_session_dialog/`. Move what both dialogs need into one module of
   the agents feature that both use; do not copy it. An answer of the
   look-up must reach the dialog that asked (only one dialog is open at a
   time; make the answer carry enough to drop it when it is not for the
   open dialog).
3. **The profile dialog's own checks stay as they are.** Its folder may
   not exist yet (a new profile can name a new folder), so a "no such
   folder" answer of the look-up must not become an error of the profile
   dialog. Only the suggestions are added. The folder choice of a new
   profile (typed or the default) keeps working; suggestions show only for
   the typed folder.
4. **Keep the focus.** The field keeps the keyboard while suggestions come
   and go (fixed in the kit in commit 269e6f4; do not break it).
5. Texts: none new is expected; if one is, all ten locales.

## Done when

- Tests, written first and seen failing, cover: typing in the profile
  dialog's folder field asks for a look-up; the suggestions show; a pick
  fills the field and lists the next level; a stale answer and an answer
  for the other dialog are dropped; a missing folder is no error in the
  profile dialog; Esc closes the list before it cancels the dialog; the
  New session dialog still works as before (its tests stay green).
- The binary: typing in the profile dialog's folder field through the
  whole window (the guard allow-list, issue 26) shows the suggestions.
- Run by hand: Add profile, type `~/.c`: the hidden config folders show;
  pick one; the field keeps the focus while typing.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

### 2026-11-17 — built

**What was built**

- **One folder field with suggestions.** The new module
  `folder_suggestions.rs` of the agents feature holds what both dialogs
  need: the typed path, the latest answer of the look-up, whether the list
  is open, the query (`~`, a trailing `/` lists the folders in it), the
  shown path with `~`, the pick and the drop of stale answers. It was in
  `new_session_dialog/state.rs`; it moved, it was not copied. The New
  session dialog keeps only what is its own: whether Start can use the
  folder (`FolderState`).
- **The answer reaches the dialog that asked.** `FolderQuery` has a new
  field `asker` (`FolderAsker::NewSessionDialog` or
  `FolderAsker::ProfileDialog`); the answer carries the query back, so the
  feature gives it to that dialog only, and a field drops an answer whose
  query is not its own (an older text, or the other dialog).
- **The profile dialog.** The typed config folder (Add profile with "Type
  a path" picked, and Edit profile) asks for a look-up as it is typed and
  shows the folders under the field. A pick puts the folder with a `/` in the field
  and lists the next level; Esc closes the list first, then cancels the
  dialog; a press away closes it. Two new intents,
  `ProfileDialogIntent::SuggestionPicked` and `SuggestionsClosed`. The
  answer "no such folder" is never an error here: the dialog's checks are
  the same as before. The new folder OTTY creates (the default way) gets no
  suggestions; switching the way closes the list.
- **Focus.** The config folder field always takes suggestions, also while
  there are none, so the kit keeps the field wrapped and its keyboard
  (commit 269e6f4).
- The io look-up is unchanged but for its doc; the binary only passes the
  new field. The profile dialog guard already lets every
  `AgentsIntent::Dialog(_)` through; the guard test lists the two new
  intents. `PROFILE_CONFIG_FOLDER` is the public id of the field, for the
  binary's tests. No new texts.

**Choices and deviations**

- A picked folder ends with `/`; the profile saves its config folder
  without the slash at the end (the path is cleaned before the checks and
  the save). The checks themselves are unchanged.
- The edit dialog shows the saved folder as before (a full path, not
  `~/…`); its suggestions start once the user types.

**Tests seen failing first** (each failed to compile before the code):
the shared field (queries, the asker in the query, stale answers, an
answer for the other dialog, suggestions, pick, close, the `~` path, moved
from the New session dialog); the New session dialog's folder state; the
profile dialog reducer (typing `~/.c` asks the look-up for the profile
dialog, the new folder asks nothing, the edit dialog asks too, the
suggestions show, a pick fills the field, lists the next level and saves
without the slash, closing, a stale answer and an answer for the New
session dialog dropped, a missing folder does not block Add, switching the
way closes the list), its view model (suggestions, no error for a missing
folder) and its view (a press on a suggestion picks it, Esc closes the list
first). In the binary, through the whole window and the input checks:
typing `~/.c` in the profile dialog field and the made-up answer show
`~/.claude-work/` under it; with the view's suggestions taken out this test
was also seen failing on its assert. The io test that `.c` lists
`.claude-work`, `.codex-work` and `.config` passed at once (the look-up
already did it). The New session dialog tests stay green.

**Check by hand**

1. Agents mode → Add profile → "Type a path": type `~/.c`. The hidden
   config folders (`~/.claude-work/`, `~/.codex/`, …) show under the field, and
   the field keeps the keyboard while typing letter by letter.
2. Pick one: it goes in the field with a `/` and its folders show. Add
   saves the folder without the `/` at the end.
3. Esc with the list open closes it; Esc again cancels the dialog.
4. Type a folder that is not there: no error, Add still works.
5. "New folder": typing its path shows no suggestions.
6. Edit profile: typing in the config folder field shows suggestions.
7. The New session dialog still suggests folders as before.
