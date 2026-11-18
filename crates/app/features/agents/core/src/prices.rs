//! The list prices of the money estimate as Agents mode read them (spec
//! Decision 16): the LiteLLM price table, fetched at most once a day, or
//! its kept copy. One read serves every API key profile.

use std::time::SystemTime;

use otty_agents::{Answer, FetchedPrices, PriceTable, SourceFailure, SourceId};

use super::freshness::{self, Which};

/// What the walk over the price sources gave.
type PriceAnswer = Result<Answer<FetchedPrices>, Vec<SourceFailure>>;

/// The prices: the answer of the latest read and when it ended, the last
/// prices a read gave, and whether a read is on its way.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ListPrices {
    latest: Option<(PriceAnswer, SystemTime)>,
    last_good: Option<FetchedPrices>,
    is_reading: bool,
}

impl ListPrices {
    /// The prices of the last read that gave some; `None` while no read
    /// gave any.
    pub(crate) fn table(&self) -> Option<&PriceTable> {
        self.last_good.as_ref().map(FetchedPrices::table)
    }

    /// The last prices a read gave, with when their file was fetched.
    pub(crate) fn last_good(&self) -> Option<&FetchedPrices> {
        self.last_good.as_ref()
    }

    /// What the latest read gave: the source that answered with the
    /// failures before it, or the failure of each source.
    pub(crate) fn latest(&self) -> Option<&PriceAnswer> {
        self.latest.as_ref().map(|(answer, _)| answer)
    }

    /// Whether a read is on its way.
    pub(crate) fn is_reading(&self) -> bool {
        self.is_reading
    }

    /// Whether the latest read ended with no prices from any source.
    pub(crate) fn has_failed(&self) -> bool {
        matches!(self.latest, Some((Err(_), _)))
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
        let read_at = self.latest.as_ref().map(|(_, read_at)| *read_at);

        !has_fresh_fetch && freshness::is_wanted(read_at, which)
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
        if self.is_reading {
            return false;
        }

        self.is_reading = true;
        true
    }

    /// Take the answer of a read that ended at `read_at`. Prices a read
    /// gave stay until another read gives new ones.
    pub(crate) fn apply(&mut self, answer: PriceAnswer, read_at: SystemTime) {
        self.is_reading = false;
        if let Ok(answer) = &answer {
            self.last_good = Some(answer.value().clone());
        }
        self.latest = Some((answer, read_at));
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

    fn network() -> SourceError {
        SourceError::Network(String::from("down"))
    }

    #[test]
    fn a_read_starts_once_until_it_ends() {
        let mut prices = ListPrices::default();

        assert!(prices.start_reading());
        assert!(!prices.start_reading());
        assert!(prices.is_reading());

        prices.apply(prices_answer(&[], Some(fetched_prices(at(10)))), at(10));

        assert!(!prices.is_reading());
        assert!(prices.start_reading());
    }

    #[test]
    fn before_the_first_read_the_prices_are_wanted_with_no_fetch_forced() {
        let prices = ListPrices::default();

        assert!(prices.is_wanted(Which::Every, at(10)));
        assert!(!prices.fetch_again());
        assert_eq!(prices.table(), None);
        assert!(!prices.has_failed());
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
        assert!(!kept.has_failed());
        assert!(none.has_failed());
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
    fn the_last_prices_stay_when_a_later_read_gives_none() {
        let mut prices =
            read(prices_answer(&[], Some(fetched_prices(at(10)))), at(10));

        prices.apply(
            prices_answer(&[(SourceId::LiteLlmPrices, network())], None),
            at(20),
        );

        assert_eq!(prices.last_good(), Some(&fetched_prices(at(10))));
        assert!(prices.has_failed());
    }
}
