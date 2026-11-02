# 37 — Inspect: Copy JSON as an icon, links in the tree

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 32
Spec: [../spec.md](../spec.md)

## What

Owner request (2026-11-02), spec Decision 20.

- **Copy JSON is an icon button** with a tooltip, using a new original `copy`
  icon in `assets/svg/` in the style of the others. It sits on the same row as
  the Raw JSON and Show empty switches.
- **Links in the Inspect tree, wherever a value names something OTTY can
  show.** They behave exactly like the Overview links (ticket 31: same
  `LinkTarget`, same `FollowLink`, same check against the loaded lists, and a
  value that names nothing stays text):
  - Container: `Image` (the image id) and `Config.Image` → Images;
    each key under `NetworkSettings.Networks` and `HostConfig.NetworkMode`
    → Networks; `Mounts[].Name` → Volumes; the
    `com.docker.compose.project` label under `Config.Labels` → the compose
    filter; each published TCP port under `NetworkSettings.Ports`
    (`HostPort`) → `http://localhost:<port>`, by the same `port_url` rule.
  - Image: `Id` and `RepoTags[]` of the shown image stay text (they point at
    itself); any container reference, if present, → Containers.
  - Volume: the compose label → the compose filter.
  - Network: each container under `Containers` (`Name`) → Containers.
- The kit `DataRow` value can hold links, reusing the `ValuePart` of
  `KeyValueList`, so wrapping and the narrow layout stay the same.
- Raw JSON stays plain text.

Tests first. Run fmt, clippy, tests, `cargo deny check` and compare
`cargo llvm-cov` with the merge base before the commit.

## Done when

- Pure tests: for each listed path, a value that exists in the loaded lists
  becomes a link with the right target; one that does not stays text.
- Reducer test: a tree link goes through the same `FollowLink` path.
- `iced_test`: pressing a tree link emits `FollowLink`; the copy icon button
  emits the copy intent and shows its tooltip; a kit `DataTree` row with two
  links emits each one's message.

## Answer

Copy JSON is an icon button with a tooltip, and the Inspect tree has links.
Tests were written first and seen failing against stubs: 7 kit tests, then 23
app tests (a stub gave no link targets, the view sent the wrong intent and kept
the text button). Then the code. Raw JSON is unchanged, plain text.

### Kit (`otty-ui-kit`)

- `DataRow<Message>` is now generic over the message.
  - `DataRow::new` and `DataRow::heading` work as before.
  - New `DataRow::with_parts(depth, key, parts)` takes the `ValuePart`s of
    `KeyValueList`. It goes through the same `entry_view`, so wrapping and the
    narrow layout are the same code.
  - New `DataRow::heading_link(depth, key, on_press)`: the key of a heading is
    a link (accent, underline under the pointer, wraps). This one is needed
    because the name of a container's network is a JSON key with rows under it,
    not a value.
  - New `DataRow::map(change)` and `ValuePart::map(change)` change the message,
    so the app builds rows that carry a `LinkTarget` and the view turns them
    into `DockerIntent::FollowLink`.
- `ValuePart` derives `Debug`, `Clone`, `PartialEq` and `Eq`.
- Gallery, inspector part: the tree has links in General, two links in one
  value, a heading link and a link in Mounts; the last press is printed. The
  data part shows the new icon among the action icons.

### Icon

`assets/svg/copy.svg`, an original drawing in the style of the others (24×24,
`stroke-width="1.8"`, round caps and joins, no fill): one sheet over another.
The app has it as `icons::COPY`.

### App (`app/src/widgets/docker/`)

- `links.rs` (new) holds the link rules both tabs use: `LinkTarget` and
  `port_url` moved here from `overview.rs`, and the checks against the lists
  are one function each: `image_target`, `volume_target`, `network_target`,
  `named_container_target`, `container_id_target`, `compose_target`. The
  Overview now calls these. The only new rule is that `image_target` also
  matches a tag, which `Config.Image` needs; the Overview passes an id, so it
  behaves as before.
