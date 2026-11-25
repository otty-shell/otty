//! The list prices of the money at API prices as Agents mode read them (spec
//! Decision 16): the LiteLLM price table, fetched at most once a day, or
//! its kept copy. One read serves every profile.

use std::time::SystemTime;

use otty_agents::{Answer, FetchedPrices, PriceTable, SourceFailure, SourceId};

use super::freshness::{ReadState, Which};

/// What the walk over the price sources gave.
type PriceAnswer = Result<Answer<FetchedPrices>, Vec<SourceFailure>>;

/// The prices: the answer of the latest read, the prices of an earlier
/// read kept while the latest one gave none, and the bookkeeping of the
/// reads. The table is kept once: in the latest answer when it gave
/// prices, else in the earlier ones.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ListPrices {
    latest: Option<PriceAnswer>,
    earlier: Option<FetchedPrices>,
    reads: ReadState,
}

impl ListPrices {
    /// The prices of the last read that gave some; `None` while no read
    /// gave any.
    pub(crate) fn table(&self) -> Option<&PriceTable> {
        self.last_good().map(FetchedPrices::table)
    }

    /// The last prices a read gave, with when their file was fetched.
    pub(crate) fn last_good(&self) -> Option<&FetchedPrices> {
        match &self.latest {
            Some(Ok(answer)) => Some(answer.value()),
            _ => self.earlier.as_ref(),
        }
    }

    /// What the latest read gave: the source that answered with the
    /// failures before it, or the failure of each source.
    pub(crate) fn latest(&self) -> Option<&PriceAnswer> {
        self.latest.as_ref()
    }

    /// Whether the prices need a read for `which` at `now`. Prices fetched
    /// from LiteLLM less than a day ago are never read again; otherwise
    /// the read follows `which` like any other kind.
    pub(crate) fn is_wanted(&self, which: Which, now: SystemTime) -> bool {
        let has_fresh_fetch = match self.latest() {
            Some(Ok(answer)) => {
                answer.source() == SourceId::LiteLlmPrices
                    && answer.value().is_fresh_at(now)
            },
            _ => false,
        };

        !has_fresh_fetch && self.reads.is_wanted(which)
    }

    /// Whether the first read of the prices is on its way: none answered
    /// yet. The first count of the tokens waits for it, so a start reads
    /// the session files once (issue 11 of spec
    /// 2026-11-23-accounts-apart-from-profiles).
    pub(crate) fn is_first_read_on_its_way(&self) -> bool {
        self.latest.is_none() && self.reads.is_reading()
    }

    /// Whether the next read must fetch the file even within the day: the
    /// fetch of the latest read failed. Before the first read the kept
    /// file of the day is used.
    pub(crate) fn fetch_again(&self) -> bool {
        let failures = match self.latest() {
            Some(Ok(answer)) => answer.earlier_failures(),
            Some(Err(failures)) => failures.as_slice(),
            None => return false,
        };

        failures
            .iter()
            .any(|failure| failure.source() == SourceId::LiteLlmPrices)
    }

    /// Start a read; `false` when one is on its way.
    pub(crate) fn start_reading(&mut self) -> bool {
        self.reads.start()
    }

