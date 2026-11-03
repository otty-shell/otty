# 49 — Search keys are written in the interface language

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 48
Spec: [../spec.md](../spec.md)

## What

Owner request (2026-11-03), spec Decision 21 is updated. Keys are English in every
language, so in Russian a query means typing half in Latin and half in Cyrillic:
`status:работает`. The rule becomes: **you write what you see on screen**. In
Russian that is `Имя:web-1` and `Состояние:работает`.

- A key is written as its column header in the interface language, in suggestions,
  in the field and on tags. English stops being special: in Russian `status:x` is
  not a key, so the whole text is a name filter.
- Case does not matter for any alphabet. `parse` in
  `crates/ui/kit/src/components/tag_search.rs` uses `eq_ignore_ascii_case` today,
  which does not match `имя` with `Имя`.
- A header with a `:` gives the part before it: `Repository:Tag` gives
  `Repository`, `Репозиторий:тег` gives `Репозиторий`.
- A wide `：`, which Chinese and Japanese input often produces, counts as `:`. The
  first colon of either kind ends the key, so `ports:8080:80/tcp` still works.
- A header with a space stays whole: `Used by:db-1`, `Genutzt von:db-1`. This
  replaces the typed key `used-by`.
- Values do not change: they are compared with the text in the cell.
- A column with no translation shows English, and its key is English too.

**Keep a stable name inside.** Each key keeps its internal name (`name`, `status`,
`compose`...), which is never typed or shown. A tag stores this name and shows the
current header, so a tag's key survives a change of the interface language. The
Compose link still adds its tag through `COMPOSE_KEY`.

The kit still knows nothing about languages: the widget gives the shown name of each
key through `TagSchema`, and `TagRow` gets what it needs to show it. Take the
Docker names from the same translation keys as the columns in
`app/src/widgets/docker/resource_rows.rs`, so a key and its header cannot differ. No
new strings.

Known limit, accepted by the owner: a tag's value picked in one language, such as
`Состояние:Работает`, stops matching after a switch to English until it is removed.
Tags are not saved to disk.

Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Kit tests, with names given by the test so no process-wide language changes:
  a shown name is recognized in any case, including Cyrillic; the internal name is
  not recognized when it differs from the shown name; a wide colon works;
  `redis:7` stays plain text; a value with `:` stays whole; suggestions, the text
  after applying a key and the tag all show the shown name.
- App test: in every locale and for every table, the shown names are not blank,
  have no colon and do not repeat.
- Reducer test: the Compose link replaces the `compose` tag by its internal name;
  a tag's key shows the new header after the language changes.
- `regression-checklist.md`, section "Tag search": the steps use the shown names
  (`Status:` in English) and add a step in Russian: `Сост` then Tab gives
  `Состояние:`, `раб` then Tab gives the tag `Состояние:Работает`.

## Answer

A key is now written as its column header in the interface language, in the
suggestions, in the field and on tags. Each key keeps a stable internal name that
is never typed or shown.

**Kit (`tag_search.rs`, still no languages in it).**

- New `TagKey::new(name, header)`: `name()` is the internal name, `shown()` the
  name the user reads and types. It cuts the header at its first colon of either
  kind and trims it, so `Repository:Tag` gives `Repository`, and a space stays
  (`Used by`).
- `TagSchema::new` takes `Vec<TagKey>`; `TagSearch::passes` and `name_text` take
  `&[TagKey]`.
- `parse` splits at the first `:` or `：` and compares the text before it with the
  shown names only, so `status:x` in Russian is a name filter. The comparison uses
  upper case rather than `FoldedText`: lower case does not join `Größe` and
  `GRÖSSE`, and the German header is a real one.
- Suggestions rank the shown names; applying a key writes `shown:` in the field; the
  typed row reads `shown:text`.
- `Tag` stores the internal name. `Tag::label(&keys)` looks up the current shown
  name. A key that is not in `keys` reads its internal name; this does not happen
  in the app, because tags are per table.
- `TagRow::new(tags, keys, palette)` builds its labels from the keys it gets.
- `TableColumn::title()` is a new getter.

**App.** `table_search::keys(resource)` builds the keys from the titles of
`columns(resource)`, so a key and its header cannot differ and there are no new
strings. The internal names stay in `CONTAINER_KEYS` and the others, in column
order. `keys_in(locale, resource)` and `columns_in(locale, resource)` take the
language from the caller, using `i18n::text_in`, which is now `pub(crate)`, so
tests read every locale without touching the process-wide language.
`TableViewModel` gets a `keys` field, built at each view, so a tag shows the new
header right after a language change. The Compose link still calls
`set_tag(COMPOSE_KEY, ...)`.

**Values.** The status cell is translated: in Russian a running container shows
`Работает` and an exited one `Остановлен (3)`, so the example `Состояние:Работает`
is what the screen shows. A test checks it with the real Russian texts.

**Tests changed for the new rule.** Expected labels now read `Status:...`,
`Compose:...` and so on. The kit tests for rules that do not depend on the language
use keys whose shown name equals the internal name, so their typed text stays.

**Known limit besides the accepted one.** Text in the field is not converted: if
`Имя:web` is in the field during a switch to English, it becomes a name filter
until it is edited.
