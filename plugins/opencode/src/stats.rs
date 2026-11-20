//! The statistics of a session and of a project folder, read from the
//! database over the whole history. A session counts the tokens of the
//! helper agents it started, as the tokens of the profile do.

use std::collections::BTreeMap;

use otty_plugin_sdk::{
    Answer, HostError, ModelTokens, ProjectStats, ReadError, SessionStats,
    TokenCount,
};

use crate::database::{Database, SessionRow};
use crate::message::{Record, read_record};

/// The statistics of the session `session` in the database at `path`. A
/// session the database does not have is not found.
pub(crate) fn session_stats(
    path: &str,
    session: &str,
) -> Result<SessionStats, ReadError> {
    let missing = || {
        ReadError::Host(HostError::NotFound(format!("the session {session}")))
    };

    let database = Database::open(path)
        .map_err(ReadError::Host)?
        .ok_or_else(missing)?;
    let sessions = database.sessions().map_err(ReadError::Host)?;
    let Some(row) = sessions.iter().find(|row| row.id == session) else {
        return Err(missing());
    };
    let family = family_of(&sessions, &row.id);

    let mut questions = 0;
    let mut answers = Vec::new();
    database
        .for_each_message(0, |message| {
            if !family.contains(&message.session_id.as_str()) {
                return;
            }
            match read_record(&message) {
                Record::Question if message.session_id == row.id => {
                    questions += 1;
                },
                Record::Answer(answer) => answers.push(*answer),
                _ => {},
            }
        })
        .map_err(ReadError::Host)?;

    Ok(SessionStats {
        started_ms: Some(row.created_ms),
        last_active_ms: Some(row.updated_ms),
        messages: questions,
        models: models_of(&answers),
    })
}

/// The statistics of every session in the project folder `folder`, with
/// the sessions of their helper agents, in the database at `path`. No
/// database has none.
pub(crate) fn project_stats(
    path: &str,
    folder: &str,
) -> Result<ProjectStats, ReadError> {
    let Some(database) = Database::open(path).map_err(ReadError::Host)? else {
        return Ok(ProjectStats {
            first_ms: None,
            last_ms: None,
            models: Vec::new(),
        });
    };
    let sessions = database.sessions().map_err(ReadError::Host)?;
    let in_folder: Vec<&str> = sessions
        .iter()
        .filter(|row| row.directory.trim() == folder)
        .map(|row| row.id.as_str())
        .collect();

    let mut answers = Vec::new();
    database
        .for_each_message(0, |message| {
            if !in_folder.contains(&message.session_id.as_str()) {
                return;
            }
            if let Record::Answer(answer) = read_record(&message) {
                answers.push(*answer);
            }
        })
        .map_err(ReadError::Host)?;

    Ok(ProjectStats {
        first_ms: answers.iter().map(|answer| answer.at_ms).min(),
        last_ms: answers.iter().map(|answer| answer.at_ms).max(),
        models: models_of(&answers),
    })
}

/// The ids of the session `id` and of every session it started, and so on
/// down, among `sessions`.
fn family_of<'a>(sessions: &'a [SessionRow], id: &'a str) -> Vec<&'a str> {
    let mut family = vec![id];
    let mut index = 0;

    while let Some(parent) = family.get(index).copied() {
        let children: Vec<&str> = sessions
            .iter()
            .filter(|row| row.parent_id.as_deref() == Some(parent))
            .map(|row| row.id.as_str())
            .filter(|child| !family.contains(child))
            .collect();
        family.extend(children);
        index += 1;
    }

    family
}

/// The tokens and answers of each model of `answers`, the most tokens
/// first.
fn models_of(answers: &[Answer]) -> Vec<ModelTokens> {
    let mut by_model: BTreeMap<&str, ModelTokens> = BTreeMap::new();

    for answer in answers {
        let name = answer.model.as_deref().unwrap_or_default();
        let model = by_model.entry(name).or_insert_with(|| ModelTokens {
            model: name.to_string(),
            tokens: TokenCount::default(),
            answers: 0,
        });
        add_tokens(&mut model.tokens, &answer.tokens);
        model.answers += 1;
    }

    let mut models: Vec<ModelTokens> = by_model.into_values().collect();
    models.sort_by(|one, other| {
        total(&other.tokens)
            .cmp(&total(&one.tokens))
            .then_with(|| one.model.cmp(&other.model))
    });

    models
}

