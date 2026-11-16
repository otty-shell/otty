//! Adds up the answers found in the session files into [`Spend`]: each
//! answer once, on its day in the user's time zone, under its model and
//! its project.

use std::collections::HashMap;
use std::path::Path;

use crate::bundled_prices::long_prompt_limit;
use crate::local_time::{CalendarDate, LocalNow};
use crate::spend::{
    COUNTED_DAYS, DaySpend, ModelTokens, ProjectTokens, Spend, sort_models,
};
use crate::spend_files::{CountedAnswer, FileSpend};
use crate::token_count::TokenCount;

/// The number of the first day whose tokens are counted at `now`: 30 days
/// back with today, or the first day of the current month when it is
/// earlier, which happens only on the 31st day of a month.
pub(crate) fn first_counted_day(now: LocalNow) -> i64 {
    let today = now.today();
    let thirty_days_back = today.day_number() - (COUNTED_DAYS - 1);

    thirty_days_back.min(today.first_of_month().day_number())
}

/// The tokens of the answers in `files`, by day from
/// [`first_counted_day`] to today, by model, and by project for the last
/// 30 days.
///
/// The same answer can be written in more than one file, or more than
/// once in one file: the answers with the same identity are counted once,
/// the one with the most output tokens, which is the last and complete
/// record of the answer. An answer outside the counted days is left out.
pub(crate) fn tally<'a>(
    files: impl Iterator<Item = &'a FileSpend>,
    now: LocalNow,
) -> Spend {
    let today = now.day_of(now.now());
    let first_day = first_counted_day(now);
    let first_project_day = today - (COUNTED_DAYS - 1);

    let answers = once_each(files);

    let mut days: Vec<HashMap<Option<&str>, ModelTokens>> =
        vec![HashMap::new(); day_count(first_day, today)];
    let mut projects: HashMap<&Path, TokenCount> = HashMap::new();
    for (project, answer) in answers {
        let day = now.day_of(answer.at());
        if day < first_day || day > today {
            continue;
        }

        let index = usize::try_from(day - first_day).unwrap_or_default();
        if let Some(models) = days.get_mut(index) {
            models
                .entry(answer.model())
                .or_insert_with(|| {
                    ModelTokens::new(answer.model(), TokenCount::default(), 0)
                })
                .count_request(answer.tokens(), is_long_prompt(answer));
        }
        if let Some(project) = project
            && day >= first_project_day
        {
            *projects.entry(project).or_default() += answer.tokens();
        }
    }

    let days = days
        .into_iter()
        .zip(first_day..)
        .map(|(models, day)| day_spend(day, models))
        .collect();

    Spend::new(days, sorted_projects(projects))
}

/// The answers of `files` with their project, each answer once: of the
/// answers with the same identity, the one with the most output tokens.
/// An answer with no identity is always counted.
fn once_each<'a>(
    files: impl Iterator<Item = &'a FileSpend>,
) -> Vec<(Option<&'a Path>, &'a CountedAnswer)> {
    let mut by_identity: HashMap<u64, (Option<&Path>, &CountedAnswer)> =
        HashMap::new();
    let mut answers = Vec::new();

    for file in files {
        for answer in file.answers() {
            let Some(identity) = answer.identity() else {
                answers.push((file.project(), answer));
                continue;
            };

            let is_better =
                by_identity.get(&identity).is_none_or(|(_, kept)| {
                    answer.tokens().output() > kept.tokens().output()
                });
            if is_better {
                by_identity.insert(identity, (file.project(), answer));
            }
        }
    }

    answers.extend(by_identity.into_values());
    answers
}

/// How many days there are from `first_day` to `today`, both included.
fn day_count(first_day: i64, today: i64) -> usize {
    usize::try_from(today - first_day + 1).unwrap_or_default()
}

/// Whether `answer` had a long prompt: one over the limit above which its
/// model costs more, by the bundled prices.
///
/// The prompt is the input of the request with its cached part: the input
/// sent, the input written to the cache and the input read from it. Both
/// price pages count it so: Anthropic's for Claude Haiku 5.5, OpenAI's for
/// the long context of its models.
fn is_long_prompt(answer: &CountedAnswer) -> bool {
    let Some(limit) = answer.model().and_then(long_prompt_limit) else {
        return false;
    };
    let tokens = answer.tokens();

    tokens.input().saturating_add(tokens.cached()) > limit
}

