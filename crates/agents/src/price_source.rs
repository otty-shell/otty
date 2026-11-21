//! Where the list prices of the money estimate come from (spec Decisions 5
//! and 16): the LiteLLM price table, fetched at most once a day and kept in
//! a folder, then the kept file when the fetch fails.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::error::{SourceError, Unavailable};
use crate::https::{HttpsGet, HttpsRequest, UreqClient, ok_body};
use crate::litellm::read_litellm;
use crate::price_table::PriceTable;
use crate::source_id::SourceId;
use crate::walk::{Answer, SourceFailure, walk};

/// The address of the LiteLLM price table.
const LITELLM_URL: &str = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";
/// The kept copy of the last good file, in the kept folder.
const KEPT_FILE: &str = "litellm.json";
/// The file a new copy is written to before it takes the place of the kept
/// one, so a broken write never leaves half a file.
const PART_FILE: &str = "litellm.json.part";
/// How long a fetched file is used before it is fetched again.
const FETCH_PERIOD: Duration = Duration::from_secs(24 * 3600);

/// List prices and when their file was fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedPrices {
    table: PriceTable,
    fetched_at: SystemTime,
}

impl FetchedPrices {
    /// The prices.
    pub fn table(&self) -> &PriceTable {
        &self.table
    }

    /// When the file of the prices was fetched.
    pub fn fetched_at(&self) -> SystemTime {
        self.fetched_at
    }

    /// The prices of `table`, fetched at `fetched_at`.
    ///
    /// ```
    /// use std::time::SystemTime;
    ///
    /// use otty_agents::{FetchedPrices, PriceTable};
    ///
    /// let prices =
    ///     FetchedPrices::new(PriceTable::default(), SystemTime::UNIX_EPOCH);
    ///
    /// assert_eq!(prices.table().price_of("gpt-5", None), None);
    /// ```
    pub fn new(table: PriceTable, fetched_at: SystemTime) -> Self {
        Self { table, fetched_at }
    }

    /// Whether the file was fetched less than a day before `now`, so it is
    /// not fetched again yet. A time after `now`, from a clock that was set
    /// back, counts as within the day.
    ///
    /// ```
    /// use std::time::{Duration, SystemTime};
    ///
    /// use otty_agents::{FetchedPrices, PriceTable};
    ///
    /// let fetched_at = SystemTime::UNIX_EPOCH;
    /// let prices = FetchedPrices::new(PriceTable::default(), fetched_at);
    ///
    /// assert!(prices.is_fresh_at(fetched_at + Duration::from_secs(3600)));
    /// assert!(!prices.is_fresh_at(fetched_at + Duration::from_secs(86_400)));
    /// ```
    pub fn is_fresh_at(&self, now: SystemTime) -> bool {
        now.duration_since(self.fetched_at)
            .map_or(true, |age| age < FETCH_PERIOD)
    }
}

/// The LiteLLM price table with its kept copy.
///
/// The request is a plain `GET` with OTTY's own `User-Agent` and nothing
/// about the user. The kept copy and the time it was fetched, the time it
/// was written, are kept in the folder the app gives.
pub struct PriceSource {
    client: Box<dyn HttpsGet>,
    kept_dir: PathBuf,
}

impl PriceSource {
    /// The ids of the sources of the list prices, best first: the order
    /// [`PriceSource::read`] tries them in. The dashboard lists them in it.
    pub const SOURCES: [SourceId; 2] =
        [SourceId::LiteLlmPrices, SourceId::KeptPrices];

    /// The kept copy of the last good file.
    pub(crate) fn kept_file(&self) -> PathBuf {
        self.kept_dir.join(KEPT_FILE)
    }

    /// The source that keeps its copy in `kept_dir`, such as
    /// `~/.cache/otty/prices`.
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// let source = otty_agents::PriceSource::new(Path::new("/tmp/otty-prices"));
    /// ```
    pub fn new(kept_dir: &Path) -> Self {
        Self::with_client(Box::new(UreqClient::new()), kept_dir)
    }

