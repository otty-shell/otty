//! Adds up the answers found in the session files into [`Spend`]: each
//! answer once, on its day in the user's time zone, under its model and
//! its project.
//!
//! [`whole_history`] adds up the same answers into the [`ProjectStats`] of
//! a project over its whole history; it lives here because it counts each
//! answer once by the same rule, `once_each`.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::time::SystemTime;

use jiff::ToSpan;
use jiff::civil::Date;

use crate::account::AccountIdentity;
use crate::local_time::{LocalNow, days_before};
use crate::provider::Provider;
use crate::spend::{
    COUNTED_DAYS, DaySpend, HourSpend, ModelTokens, ProjectModelTokens,
    ProjectTokens, Spend, sort_models,
};
use crate::spend_files::{CountedAnswer, FileSpend};
use crate::stats::ProjectStats;
use crate::token_count::TokenCount;

/// What the tokens of the answers are added up by: the provider, the
/// account and the model.
type ModelKey<'a> = (
    Option<Provider>,
    Option<&'a AccountIdentity>,
    Option<&'a str>,
);

/// The tokens of each model in each project, by the project and the
/// [`ModelKey`].
type Parts<'a> = HashMap<(Option<&'a Path>, ModelKey<'a>), ModelTokens>;

/// Hours in a day: today is split into this many hours.
const HOURS: i8 = 24;

/// The first day whose tokens are counted at `now`: 30 days back with
/// today, or the first day of the current month when it is earlier, which
/// happens only on the 31st day of a month.
pub(crate) fn first_counted_day(now: &LocalNow) -> Date {
    let today = now.today();
    let thirty_days_back = days_before(today, COUNTED_DAYS - 1);

    thirty_days_back.min(today.first_of_month())
}

/// The tokens of the answers in `files`, by day from
/// [`first_counted_day`] to today, by provider, account and model, and by
/// project for the last 30 days. The answers of a file that names no
/// provider are counted under `profile_provider`.
///
/// The same answer can be written in more than one file, or more than
/// once in one file: the answers with the same identity are counted once,
/// the one with the most output tokens, which is the last and complete
/// record of the answer. An answer outside the counted days is left out.
pub(crate) fn tally<'a>(
    files: impl Iterator<Item = &'a FileSpend>,
    now: &LocalNow,
    profile_provider: Option<Provider>,
) -> Spend {
    let today = now.today();
    let first_day = first_counted_day(now);
    let first_project_day = days_before(today, COUNTED_DAYS - 1);

    let answers = once_each(files);

    let mut days: BTreeMap<Date, (HashMap<ModelKey, ModelTokens>, Parts)> =
        first_day
            .series(1.day())
            .take_while(|day| *day <= today)
            .map(|day| (day, (HashMap::new(), HashMap::new())))
            .collect();
    let mut hours: Vec<Parts> = (0..HOURS).map(|_| HashMap::new()).collect();
    let mut projects: HashMap<&Path, TokenCount> = HashMap::new();
    for (file, answer) in answers {
        let day = now.date_of(answer.at());
        let Some((models, parts)) = days.get_mut(&day) else {
            continue;
        };

        count_answer(models, file, answer, profile_provider);
        count_part(parts, file, answer, profile_provider);
        if day == today {
            let hour = now.time_of_day(answer.at()).hour();
            if let Some(parts) = hours.get_mut(hour.unsigned_abs() as usize) {
                count_part(parts, file, answer, profile_provider);
            }
        }
        if let Some(project) = file.project()
            && day >= first_project_day
        {
            *projects.entry(project).or_default() += answer.tokens();
        }
    }

    let days = days
        .into_iter()
        .map(|(day, (models, parts))| {
            day_spend(day, models).with_parts(sorted_parts(parts))
        })
        .collect();
    let hours = (0..HOURS)
        .zip(hours)
        .map(|(hour, parts)| HourSpend::new(hour, sorted_parts(parts)))
        .collect();

    Spend::new(days, sorted_projects(projects)).with_hours(hours)
}

