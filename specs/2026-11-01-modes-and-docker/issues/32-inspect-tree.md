# 32 — Inspect as a readable tree with Raw JSON and Copy

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 28
Spec: [../spec.md](../spec.md)

## What

Spec Decision 20 (Inspect). Tests first.

- Kit: a tree view for structured data (collapsible sections, indented key
  and value rows, numbered list rows, long values wrap), with a gallery
  section and `iced_test`. It takes plain rows built by the app; no JSON
  inside the kit.
- App: build the tree from the inspect JSON (`serde_json::Value`, already a
  dependency of the app through otty-docker? — check; if the app needs
  `serde_json` it is already a workspace dependency of the app). Top-level
  keys become sections in a sensible order (State, Config, HostConfig,
  NetworkSettings, Mounts, then the rest); empty values (null, "", [], {})
  are hidden until "Show empty" is on.
- A "Raw JSON" switch shows today's JSON view; "Copy JSON" copies it with the
  iced clipboard.

All new text goes through `i18n` in the ten locales (spec Decision 17). Any missing visual piece goes into `otty-ui-kit` first with a gallery section and an `iced_test`. Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Pure tests: JSON → rows for objects, nested objects, lists, empty values,
  section order.
- Reducer tests: section open/close state per selection, Raw switch, copy
  effect.
- `iced_test`: sections toggle; Raw shows the JSON; Copy emits its intent.

## Answer

The Inspect tab shows the engine's answer as a tree, with a "Raw JSON" switch,
a "Show empty" switch and a "Copy JSON" button above it. Tests were written
first and seen failing (12 kit tests, 30 app tests), then the code.
`otty-docker` did not change.

### Kit: `components/data_tree.rs`

- `DataRow::new(depth, key, value)` is a key and value row;
  `DataRow::heading(depth, key)` is a key that titles the deeper rows after
  it. They are plain data: no JSON in the kit.
- `DataTree::new(&palette).group(title, rows)` adds a group that is always
  open; `.section(title, is_open, rows, on_toggle)` adds a section with a
  caret. Parts show in the order they are added.
- Rows reuse the private `key_value_list::entry_view`, so wrapping and the
  stacking of ticket 35 are the same code. Each level indents 12 px, at most
  48 px, so deep rows keep room for their value. A row stacks by the width
  left after its indent, so at 264 px the top rows have two columns and deeper
  rows are stacked.
- Gallery: the inspector part shows the tree at 360 and 200 px, in both themes
  (`cargo run -p otty-ui-kit --example gallery -- inspector`, add `light`).
- No new color pairs: `text-strong` titles, `text-body` headings. A test still
  checks both on the panel and canvas surfaces, in both themes.

### App

- `inspect_tree.rs`: `inspect_tree(&Value, show_empty) -> Option<InspectTree>`
  is the pure function; `None` when the value is not an object.
  - A value that reads in one line is a key and value row: a scalar, an empty
    list or object, or a list of scalars of at most 60 characters joined with
    ", ". At the top level these rows form General, with Id, Name and Created
    first.
  - Other top-level keys are sections: State, Config, HostConfig,
    NetworkSettings, Mounts, then the rest.
  - Nested objects are a heading with rows one level deeper. Lists are numbered
    from 1, and a hidden item keeps its number free.
  - Hidden unless "Show empty" is on: null, "", [], {} and any list or object
    holding only those, heading included. 0 and false stay. With it on they
    read `null`, `""`, `[]` and `{}`.
- `inspector.rs`: `InspectLoad::Loaded` holds `LoadedInspect` (the JSON view,
  the parsed value, the open sections). The first section shown starts open.
  `InspectorState` keeps the two switches.
- `reducer.rs`: new intents `InspectSectionToggled`, `InspectShowEmptyToggled`,
  `InspectRawJsonToggled` and `CopyInspectJson`, which uses the existing
  `CopyToClipboard` effect. `sync_inspector` now forgets the kept JSON as soon
  as another row is selected, on any tab.
- `inspector_tabs.rs` builds the rows for the view; `view/inspector.rs` draws
  the controls in a wrapping row and the tree in a scrollable with the thin kit
  scrollbar. The tree has 8 px of right padding so values stay clear of the bar.
- i18n: 4 keys in the ten locales (`Key::ALL` is 407). Section titles and JSON
  keys are shown as the engine sends them; only "General" is translated.

### Tests

- Pure (15): General order, section order, nested objects, joined and numbered
  lists and the 60 character limit, lists of objects, top-level lists, empty
  values with 0 and false kept, keys holding only empty values, hidden items
  keeping their number, Show empty, values that are not objects.
- Reducer (11): first section open, first shown section, open and close, press
  before the JSON arrives, kept over a tab change, reset by a new selection on
  the Inspect tab and on another tab, switches, copy with and without JSON, an
  answer that is not an object.
- View model (4) and `iced_test` (7 in the app, 12 in the kit): sections toggle,
  closed rows hidden, controls send their intents, Raw replaces the tree and
  draws the JSON (a picture hash, since widget queries do not reach a text
  editor), padding, stacking, indents, wrapping, both themes.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 87.23% (67288
lines, 8596 missed) on `e1996b2` -> 87.53% (68457 lines, 8536 missed).

### Deviations and limits

- "The remaining keys in the order they come" is alphabetical today:
  `serde_json::Value` keeps object keys sorted unless serde_json's
  `preserve_order` feature is on. That feature changes key order for all of
  the app's JSON, so it is left for the owner to decide. The same limit is why
  General lists Id, Name and Created first by name, and nested keys read
  alphabetically.
- The two switches stay on for the next selected item; only the open sections
  reset. The design says nothing about the switches, and someone reading raw
  JSON across containers likely wants it to stay.
- A row selected again after another selection, on any tab, asks the engine for
  its JSON again. Before this ticket it reused the old JSON when the change
  happened on another tab.
- An answer that is not a JSON object shows as JSON, with the Raw JSON switch
  on; turning it off does nothing. The engine calls always return objects.
- Long keys break between letters inside the 96 px key column
  (`AppArmorProfil` / `e`), the existing rule of the key-value list.
- Found on the way, not changed: the Overview tab still uses the default thick
  scrollbar, which draws over the end of its values.
- Not run against a live engine; the look was checked through `iced_test`
  pictures only.