    /// The source that sends its request with `client`.
    pub(crate) fn with_client(
        client: Box<dyn HttpsGet>,
        kept_dir: &Path,
    ) -> Self {
        Self {
            client,
            kept_dir: kept_dir.to_path_buf(),
        }
    }

    /// The prices at `now`, from the first source that answers:
    ///
    /// 1. LiteLLM: the kept file when it was fetched less than a day ago,
    ///    else a new fetch, which then becomes the kept file. With
    ///    `fetch_again`, the file is fetched even within the day.
    /// 2. The kept file, however old, when the fetch failed.
    ///
    /// A fetch that fails, or gives a file that can not be read, keeps the
    /// last good file.
    pub fn read(
        &self,
        now: SystemTime,
        fetch_again: bool,
    ) -> Result<Answer<FetchedPrices>, Vec<SourceFailure>> {
        let fetched = std::iter::once_with(|| {
            (SourceId::LiteLlmPrices, self.fetched(now, fetch_again))
        });
        let kept = std::iter::once_with(|| (SourceId::KeptPrices, self.kept()));

        walk(fetched.chain(kept))
    }

    /// The prices of a fetch less than a day old at `now`: the kept file,
    /// or, when it is older, broken or missing, or with `fetch_again`, a new
    /// fetch, which is then kept.
    fn fetched(
        &self,
        now: SystemTime,
        fetch_again: bool,
    ) -> Result<FetchedPrices, SourceError> {
        if !fetch_again && let Some(prices) = self.kept_of_today(now) {
            return Ok(prices);
        }

        let request = HttpsRequest {
            url: String::from(LITELLM_URL),
            headers: Vec::new(),
        };
        let body = ok_body(self.client.get(&request))?;
        let table = read_litellm(&body)?;

        // A copy that can not be kept leaves the prices of this fetch in
        // use; the next read fetches again.
        let _ = self.keep(&body);
        Ok(FetchedPrices::new(table, now))
    }

    /// The kept file, when it was fetched less than a day before `now`
    /// and can be read. A time after `now`, from a clock that was set
    /// back, counts as within the day.
    fn kept_of_today(&self, now: SystemTime) -> Option<FetchedPrices> {
        let prices = self.kept().ok()?;

        prices.is_fresh_at(now).then_some(prices)
    }

    /// The kept file, with the time it was written as the time it was
    /// fetched.
    fn kept(&self) -> Result<FetchedPrices, SourceError> {
        let path = self.kept_file();
        let unreadable = |error: std::io::Error| SourceError::Unreadable {
            path: path.clone(),
            detail: error.to_string(),
        };

        let fetched_at = match std::fs::metadata(&path) {
            Ok(metadata) => metadata.modified().map_err(unreadable)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(SourceError::NotAvailable(
                    Unavailable::NoKeptPriceFile,
                ));
            },
            Err(error) => return Err(unreadable(error)),
        };
        let json = std::fs::read(&path).map_err(unreadable)?;

