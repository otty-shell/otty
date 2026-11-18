//! The top section of a profile's dashboard, "Top, last 30 days": the
//! projects or the models that spent the most tokens, the same for every
//! billing (spec Decisions 6, 12 and 13).

use otty_agents::{AgentSession, Spend};
use otty_app_i18n::{self as i18n, Key};

use super::super::top_tab::TopTab;
use super::folder_name;
use super::numbers::{digit_groups, short_count};

/// How many rows a tab of the top section lists at most.
pub(crate) const TOP_ROWS: usize = 5;

/// The top section: the chosen tab and its rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TopViewModel {
    /// The tab that is chosen.
    pub(crate) tab: TopTab,
    /// The rows of the chosen tab; none when it has nothing to list.
    pub(crate) rows: TopRowsViewModel,
}

/// The rows of the chosen tab of the top section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TopRowsViewModel {
    /// The projects that spent the most tokens, the most first.
    Projects(Vec<TopProjectRowViewModel>),
    /// The models that spent the most tokens, the most first.
    Models(Vec<TopModelRowViewModel>),
}

/// One project of the top section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TopProjectRowViewModel {
    /// The folder name of the project.
    pub(crate) name: String,
    /// How many sessions of the profile ran in it.
    pub(crate) sessions: String,
    /// The tokens it spent in the last 30 days, written short.
    pub(crate) tokens: String,
}

/// One model of the top section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TopModelRowViewModel {
    /// The model as the session files name it, or "Unknown model".
    pub(crate) name: String,
    /// How many requests it answered in the last 30 days.
    pub(crate) requests: String,
    /// The tokens it spent in the last 30 days, written short.
    pub(crate) tokens: String,
}

/// The top section of `spend` with the rows of `tab`, the projects with
/// the number of `sessions` of the profile in each; `None` when neither
/// tab has a row.
pub(crate) fn top(
    spend: &Spend,
    sessions: &[AgentSession],
    tab: TopTab,
) -> Option<TopViewModel> {
    let projects = top_projects(spend, sessions);
    let models = top_models(spend);

    if projects.is_empty() && models.is_empty() {
        return None;
    }

    let rows = match tab {
        TopTab::ByProject => TopRowsViewModel::Projects(projects),
        TopTab::ByModel => TopRowsViewModel::Models(models),
    };
    Some(TopViewModel { tab, rows })
}

/// The projects of `spend` that spent the most tokens, the most first,
/// each with the number of `sessions` of the profile in it.
fn top_projects(
    spend: &Spend,
    sessions: &[AgentSession],
) -> Vec<TopProjectRowViewModel> {
    spend
        .projects()
        .iter()
        .filter(|project| !project.tokens().is_zero())
        .take(TOP_ROWS)
        .map(|project| {
            let count = sessions
                .iter()
                .filter(|session| session.folder() == project.folder())
                .count();
            TopProjectRowViewModel {
                name: folder_name(project.folder()),
                sessions: format!("{count}"),
                tokens: short_count(project.tokens().total()),
            }
        })
        .collect()
}

