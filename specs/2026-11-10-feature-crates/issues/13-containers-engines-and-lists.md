# 13 Containers: engines and lists

Status: needs-triage
Blocked by: 04, 12

## What to do

Create `otty-app-containers` and move the first half of
`app/src/widgets/containers`, following "Moving a feature" in the spec. It adds
the `StatusBar` trait to `otty-app-slots`.

This issue covers:

- the engine tree in the left panel;
- the main area: the open list with sorting and table search, the searching
  state and the no-engine cards;
- the status bar;
- the Docker calls in `effects/containers.rs` (finding engines, probing, lists)
  and the engine events stream in the binary's `subscription.rs`.

The crate uses `otty-containers` with `default-features = false` (issue 04), so
it has the types and cannot reach the client.

The child modules of the old widget (`actions`, `create`, `inspector`, `run`)
come in issue 14. Inside the crate they are plain modules with no layout rules.

## Done when

- Containers mode shows the engines, the lists of containers, images, volumes
  and networks, sorting, search and the no-engine cards as in the old app, and
  lists update on engine events.
- `otty-app-containers` has no `bollard` and no `tokio` in its dependency tree.
- The steps and checks in "Moving a feature" are done.

## Comments
