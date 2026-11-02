# 34 — Port links in the Containers table

Status: ready-for-agent
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
