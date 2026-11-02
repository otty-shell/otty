# 37 — Inspect: Copy JSON as an icon, links in the tree

Status: ready-for-agent
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