/// What the answers of `files` tell over the whole history, each answer
/// once as in [`tally`]: when the first and the last were written, and the
/// tokens and requests of each provider, account and model, the most
/// first, with the part spent in requests with a long prompt. The answers
/// of a file that names no provider are counted under `profile_provider`.
pub(crate) fn whole_history<'a>(
    files: impl Iterator<Item = &'a FileSpend>,
    profile_provider: Option<Provider>,
) -> ProjectStats {
    let answers = once_each(files);

    let mut by_model: HashMap<ModelKey, ModelTokens> = HashMap::new();
    let mut first_at = None;
    let mut last_at = None;
    for (file, answer) in answers {
        count_answer(&mut by_model, file, answer, profile_provider);
        first_at =
            Some(first_at.map_or(answer.at(), |first: SystemTime| {
                first.min(answer.at())
            }));
        last_at = last_at.max(Some(answer.at()));
    }

    let mut models: Vec<ModelTokens> = by_model.into_values().collect();
    sort_models(&mut models);
    ProjectStats::new(first_at, last_at, models)
}

/// The answers of `files` with their file, each answer once: of the
/// answers with the same identity, the one with the most output tokens.
/// An answer with no identity is always counted.
fn once_each<'a>(
    files: impl Iterator<Item = &'a FileSpend>,
) -> Vec<(&'a FileSpend, &'a CountedAnswer)> {
    let mut by_identity: HashMap<u64, (&FileSpend, &CountedAnswer)> =
        HashMap::new();
    let mut answers = Vec::new();

    for file in files {
        for answer in file.answers() {
            let Some(identity) = answer.identity() else {
                answers.push((file, answer));
                continue;
            };

            let is_better =
                by_identity.get(&identity).is_none_or(|(_, kept)| {
                    answer.tokens().output() > kept.tokens().output()
                });
            if is_better {
                by_identity.insert(identity, (file, answer));
            }
        }
    }

    answers.extend(by_identity.into_values());
    answers
}

/// Counts `answer` of `file` among `models`, under its provider, account
/// and model. A file that names no provider is counted under
/// `profile_provider`.
fn count_answer<'a>(
    models: &mut HashMap<ModelKey<'a>, ModelTokens>,
    file: &'a FileSpend,
    answer: &'a CountedAnswer,
    profile_provider: Option<Provider>,
) {
    let provider = file.provider().or(profile_provider);
    let key = (provider, file.account(), answer.model());

    models
        .entry(key)
        .or_insert_with(|| {
            ModelTokens::new(answer.model(), TokenCount::default(), 0)
                .with_provider(provider)
                .with_account(file.account().cloned())
        })
        .count_request(answer.tokens(), prompt_of(answer), answer.cost());
}

/// Counts `answer` of `file` among `parts`, under its project, provider,
/// account and model, as [`count_answer`] does.
fn count_part<'a>(
    parts: &mut Parts<'a>,
    file: &'a FileSpend,
    answer: &'a CountedAnswer,
    profile_provider: Option<Provider>,
) {
    let provider = file.provider().or(profile_provider);
    let key = (file.project(), (provider, file.account(), answer.model()));

    parts
        .entry(key)
        .or_insert_with(|| {
            ModelTokens::new(answer.model(), TokenCount::default(), 0)
                .with_provider(provider)
                .with_account(file.account().cloned())
        })
        .count_request(answer.tokens(), prompt_of(answer), answer.cost());
}

/// The size of the prompt of `answer`, which tells whether the request
/// had a long prompt: the input sent, the input written to the cache and
/// the input read from it. Both price pages count it so: Anthropic's for
/// Claude Haiku 5.5, OpenAI's for the long context of its models.
fn prompt_of(answer: &CountedAnswer) -> u64 {
    let tokens = answer.tokens();

    tokens.input().saturating_add(tokens.cached())
}

/// The spend of the day `date` from the tokens of its models.
fn day_spend(date: Date, models: HashMap<ModelKey, ModelTokens>) -> DaySpend {
    let mut models: Vec<ModelTokens> = models.into_values().collect();
    sort_models(&mut models);

    DaySpend::new(date, models)
}

/// The tokens of each model in each project, the most first; parts with
/// as many tokens by their project, then as [`sort_models`] orders them.
fn sorted_parts(parts: Parts) -> Vec<ProjectModelTokens> {
    let mut parts: Vec<ProjectModelTokens> = parts
        .into_iter()
        .map(|((project, _), tokens)| {
            ProjectModelTokens::new(project.map(Path::to_path_buf), tokens)
        })
        .collect();

    parts.sort_by(|a, b| {
        let (a_tokens, b_tokens) = (a.tokens(), b.tokens());
        b_tokens
            .tokens()
            .total()
            .cmp(&a_tokens.tokens().total())
            .then_with(|| b.project().is_some().cmp(&a.project().is_some()))
            .then_with(|| a.project().cmp(&b.project()))
            .then_with(|| a_tokens.model().cmp(&b_tokens.model()))
            .then_with(|| a_tokens.provider().cmp(&b_tokens.provider()))
            .then_with(|| a_tokens.account().cmp(&b_tokens.account()))
    });
    parts
}

