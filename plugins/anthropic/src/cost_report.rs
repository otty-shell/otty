//! The bill of an Anthropic organization: the cost report of the Admin
//! API, read with an admin key.
//!
//! The request follows Anthropic's docs,
//! <https://platform.claude.com/docs/en/api/admin/cost_report/retrieve> and
//! <https://platform.claude.com/docs/en/manage-claude/usage-cost-api>: the
//! key goes in `x-api-key`, the days are UTC days, `amount` is a decimal
//! text in cents, and the pages are walked with `next_page` while
//! `has_more` is true. The version header follows CodexBar (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeAdminAPIUsageFetcher.swift>.
//! CodexBar groups the report by `description` to list what the money was
//! spent on; OTTY shows only the money of each day, so it asks for the
//! totals of the days and groups nothing.

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

/// The address of the cost report.
const COST_REPORT_URL: &str =
    "https://api.anthropic.com/v1/organizations/cost_report";
/// The version of the API the answer is read in.
const ANTHROPIC_VERSION: &str = "2023-06-01";
/// The most days one page holds; a bill of more days has more pages.
const PAGE_DAYS: u32 = 31;
/// The most pages read: a report that has more is not what the code
/// expects, and the walk stops instead of asking for ever.
const MAX_PAGES: usize = 10;
/// Micro-dollars in one cent.
const MICRO_DOLLARS_IN_CENT: i64 = 10_000;

/// One page of the cost report.
#[derive(Debug, Deserialize)]
struct ReportPage {
    data: Vec<Bucket>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    next_page: Option<String>,
}

impl ReportPage {
    /// The token of the next page, while there is one.
    fn next_page(&self) -> Option<&str> {
        self.next_page.as_deref().filter(|_| self.has_more)
    }
}

/// One day of the report.
#[derive(Debug, Deserialize)]
struct Bucket {
    #[serde(default)]
    starting_at: Option<String>,
    #[serde(default)]
    results: Vec<CostItem>,
}

impl Bucket {
    /// The UTC day the bucket starts on.
    fn day(&self) -> Option<Date> {
        let start: Timestamp = self.starting_at.as_deref()?.parse().ok()?;

        Some(start.to_zoned(TimeZone::UTC).date())
    }

    /// The money of the day in micro-dollars: the sum of its costs in US
    /// dollars that can be read.
    fn micro_dollars(&self) -> i64 {
        self.results
            .iter()
            .filter(|item| item.is_in_dollars())
            .filter_map(|item| cents_to_micro_dollars(item.amount.as_ref()?))
            .fold(0, i64::saturating_add)
    }
}

/// One cost of a day.
#[derive(Debug, Deserialize)]
struct CostItem {
    #[serde(default)]
    amount: Option<Value>,
    #[serde(default)]
    currency: Option<String>,
}

impl CostItem {
    /// Whether the cost is in US dollars. The docs say it always is; a
    /// cost with no currency counts as dollars too.
    fn is_in_dollars(&self) -> bool {
        self.currency.as_deref().is_none_or(|code| code == "USD")
    }
}

