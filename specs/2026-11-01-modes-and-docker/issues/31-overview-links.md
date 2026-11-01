# 31 — Overview links: ports to localhost, image/network/volume jump, compose filter

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 28
Spec: [../spec.md](../spec.md)

## What

Spec Decision 20 (links). Tests first.

- Kit `KeyValueList`: a value can be a link (accent color, hover underline)
  with a message; several links in one value (for example two ports) are
  separate links.
- Container Overview: each published TCP port links to
  `http://localhost:<host port>` (opened with the existing `open_url`
  effect; a port bound to a specific host IP other than 0.0.0.0, :: or
  127.0.0.1 uses that IP); the image links to the Images list with that
  image selected; each network to the Networks list; each named volume in
  mounts to the Volumes list; the compose project opens the compose filter.
- Image, volume and network Overviews link back the same way where a value
  names another item (for example "Used by" containers).
- A value that names nothing that exists stays text.

All new text goes through `i18n` in the ten locales (spec Decision 17). Any missing visual piece goes into `otty-ui-kit` first with a gallery section and an `iced_test`. Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Pure tests: URL building for each host IP case; link targets resolved by
  id/name, missing targets stay text.
- Reducer tests: a link opens the list and selects the item (selection kept
  through the sort and filter rules).
- Kit and app `iced_test`: pressing a link emits its message.