/// The projects the most tokens first; projects with as many tokens by
/// their folder.
fn sorted_projects(projects: HashMap<&Path, TokenCount>) -> Vec<ProjectTokens> {
    let mut projects: Vec<ProjectTokens> = projects
        .into_iter()
        .map(|(folder, tokens)| {
            ProjectTokens::new(folder.to_path_buf(), tokens)
        })
        .collect();

    projects.sort_by(|a, b| {
        b.tokens()
            .total()
            .cmp(&a.tokens().total())
            .then_with(|| a.folder().cmp(b.folder()))
    });
    projects
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use jiff::tz::TimeZone;

    use super::*;
    use crate::account::AccountIdentity;
    use crate::testing::{NOW, at, berlin, hours_east, utc_now};

    /// 2026-11-16T10:00:00Z, a Monday.
    const DAY: u64 = 24 * 3600;

    fn input(count: u64) -> TokenCount {
        TokenCount::new(count, 0, 0)
    }

    fn answer(seconds: u64, model: &str, tokens: TokenCount) -> CountedAnswer {
        CountedAnswer::new(at(seconds), Some(model), tokens)
    }

    fn file(project: &str, answers: Vec<CountedAnswer>) -> FileSpend {
        FileSpend::new(Some(PathBuf::from(project)), answers)
    }

    fn date(month: i8, day: i8) -> Date {
        jiff::civil::date(2026, month, day)
    }

    #[test]
    fn answers_are_added_by_day_model_and_project() {
        let files = [
            file(
                "/home/user/otty",
                vec![
                    answer(NOW, "opus", TokenCount::new(10, 100, 1)),
                    answer(NOW - 60, "sonnet", TokenCount::new(20, 0, 2)),
                    answer(NOW - DAY, "opus", TokenCount::new(30, 0, 3)),
                ],
            ),
            file(
                "/home/user/docs",
                vec![answer(NOW - 8 * DAY, "opus", TokenCount::new(5, 5, 5))],
            ),
        ];

        let spend = tally(files.iter(), &utc_now(), None);

        assert_eq!(spend.today(), TokenCount::new(30, 100, 3));
        assert_eq!(spend.last_7_days(), TokenCount::new(60, 100, 6));
        assert_eq!(spend.last_30_days(), TokenCount::new(65, 105, 11));
        let bars = spend.last_30_days_by_day();
        assert_eq!(bars.len(), 30);
        assert_eq!(bars[29].date(), date(11, 16));
        assert_eq!(
            bars[29].models(),
            [
                ModelTokens::new(Some("opus"), TokenCount::new(10, 100, 1), 1),
                ModelTokens::new(Some("sonnet"), TokenCount::new(20, 0, 2), 1),
            ]
        );
        assert_eq!(bars[28].tokens(), TokenCount::new(30, 0, 3));
        assert_eq!(bars[21].date(), date(11, 8));
        assert_eq!(bars[21].tokens(), TokenCount::new(5, 5, 5));
        assert_eq!(
            spend.projects(),
            [
                ProjectTokens::new(
                    PathBuf::from("/home/user/otty"),
                    TokenCount::new(60, 100, 6)
                ),
                ProjectTokens::new(
                    PathBuf::from("/home/user/docs"),
                    TokenCount::new(5, 5, 5)
                ),
            ]
        );
    }

    fn part(
        project: Option<&str>,
        model: &str,
        tokens: TokenCount,
        requests: u64,
    ) -> ProjectModelTokens {
        ProjectModelTokens::new(
            project.map(PathBuf::from),
            ModelTokens::new(Some(model), tokens, requests),
        )
    }

    #[test]
    fn each_day_and_each_hour_of_today_are_split_by_project_and_model() {
        let otty = Some("/home/user/otty");
        let docs = Some("/home/user/docs");
        let files = [
            file(
                "/home/user/otty",
                vec![
                    answer(NOW, "opus", input(10)),
                    answer(NOW - 60, "opus", input(5)),
                    answer(NOW - 3600, "sonnet", input(20)),
                    answer(NOW - DAY, "opus", input(30)),
                ],
            ),
            file(
                "/home/user/docs",
                vec![answer(NOW - 7200, "opus", input(7))],
            ),
            FileSpend::new(None, vec![answer(NOW, "opus", input(1))]),
        ];

        let spend = tally(files.iter(), &utc_now(), None);

        let days = spend.last_30_days_by_day();
        assert_eq!(
            days[29].parts(),
            [
                part(otty, "sonnet", input(20), 1),
                part(otty, "opus", input(15), 2),
                part(docs, "opus", input(7), 1),
                part(None, "opus", input(1), 1),
            ]
        );
        assert_eq!(days[28].parts(), [part(otty, "opus", input(30), 1)]);
        assert!(days[27].parts().is_empty());
        let hours = spend.today_by_hour();
        assert_eq!(hours.len(), 24);
        assert_eq!(hours[0].hour(), 0);
        assert_eq!(hours[23].hour(), 23);
        assert_eq!(
            hours[10].parts(),
            [
                part(otty, "opus", input(10), 1),
                part(None, "opus", input(1), 1)
            ]
        );
        assert_eq!(
            hours[9].parts(),
            [
                part(otty, "sonnet", input(20), 1),
                part(otty, "opus", input(5), 1)
            ]
        );
        assert_eq!(hours[8].parts(), [part(docs, "opus", input(7), 1)]);
        assert!(hours[11].parts().is_empty());
    }

    #[test]
    fn the_hours_of_today_follow_the_users_clock() {
        let files = [file("/a", vec![answer(NOW, "m", input(1))])];

        let in_moscow =
            tally(files.iter(), &LocalNow::new(at(NOW), hours_east(3)), None);

        let hours = in_moscow.today_by_hour();
        assert_eq!(hours[13].parts(), [part(Some("/a"), "m", input(1), 1)]);
        assert_eq!(hours[13].tokens(), input(1));
    }

    #[test]
    fn days_follow_the_users_midnight() {
        // 22:30 and 23:30 UTC on the 15th: the 15th in UTC, the 16th in
        // Moscow (UTC+3).
        let late = NOW - 11 * 3600 - 1800;
        let later = NOW - 10 * 3600 - 1800;
        let files = [file(
            "/a",
            vec![answer(late, "m", input(1)), answer(later, "m", input(2))],
        )];

        let in_utc = tally(files.iter(), &utc_now(), None);
        let in_moscow =
            tally(files.iter(), &LocalNow::new(at(NOW), hours_east(3)), None);

        assert_eq!(in_utc.today(), TokenCount::default());
        assert_eq!(in_utc.last_7_days(), input(3));
        assert_eq!(in_moscow.today(), input(3));
    }

    #[test]
    fn a_summer_day_keeps_its_summer_time_when_now_is_in_winter() {
        // 00:15 on 2026-10-21 in Berlin, on summer time (UTC+2), is 22:15
        // UTC on the 20th. Now, on 2026-11-16, Berlin is on winter time
        // (UTC+1), which would put the answer at 23:15 on the 20th.
        let just_after_midnight = NOW - 26 * DAY - 11 * 3600 - 45 * 60;
        let files =
            [file("/a", vec![answer(just_after_midnight, "m", input(1))])];

        let spend =
            tally(files.iter(), &LocalNow::new(at(NOW), berlin()), None);

        let bars = spend.last_30_days_by_day();
        assert_eq!(bars[3].date(), date(10, 21));
        assert_eq!(bars[3].tokens(), input(1));
        assert_eq!(bars[2].tokens(), TokenCount::default());
    }

    #[test]
    fn answers_older_than_30_days_or_in_the_future_are_left_out() {
        let first_day_start = NOW - 10 * 3600 - 29 * DAY;
        let files = [file(
            "/a",
            vec![
                answer(first_day_start, "m", input(1)),
                answer(first_day_start - 1, "m", input(10)),
                answer(NOW + DAY, "m", input(100)),
            ],
        )];

        let spend = tally(files.iter(), &utc_now(), None);

        assert_eq!(spend.last_30_days(), input(1));
        assert_eq!(spend.projects()[0].tokens(), input(1));
        assert_eq!(spend.last_30_days_by_day()[0].tokens(), input(1));
    }

    #[test]
    fn on_the_31st_the_month_reaches_one_day_past_the_30_days() {
        // 2026-12-31T12:00:00Z.
        let new_years_eve = 1_798_718_400;
        let first_of_december = new_years_eve - 30 * DAY;
        let files = [file(
            "/a",
            vec![
                answer(first_of_december, "m", input(1)),
                answer(new_years_eve, "m", input(2)),
            ],
        )];
        let now = LocalNow::new(at(new_years_eve), TimeZone::UTC);

        let spend = tally(files.iter(), &now, None);

        assert_eq!(spend.last_30_days(), input(2));
        assert_eq!(spend.projects()[0].tokens(), input(2));
        assert_eq!(
            spend.this_month_by_model(),
            [ModelTokens::new(Some("m"), input(3), 2)]
        );
        assert_eq!(first_counted_day(&now), date(12, 1));
    }

    #[test]
    fn the_first_counted_day_is_30_days_back_on_other_days() {
        assert_eq!(first_counted_day(&utc_now()), date(10, 18));
    }

    #[test]
    fn the_same_answer_is_counted_once_with_its_most_output() {
        let partial = answer(NOW, "opus", TokenCount::new(10, 0, 1))
            .with_identity(&("msg_1", "req_1"));
        let complete = answer(NOW, "opus", TokenCount::new(10, 0, 50))
            .with_identity(&("msg_1", "req_1"));
        let copy = complete.clone();
        let other = answer(NOW, "opus", TokenCount::new(1, 0, 1))
            .with_identity(&("msg_2", "req_2"));
        let unnamed = answer(NOW, "opus", TokenCount::new(1, 0, 1));
        let files = [
            file("/a", vec![partial, complete, other, unnamed.clone()]),
            file("/a", vec![copy, unnamed]),
        ];

        let spend = tally(files.iter(), &utc_now(), None);

        // The complete answer once, the other one, and both answers with
        // no identity.
        assert_eq!(spend.today(), TokenCount::new(13, 0, 53));
        assert_eq!(spend.last_30_days_by_day()[29].models()[0].requests(), 4);
    }

    #[test]
    fn the_cost_the_agent_wrote_is_added_up_by_day_hour_and_project() {
        let micro = crate::dollars::Dollars::from_micro_dollars;
        let costly = |seconds, cost| {
            answer(seconds, "gpt-5", input(10)).with_cost(Some(micro(cost)))
        };
        let files = [file(
            "/a",
            vec![
                costly(NOW, 300),
                costly(NOW - 60, 200),
                costly(NOW - DAY, 7),
            ],
        )];

        let spend = tally(files.iter(), &utc_now(), None);
        let history = whole_history(files.iter(), None);

        let today = &spend.last_30_days_by_day()[29];
        assert_eq!(today.models()[0].cost(), Some(micro(500)));
        assert_eq!(today.parts()[0].tokens().cost(), Some(micro(500)));
        let hours = spend.today_by_hour();
        assert_eq!(hours[9].parts()[0].tokens().cost(), Some(micro(200)));
        assert_eq!(hours[10].parts()[0].tokens().cost(), Some(micro(300)));
        assert_eq!(history.models()[0].cost(), Some(micro(507)));
    }

    #[test]
    fn a_request_over_a_prompt_limit_is_counted_as_a_long_prompt_of_it() {
        // The prompt is the input and the cached input of one request.
        let under = TokenCount::new(60_000, 40_000, 7);
        let over = TokenCount::new(60_000, 40_001, 9).with_cache_written(5);
        let longer = TokenCount::new(200_000, 72_001, 2);
        let files = [file(
            "/a",
            vec![
                answer(NOW, "m", under),
                answer(NOW, "m", over),
                answer(NOW, "m", longer),
            ],
        )];

        let spend = tally(files.iter(), &utc_now(), None);

        let model = &spend.last_30_days_by_day()[29].models()[0];
        assert_eq!(model.long_prompts_over(100_000), Some(over + longer));
        assert_eq!(model.long_prompts_over(272_000), Some(longer));
        assert_eq!(
            model.long_prompts_over(512_000),
            Some(TokenCount::default())
        );
        assert_eq!(model.tokens(), under + over + longer);
    }

    #[test]
    fn a_file_with_no_project_counts_by_day_and_model_only() {
        let files = [FileSpend::new(None, vec![answer(NOW, "m", input(4))])];

        let spend = tally(files.iter(), &utc_now(), None);

        assert_eq!(spend.today(), input(4));
        assert!(spend.projects().is_empty());
    }

    #[test]
    fn the_whole_history_counts_every_answer_once_by_model() {
        let complete =
            answer(NOW - 90 * DAY, "opus", TokenCount::new(10, 0, 9))
                .with_identity(&("msg_1", "req_1"));
        let partial = answer(NOW - 90 * DAY, "opus", TokenCount::new(10, 0, 1))
            .with_identity(&("msg_1", "req_1"));
        let long = TokenCount::new(272_001, 0, 2);
        let files = [
            file(
                "/a",
                vec![
                    partial,
                    complete.clone(),
                    answer(NOW, "gpt-6.1-sol", long),
                ],
            ),
            file("/a", vec![complete, answer(NOW - DAY, "opus", input(5))]),
        ];

        let history = whole_history(files.iter(), None);

        assert_eq!(history.first_at(), Some(at(NOW - 90 * DAY)));
        assert_eq!(history.last_at(), Some(at(NOW)));
        assert_eq!(
            history.models(),
            [
                ModelTokens::new(Some("gpt-6.1-sol"), long, 1)
                    .with_long_prompts(272_000, long)
                    .with_long_prompts(256_000, long)
                    .with_long_prompts(200_000, long)
                    .with_long_prompts(128_000, long)
                    .with_long_prompts(100_000, long)
                    .with_long_prompts(32_000, long),
                ModelTokens::new(Some("opus"), TokenCount::new(15, 0, 9), 2),
            ]
        );
    }

    #[test]
    fn no_answers_have_no_history() {
        let history = whole_history(std::iter::empty(), None);

        assert_eq!(history, ProjectStats::default());
    }

    #[test]
    fn the_tokens_of_each_provider_and_account_are_kept_apart() {
        let one = AccountIdentity::AccountId("made-up-one".into());
        let two = AccountIdentity::AccountId("made-up-two".into());
        let files = [
            file("/a", vec![answer(NOW, "gpt-5", input(1))])
                .with_provider(Some(Provider::OpenAI))
                .with_account(Some(one.clone())),
            file("/a", vec![answer(NOW, "gpt-5", input(2))])
                .with_provider(Some(Provider::OpenAI))
                .with_account(Some(two.clone())),
            file("/a", vec![answer(NOW, "gpt-5", input(4))]),
        ];

        let spend = tally(files.iter(), &utc_now(), None);

        let openai = |tokens, account: &AccountIdentity| {
            ModelTokens::new(Some("gpt-5"), tokens, 1)
                .with_provider(Some(Provider::OpenAI))
                .with_account(Some(account.clone()))
        };
        let expected = [
            ModelTokens::new(Some("gpt-5"), input(4), 1),
            openai(input(2), &two),
            openai(input(1), &one),
        ];
        assert_eq!(spend.last_30_days_by_day()[29].models(), expected);
        assert_eq!(spend.this_month_by_model(), expected);
        assert_eq!(spend.today(), input(7));
    }

    /// Claude Code does not write the provider, so its answers take the
    /// provider of the profile's login or server.
    #[test]
    fn a_file_with_no_provider_takes_the_profiles() {
        let files = [
            file("/a", vec![answer(NOW, "m", input(1))]),
            file("/a", vec![answer(NOW, "m", input(2))])
                .with_provider(Some(Provider::DeepSeek)),
        ];

        let spend = tally(files.iter(), &utc_now(), Some(Provider::Anthropic));
        let history = whole_history(files.iter(), Some(Provider::Anthropic));

        let expected = [
            ModelTokens::new(Some("m"), input(2), 1)
                .with_provider(Some(Provider::DeepSeek)),
            ModelTokens::new(Some("m"), input(1), 1)
                .with_provider(Some(Provider::Anthropic)),
        ];
        assert_eq!(spend.last_30_days_by_day()[29].models(), expected);
        assert_eq!(history.models(), expected);
    }

    #[test]
    fn no_answers_give_30_empty_days() {
        let spend = tally(std::iter::empty(), &utc_now(), None);

        assert_eq!(spend.last_30_days_by_day().len(), 30);
        assert!(spend.last_30_days().is_zero());
        assert_eq!(spend.last_30_days_by_day()[0].models(), []);
    }
}
