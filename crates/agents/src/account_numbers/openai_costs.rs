//! The real bill of an OpenAI organization: the costs of the Admin API,
//! `GET /v1/organization/costs`, read with an admin key, as OpenAI
//! documents it:
//! <https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage/methods/costs>.
//!
//! The days are UTC days, each bucket starts at `start_time` in seconds,
//! `amount.value` is in the currency `amount.currency` (US dollars), and
//! the pages are walked with `page` = `next_page` while `has_more` is
//! true. The range is the same as the Anthropic cost report's: 31 days up
//! to the end of today.

use std::time::SystemTime;

use jiff::ToSpan;
use jiff::tz::TimeZone;
use serde::Deserialize;
use serde_json::Value;

use super::anthropic_costs::{bill_of, day_index};
use crate::admin_key::AdminKey;
use crate::bill::Bill;
use crate::dollars::Dollars;
use crate::error::{SourceError, Unexpected};
use crate::https::{HttpsGet, HttpsRequest, json_answer, percent_encoded};
use crate::local_time::{LocalNow, days_before};
use crate::timestamp::time_from_seconds;

/// The address of the costs.
const COSTS_URL: &str = "https://api.openai.com/v1/organization/costs";
/// The days of the bill: today and the 30 days before it, which always
/// hold the first day of the month.
const DAYS: i32 = 31;
/// The most pages read: more is not what the code expects, and the walk
/// stops instead of asking for ever.
const MAX_PAGES: usize = 10;

/// The bill of the organization of the admin key `key`, of the 31 UTC days
/// up to `now`. A refused key is "key refused"; an answer that is not a
/// page of costs is unexpected. A day, or a cost, that can not be read is
/// skipped.
pub(super) fn bill(
    client: &dyn HttpsGet,
    key: &AdminKey,
    now: SystemTime,
) -> Result<Bill, SourceError> {
    let utc = LocalNow::new(now, TimeZone::UTC);
    let today = utc.today();
    let first_day = days_before(today, DAYS - 1);
    let start = seconds_of(utc.start_of(first_day));
    let end = seconds_of(utc.start_of(today.saturating_add(1.day())));
    let mut micro_dollars = [0_i64; DAYS as usize];
    let mut page: Option<String> = None;

    for _ in 0..MAX_PAGES {
        let request = request(key, start, end, page.as_deref());
        let answer =
            json_answer(client.get(&request)).map_err(|error| match error {
                SourceError::NotLoggedIn => SourceError::KeyRefused,
                other => other,
            })?;
        let costs: CostsPage =
            serde_json::from_value(answer).map_err(|_| {
                SourceError::UnexpectedAnswer(Unexpected::NotCostReport)
            })?;

        for bucket in &costs.data {
            let Some(day) = time_from_seconds(bucket.start_time)
                .map(|start| utc.date_of(start))
            else {
                continue;
            };
            let Some(index) = day_index(first_day, day) else {
                continue;
            };
            if let Some(sum) = micro_dollars.get_mut(index) {
                *sum = sum.saturating_add(bucket.micro_dollars());
            }
        }

        match costs.next_page() {
            Some(next) => page = Some(next.to_string()),
            None => return Ok(bill_of(first_day, &micro_dollars)),
        }
    }

    Err(SourceError::UnexpectedAnswer(Unexpected::TooManyPages))
}

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
    start_time: u64,
    #[serde(default)]
    results: Vec<CostResult>,
}

