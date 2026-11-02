# 34 — Port links in the Containers table

Status: resolved
Type: task
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 31, 33
Spec: [../spec.md](../spec.md)

## What

Owner request (2026-11-02): the Ports column of the Containers table links
like the Overview does (ticket 31). Each published TCP port in a cell is its
own link to `http://localhost:<host port>` (or the bound host address, the
same rule as ticket 31's `port_url`); UDP and exposed-only ports stay text.
Pressing a port link opens the browser through the existing `OpenUrl` effect
and does not select the row or open the right panel.

- Kit `DataTable`: a cell can hold several links next to each other with
  text between them (today `TableCell::link(label)` holds one). Keep the
  table-level message style: extend `on_link_press` to tell which link in
  the cell was pressed (row, column, link index), and keep existing callers
  (the Compose column) working. Gallery update and `iced_test`.
- App: build the Ports cell from `format::shown_ports` / `port_text` and
  `overview::port_url`, so the table and the Overview share one rule.
- Sorting by Ports stays off (ticket 26).

Tests first. Run fmt, clippy, tests, `cargo deny check` and compare
`cargo llvm-cov` with the merge base before the commit.

## Done when

- Kit `iced_test`: two links in one cell each emit their own index; text
  between them emits nothing; a link press is not a row press.
- App tests: port cell parts for TCP, UDP, exposed-only and bound-address
  ports; pressing a port link emits `OpenUrl` with the right URL and leaves
  the selection and the right panel as they were.

## Answer

Tests were written first and seen failing: 3 kit tests (against a `Parts`
cell that drew plain text) and 7 app tests (against a Ports cell that was
one text and a reducer that ignored the new intent). Then the code.

### Kit (`otty-ui-kit`)

- New `CellPart` (`CellPart::text(..)`, `CellPart::link(..)`) and
  `TableCell::Parts(Vec<CellPart>)` / `TableCell::parts(..)`: links and text
  side by side on one line in the mono font (text muted, links `accent`,
  underlined under the pointer). Each link is its own button, so a press
  does not reach the row; the text between links presses the row. A part
  that does not fit is cut at the cell edge.
- `DataTable::on_link_press` now takes `Fn(row, column, link)`; `link` is
  the position among the links of the cell (0 for `TableCell::Link`). The
  Compose caller uses `|row, _, _|`. Without a handler links are drawn
  but a press selects the row.
- Gallery: the table section has a Ports column with two links and text
  between and after them; the event line shows the link index.

### App

- `overview::ports_value` is public to the crate; `resource_rows::ports_cell`
  builds the Ports cell from it (so `shown_ports`, `port_text` and
  `port_url` stay the one rule) and `port_urls` lists the URLs of its
  links. A cell without any link stays one muted text.
- New intent `DockerIntent::OpenPortLink { row, link }`; the reducer
  finds the URL with `DockerState::port_url_of_row` (shown, filtered and
  sorted containers, like the Compose link) and sends `OpenUrl`. It does
  not select the row, so the events layer does not open the right panel.
  `main_area` maps column `PORTS_COLUMN` to it.
- `otty_docker::fixtures::container_with_ports` builds a container with
  given ports for tests (the port type has no public constructor).
- No new text, so no i18n change.

Coverage (lines): 87.15% (66893 lines, 8597 missed) -> 87.21% (67204
lines, 8597 missed). fmt, clippy, workspace tests and `cargo deny check`
pass.

### Deviations and notes

- A Parts cell that is too long is cut at the cell edge without "…" (a
  single-text Ports cell still ends with "…"); the kit has no one-line
  ellipsis across several parts.
- An all-text Ports cell (UDP or exposed only) stays a `muted_mono` cell.
