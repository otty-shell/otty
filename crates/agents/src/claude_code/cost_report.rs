//! The real bill of an Anthropic organization: the cost report of the Admin
//! API, read with an admin key (spec Decision 9).
//!
//! The request follows Anthropic's docs,
//! <https://platform.claude.com/docs/en/api/admin/cost_report/retrieve> and
//! <https://platform.claude.com/docs/en/manage-claude/usage-cost-api>: the
//! key goes in `x-api-key`, the days are UTC days, `amount` is a decimal
//! text in cents, and the pages are walked with `next_page` while
//! `has_more` is true. The range of 31 days up to the end of today and the
//! version header follow CodexBar (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeAdminAPIUsageFetcher.swift>.
//! CodexBar groups the report by `description` to list what the money was
//! spent on; OTTY shows only the money of each day, so it asks for the
//! totals of the days and groups nothing.

use std::time::SystemTime;

use jiff::ToSpan;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde::Deserialize;
use serde_json::Value;

use crate::admin_key::AdminKey;
use crate::bill::{Bill, BillSource, DayCost};
use crate::dollars::Dollars;
use crate::error::SourceError;
use crate::https::{HttpsGet, HttpsRequest, percent_encoded, usage_json};
use crate::local_time::{LocalNow, days_before};
use crate::timestamp::parse_rfc3339;

/// The address of the cost report.
const COST_REPORT_URL: &str =
    "https://api.anthropic.com/v1/organizations/cost_report";
/// The version of the API the answer is read in.
const ANTHROPIC_VERSION: &str = "2023-06-01";
/// The days of the bill: today and the 30 days before it, which always
/// hold the first day of the month. It is also the most days one page
/// may hold.
const DAYS: i32 = 31;
/// The most pages read: a report that has more is not what the code
/// expects, and the walk stops instead of asking for ever.
const MAX_PAGES: usize = 10;
/// Micro-dollars in one cent.
const MICRO_DOLLARS_IN_CENT: i64 = 10_000;

/// The cost report request with the admin key of a profile.
pub(super) struct CostReport {
    client: Box<dyn HttpsGet>,
}

impl CostReport {
    /// The source that sends its requests with `client`.
    pub(super) fn new(client: Box<dyn HttpsGet>) -> Self {
        Self { client }
    }

    /// The request of the page `page`, the first when `None`, of the
    /// report of the days from `first_day` to `today`.
    fn request(
        key: &AdminKey,
        first_day: Date,
        today: Date,
        page: Option<&str>,
    ) -> HttpsRequest {
        let starting_at = utc_midnight(first_day);
        let ending_at = utc_midnight(today.saturating_add(1.day()));

        let mut url = format!(
            "{COST_REPORT_URL}?starting_at={starting_at}&ending_at={ending_at}&bucket_width=1d&limit={DAYS}"
        );
        if let Some(page) = page {
            let page = percent_encoded(page);
            url.push_str(&format!("&page={page}"));
        }

        HttpsRequest {
            url,
            headers: vec![
                ("x-api-key", key.secret().to_string()),
                ("anthropic-version", String::from(ANTHROPIC_VERSION)),
                ("Accept", String::from("application/json")),
            ],
        }
    }
}