/// The spend of the day numbered `day` from the tokens of its models.
fn day_spend(day: i64, models: HashMap<Option<&str>, ModelTokens>) -> DaySpend {
    let mut models: Vec<ModelTokens> = models.into_values().collect();
    sort_models(&mut models);

    DaySpend::new(CalendarDate::from_day_number(day), models)
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

    use super::*;
    use crate::testing::at;

    /// 2026-11-16T10:00:00Z, a Monday.
    const NOW: u64 = 1_794_823_200;
    const DAY: u64 = 24 * 3600;

    fn now() -> LocalNow {
        LocalNow::new(at(NOW), 0)
    }

    fn input(count: u64) -> TokenCount {
        TokenCount::new(count, 0, 0)
    }

    fn answer(seconds: u64, model: &str, tokens: TokenCount) -> CountedAnswer {
        CountedAnswer::new(at(seconds), Some(model), tokens)
    }

    fn file(project: &str, answers: Vec<CountedAnswer>) -> FileSpend {
        FileSpend::new(Some(PathBuf::from(project)), answers)
    }

    fn date(month: u8, day: u8) -> CalendarDate {
        CalendarDate::new(2026, month, day)
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

        let spend = tally(files.iter(), now());

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

        let in_utc = tally(files.iter(), LocalNow::new(at(NOW), 0));
        let in_moscow = tally(files.iter(), LocalNow::new(at(NOW), 3 * 3600));

        assert_eq!(in_utc.today(), TokenCount::default());
        assert_eq!(in_utc.last_7_days(), input(3));
        assert_eq!(in_moscow.today(), input(3));
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

        let spend = tally(files.iter(), now());

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
        let now = LocalNow::new(at(new_years_eve), 0);

        let spend = tally(files.iter(), now);

        assert_eq!(spend.last_30_days(), input(2));
        assert_eq!(spend.projects()[0].tokens(), input(2));
        assert_eq!(
            spend.this_month_by_model(),
            [ModelTokens::new(Some("m"), input(3), 2)]
        );
        assert_eq!(first_counted_day(now), date(12, 1).day_number());
    }

    #[test]
    fn the_first_counted_day_is_30_days_back_on_other_days() {
        assert_eq!(first_counted_day(now()), date(10, 18).day_number());
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

        let spend = tally(files.iter(), now());

        // The complete answer once, the other one, and both answers with
        // no identity.
        assert_eq!(spend.today(), TokenCount::new(13, 0, 53));
        assert_eq!(spend.last_30_days_by_day()[29].models()[0].requests(), 4);
    }

    #[test]
    fn a_request_over_the_prompt_limit_of_its_model_is_a_long_prompt() {
        // The prompt is the input and the cached input of one request:
        // 100,000 tokens for Claude Haiku 5.5, 272,000 for GPT-6.1 Sol.
        let haiku_under = TokenCount::new(60_000, 40_000, 7);
        let haiku_over =
            TokenCount::new(60_000, 40_001, 9).with_cache_written(5);
        let sol_under = TokenCount::new(272_000, 0, 1);
        let sol_over = TokenCount::new(200_000, 72_001, 2);
        let files = [file(
            "/a",
            vec![
                answer(NOW, "claude-haiku-5-5", haiku_under),
                answer(NOW, "claude-haiku-5-5", haiku_over),
                answer(NOW, "gpt-6.1-sol", sol_under),
                answer(NOW, "gpt-6.1-sol", sol_over),
                answer(NOW, "claude-opus-5-5", input(900_000)),
            ],
        )];

        let spend = tally(files.iter(), now());

        let today = &spend.last_30_days_by_day()[29];
        let long_prompts = |name: &str| {
            today
                .models()
                .iter()
                .find(|model| model.model() == Some(name))
                .map(ModelTokens::long_prompts)
        };
        assert_eq!(long_prompts("claude-haiku-5-5"), Some(haiku_over));
        assert_eq!(long_prompts("gpt-6.1-sol"), Some(sol_over));
        assert_eq!(
            long_prompts("claude-opus-5-5"),
            Some(TokenCount::default())
        );
        assert_eq!(today.tokens().input(), 60_000 + 60_000 + 472_000 + 900_000);
    }

    #[test]
    fn a_file_with_no_project_counts_by_day_and_model_only() {
        let files = [FileSpend::new(None, vec![answer(NOW, "m", input(4))])];

        let spend = tally(files.iter(), now());

        assert_eq!(spend.today(), input(4));
        assert!(spend.projects().is_empty());
    }

    #[test]
    fn no_answers_give_30_empty_days() {
        let spend = tally(std::iter::empty(), now());

        assert_eq!(spend.last_30_days_by_day().len(), 30);
        assert!(spend.last_30_days().is_zero());
        assert_eq!(spend.last_30_days_by_day()[0].models(), []);
    }
}
