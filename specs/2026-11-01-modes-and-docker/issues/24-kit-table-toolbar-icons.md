# 24 — Kit: sortable DataTable, link cell, row ⋯ button, collapsing Toolbar, icon tooltips

Status: resolved
Type: task
Class: A
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 02
Spec: [../spec.md](../spec.md)

## What

Kit parts for spec Decision 19. Each gets a gallery section in both themes and
`iced_test` tests, styled only with `UiPalette` and kit tokens.

- `DataTable`:
  - Sortable headers: a column can be sortable; pressing its header emits a
    message with the column index; the sorted column shows an up or down
    arrow. The table does not sort rows itself (the app passes rows in order).
  - Link cell: `TableCell::link(label, message)`, drawn like text in the
    accent color with a hover underline; pressing it emits its message and
    does not select the row.
  - Row "⋯" button: an optional last column with a small "⋯" icon button per
    row; pressing it emits `(row index, point)` for a menu and does not select
    the row. Rows without actions can hide it.
- `Toolbar`: a row of items (a flexible field such as the filter, then icon
  buttons with labels for tooltips and menus). When the row is too narrow, the
  buttons that do not fit move into a trailing "⋯" menu, in order; the field
  shrinks first down to a minimum width. Built with `responsive`; no layout
  logic in the app.
- Icon buttons with tooltips: check `IconButton`; it must take a tooltip text
  and a danger tone. Use it for the toolbar and the inspector actions.
- New stroke icons in `assets/svg/` in the style of the existing ones (24×24,
  round caps): play/start, stop, restart, pause, resume, trash/remove,
  shell/terminal, download/pull, prune (broom), plus/create, more (⋯) if
  missing. Original drawings only; no third-party icon set is copied. Since
  the kit never bundles icons (callers pass `svg::Handle`), the app wires
  them; the gallery may load them from `assets/svg`.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test`: header press emits its column; link press emits its message
  and not a row press; row "⋯" emits index and point and not a row press;
  toolbar overflow puts the last buttons into "⋯" at a narrow width and
  pressing an overflow item emits its message; icon button tooltip text is
  present.
- Contrast tests for the link color and the sort arrow on `surface-canvas`
  and `surface-selected`, both themes.

## Answer

Kit and `assets/svg/` only; the app is unchanged and still compiles (the
new `TableCell::Link` variant needs no app edit: the app has no exhaustive
match on `TableCell`). Tests were written first and seen failing (21 of
the new tests against stubs), then the code.

### Public API (`otty_ui_kit::components`)

`data_table`:

- `SortDirection { Ascending, Descending }`. Ascending draws an up arrow,
  Descending a down arrow; the app decides what each means for a column
  (for "newest first" pass the direction whose arrow the design wants).
- `TableColumn::new(title, share).sortable(bool)` (default `false`).
- `TableCell::link(label)` / `TableCell::Link(String)`.
- `DataTable` new builders:
  - `.sorted_by(column: usize, direction: SortDirection)`: arrow and
    `text-body` title on that column. The table never reorders rows.
  - `.on_sort(Fn(usize) -> Message)`: pressing the title of a sortable
    column sends its index. Without it titles are not pressable.
  - `.on_link_press(Fn(usize, usize) -> Message)`: pressing a link sends
    `(row index, column index)`; the row is not pressed. Without it links
    are drawn but not pressable (a press then selects the row).
  - `.on_row_menu(label, Fn(usize, Point) -> Message)`: adds a 36px last
    column with a "⋯" button per row; `label` is its tooltip. Pressing it
    sends `(row index, bottom-left corner of the button)` in the same
    coordinates as `on_row_right_press` (the table's parent: window
    coordinates unless the table sits in another `scrollable`; the scroll
    offset is taken out). The row is not pressed and not selected.
  - `.row_without_menu(cells)`: a row with a blank "⋯" cell, for a
    resource without actions. `.row(cells)` keeps working and shows the
    button when the table has a row menu.

`icon_button`:

- `IconTone::Danger`: icon `status-error`, `surface-danger` hover
  background, `text-danger` icon under the pointer.
- `IconButton::on_press_maybe(Option<Message>)`. A button without a
  message is disabled and now draws its icon faded (45%, like other
  disabled controls) without hover brightening. Every existing caller
  passes a message, so nothing changes for them.
- The tooltip text was already the `label` argument; it is now tested.

`toolbar` (new):

- `ToolbarItem::new(icon: svg::Handle, label).on_press(msg)` /
  `.on_press_maybe(Option<msg>)`. `label` is the tooltip and, once the
  item moves, its menu item text. An item without a message is a disabled
  button and a disabled menu item.
- `Toolbar::new(more_label, &palette)`, `.field(min_width, max_width,
  Fn() -> Element)`, `.item(ToolbarItem)`; converts into `Element`.
  Built with `responsive`: the field (left) is `max_width` when there is
  room and shrinks to `min_width` first; then the last buttons move, in
  order, into a "⋯" menu at the end of the row. Buttons sit at the right
  end (medium icon buttons, 6px apart, at least 12px from the field). The
  field builder is called on each layout, so the caller's field should use
  `width(Length::Fill)`; it may borrow `&'a str` from the app state.
