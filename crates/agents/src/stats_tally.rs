//! The statistics of one session and of one project folder, made by OTTY
//! from the answers of the session files (issue 22): a plugin gives the
//! answers as rows, and OTTY sums them with the same dedupe and the same
//! tokens by model as the totals.

use std::path::Path;
use std::time::SystemTime;

use crate::local_time::LocalNow;
use crate::session::SessionId;
use crate::spend_files::{CountedAnswer, FileSpend};
use crate::spend_tally::{models_of, once_each, tally};
use crate::stats::{ProjectStats, SessionStats};

/// The statistics of the session `session` from the answers of `files`
/// that name it: its first and last answer, the tokens of each model, and
/// its tokens by day and today by the hour up to `now`. An answer written
/// more than once counts once, as in the totals.
pub(crate) fn session_stats(
    files: &[&FileSpend],
    session: &SessionId,
    now: &LocalNow,
) -> SessionStats {
    let session = session.as_str();
    let files: Vec<FileSpend> = files
        .iter()
        .map(|file| file.only(|answer| answer.session() == Some(session)))
        .filter(|file| !file.answers().is_empty())
        .collect();

    let answers = once_each(files.iter());
    let (first_at, last_at) = first_and_last(&answers);
    let models = models_of(answers.iter().map(|(_, answer)| *answer));
    let spend = tally(files.iter(), now, None);

    SessionStats::new(first_at, last_at, models).with_spend(spend)
}

/// The statistics of the project `folder` from the answers of `files`
/// given in it, over the whole history: its first and last answer and the
/// tokens of each model. An answer written more than once counts once, as
/// in the totals.
pub(crate) fn project_stats(
    files: &[&FileSpend],
    folder: &Path,
) -> ProjectStats {
    let files: Vec<FileSpend> = files
        .iter()
        .map(|file| {
            file.only(|answer| {
                answer.project().or(file.project()) == Some(folder)
            })
        })
        .filter(|file| !file.answers().is_empty())
        .collect();

    let answers = once_each(files.iter());
    let (first_at, last_at) = first_and_last(&answers);
    let models = models_of(answers.iter().map(|(_, answer)| *answer));

    ProjectStats::new(first_at, last_at, models)
}

/// When the first and the last of `answers` were written; `None` for no
/// answers.
fn first_and_last(
    answers: &[(&FileSpend, &CountedAnswer)],
) -> (Option<SystemTime>, Option<SystemTime>) {
    let times = answers.iter().map(|(_, answer)| answer.at());

    (times.clone().min(), times.max())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::provider::ProviderRef;
    use crate::spend::ModelTokens;
    use crate::testing::{NOW, at, utc_now};
    use crate::token_count::TokenCount;

    const DAY: u64 = 24 * 3600;

    fn answer(
        session: &str,
        seconds: u64,
        model: &str,
        input: u64,
    ) -> CountedAnswer {
        CountedAnswer::new(
            at(seconds),
            Some(model),
            TokenCount::new(input, 0, 1),
        )
        .with_session(Some(session.to_string()))
    }

    fn file(project: &str, answers: Vec<CountedAnswer>) -> FileSpend {
        FileSpend::new(Some(PathBuf::from(project)), answers)
    }

    fn model(name: &str, input: u64, requests: u64) -> ModelTokens {
        ModelTokens::new(
            Some(name),
            TokenCount::new(input, 0, requests),
            requests,
        )
    }

    #[test]
    fn a_session_sums_the_answers_that_name_it() {
        let main = file(
            "/work/otty",
            vec![
                answer("s1", NOW - 600, "opus", 30),
                answer("s1", NOW - 60, "haiku", 5),
                answer("s2", NOW - 30, "opus", 1000),
            ],
        );
        let helper =
            file("/work/otty", vec![answer("s1", NOW - 300, "opus", 10)]);

        let stats =
            session_stats(&[&main, &helper], &SessionId::new("s1"), &utc_now());

        assert_eq!(stats.started_at(), Some(at(NOW - 600)));
        assert_eq!(stats.last_active_at(), Some(at(NOW - 60)));
        assert_eq!(
            stats.models(),
            [model("opus", 40, 2), model("haiku", 5, 1)]
        );
        assert_eq!(stats.answers(), 3);
        assert_eq!(stats.spend().today(), TokenCount::new(45, 0, 3));
    }

    /// Issue 22: an answer written twice in the files, such as one a
    /// resumed session copied, counts once in the panel as in the totals.
    #[test]
    fn an_answer_written_twice_counts_once_in_a_session_and_a_project() {
        let first = answer("s1", NOW, "opus", 30).with_identity(&"msg_1");
        let copy = first.clone();
        let other = answer("s1", NOW, "opus", 2);
        let files = [file("/a", vec![first, other]), file("/a", vec![copy])];
        let files: Vec<&FileSpend> = files.iter().collect();

        let session = session_stats(&files, &SessionId::new("s1"), &utc_now());
        let project = project_stats(&files, Path::new("/a"));
        let totals = tally(files.iter().copied(), &utc_now(), None);

        assert_eq!(session.tokens(), TokenCount::new(32, 0, 2));
        assert_eq!(project.tokens(), TokenCount::new(32, 0, 2));
        assert_eq!(totals.today(), session.tokens());
        assert_eq!(totals.projects()[0].tokens(), project.tokens());
    }

    #[test]
    fn a_session_no_answer_names_has_nothing() {
        let files = [file("/a", vec![answer("s1", NOW, "opus", 3)])];
        let files: Vec<&FileSpend> = files.iter().collect();

        let stats = session_stats(&files, &SessionId::new("gone"), &utc_now());

        assert_eq!(stats.started_at(), None);
        assert_eq!(stats.length(), None);
        assert!(stats.models().is_empty());
        assert!(stats.spend().today().is_zero());
    }

    #[test]
    fn a_project_sums_its_answers_over_the_whole_history() {
        let otty = file(
            "/work/otty",
            vec![
                answer("s1", NOW - 400 * DAY, "opus", 30),
                answer("s1", NOW, "opus", 10)
                    .with_project(Some(PathBuf::from("/work/docs"))),
            ],
        );
        let more =
            file("/work/otty", vec![answer("s2", NOW - DAY, "gpt-5", 4)]);
        let docs = file("/work/docs", vec![answer("s3", NOW, "opus", 99)]);
        let files = [&otty, &more, &docs];

        let project = project_stats(&files, Path::new("/work/otty"));

        assert_eq!(project.first_at(), Some(at(NOW - 400 * DAY)));
        assert_eq!(project.last_at(), Some(at(NOW - DAY)));
        assert_eq!(
            project.models(),
            [model("opus", 30, 1), model("gpt-5", 4, 1)]
        );
    }

    /// Spec Decision 6: the panel prices the tokens the same way as the
    /// totals, so it keeps each provider apart.
    #[test]
    fn the_tokens_of_each_provider_are_kept_apart_as_in_the_totals() {
        let files = [file(
            "/a",
            vec![
                answer("s1", NOW, "m", 1)
                    .with_provider(Some(ProviderRef::of_id("example-cloud"))),
                answer("s1", NOW, "m", 2),
            ],
        )];
        let files: Vec<&FileSpend> = files.iter().collect();

        let session = session_stats(&files, &SessionId::new("s1"), &utc_now());
        let totals = tally(files.iter().copied(), &utc_now(), None);

        assert_eq!(session.models(), totals.this_month_by_model());
    }
}
