# 31 — Overview links: ports to localhost, image/network/volume jump, compose filter

Status: resolved
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

## Answer

Tests were written first and seen failing: 10 kit tests (link presses,
link layout, hover underline, and the span split, against a
`WrappedText` that ignored links) and 14 app tests (link targets and URLs
against a stub that always gave text, and the reducer against a
`FollowLink` that did nothing). Then the code.

### Kit (`otty-ui-kit`)

- `KeyValueList<Message>` is now generic over the message.
  `.entry(key, value)` works as before; new
  `.entry_parts(key, parts)` takes `ValuePart::text(..)` and
  `ValuePart::link(text, message)` pieces in order. Each link is pressed on
  its own, so two ports in one value are two links.
- Links use `accent` (the table's link color) and get a 1px underline in
  that color while the pointer is over them; the pointer becomes a hand.
  Only a press and release on the same link sends its message; the text
  around a link does nothing.
- The kit-private `WrappedText` now draws spans (iced's paragraph spans,
  hit-tested with `hit_span`). The value still wraps by the ticket 28 rules:
  the joined text is wrapped as before, and the pure `split_wrapped` maps
  the inserted line breaks back onto the parts, so a link can break onto
  the next line and stays one link (both halves underline together).
- `iced_test` queries see each link's text at its place (first line of the
  link) and then the whole value, so `ui.click("8080:80/tcp")` presses
  that link and `ui.find(whole value)` still works.
- Gallery, inspector section: Image, two port links plus a UDP port as
  text, and a volume link followed by text; the last pressed link is
  printed under the list.

### App (`app/src/widgets/docker/`)

- `overview.rs`: a field value is `Vec<Piece>` (`Piece::Text` or
  `Piece::Link(text, LinkTarget)`). `LinkTarget` is `Url(..)`,
  `Item(ResourceKind, id)` (container, image or network id, volume name)
  or `ComposeProject(..)`. Links are resolved against the engine's loaded
  lists; a value that names nothing listed stays text:
  - container: Image (by image id), each published TCP port, each network
    (by id, or by name when the engine sent no id, e.g. a container that
    never ran; Podman's `pasta` stays text), each named volume in Mounts
    (the name is the link, " → /data" stays text), Compose project;
  - image: each "Used by" container;
  - volume: each "Used by" container (by name), Compose project only when
    some container belongs to it;
  - network: each connected container (by name).
- `port_url(host_ip, host_port, protocol)`: `None` for an exposed-only or
  non-TCP port; `http://localhost:<port>` for `0.0.0.0`, `::`,
  `127.0.0.1` or no address; otherwise the bound address
  (`http://192.168.1.20:8080`, IPv6 in brackets: `http://[fd00::5]:8443`).
  With Docker's two mappings (`0.0.0.0` and `::`) the first one is used.
- `format.rs`: `shown_ports` (the dedup that `ports_text` did) and
  `port_text` are split out, so the table text and the Overview pieces
  come from one place.
- New intent `DockerIntent::FollowLink(LinkTarget)`; the reducer's
  `follow_link` sends the existing `OpenUrl` effect for a port, opens the
  compose filter for a project (`DockerState::open_compose_project`, now
  also used by the Compose column), and for an item calls
  `DockerState::show_item(engine, resource, id)`: the whole list (no name
  filter, no compose filter), the item selected by id, the Overview tab.
  `show_item` replaces `show_new_container` (the Run form uses it with
  Containers). The table keeps its sort; the selection is the id, so a
  later sort keeps it.
- `view/inspector.rs` maps pieces to `ValuePart`s; a link sends
  `FollowLink`.
- No new text, so no i18n change (`Key::ALL` unchanged).

### Tests

- Kit: two links in one value each send their own message; links sit side
  by side and are as wide as their text; text beside a link and the key
  send nothing; a long linked value wraps inside 200 px and both links
  still work; underline only under the pointer (hash); `accent` reaches
  4.5:1 on `surface-panel` and `surface-canvas` in both themes;
  `split_wrapped` (no break, break inside a link, break at a dropped space,
  break between parts, empty parts and kept new lines).
- App, pure: `port_url` for no address, `0.0.0.0`, `::`, `127.0.0.1`, an
  IPv4 and an IPv6 address, exposed-only, UDP and SCTP; container links
  (image, port, network, project), the volume link in mounts, a network
  without id linked by name, values that name nothing listed stay text,
  Podman `pasta` stays text, image / volume / network container links,
  unlisted volume users stay text, a project without containers stays
  text. The text-only checks of the old tests are kept.
- App, reducer: an image link opens Images with the image selected, the
  name filter cleared, the user's sort kept, the row shown, the Overview
  tab open, and a later sort keeps it; a container link from a compose
  filter opens every container with that one selected; volume and network
  links; a port link sends `OpenUrl` and keeps the list; a project link
  opens the compose filter; links without an engine do nothing.
- App, `iced_test`: pressing port, image and network links in the
  inspector sends their `FollowLink` in both themes; the label sends
  nothing. (Written with the view mapping, so it passed at once.)

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 86.84%
(65774 lines, 8653 missed) on `2caef66` -> 87.08% (66788 lines, 8629
missed). `cargo +nightly fmt`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, the workspace tests and `cargo deny check`
pass (deny shows only the yanked warnings that were there before).

### Deviations and notes

- A compose link from a volume keeps the containers list's selection by
  id, as the Compose column does; if that container is not in the project
  the inspector shows it while the table does not (unchanged behavior).
- Links are resolved when the Overview is drawn; a list reload between the
  draw and the press can leave the selection on an id that is gone, which
  the next reload replaces with the first row as before.
- A throwaway Dark snapshot of a container Overview with links (not
  committed) was checked: accent links, underline under the pointer, a
  port link wrapping after its `:`.