/// Adds the tokens `more` to `sum`.
fn add_tokens(sum: &mut TokenCount, more: &TokenCount) {
    sum.input = sum.input.saturating_add(more.input);
    sum.cached = sum.cached.saturating_add(more.cached);
    sum.cache_written = sum.cache_written.saturating_add(more.cache_written);
    sum.output = sum.output.saturating_add(more.output);
    sum.reasoning = sum.reasoning.saturating_add(more.reasoning);
}

/// All the tokens of `tokens`.
fn total(tokens: &TokenCount) -> u64 {
    tokens
        .input
        .saturating_add(tokens.cached)
        .saturating_add(tokens.output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{
        DataFolder, NOW, first_shape_answer, first_shape_question, millis,
    };

    /// A made-up profile: `ses_1` in `/home/user/otty`, with the helper
    /// session `ses_2`, and `ses_3` in `/home/user/site`.
    fn data_folder() -> DataFolder {
        let data = DataFolder::new();
        let mut made_up = data.first_shape();
        made_up.session(
            "ses_1",
            None,
            "/home/user/otty",
            "1.4.0",
            (NOW - 600, NOW),
        );
        made_up.session(
            "ses_2",
            Some("ses_1"),
            "/home/user/otty",
            "1.4.0",
            (NOW, NOW),
        );
        made_up.session("ses_3", None, "/home/user/site", "1.4.0", (NOW, NOW));
        let answer = |input| {
            first_shape_answer("openai", "gpt-5.5", (input, 1, 0, 0, 0), 0.5)
        };
        made_up.message("q_1", "ses_1", "", NOW - 600, &first_shape_question());
        made_up.message("q_2", "ses_1", "", NOW - 300, &first_shape_question());
        made_up.message("a_1", "ses_1", "", NOW - 590, &answer(10));
        made_up.message("q_3", "ses_2", "", NOW - 100, &first_shape_question());
        made_up.message("a_2", "ses_2", "", NOW - 90, &answer(20));
        made_up.message("a_3", "ses_3", "", NOW - 80, &answer(40));

        data
    }

    #[test]
    fn a_session_counts_its_questions_and_the_tokens_of_its_helpers() {
        let data = data_folder();
        let path = data.file("opencode.db");

        let stats = data
            .run(|| session_stats(&path, "ses_1"))
            .expect("the session is there");

        assert_eq!(stats.started_ms, Some(millis(NOW - 600)));
        assert_eq!(stats.last_active_ms, Some(millis(NOW)));
        assert_eq!(
            stats.messages, 2,
            "the helper's question is not the user's"
        );
        assert_eq!(stats.models.len(), 1);
        assert_eq!(stats.models[0].model, "gpt-5.5");
        assert_eq!(stats.models[0].answers, 2);
        assert_eq!(stats.models[0].tokens.input, 30);
    }

    #[test]
    fn a_session_the_database_does_not_have_is_not_found() {
        let data = data_folder();
        let path = data.file("opencode.db");

        let stats = data.run(|| session_stats(&path, "ses_9"));

        assert!(matches!(
            stats,
            Err(ReadError::Host(HostError::NotFound(_)))
        ));
    }

    #[test]
    fn a_project_counts_every_session_in_its_folder() {
        let data = data_folder();
        let path = data.file("opencode.db");

        let otty = data
            .run(|| project_stats(&path, "/home/user/otty"))
            .expect("the database is read");
        let site = data
            .run(|| project_stats(&path, "/home/user/site"))
            .expect("the database is read");

        assert_eq!(otty.models[0].tokens.input, 30);
        assert_eq!(otty.first_ms, Some(millis(NOW - 590)));
        assert_eq!(otty.last_ms, Some(millis(NOW - 90)));
        assert_eq!(site.models[0].tokens.input, 40);
    }
}