- `inspect_links.rs` (new): `InspectLinks` holds the kind of the shown item and
  the engine's lists. It says which paths of the JSON name something, and for
  which kind, by matching the path of keys (`["Mounts", _, "Name"]`).
- `inspect_tree(&Value, show_empty, &InspectLinks)` stays pure and returns rows
  of `DataRow<LinkTarget>`. Its walk now keeps the path. `first_section_title`
  is new: the reducer's choice of the first open section needs no links.
- `otty-docker` exports `COMPOSE_PROJECT_LABEL`, so the label has one
  definition.
- No new text: the tooltip is the existing "Copy JSON" key (`Key::ALL` is
  unchanged).

### What links

- Container:
  - `Image` (by id) and `Config.Image` (by id or tag) go to Images.
  - Each key under `NetworkSettings.Networks` (by `NetworkID`, or by name when
    the id is missing or empty, the Overview's rule) and `HostConfig.NetworkMode`
    (by name) go to Networks.
  - `Mounts[].Name` goes to Volumes.
  - `Config.Labels."com.docker.compose.project"` goes to the compose filter.
  - `NetworkSettings.Ports.<port>/tcp[].HostPort` goes to `port_url` with the
    `HostIp` beside it.
- Image: `Container` (by id) goes to Containers. `Id`, `RepoTags` and the
  image's own `Config.Image` stay text.
- Volume: `Labels."com.docker.compose.project"` goes to the compose filter.
- Network: `Containers.<id>.Name` goes to Containers.
- Anything else stays text, and so does a value that names nothing listed, a
  value that is not text, the same key elsewhere in the answer, and a path of
  another kind of item.

### Controls

The switches and the icon share one row: the switches take the room and wrap
when narrow, and the 24 px icon stays at the end of the first row. Each switch
sits in a 24 px tall box so it lines up with the icon on every row.

### Tests

- Kit (8): two links in one value, links in the narrow layout, a long linked
  value wraps, text and keys send nothing, a heading link sends its message,
  its indent, its color and hover underline (picture hash), `map`.
- Pure: 18 for the paths in `inspect_links.rs` (each listed path both ways, the
  kinds, the same key elsewhere, non-text values, Show empty, empty lists), 5
  new in `links.rs` (the 3 `port_url` tests moved there), 1 for
  `first_section_title`.
- View model (2): links point into the engine's lists and follow the kind of
  the selected row.
- Reducer (3): a tree link goes through `FollowLink` for an item, a compose
  label and a port.
- `iced_test` (4): pressing tree links sends `FollowLink` in both themes; the
  icon sends the copy intent on the row of the switches at 260 and 420 px; the
  "Copy JSON" text is only a tooltip; the tooltip shows that text (picture
  hash against a button with the same label and with another one). The older
  tests that pressed the text button now press the icon.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 87.59% (68976
lines, 8559 missed) on `eb036b5` -> 87.78% (70240 lines, 8585 missed).
`cargo +nightly fmt`, `cargo clippy --workspace --all-targets --all-features --
-D warnings`, the workspace tests and `cargo deny check` pass.

### Deviations and limits

- The kit gained `heading_link`, which the ticket did not name; see above.
- Docker image inspect goes through bollard's typed answer, which has no
  `Container` field. The image rule works and is tested on JSON, but it will
  not fire until the app passes the engine's raw answer.
- `Config.Image` is matched by what it says. After a tag moves to a newer image
  it opens the image with that tag, while the Overview's Image opens the one
  the container runs (by id). If the tag is gone it stays text.
- With Show empty on, a network whose value is exactly `{}` reads as a key and
  value row, and its key stays text. The engines send fields there, so this
  should not come up.
- Podman's `ImageName` is not linked: the ticket does not list it.
- Not run against a live engine. A throwaway Dark `iced_test` snapshot at 340
  and 260 px with the bundled fonts was checked (not committed).
