# Phase 6: latest-frame transport

Status: **partially complete**.

Parent document: [Blocks v2 specification](spec.md). IDs: B2-060–B2-066.

## Goal

Separate replaceable visual state from lossless terminal events. A slow UI must receive the
latest available frame revision without accumulating full-snapshot backlog, while child exit,
errors, and other critical events remain lossless.

## Current state

A latest-frame mailbox with capacity one, a frame-replacement counter, a separate `FrameReady`
notification, and bounded default capacities for event and request queues are implemented.
Tests cover slow, full, and disconnected channel cases.

The phase remains incomplete because PTY reads are not coalesced into render ticks, model and
viewport revisions are absent, stale-coordinate handling and partial damage are not
implemented, and a complete burst test with an artificially slow consumer and guaranteed
lossless child exit is still required.

## Scope

- [ ] **B2-060** Extend slow-consumer tests to burst PTY output plus lossless child exit;
  current mailbox and channel tests cover only part of this item.
- [x] **B2-061** Separate replaceable frame notification from lossless terminal events.
- [x] **B2-062** Implement a latest-frame mailbox with capacity one and no new dependency.
- [x] **B2-063** Use bounded default queues with explicit full and disconnected semantics.
- [ ] **B2-064** Coalesce PTY reads into render ticks; resize, scroll, and selection must be
  able to request an immediate render.
- [ ] **B2-065** Carry `model_revision` and `viewport_revision`; reject or resolve stale
  coordinate requests again through stable positions.
- [ ] **B2-066** Implement partial damage for output and full damage for resize, reset, and
  alternate-screen transitions.

## Transport invariants

- At most one unread replaceable frame is retained in memory.
- Frame replacement does not block the PTY reader and increments a safe counter.
- A lossless event is never hidden by frame notification and explicitly handles a full queue.
- Presented revision never moves backward.
- A coordinate request either targets a declared revision or resolves through a stable ID.
- Shutdown or disconnect never leaves a producer in an infinite retry loop.

## Automated verification

```bash
cargo test -p otty-libterm terminal::channel
cargo test -p otty-libterm --all-features
cargo test -p otty-ui-term --all-features
```

Add a deterministic test with a paused or slow consumer: the producer sends burst frames and a
child exit, backlog remains at most one frame, and the consumer receives both the latest
revision and the child exit.

## Manual verification

1. Launch `cargo run -p otty` and run `yes transport | head -n 1000000`.
2. Resize, switch terminal tabs, and scroll history during output. The UI must remain
   responsive and PTY output must finish without hanging.
3. Artificially slow the UI consumer through a test or debug option. Diagnostics must show a
   replaceable backlog of at most one, increasing `replaced_frames`, and bounded lossless
   depth.
4. Start a short-lived child during burst output. Its exit event must arrive even if several
   frames are replaced.
5. Send a scroll or selection request for an old revision. The backend must explicitly reject
   it or resolve it again through stable `BlockPoint`, never apply it to different text.
6. Verify resize, reset, and alternate-screen transitions produce full damage while ordinary
   append output reports only the changed region.
7. Close a terminal tab during the burst. Processes and channels must finish without panic,
   deadlock, or an infinite CPU loop.

The phase is complete when one million lines create no frame backlog, the latest revision
reaches the UI, and lossless lifecycle events are demonstrably delivered.
