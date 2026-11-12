//! Keeps only the newest terminal snapshot until a consumer takes it.
//!
//! The engine can produce snapshots much faster than a front-end draws
//! them. Queueing every snapshot would hold one full screen copy per PTY
//! wake, so the engine publishes into this single slot instead and the
//! consumer always gets the newest picture.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::surface::SnapshotOwned;

/// Shared slot holding the newest snapshot that no consumer has taken yet.
#[derive(Clone, Default)]
pub(crate) struct LatestSnapshot {
    slot: Arc<Mutex<Option<SnapshotOwned>>>,
}

impl LatestSnapshot {
    /// Make `snapshot` the newest one, dropping an older one nobody took.
    ///
    /// Each snapshot reports only the lines changed since the snapshot
    /// before it. When that earlier snapshot is dropped untaken, its changed
    /// lines would be lost, so the snapshot replacing it reports the whole
    /// surface as changed.
    ///
    /// Returns `true` when no snapshot was waiting, which means the consumer
    /// has to be told that one is waiting now.
    pub(crate) fn publish(&self, mut snapshot: SnapshotOwned) -> bool {
        let mut slot = self.lock();

        let replaces_untaken_snapshot = slot.is_some();
        if replaces_untaken_snapshot {
            snapshot.mark_fully_damaged();
        }
        *slot = Some(snapshot);

        !replaces_untaken_snapshot
    }

    /// Take the waiting snapshot out of the slot, if there is one. The caller
    /// owns it from then on.
    pub(crate) fn take(&self) -> Option<SnapshotOwned> {
        self.lock().take()
    }

    fn lock(&self) -> MutexGuard<'_, Option<SnapshotOwned>> {
        // The slot holds a plain value, so it stays usable even if another
        // thread panicked while holding the lock.
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl fmt::Debug for LatestSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LatestSnapshot")
            .field("has_snapshot", &self.lock().is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TerminalSize;
    use crate::surface::{
        SnapshotDamage, Surface, SurfaceActor, SurfaceConfig, SurfaceModel,
    };

    /// A snapshot that shows `ch` and reports only the changed line as damaged.
    fn snapshot_showing(ch: char) -> SnapshotOwned {
        let mut surface =
            Surface::new(SurfaceConfig::default(), &TerminalSize::default());
        let _ = surface.snapshot_owned();
        surface.reset_damage();
        surface.print(ch);

        surface.snapshot_owned()
    }

    fn shown_char(snapshot: &SnapshotOwned) -> char {
        snapshot.view().cells[0].cell.c
    }

    #[test]
    fn publish_asks_for_an_announcement_only_when_no_snapshot_was_waiting() {
        let latest = LatestSnapshot::default();

        assert!(latest.publish(snapshot_showing('a')));
        assert!(!latest.publish(snapshot_showing('b')));
    }

    #[test]
    fn take_returns_the_newest_published_snapshot_once() {
        let latest = LatestSnapshot::default();
        latest.publish(snapshot_showing('a'));
        latest.publish(snapshot_showing('b'));

        let taken = latest.take().expect("a snapshot is waiting");

        assert_eq!(shown_char(&taken), 'b');
        assert!(latest.take().is_none());
    }

    #[test]
    fn publish_after_take_asks_for_an_announcement_again() {
        let latest = LatestSnapshot::default();
        latest.publish(snapshot_showing('a'));
        latest.take();

        assert!(latest.publish(snapshot_showing('b')));
    }

    #[test]
    fn snapshot_published_after_a_take_keeps_its_own_damage() {
        let latest = LatestSnapshot::default();
        latest.publish(snapshot_showing('a'));
        latest.take();

        latest.publish(snapshot_showing('b'));

        let taken = latest.take().expect("a snapshot is waiting");
        assert!(matches!(taken.view().damage, SnapshotDamage::Partial(_)));
    }

    #[test]
    fn snapshot_that_replaces_an_untaken_one_reports_full_damage() {
        let latest = LatestSnapshot::default();
        latest.publish(snapshot_showing('a'));

        // Nobody took the snapshot with 'a', so the snapshot with 'b' has
        // to cover the lines that only the dropped snapshot reported.
        latest.publish(snapshot_showing('b'));

        let taken = latest.take().expect("a snapshot is waiting");
        assert_eq!(shown_char(&taken), 'b');
        assert_eq!(taken.view().damage, &SnapshotDamage::Full);
    }

    #[test]
    fn clones_share_one_slot() {
        let latest = LatestSnapshot::default();
        let other = latest.clone();

        latest.publish(snapshot_showing('a'));

        assert!(other.take().is_some());
        assert!(latest.take().is_none());
    }

    #[test]
    fn slot_stays_usable_after_a_panic_while_locked() {
        let latest = LatestSnapshot::default();
        let other = latest.clone();

        let panicked = std::thread::spawn(move || {
            let _guard = other.lock();
            panic!("poison the slot lock");
        })
        .join();

        assert!(panicked.is_err());
        assert!(latest.publish(snapshot_showing('a')));
        assert!(latest.take().is_some());
    }

    #[test]
    fn debug_output_tells_whether_a_snapshot_is_waiting() {
        let latest = LatestSnapshot::default();
        assert_eq!(
            format!("{latest:?}"),
            "LatestSnapshot { has_snapshot: false }"
        );

        latest.publish(snapshot_showing('a'));
        assert_eq!(
            format!("{latest:?}"),
            "LatestSnapshot { has_snapshot: true }"
        );
    }
}
