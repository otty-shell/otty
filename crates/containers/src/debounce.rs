//! Keeps list reloads apart: each list reloads at most once per gap, and a
//! change inside the gap still gets its reload when the gap is over.

use std::time::{Duration, Instant};

use crate::watch_event::ListKind;

/// Reload pacing for the four lists of one engine (spec Decision 14).
///
/// The first change of a list reloads it at once. Changes that come before
/// the gap since that reload has passed wait, and the list reloads once
/// more when the gap is over, so the table ends up showing the last state.
#[derive(Debug, Clone)]
pub(crate) struct ReloadDebounce {
    gap: Duration,
    lists: [ListPace; 4],
}

impl ReloadDebounce {
    /// No list reloaded yet; `gap` is the shortest time between two reloads
    /// of one list.
    pub(crate) fn new(gap: Duration) -> Self {
        Self {
            gap,
            lists: [ListPace::default(); 4],
        }
    }

    /// `list` changed at `now`. Returns `true` when it may reload at once
    /// (the reload counts from `now`); otherwise the reload waits for
    /// [`Self::take_due`].
    pub(crate) fn change(&mut self, list: ListKind, now: Instant) -> bool {
        let gap = self.gap;
        let pace = &mut self.lists[index_of(list)];

        let is_inside_gap = pace
            .last_reload
            .is_some_and(|last| now.saturating_duration_since(last) < gap);
        if is_inside_gap {
            pace.is_waiting = true;
            return false;
        }

        pace.last_reload = Some(now);
        pace.is_waiting = false;
        true
    }

    /// When the earliest waiting reload is due.
    pub(crate) fn next_due(&self) -> Option<Instant> {
        self.lists
            .iter()
            .filter(|pace| pace.is_waiting)
            .filter_map(|pace| pace.last_reload)
            .map(|last| last + self.gap)
            .min()
    }

    /// The waiting lists that are due at `now`, in [`ListKind::ALL`] order.
    /// Their reload counts from `now`.
    pub(crate) fn take_due(&mut self, now: Instant) -> Vec<ListKind> {
        let mut due = Vec::new();

        for (list, pace) in ListKind::ALL.into_iter().zip(&mut self.lists) {
            let is_due = pace.is_waiting
                && pace.last_reload.is_none_or(|last| last + self.gap <= now);
            if is_due {
                pace.last_reload = Some(now);
                pace.is_waiting = false;
                due.push(list);
            }
        }

        due
    }
}

/// When one list last reloaded and whether a change waits for the next
/// reload.
#[derive(Debug, Clone, Copy, Default)]
struct ListPace {
    last_reload: Option<Instant>,
    is_waiting: bool,
}

/// The place of `list` in [`ListKind::ALL`].
fn index_of(list: ListKind) -> usize {
    match list {
        ListKind::Containers => 0,
        ListKind::Images => 1,
        ListKind::Volumes => 2,
        ListKind::Networks => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAP: Duration = Duration::from_millis(500);

    fn after(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    #[test]
    fn the_first_change_reloads_at_once() {
        let start = Instant::now();
        let mut debounce = ReloadDebounce::new(GAP);

        assert!(debounce.change(ListKind::Containers, start));
        assert_eq!(debounce.next_due(), None);
    }

    #[test]
    fn a_change_inside_the_gap_waits_until_the_gap_is_over() {
        let start = Instant::now();
        let mut debounce = ReloadDebounce::new(GAP);
        debounce.change(ListKind::Containers, start);

        let reloads_now =
            debounce.change(ListKind::Containers, after(start, 100));

        assert!(!reloads_now);
        assert_eq!(debounce.next_due(), Some(after(start, 500)));
        assert!(debounce.take_due(after(start, 499)).is_empty());
        assert_eq!(
            debounce.take_due(after(start, 500)),
            vec![ListKind::Containers]
        );
        assert_eq!(debounce.next_due(), None);
    }

    #[test]
    fn many_changes_inside_the_gap_give_one_more_reload() {
        let start = Instant::now();
        let mut debounce = ReloadDebounce::new(GAP);
        debounce.change(ListKind::Images, start);

        for millis in [10, 20, 300, 450] {
            assert!(!debounce.change(ListKind::Images, after(start, millis)));
        }

        assert_eq!(
            debounce.take_due(after(start, 500)),
            vec![ListKind::Images]
        );
        assert!(debounce.take_due(after(start, 2000)).is_empty());
    }

    #[test]
    fn a_waiting_reload_starts_a_new_gap() {
        let start = Instant::now();
        let mut debounce = ReloadDebounce::new(GAP);
        debounce.change(ListKind::Volumes, start);
        debounce.change(ListKind::Volumes, after(start, 100));
        debounce.take_due(after(start, 520));

        let reloads_now = debounce.change(ListKind::Volumes, after(start, 900));

        assert!(!reloads_now);
        assert_eq!(debounce.next_due(), Some(after(start, 1020)));
    }

    #[test]
    fn a_change_after_the_gap_reloads_at_once() {
        let start = Instant::now();
        let mut debounce = ReloadDebounce::new(GAP);
        debounce.change(ListKind::Networks, start);

        assert!(debounce.change(ListKind::Networks, after(start, 500)));
        assert!(debounce.change(ListKind::Networks, after(start, 1200)));
        assert_eq!(debounce.next_due(), None);
    }

    #[test]
    fn each_list_has_its_own_gap() {
        let start = Instant::now();
        let mut debounce = ReloadDebounce::new(GAP);
        debounce.change(ListKind::Containers, start);

        assert!(debounce.change(ListKind::Images, after(start, 100)));
        assert!(debounce.change(ListKind::Volumes, after(start, 100)));
        assert!(!debounce.change(ListKind::Containers, after(start, 100)));
        assert!(!debounce.change(ListKind::Images, after(start, 300)));
        assert_eq!(debounce.next_due(), Some(after(start, 500)));
        assert_eq!(
            debounce.take_due(after(start, 500)),
            vec![ListKind::Containers]
        );
        assert_eq!(debounce.next_due(), Some(after(start, 600)));
        assert_eq!(
            debounce.take_due(after(start, 600)),
            vec![ListKind::Images]
        );
    }

    #[test]
    fn lists_due_together_come_in_list_order() {
        let start = Instant::now();
        let mut debounce = ReloadDebounce::new(GAP);
        for list in [ListKind::Networks, ListKind::Containers] {
            debounce.change(list, start);
            debounce.change(list, after(start, 10));
        }

        let due = debounce.take_due(after(start, 600));

        assert_eq!(due, vec![ListKind::Containers, ListKind::Networks]);
    }
}
