# 49 — Search keys are written in the interface language

Status: ready-for-agent
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
