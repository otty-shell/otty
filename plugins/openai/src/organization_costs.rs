//! The bill of an OpenAI organization: the costs of the Admin API,
//! `GET /v1/organization/costs`, read with an admin key, as OpenAI
//! documents it:
//! <https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage/methods/costs>.
//!
//! The days are UTC days, each bucket starts at `start_time` in seconds,
//! `amount.value` is in the currency `amount.currency` (US dollars), and
//! the pages are walked with `page` = `next_page` while `has_more` is
//! true.

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use otty_plugin_sdk::Key;
use otty_plugin_sdk::agents::{Bill, DayRange, ReadError};
use otty_plugin_sdk::http::Request;
use serde::Deserialize;
use serde_json::Value;

use crate::bill_days::DaySums;
use crate::needs;
use crate::requests::{json_answer, percent_encoded};

/// The address of the costs.
const COSTS_URL: &str = "https://api.openai.com/v1/organization/costs";
/// The most days one page holds; a bill of more days has more pages.
const PAGE_DAYS: u32 = 31;
/// The most pages read: more is not what the code expects, and the walk
/// stops instead of asking for ever.
const MAX_PAGES: usize = 10;

/// One page of the costs.
#[derive(Debug, Deserialize)]
struct CostsPage {
    data: Vec<Bucket>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    next_page: Option<String>,
}

impl CostsPage {
    /// The token of the next page, while there is one.
    fn next_page(&self) -> Option<&str> {
        self.next_page.as_deref().filter(|_| self.has_more)
    }
}

/// One day of the costs.
#[derive(Debug, Deserialize)]
struct Bucket {
    start_time: i64,
    #[serde(default)]
    results: Vec<CostResult>,
}

impl Bucket {
    /// The UTC day the bucket starts on.
    fn day(&self) -> Option<Date> {
        let start = Timestamp::from_second(self.start_time).ok()?;

        Some(start.to_zoned(TimeZone::UTC).date())
    }

    /// The money of the day in micro-dollars: the sum of its costs in US
    /// dollars that can be read. A cost below zero, such as a refund, is
    /// left out.
    fn micro_dollars(&self) -> i64 {
        self.results
            .iter()
            .filter_map(CostResult::micro_dollars)
            .fold(0, i64::saturating_add)
    }
}

/// One cost of a day.
#[derive(Debug, Deserialize)]
struct CostResult {
    #[serde(default)]
    amount: Option<Value>,
}

impl CostResult {
    /// The cost in US micro-dollars; `None` for another currency, a cost
    /// below zero or an amount that is not a number.
    fn micro_dollars(&self) -> Option<i64> {
        let amount = self.amount.as_ref()?;
        let currency = amount
            .get("currency")
            .and_then(Value::as_str)
            .unwrap_or("usd");
        if !currency.eq_ignore_ascii_case("usd") {
            return None;
        }

        let dollars = amount.get("value")?.as_f64()?;
        if !dollars.is_finite() || dollars < 0.0 {
            return None;
        }
        Some((dollars * 1_000_000.0).round() as i64)
    }
}

/// The bill of the organization of the admin key `key` over the UTC days
/// of `days`. A refused key asks for a new one under the admin slot; an
/// answer that is not a page of costs is unexpected. A day, or a cost, that can not be read
/// is skipped.
pub(crate) fn bill(key: &Key, days: &DayRange) -> Result<Bill, ReadError> {
    let mut sums = DaySums::new(days)?;
    let start = utc_seconds(sums.first())?;
    let end = utc_seconds(next_day(sums.last())?)?;
    let mut page: Option<String> = None;

    for _ in 0..MAX_PAGES {
        let url = page_url(start, end, page.as_deref());
        let request = Request::get(&url)
            .header_with_key("authorization", "Bearer ", key)
            .header("accept", "application/json");
        let answer = json_answer(&request, COSTS_URL, || {
            ReadError::NeedsUser(needs::new_bill_key())
        })?;
        let costs: CostsPage =
            serde_json::from_value(answer).map_err(|_| {
                ReadError::Unexpected(format!(
                    "the answer of {COSTS_URL} is not a page of costs"
                ))
            })?;

        for bucket in &costs.data {
            if let Some(day) = bucket.day() {
                sums.add(day, bucket.micro_dollars());
            }
        }

        match costs.next_page() {
            Some(next) => page = Some(next.to_string()),
            None => return Ok(sums.into_bill()),
        }
    }

    Err(ReadError::Unexpected(format!(
        "{COSTS_URL} gave more than {MAX_PAGES} pages"
    )))
}

/// The address of the page `page`, the first when `None`, of the costs of
/// the UTC days from `start` to `end`, in seconds since 1970.
fn page_url(start: i64, end: i64, page: Option<&str>) -> String {
    let mut url = format!(
        "{COSTS_URL}?start_time={start}&end_time={end}&bucket_width=1d&limit={PAGE_DAYS}"
    );
    if let Some(page) = page {
        url.push_str(&format!("&page={}", percent_encoded(page)));
    }

    url
}

/// The start of the UTC day `day`, in seconds since 1970.
fn utc_seconds(day: Date) -> Result<i64, ReadError> {
    let start = day
        .to_zoned(TimeZone::UTC)
        .map_err(|_| ReadError::Unexpected(format!("{day} has no start")))?;

    Ok(start.timestamp().as_second())
}