/// The bill of the organization of the admin key `key` over the UTC days
/// of `days`. A refused key asks for a new one under the admin slot; any
/// other status fails and an answer that is not a report is unexpected. A
/// day, or a cost of a day, that can not be read is skipped.
pub(crate) fn bill(key: &Key, days: &DayRange) -> Result<Bill, ReadError> {
    let mut sums = DaySums::new(days)?;
    let end = sums
        .last()
        .tomorrow()
        .map_err(|_| ReadError::Unexpected("the bill has no end".into()))?;
    let mut page: Option<String> = None;

    for _ in 0..MAX_PAGES {
        let url = page_url(sums.first(), end, page.as_deref());
        let request = Request::get(&url)
            .header_with_key("x-api-key", "", key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("accept", "application/json");
        let answer = json_answer(&request, COST_REPORT_URL, || {
            ReadError::NeedsUser(needs::new_bill_key())
        })?;
        let report: ReportPage =
            serde_json::from_value(answer).map_err(|_| {
                ReadError::Unexpected(format!(
                    "the answer of {COST_REPORT_URL} is not a cost report"
                ))
            })?;

        for bucket in &report.data {
            if let Some(day) = bucket.day() {
                sums.add(day, bucket.micro_dollars());
            }
        }

        match report.next_page() {
            Some(next) => page = Some(next.to_string()),
            None => return Ok(sums.into_bill()),
        }
    }

    Err(ReadError::Unexpected(format!(
        "{COST_REPORT_URL} gave more than {MAX_PAGES} pages"
    )))
}

/// The address of the page `page`, the first when `None`, of the report
/// of the UTC days from `first` up to the start of `end`.
fn page_url(first: Date, end: Date, page: Option<&str>) -> String {
    let mut url = format!(
        "{COST_REPORT_URL}?starting_at={first}T00:00:00Z&ending_at={end}T00:00:00Z&bucket_width=1d&limit={PAGE_DAYS}"
    );
    if let Some(page) = page {
        url.push_str(&format!("&page={}", percent_encoded(page)));
    }

    url
}

/// An amount in cents written as decimal text, such as `"123.78912"`, in
/// micro-dollars, rounded to the nearest one. A number is read the same
/// way. `None` for any other text.
fn cents_to_micro_dollars(amount: &Value) -> Option<i64> {
    let text = match amount {
        Value::String(text) => text.trim().to_string(),
        Value::Number(number) => number.to_string(),
        _ => return None,
    };
    let (is_negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.as_str()),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let is_number = !whole.is_empty()
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit());
    if !is_number {
        return None;
    }

    let whole: i64 = whole.parse().ok()?;
    let mut fraction_digits =
        fraction.bytes().map(|byte| i64::from(byte - b'0'));
    let mut part = 0_i64;
    for _ in 0..4 {
        part = part * 10 + fraction_digits.next().unwrap_or(0);
    }
    let rounds_up = fraction_digits.next().is_some_and(|digit| digit >= 5);

    let micro_dollars = whole
        .checked_mul(MICRO_DOLLARS_IN_CENT)?
        .checked_add(part + i64::from(rounds_up))?;

    Some(if is_negative {
        -micro_dollars
    } else {
        micro_dollars
    })
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::NeedAction;
    use otty_plugin_sdk::test_host::TestHost;

    use super::*;
    use crate::testing::host;

    /// A made-up admin key, never a real one.
    const MADE_UP_KEY: &str = "sk-ant-admin01-made-up-for-tests";
    /// The address of the first page of the 31 UTC days from 2026-10-17
    /// to the end of 2026-11-16.
    const FIRST_PAGE: &str = "https://api.anthropic.com/v1/organizations/cost_report?starting_at=2026-10-17T00:00:00Z&ending_at=2026-11-17T00:00:00Z&bucket_width=1d&limit=31";

    fn days() -> DayRange {
        DayRange {
            first: "2026-10-17".to_string(),
            last: "2026-11-16".to_string(),
        }
    }

    fn bill_of(host: &TestHost) -> Result<Bill, ReadError> {
        let key = host.key(MADE_UP_KEY);

        host.as_provider("anthropic", || bill(&key, &days()))
    }

    fn report(host: &TestHost, body: &str) -> Result<Bill, ReadError> {
        host.answer("GET", FIRST_PAGE, 200, body.as_bytes());

        bill_of(host)
    }

    fn days_of(bill: &Bill) -> Vec<(&str, f64)> {
        bill.days
            .iter()
            .map(|day| (day.day.as_str(), day.cost.amount))
            .collect()
    }

    #[test]
    fn a_report_gives_the_money_of_each_day_from_its_cents() {
        let host = host();

        let bill = report(
            &host,
            r#"{
                "data": [
                    {
                        "starting_at": "2026-10-31T00:00:00Z",
                        "ending_at": "2026-11-01T00:00:00Z",
                        "results": [{"amount": "1000", "currency": "USD"}]
                    },
                    {
                        "starting_at": "2026-11-15T00:00:00Z",
                        "ending_at": "2026-11-16T00:00:00Z",
                        "results": [
                            {"amount": "123.78912", "currency": "USD", "description": null},
                            {"amount": "76.5", "currency": "USD"}
                        ]
                    },
                    {
                        "starting_at": "2026-11-16T00:00:00Z",
                        "ending_at": "2026-11-17T00:00:00Z",
                        "results": [{"amount": "1240", "currency": "USD"}]
                    }
                ],
                "has_more": false,
                "next_page": null
            }"#,
        )
        .expect("the report is read");

        assert_eq!(
            days_of(&bill),
            [
                ("2026-10-31", 10.0),
                ("2026-11-15", 2.002891),
                ("2026-11-16", 12.4)
            ]
        );
        let requests = host.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].header("x-api-key"), Some(MADE_UP_KEY));
        assert_eq!(
            requests[0].header("anthropic-version"),
            Some(ANTHROPIC_VERSION)
        );
    }

    #[test]
    fn every_page_is_read_while_the_report_has_more() {
        let host = host();
        host.answer(
            "GET",
            &format!("{FIRST_PAGE}&page=page_MjAy%2B%2F%3D"),
            200,
            br#"{
                "data": [{"starting_at": "2026-11-16T00:00:00Z", "results": [{"amount": "250"}]}],
                "has_more": false,
                "next_page": null
            }"#,
        );

        let bill = report(
            &host,
            r#"{
                "data": [{"starting_at": "2026-11-15T00:00:00Z", "results": [{"amount": "100"}]}],
                "has_more": true,
                "next_page": "page_MjAy+/="
            }"#,
        )
        .expect("both pages are read");

        assert_eq!(days_of(&bill), [("2026-11-15", 1.0), ("2026-11-16", 2.5)]);
    }

    #[test]
    fn a_report_that_never_ends_is_unexpected() {
        let host = host();
        let again = r#"{"data": [], "has_more": true, "next_page": "again"}"#;
        host.answer(
            "GET",
            &format!("{FIRST_PAGE}&page=again"),
            200,
            again.as_bytes(),
        );

        let bill = report(&host, again);

        assert!(matches!(bill, Err(ReadError::Unexpected(_))), "{bill:?}");
        assert_eq!(host.requests().len(), MAX_PAGES);
    }

    /// A refused admin key asks for a new one under the same slot.
    #[test]
    fn a_refused_key_asks_for_a_new_one() {
        for status in [401, 403] {
            let host = host();
            host.answer("GET", FIRST_PAGE, status, b"{}");

            let bill = bill_of(&host);

            let Err(ReadError::NeedsUser(need)) = bill else {
                panic!("a need: {bill:?}");
            };
            assert_eq!(need.id, "new-bill-key", "{status}");
            assert!(
                need.text.starts_with("Anthropic refused"),
                "{}",
                need.text
            );
            let NeedAction::AskKey(ask) = need.action else {
                panic!("a key to ask for: {need:?}");
            };
            assert_eq!(ask.slot.as_deref(), Some("admin"));
        }
    }

    #[test]
    fn another_answer_is_a_failure() {
        let server_error = host();
        server_error.answer("GET", FIRST_PAGE, 500, b"{}");
        let not_a_report = host();

        assert!(matches!(bill_of(&server_error), Err(ReadError::Failed(_))));
        assert!(matches!(
            report(&not_a_report, r#"{"error": "no data"}"#),
            Err(ReadError::Unexpected(_))
        ));
    }

    #[test]
    fn what_can_not_be_read_is_skipped() {
        let bill = report(
            &host(),
            r#"{"data": [
                {"starting_at": "yesterday", "results": [{"amount": "100"}]},
                {"starting_at": "2026-09-01T00:00:00Z", "results": [{"amount": "100"}]},
                {"starting_at": "2026-11-16T00:00:00Z", "results": [
                    {"amount": "lots"},
                    {"amount": "100", "currency": "EUR"},
                    {"amount": 25.5},
                    {"currency": "USD"},
                    {"amount": "300"}
                ]}
            ]}"#,
        )
        .expect("the readable parts are read");

        assert_eq!(days_of(&bill), [("2026-11-16", 3.255)]);
    }

    #[test]
    fn cents_are_read_to_the_nearest_micro_dollar() {
        let cents = |text: &str| cents_to_micro_dollars(&Value::from(text));

        assert_eq!(cents("50"), Some(500_000));
        assert_eq!(cents("0.00005"), Some(1));
        assert_eq!(cents("0.00004"), Some(0));
        assert_eq!(cents("-25.5"), Some(-255_000));
        assert_eq!(cents("1e3"), None);
        assert_eq!(cents(".5"), None);
        assert_eq!(cents_to_micro_dollars(&Value::Null), None);
    }

    #[test]
    fn a_day_of_refunds_only_costs_nothing() {
        let bill = report(
            &host(),
            r#"{"data": [{"starting_at": "2026-11-16T00:00:00Z", "results": [{"amount": "-500"}]}]}"#,
        )
        .expect("the report is read");

        assert_eq!(days_of(&bill), [("2026-11-16", 0.0)]);
    }
}