    /// Take the answer of a read that ended at `read_at`. Prices a read
    /// gave stay until another read gives new ones.
    pub(crate) fn apply(&mut self, answer: PriceAnswer, read_at: SystemTime) {
        self.reads.end_at(read_at);

        if answer.is_ok() {
            self.earlier = None;
        } else if let Some(Ok(previous)) = self.latest.take() {
            self.earlier = Some(previous.into_value());
        }
        self.latest = Some(answer);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_agents::{SourceError, Unavailable};

    use super::*;
    use crate::freshness::READ_PERIOD;
    use crate::testing::{at, fetched_prices, prices_answer};

    const DAY: Duration = Duration::from_secs(24 * 3600);

    fn read(answer: PriceAnswer, read_at: SystemTime) -> ListPrices {
        let mut prices = ListPrices::default();
        prices.apply(answer, read_at);
        prices
    }

    /// Whether the latest read of `prices` ended with no prices from any
    /// source.
    fn has_failed(prices: &ListPrices) -> bool {
        matches!(prices.latest(), Some(Err(_)))
    }

    fn network() -> SourceError {
        SourceError::Network(String::from("down"))
    }

    #[test]
    fn before_the_first_read_the_prices_are_wanted_with_no_fetch_forced() {
        let prices = ListPrices::default();

        assert!(prices.is_wanted(Which::Every, at(10)));
        assert!(!prices.fetch_again());
        assert_eq!(prices.table(), None);
        assert!(!has_failed(&prices));
    }

    #[test]
    fn only_the_read_before_any_answer_is_the_first_read() {
        let mut prices = ListPrices::default();
        let before = prices.is_first_read_on_its_way();

        let _ = prices.start_reading();
        let first = prices.is_first_read_on_its_way();
        prices.apply(
            prices_answer(&[(SourceId::LiteLlmPrices, network())], None),
            at(10),
        );
        let _ = prices.start_reading();
        let second = prices.is_first_read_on_its_way();

        assert!(!before, "no read is on its way");
        assert!(first);
        assert!(!second, "a read answered, even with no prices");
    }

    #[test]
    fn a_good_fetch_is_not_read_again_within_the_day() {
        let fetched_at = at(1_000);
        let prices = read(
            prices_answer(&[], Some(fetched_prices(fetched_at))),
            fetched_at,
        );

        assert!(!prices.is_wanted(Which::Every, fetched_at + DAY / 2));
        assert!(prices.is_wanted(Which::Every, fetched_at + DAY));
        assert!(!prices.fetch_again());
        assert!(prices.table().is_some());
    }

    #[test]
    fn after_a_failed_fetch_the_next_read_fetches_again() {
        let kept = read(
            prices_answer(
                &[(SourceId::LiteLlmPrices, network())],
                Some(fetched_prices(at(10))),
            ),
            at(1_000),
        );
        let none = read(
            prices_answer(
                &[
                    (SourceId::LiteLlmPrices, network()),
                    (
                        SourceId::KeptPrices,
                        SourceError::NotAvailable(Unavailable::NoKeptPriceFile),
                    ),
                ],
                None,
            ),
            at(1_000),
        );

        for prices in [&kept, &none] {
            assert!(prices.is_wanted(Which::Every, at(1_001)));
            assert!(prices.fetch_again());
        }
        assert!(kept.table().is_some(), "the kept file gave prices");
        assert!(!has_failed(&kept));
        assert!(has_failed(&none));
        assert_eq!(none.table(), None);
    }

    #[test]
    fn a_failed_read_is_read_again_on_open_only_after_the_period() {
        let failed_at = at(10_000);
        let prices = read(
            prices_answer(&[(SourceId::LiteLlmPrices, network())], None),
            failed_at,
        );

        assert!(!prices.is_wanted(Which::Stale(failed_at), failed_at));
        assert!(prices.is_wanted(
            Which::Stale(failed_at + READ_PERIOD),
            failed_at + READ_PERIOD
        ));
        assert!(!prices.is_wanted(Which::WaitingForPrograms, failed_at));
    }

    #[test]
    fn the_last_prices_stay_when_later_reads_give_none() {
        let mut prices =
            read(prices_answer(&[], Some(fetched_prices(at(10)))), at(10));
        let failed =
            || prices_answer(&[(SourceId::LiteLlmPrices, network())], None);

        prices.apply(failed(), at(20));
        let after_one = prices.last_good().cloned();
        prices.apply(failed(), at(30));

        assert_eq!(after_one, Some(fetched_prices(at(10))));
        assert_eq!(prices.last_good(), Some(&fetched_prices(at(10))));
        assert!(has_failed(&prices));
    }

    #[test]
    fn new_prices_take_the_place_of_the_kept_ones() {
        let mut prices =
            read(prices_answer(&[], Some(fetched_prices(at(10)))), at(10));

        prices.apply(
            prices_answer(&[(SourceId::LiteLlmPrices, network())], None),
            at(20),
        );
        prices.apply(prices_answer(&[], Some(fetched_prices(at(30)))), at(30));

        assert_eq!(prices.last_good(), Some(&fetched_prices(at(30))));
        assert!(!has_failed(&prices));
    }
}