        let table = read_litellm(&json)?;
        Ok(FetchedPrices::new(table, fetched_at))
    }

    /// Keep `json` as the kept file: written next to it first, then put in
    /// its place.
    fn keep(&self, json: &[u8]) -> std::io::Result<()> {
        let part = self.kept_dir.join(PART_FILE);

        std::fs::create_dir_all(&self.kept_dir)?;
        std::fs::write(&part, json)?;
        std::fs::rename(&part, self.kept_file())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Unexpected;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::testing::{TempDir, at};

    /// 2026-11-17T10:00:00Z.
    const NOW: u64 = 1_794_909_600;
    const HOUR: u64 = 3600;

    /// A made-up price file with one model.
    const GOOD: &str = r#"{"gpt-5": {"input_cost_per_token": 1.25e-06, "output_cost_per_token": 1e-05, "litellm_provider": "openai"}}"#;
    /// Another made-up price file, with another price.
    const NEWER: &str = r#"{"gpt-5": {"input_cost_per_token": 2e-06, "output_cost_per_token": 1e-05, "litellm_provider": "openai"}}"#;

    /// A client whose one request is answered with `answer`, and which
    /// checks the request is a plain `GET` of the LiteLLM file.
    fn answering(
        answer: Result<HttpsAnswer, SourceError>,
    ) -> Box<MockHttpsGet> {
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| {
                request.url == LITELLM_URL && request.headers.is_empty()
            })
            .times(1)
            .return_once(move |_| answer);
        Box::new(client)
    }

    fn ok(body: &str) -> Result<HttpsAnswer, SourceError> {
        Ok(HttpsAnswer {
            status: 200,
            body: body.as_bytes().to_vec(),
        })
    }

    /// A client that must not be asked.
    fn never_asked() -> Box<MockHttpsGet> {
        let mut client = MockHttpsGet::new();
        client.expect_get().never();
        Box::new(client)
    }

    /// Keep `body` in `dir` as the file fetched at `fetched_at`.
    fn keep(dir: &Path, body: &str, fetched_at: SystemTime) {
        let path = dir.join(KEPT_FILE);
        std::fs::write(&path, body).expect("the kept file");
        std::fs::File::options()
            .write(true)
            .open(&path)
            .and_then(|file| file.set_modified(fetched_at))
            .expect("the time of the kept file");
    }

    fn input_price(prices: &FetchedPrices) -> Option<u64> {
        prices
            .table()
            .price_of("gpt-5", None)
            .map(|price| price.standard().input.micro_dollars())
    }

    fn failures(
        read: &Result<Answer<FetchedPrices>, Vec<SourceFailure>>,
    ) -> Vec<(SourceId, SourceError)> {
        let failures = match read {
            Ok(answer) => answer.earlier_failures(),
            Err(failures) => failures.as_slice(),
        };

        failures
            .iter()
            .map(|failure| (failure.source(), failure.error().clone()))
            .collect()
    }

    #[test]
    fn with_no_kept_file_the_prices_are_fetched_and_kept() {
        let dir = TempDir::new();
        let kept_dir = dir.path().join("prices");
        let source = PriceSource::with_client(answering(ok(GOOD)), &kept_dir);

        let answer = source.read(at(NOW), false).expect("the fetch answers");

        assert_eq!(answer.source(), SourceId::LiteLlmPrices);
        assert_eq!(answer.value().fetched_at(), at(NOW));
        assert_eq!(input_price(answer.value()), Some(1_250_000));
        assert_eq!(
            std::fs::read_to_string(kept_dir.join(KEPT_FILE)).ok(),
            Some(String::from(GOOD))
        );
        assert!(!kept_dir.join(PART_FILE).exists());
    }

    #[test]
    fn a_file_fetched_less_than_a_day_ago_is_not_fetched_again() {
        let dir = TempDir::new();
        let fetched_at = at(NOW - 23 * HOUR);
        keep(dir.path(), GOOD, fetched_at);
        let source = PriceSource::with_client(never_asked(), dir.path());

        let answer =
            source.read(at(NOW), false).expect("the kept file answers");

        assert_eq!(answer.source(), SourceId::LiteLlmPrices);
        assert_eq!(answer.value().fetched_at(), fetched_at);
        assert_eq!(input_price(answer.value()), Some(1_250_000));
    }

    #[test]
    fn a_file_fetched_a_day_ago_is_fetched_again() {
        let dir = TempDir::new();
        keep(dir.path(), GOOD, at(NOW - 24 * HOUR));
        let source = PriceSource::with_client(answering(ok(NEWER)), dir.path());

        let answer = source.read(at(NOW), false).expect("the fetch answers");

        assert_eq!(answer.value().fetched_at(), at(NOW));
        assert_eq!(input_price(answer.value()), Some(2_000_000));
    }

    #[test]
    fn fetching_again_fetches_even_within_the_day() {
        let dir = TempDir::new();
        keep(dir.path(), GOOD, at(NOW - HOUR));
        let source = PriceSource::with_client(answering(ok(NEWER)), dir.path());

        let answer = source.read(at(NOW), true).expect("the fetch answers");

        assert_eq!(answer.source(), SourceId::LiteLlmPrices);
        assert_eq!(input_price(answer.value()), Some(2_000_000));
    }

    #[test]
    fn a_failed_fetch_keeps_the_last_good_file_which_answers() {
        let dir = TempDir::new();
        let fetched_at = at(NOW - 72 * HOUR);
        keep(dir.path(), GOOD, fetched_at);
        let down = Err(SourceError::Network(String::from("no connection")));
        let source = PriceSource::with_client(answering(down), dir.path());

        let read = source.read(at(NOW), false);

        assert_eq!(
            failures(&read),
            [(
                SourceId::LiteLlmPrices,
                SourceError::Network(String::from("no connection"))
            )]
        );
        let answer = read.expect("the kept file answers");
        assert_eq!(answer.source(), SourceId::KeptPrices);
        assert_eq!(answer.value().fetched_at(), fetched_at);
        assert_eq!(input_price(answer.value()), Some(1_250_000));
    }

    #[test]
    fn a_broken_fetched_file_fails_and_does_not_replace_the_kept_one() {
        let dir = TempDir::new();
        keep(dir.path(), GOOD, at(NOW - 72 * HOUR));
        let broken = ok("<html>rate limited</html>");
        let source = PriceSource::with_client(answering(broken), dir.path());

        let read = source.read(at(NOW), false);

        assert!(matches!(
            failures(&read).as_slice(),
            [(SourceId::LiteLlmPrices, SourceError::UnexpectedAnswer(_))]
        ));
        assert_eq!(
            read.map(|answer| answer.source()),
            Ok(SourceId::KeptPrices)
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join(KEPT_FILE)).ok(),
            Some(String::from(GOOD))
        );
    }

    #[test]
    fn a_server_that_does_not_give_the_file_is_unexpected() {
        let dir = TempDir::new();
        let missing = Ok(HttpsAnswer {
            status: 404,
            body: Vec::new(),
        });
        let source = PriceSource::with_client(answering(missing), dir.path());

        let read = source.read(at(NOW), false);

        assert_eq!(
            failures(&read),
            [
                (
                    SourceId::LiteLlmPrices,
                    SourceError::UnexpectedAnswer(Unexpected::ServerStatus(
                        404
                    ))
                ),
                (
                    SourceId::KeptPrices,
                    SourceError::NotAvailable(Unavailable::NoKeptPriceFile)
                ),
            ]
        );
        assert!(read.is_err());
    }

    #[test]
    fn the_sources_are_tried_in_the_order_of_their_list() {
        let dir = TempDir::new();
        let down = Err(SourceError::Network(String::from("no connection")));
        let source = PriceSource::with_client(answering(down), dir.path());

        let read = source.read(at(NOW), false);

        let tried: Vec<SourceId> =
            failures(&read).into_iter().map(|(id, _)| id).collect();
        assert_eq!(tried, PriceSource::SOURCES);
    }

    #[test]
    fn a_broken_kept_file_fails_too() {
        let dir = TempDir::new();
        keep(dir.path(), "{", at(NOW - 72 * HOUR));
        let down = Err(SourceError::Network(String::from("no connection")));
        let source = PriceSource::with_client(answering(down), dir.path());

        let read = source.read(at(NOW), false);

        assert!(matches!(
            failures(&read).as_slice(),
            [
                (SourceId::LiteLlmPrices, SourceError::Network(_)),
                (SourceId::KeptPrices, SourceError::UnexpectedAnswer(_)),
            ]
        ));
        assert!(read.is_err());
    }

    #[test]
    fn a_broken_kept_file_of_today_is_fetched_again() {
        let dir = TempDir::new();
        keep(dir.path(), "{", at(NOW - HOUR));
        let source = PriceSource::with_client(answering(ok(GOOD)), dir.path());

        let answer = source.read(at(NOW), false).expect("the fetch answers");

        assert_eq!(answer.source(), SourceId::LiteLlmPrices);
        assert_eq!(input_price(answer.value()), Some(1_250_000));
    }
}