/// The day after `day`.
fn next_day(day: Date) -> Result<Date, ReadError> {
    day.tomorrow()
        .map_err(|_| ReadError::Unexpected(format!("{day} has no next day")))
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::NeedAction;
    use otty_plugin_sdk::test_host::TestHost;

    use super::*;
    use crate::testing::host;

    /// A made-up admin key, never a real one.
    const MADE_UP_KEY: &str = "sk-admin-made-up-for-tests";
    /// The address of the first page of the 31 UTC days from 2026-10-17
    /// to the end of 2026-11-16.
    const FIRST_PAGE: &str = "https://api.openai.com/v1/organization/costs?start_time=1792195200&end_time=1794873600&bucket_width=1d&limit=31";

    fn days() -> DayRange {
        DayRange {
            first: "2026-10-17".to_string(),
            last: "2026-11-16".to_string(),
        }
    }

    /// The bucket of the UTC day that starts at `start_time` with costs of
    /// `values` US dollars.
    fn bucket(start_time: u64, values: &[&str]) -> String {
        let results: Vec<String> = values
            .iter()
            .map(|value| {
                format!(
                    r#"{{"object": "organization.costs.result",
                    "amount": {{"value": {value}, "currency": "usd"}},
                    "line_item": null, "project_id": null}}"#
                )
            })
            .collect();

        format!(
            r#"{{"object": "bucket", "start_time": {start_time},
            "end_time": {}, "results": [{}]}}"#,
            start_time + 86_400,
            results.join(",")
        )
    }

    fn bill_of(host: &TestHost) -> Result<Bill, ReadError> {
        let key = host.key(MADE_UP_KEY);

        host.as_provider("openai", || bill(&key, &days()))
    }

    fn days_of(bill: &Bill) -> Vec<(&str, f64)> {
        bill.days
            .iter()
            .map(|day| (day.day.as_str(), day.cost.amount))
            .collect()
    }

    #[test]
    fn the_costs_give_the_money_of_each_day() {
        // 2026-11-01 and 2026-11-16, 00:00 UTC.
        let body = format!(
            r#"{{"object": "page", "data": [{}, {}],
            "has_more": false, "next_page": null}}"#,
            bucket(1_793_491_200, &["1.5", "0.25"]),
            bucket(1_794_787_200, &["2"])
        );
        let host = host();
        host.answer("GET", FIRST_PAGE, 200, body.as_bytes());

        let bill = bill_of(&host).expect("a bill");

        assert_eq!(days_of(&bill), [("2026-11-01", 1.75), ("2026-11-16", 2.0)]);
        let requests = host.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].header("authorization"),
            Some(format!("Bearer {MADE_UP_KEY}").as_str())
        );
    }

    #[test]
    fn the_pages_are_walked_while_there_are_more() {
        let first = format!(
            r#"{{"data": [{}], "has_more": true, "next_page": "page 2"}}"#,
            bucket(1_793_491_200, &["1"])
        );
        let second = format!(
            r#"{{"data": [{}], "has_more": false, "next_page": null}}"#,
            bucket(1_794_787_200, &["3"])
        );
        let host = host();
        host.answer("GET", FIRST_PAGE, 200, first.as_bytes());
        host.answer(
            "GET",
            &format!("{FIRST_PAGE}&page=page%202"),
            200,
            second.as_bytes(),
        );

        let bill = bill_of(&host).expect("a bill");

        assert_eq!(days_of(&bill), [("2026-11-01", 1.0), ("2026-11-16", 3.0)]);
    }

    #[test]
    fn another_currency_and_a_refund_are_left_out() {
        let body = r#"{"data": [{"start_time": 1794787200, "results": [
            {"amount": {"value": 5, "currency": "eur"}},
            {"amount": {"value": -1, "currency": "usd"}},
            {"amount": {"value": 2, "currency": "usd"}}
        ]}], "has_more": false}"#;
        let host = host();
        host.answer("GET", FIRST_PAGE, 200, body.as_bytes());

        let bill = bill_of(&host).expect("a bill");

        assert_eq!(days_of(&bill), [("2026-11-16", 2.0)]);
    }

    /// A refused admin key asks for a new one under the same slot; an
    /// answer that is not a page of costs is unexpected.
    #[test]
    fn a_refused_key_asks_for_a_new_one_and_another_answer_is_unexpected() {
        let refused = host();
        refused.answer("GET", FIRST_PAGE, 401, b"{}");
        let other = host();
        other.answer("GET", FIRST_PAGE, 200, br#"{"error": "nope"}"#);

        let refused_bill = bill_of(&refused);

        let Err(ReadError::NeedsUser(need)) = refused_bill else {
            panic!("a need: {refused_bill:?}");
        };
        assert_eq!(need.id, "new-bill-key");
        assert!(need.text.starts_with("OpenAI refused"), "{}", need.text);
        let NeedAction::AskKey(ask) = need.action else {
            panic!("a key to ask for: {need:?}");
        };
        assert_eq!(ask.slot.as_deref(), Some("admin"));
        assert!(matches!(bill_of(&other), Err(ReadError::Unexpected(_))));
    }

    #[test]
    fn too_many_pages_stop_the_walk() {
        let host = host();
        let again = br#"{"data": [], "has_more": true, "next_page": "again"}"#;
        host.answer("GET", FIRST_PAGE, 200, again);
        host.answer("GET", &format!("{FIRST_PAGE}&page=again"), 200, again);

        let bill = bill_of(&host);

        assert!(matches!(bill, Err(ReadError::Unexpected(_))), "{bill:?}");
        assert_eq!(host.requests().len(), MAX_PAGES);
    }
}