impl Bucket {
    /// The money of the day in micro-dollars: the sum of its costs in US
    /// dollars that can be read. A cost below zero, such as a refund, is
    /// left out.
    fn micro_dollars(&self) -> i64 {
        self.results
            .iter()
            .filter_map(CostResult::dollars)
            .filter_map(|dollars| i64::try_from(dollars.micro_dollars()).ok())
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
    /// The cost in US dollars; `None` for another currency or an amount
    /// that is not a number.
    fn dollars(&self) -> Option<Dollars> {
        let amount = self.amount.as_ref()?;
        let currency = amount
            .get("currency")
            .and_then(Value::as_str)
            .unwrap_or("usd");
        if !currency.eq_ignore_ascii_case("usd") {
            return None;
        }

        Dollars::from_dollars(amount.get("value")?.as_f64()?)
    }
}

/// The request of the page `page`, the first when `None`, of the costs of
/// the UTC days from `start` to `end`, in seconds since 1970.
fn request(
    key: &AdminKey,
    start: u64,
    end: u64,
    page: Option<&str>,
) -> HttpsRequest {
    let mut url = format!(
        "{COSTS_URL}?start_time={start}&end_time={end}&bucket_width=1d&limit={DAYS}"
    );
    if let Some(page) = page {
        let page = percent_encoded(page);
        url.push_str(&format!("&page={page}"));
    }

    HttpsRequest {
        url,
        headers: vec![
            ("Authorization", format!("Bearer {}", key.secret())),
            ("Accept", String::from("application/json")),
        ],
    }
}

/// `time` in whole seconds since 1970; 0 before it.
fn seconds_of(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::testing::{NOW, answering, at, dollars};

    /// A made-up admin key, never a real one.
    const MADE_UP_KEY: &str = "sk-admin-made-up-for-tests";
    /// The address of the first page for [`NOW`]: the 31 UTC days from
    /// 2026-10-17 to the end of 2026-11-16.
    const FIRST_PAGE: &str = "https://api.openai.com/v1/organization/costs?start_time=1792195200&end_time=1794873600&bucket_width=1d&limit=31";

    fn key() -> AdminKey {
        AdminKey::made_up(MADE_UP_KEY)
    }

    fn ok(body: &str) -> Result<HttpsAnswer, SourceError> {
        Ok(HttpsAnswer {
            status: 200,
            body: body.as_bytes().to_vec(),
        })
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

    fn cost_on(bill: &Bill, month: i8, day: i8) -> u64 {
        bill.days()
            .iter()
            .find(|cost| cost.date() == jiff::civil::date(2026, month, day))
            .map(|cost| cost.cost().micro_dollars())
            .expect("the day is in the bill")
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
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| {
                request.url == FIRST_PAGE
                    && request.headers
                        == [
                            ("Authorization", format!("Bearer {MADE_UP_KEY}")),
                            ("Accept", String::from("application/json")),
                        ]
            })
            .times(1)
            .return_once(move |_| ok(&body));

        let bill = bill(&client, &key(), at(NOW)).expect("a bill");

        assert_eq!(bill.days().len(), 31);
        assert_eq!(cost_on(&bill, 11, 1), 1_750_000);
        assert_eq!(bill.today(), dollars(2));
        assert_eq!(bill.this_month().micro_dollars(), 3_750_000);
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
        let mut client = MockHttpsGet::new();
        let mut sequence = mockall::Sequence::new();
        client
            .expect_get()
            .withf(|request| request.url == FIRST_PAGE)
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| ok(&first));
        client
            .expect_get()
            .withf(|request| {
                request.url == format!("{FIRST_PAGE}&page=page%202")
            })
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| ok(&second));

        let bill = bill(&client, &key(), at(NOW)).expect("a bill");

        assert_eq!(bill.this_month(), dollars(4));
    }

    #[test]
    fn another_currency_and_a_refund_are_left_out() {
        let body = r#"{"data": [{"start_time": 1794787200, "results": [
            {"amount": {"value": 5, "currency": "eur"}},
            {"amount": {"value": -1, "currency": "usd"}},
            {"amount": {"value": 2, "currency": "usd"}}
        ]}], "has_more": false}"#;
        let client = answering(200, body);

        let bill = bill(&client, &key(), at(NOW)).expect("a bill");

        assert_eq!(bill.today(), dollars(2));
    }

    #[test]
    fn a_refused_key_is_key_refused_and_another_answer_unexpected() {
        let refused = answering(401, "{}");
        let other = answering(200, r#"{"error": "nope"}"#);

        assert_eq!(
            bill(&refused, &key(), at(NOW)),
            Err(SourceError::KeyRefused)
        );
        assert_eq!(
            bill(&other, &key(), at(NOW)),
            Err(SourceError::UnexpectedAnswer(Unexpected::NotCostReport))
        );
    }

    #[test]
    fn too_many_pages_stop_the_walk() {
        let mut client = MockHttpsGet::new();
        client.expect_get().times(MAX_PAGES).returning(|_| {
            ok(r#"{"data": [], "has_more": true, "next_page": "again"}"#)
        });

        assert_eq!(
            bill(&client, &key(), at(NOW)),
            Err(SourceError::UnexpectedAnswer(Unexpected::TooManyPages))
        );
    }
}