impl BillSource for CostReport {
    /// A refused key is "key refused"; any other status or an answer that
    /// is not a report is unexpected. A day, or a cost of a day, that can
    /// not be read is skipped.
    fn bill(
        &self,
        key: &AdminKey,
        now: SystemTime,
    ) -> Result<Bill, SourceError> {
        let utc = LocalNow::new(now, TimeZone::UTC);
        let today = utc.today();
        let first_day = days_before(today, DAYS - 1);
        let mut micro_dollars = [0_i64; DAYS as usize];
        let mut page: Option<String> = None;

        for _ in 0..MAX_PAGES {
            let request = Self::request(key, first_day, today, page.as_deref());
            let answer =
                usage_json(self.client.get(&request)).map_err(|error| {
                    match error {
                        SourceError::NotLoggedIn => SourceError::KeyRefused,
                        other => other,
                    }
                })?;
            let report: ReportPage =
                serde_json::from_value(answer).map_err(|_| {
                    SourceError::UnexpectedAnswer(String::from(
                        "the answer is not a cost report",
                    ))
                })?;

            for bucket in &report.data {
                let Some(day) = bucket.day(&utc) else {
                    continue;
                };
                let Some(index) = day_index(first_day, day) else {
                    continue;
                };
                if let Some(sum) = micro_dollars.get_mut(index) {
                    *sum = sum.saturating_add(bucket.micro_dollars());
                }
            }

            match report.next_page() {
                Some(next) => page = Some(next.to_string()),
                None => return Ok(bill_of(first_day, &micro_dollars)),
            }
        }

        Err(SourceError::UnexpectedAnswer(String::from(
            "the cost report has too many pages",
        )))
    }
}

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
    fn day(&self, utc: &LocalNow) -> Option<Date> {
        let start = parse_rfc3339(self.starting_at.as_deref()?)?;

        Some(utc.date_of(start))
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

/// The bill of the days from `first_day`, one sum of micro-dollars per
/// day. A day whose sum is below zero, such as a day of refunds only,
/// counts as nothing.
fn bill_of(first_day: Date, micro_dollars: &[i64]) -> Bill {
    let days = micro_dollars
        .iter()
        .zip(first_day.series(1.day()))
        .map(|(sum, day)| {
            let cost = u64::try_from(*sum).unwrap_or_default();
            DayCost::new(day, Dollars::from_micro_dollars(cost))
        })
        .collect();

    Bill::new(days)
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

/// The place of `day` among the days from `first_day`; `None` for a day
/// before it.
fn day_index(first_day: Date, day: Date) -> Option<usize> {
    let days = first_day.until(day).ok()?.get_days();

    usize::try_from(days).ok()
}

/// The start of the UTC day `date`, as RFC 3339 text.
fn utc_midnight(date: Date) -> String {
    format!("{date}T00:00:00Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::testing::{NOW, answering, at};

    /// A made-up admin key, never a real one.
    const MADE_UP_KEY: &str = "sk-ant-admin01-made-up-for-tests";
    /// The address of the first page for [`NOW`]: the 31 UTC days from
    /// 2026-10-17 to the end of 2026-11-16.
    const FIRST_PAGE: &str = "https://api.anthropic.com/v1/organizations/cost_report?starting_at=2026-10-17T00:00:00Z&ending_at=2026-11-17T00:00:00Z&bucket_width=1d&limit=31";

    fn key() -> AdminKey {
        AdminKey::made_up(MADE_UP_KEY)
    }

    fn ok(body: &str) -> Result<HttpsAnswer, String> {
        Ok(HttpsAnswer {
            status: 200,
            body: body.as_bytes().to_vec(),
        })
    }

    fn read(client: MockHttpsGet) -> Result<Bill, SourceError> {
        CostReport::new(Box::new(client)).bill(&key(), at(NOW))
    }

    fn cost_on(bill: &Bill, month: i8, day: i8) -> u64 {
        bill.last_30_days_by_day()
            .iter()
            .find(|cost| cost.date() == jiff::civil::date(2026, month, day))
            .map(|cost| cost.cost().micro_dollars())
            .expect("the day is in the bill")
    }

    #[test]
    fn a_report_gives_the_money_of_each_day_from_its_cents() {
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| {
                request.url == FIRST_PAGE
                    && request.headers
                        == [
                            ("x-api-key", String::from(MADE_UP_KEY)),
                            ("anthropic-version", String::from("2023-06-01")),
                            ("Accept", String::from("application/json")),
                        ]
            })
            .times(1)
            .return_once(|_| {
                ok(r#"{
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
                }"#)
            });

        let bill = read(client).expect("the report is read");

        assert_eq!(bill.today(), Dollars::from_micro_dollars(12_400_000));
        assert_eq!(cost_on(&bill, 11, 15), 1_237_891 + 765_000);
        assert_eq!(cost_on(&bill, 11, 14), 0);
        assert_eq!(
            bill.this_month(),
            Dollars::from_micro_dollars(12_400_000 + 1_237_891 + 765_000)
        );
        assert_eq!(cost_on(&bill, 10, 31), 10_000_000);
        assert_eq!(bill.last_30_days_by_day().len(), 30);
    }

    #[test]
    fn every_page_is_read_while_the_report_has_more() {
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| request.url == FIRST_PAGE)
            .times(1)
            .return_once(|_| {
                ok(r#"{
                    "data": [{"starting_at": "2026-11-15T00:00:00Z", "results": [{"amount": "100"}]}],
                    "has_more": true,
                    "next_page": "page_MjAy+/="
                }"#)
            });
        client
            .expect_get()
            .withf(|request| {
                request.url == format!("{FIRST_PAGE}&page=page_MjAy%2B%2F%3D")
            })
            .times(1)
            .return_once(|_| {
                ok(r#"{
                    "data": [{"starting_at": "2026-11-16T00:00:00Z", "results": [{"amount": "250"}]}],
                    "has_more": false,
                    "next_page": null
                }"#)
            });

        let bill = read(client).expect("both pages are read");

        assert_eq!(cost_on(&bill, 11, 15), 1_000_000);
        assert_eq!(bill.today(), Dollars::from_micro_dollars(2_500_000));
    }

    #[test]
    fn a_report_that_never_ends_is_unexpected() {
        let mut client = MockHttpsGet::new();
        client.expect_get().times(MAX_PAGES).returning(|_| {
            ok(r#"{"data": [], "has_more": true, "next_page": "again"}"#)
        });

        assert_eq!(
            read(client),
            Err(SourceError::UnexpectedAnswer(String::from(
                "the cost report has too many pages"
            )))
        );
    }

    #[test]
    fn a_refused_key_is_key_refused() {
        assert_eq!(read(answering(401, "")), Err(SourceError::KeyRefused));
        assert_eq!(read(answering(403, "{}")), Err(SourceError::KeyRefused));
    }

    #[test]
    fn no_answer_or_another_answer_is_a_failure() {
        let mut offline = MockHttpsGet::new();
        offline
            .expect_get()
            .times(1)
            .return_once(|_| Err(String::from("connection refused")));

        assert_eq!(
            read(offline),
            Err(SourceError::Network(String::from("connection refused")))
        );
        assert_eq!(
            read(answering(500, "{}")),
            Err(SourceError::UnexpectedAnswer(String::from(
                "the server answered 500"
            )))
        );
        assert_eq!(
            read(answering(200, r#"{"error": "no data"}"#)),
            Err(SourceError::UnexpectedAnswer(String::from(
                "the answer is not a cost report"
            )))
        );
    }

    #[test]
    fn what_can_not_be_read_is_skipped() {
        let bill = read(answering(
            200,
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
        ))
        .expect("the readable parts are read");

        assert_eq!(bill.today(), Dollars::from_micro_dollars(3_255_000));
        assert_eq!(bill.this_month(), bill.today());
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
        let bill = read(answering(
            200,
            r#"{"data": [{"starting_at": "2026-11-16T00:00:00Z", "results": [{"amount": "-500"}]}]}"#,
        ))
        .expect("the report is read");

        assert_eq!(bill.today(), Dollars::default());
    }
}