- The "⋯" menu keeps its open state inside the widget tree (like a pick
  list), so the app holds no menu state and no layout logic. It opens
  under the button, right-aligned (above it when there is no room), and
  closes on an item press, a press outside (also a second "⋯" press) and
  Escape.

### Private parts and files

- `components/overflow_menu.rs` (new): the "⋯" button widget and its
  overlay (the kit `Menu`), with `menu_position` (tested).
- `table_body.rs`: finds the "⋯" button in the row layout on a left press
  and sends the corner, taking out the scroll offset like the right click.
- `ellipsis_text.rs`: `.link()` mode, shrunk to the shown text and
  underlined (1px, text color) while the pointer is over it.
- `glyph.rs`: `sort_arrow(is_ascending)` (12px stroke arrows). The "⋯"
  dots reuse `glyph::dots()`.

### Icons (`assets/svg/`, original drawings)

24×24 view box, `stroke-width="1.8"`, round caps and joins, no fill:
`start.svg` (play), `stop.svg`, `restart.svg` (one round arrow),
`pause.svg`, `resume.svg` (bar and play), `trash.svg` (remove),
`shell.svg` (prompt), `pull.svg` (download to a tray), `prune.svg`
(broom), `plus.svg` (create), `more.svg` (three dots). The kit does not
bundle them; the app wires them (for example through `crate::icons`), the
gallery loads them with `include_bytes!`.

### Gallery

`data` group (both themes through the theme button): sortable table (Name,
Status, Size sort; pressing again flips the arrow; Compose links; "⋯" on
every row but the last; the last event is printed), toolbars 640, 330 and
230 wide (filter shrinks, then buttons move into "⋯"; Prune disabled),
the action icons as icon buttons with tooltips. The IconButton section
gained the danger tone and disabled buttons.

### Tests

`iced_test` and unit tests in the same files: header press sends its
column; a column that does not sort and a table without `on_sort` send
nothing; the arrow drawing changes with direction and column (hash
comparison); link press sends `(row, column)` and no row press; a press
beside a short link and a link without handler press the row; link
underline only under the pointer (hash); "⋯" sends index and corner and
no row press; after scrolling the corner is where the button shows; a row
without menu is pressed instead; "⋯" tooltip label (hash); toolbar `fit`
(widest field, shrinking field, buttons into the menu, below the smallest
field, no field); wide toolbar buttons send their messages; tooltip label
of a toolbar button and of `IconButton` (hash: iced tooltip overlays are
not reachable through widget queries); "⋯" shows the last items in order
and not the shown ones; an item press sends its message and closes; a
disabled item sends nothing; outside press, Escape and a second "⋯" press
close; menu position; danger and disabled icon colors and backgrounds.
Contrast, both themes: link (`accent`) reaches 4.5:1 on `surface-canvas`,
`surface-header` and `surface-selected`; sort arrow (`text-body`) reaches
3:1 on the same; danger icon reaches 3:1 on `surface-canvas`,
`surface-panel` and `surface-danger`.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 86.15% on
`b48c0da` → 86.42%.

### Deviations and choices

- Link message: `TableCell::link(label)` plus a table-level
  `on_link_press(row, column)` instead of `TableCell::link(label,
  message)`. A message inside the cell would make `TableCell` generic over
  the message type, and the app's `TableViewModel` (which derives `Eq`,
  while `DockerIntent` is only `PartialEq`) and `resource_rows` would have
  to change. Row and column indexes match the table's other messages
  (`on_row_press(index)`, `on_row_right_press(index, point)`), so ticket
  25 maps the row index to the container and its project the same way.
- The sort arrow sits at the end of the title cell (before the title in a
  right-aligned column, so the title still lines up with the numbers), so
  a long title never pushes it out.
- The row "⋯" brightens its dots under the pointer but has no hover
  background (the row already lights up), like the kit's cross over rows.
- The visual look was checked on throwaway `iced_test` snapshots with the
  bundled fonts in both themes, not in the running gallery window.

### For tickets 25–27

- 25: Compose column `TableCell::link(project)` (an empty `muted("")` for
  no project) and `.on_link_press(|row, _| DockerIntent::FilterCompose
  { index: row })` or similar; the reducer looks up the shown row.
- 26: columns `.sortable(true)`, `.sorted_by(column, direction)` from the
  kept sort, `.on_sort(DockerIntent::SortBy)`.
- 27: `Toolbar::new(i18n "More actions", palette).field(min, max, ||
  TextField::new("", vm.filter, palette).width(Length::Fill)...into())`
  and one `ToolbarItem` per action with the new icons;
  `.on_row_menu(i18n label, |index, position| DockerIntent::OpenRowMenu
  { index, position })` plus `.row_without_menu` for rows without
  actions; inspector actions as `IconButton::new(icon, label, palette)
  .tone(IconTone::Danger)` for Remove and `.on_press_maybe(..)` for
  enabled state.