/// The models of `spend` that spent the most tokens in the last 30 days,
/// the most first; a model the files did not name is "Unknown model".
fn top_models(spend: &Spend) -> Vec<TopModelRowViewModel> {
    spend
        .last_30_days_by_model()
        .iter()
        .filter(|model| !model.tokens().is_zero())
        .take(TOP_ROWS)
        .map(|model| TopModelRowViewModel {
            name: model
                .model()
                .unwrap_or(i18n::t(Key::AgentsUnknownModel))
                .to_string(),
            requests: digit_groups(model.requests()),
            tokens: short_count(model.tokens().total()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{DaySpend, ModelTokens, ProjectTokens, TokenCount};

    use super::*;
    use crate::testing::session;

    fn tokens(input: u64) -> TokenCount {
        TokenCount::new(input, 0, 0)
    }

    fn project(folder: &str, input: u64) -> ProjectTokens {
        ProjectTokens::new(PathBuf::from(folder), tokens(input))
    }

    fn model_row(
        name: &str,
        requests: &str,
        tokens: &str,
    ) -> TopModelRowViewModel {
        TopModelRowViewModel {
            name: String::from(name),
            requests: String::from(requests),
            tokens: String::from(tokens),
        }
    }

    /// The day `day` of `month` 2026 with `models`, each a name, its input
    /// tokens and its requests.
    fn day(
        month: i8,
        day: i8,
        models: &[(Option<&str>, u64, u64)],
    ) -> DaySpend {
        let models = models
            .iter()
            .map(|(model, input, requests)| {
                ModelTokens::new(*model, tokens(*input), *requests)
            })
            .collect();

        DaySpend::new(jiff::civil::date(2026, month, day), models)
    }

    #[test]
    fn the_projects_that_spent_the_most_come_first_with_their_sessions() {
        let projects = (1..=7)
            .map(|index| project(&format!("/work/p{index}"), 1_000 * index))
            .rev()
            .collect();
        let spend = Spend::new(Vec::new(), projects);
        let sessions = vec![
            session("a", "/work/p7", 1),
            session("b", "/work/p7", 2),
            session("c", "/work/p1", 3),
        ];

        let table = top(&spend, &sessions, TopTab::ByProject);

        let row =
            |name: &str, sessions: &str, tokens: &str| TopProjectRowViewModel {
                name: String::from(name),
                sessions: String::from(sessions),
                tokens: String::from(tokens),
            };
        assert_eq!(
            table,
            Some(TopViewModel {
                tab: TopTab::ByProject,
                rows: TopRowsViewModel::Projects(vec![
                    row("p7", "2", "7K"),
                    row("p6", "0", "6K"),
                    row("p5", "0", "5K"),
                    row("p4", "0", "4K"),
                    row("p3", "0", "3K"),
                ]),
            })
        );
        assert_eq!(top(&Spend::default(), &sessions, TopTab::ByProject), None);
    }

    /// Owner, issue 33: the models of the same 30 days as the tokens, the
    /// most first, at most as many as the projects, each with its
    /// requests; a model the files did not name is "Unknown model".
    #[test]
    fn the_models_of_the_last_30_days_that_spent_the_most_come_first() {
        let days = vec![
            day(10, 17, &[(Some("too-old"), 9_000_000, 1)]),
            day(
                10,
                31,
                &[
                    (Some("gpt-5"), 4_000, 2),
                    (Some("o3"), 3_000, 1),
                    (Some("o4-mini"), 500, 1),
                ],
            ),
            day(
                11,
                16,
                &[
                    (Some("gpt-5"), 1_500, 1_200),
                    (None, 2_000, 3),
                    (Some("gpt-5-codex"), 1_000, 1),
                    (Some("codex-mini"), 900, 1),
                ],
            ),
        ];
        let spend = Spend::new(days, Vec::new());

        let table = top(&spend, &[], TopTab::ByModel);

        assert_eq!(
            table,
            Some(TopViewModel {
                tab: TopTab::ByModel,
                rows: TopRowsViewModel::Models(vec![
                    model_row("gpt-5", "1 202", "5.5K"),
                    model_row("o3", "1", "3K"),
                    model_row(i18n::t(Key::AgentsUnknownModel), "3", "2K"),
                    model_row("gpt-5-codex", "1", "1K"),
                    model_row("codex-mini", "1", "900"),
                ]),
            }),
            "October 17 is out of the 30 days; October 31, in the month \
             before, is in; o4-mini is the sixth"
        );
    }

    /// Owner, issue 33: a tab with nothing to list keeps the section, so
    /// the other tab can be chosen.
    #[test]
    fn a_tab_with_nothing_to_list_has_no_rows_while_the_other_has_some() {
        let only_models = Spend::new(
            vec![day(11, 16, &[(Some("gpt-5"), 1_000, 1)])],
            Vec::new(),
        );
        let only_projects =
            Spend::new(Vec::new(), vec![project("/work/otty", 1_000)]);

        assert_eq!(
            top(&only_models, &[], TopTab::ByProject),
            Some(TopViewModel {
                tab: TopTab::ByProject,
                rows: TopRowsViewModel::Projects(Vec::new()),
            })
        );
        assert_eq!(
            top(&only_projects, &[], TopTab::ByModel),
            Some(TopViewModel {
                tab: TopTab::ByModel,
                rows: TopRowsViewModel::Models(Vec::new()),
            })
        );
        assert_eq!(top(&Spend::default(), &[], TopTab::ByModel), None);
    }
}
