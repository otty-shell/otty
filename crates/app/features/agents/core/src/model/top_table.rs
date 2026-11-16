//! The table in the right column of a profile's dashboard: "Top projects,
//! 30 days" for a subscription or credits, "By model, this month" for an
//! API key (spec Decision 6).

use otty_agents::{AgentSession, MoneyEstimate, Spend};
use otty_app_i18n::{self as i18n, Key};

use super::folder_name;
use super::numbers::{dollars_text, short_count};

/// How many projects the top projects table lists.
const TOP_PROJECTS: usize = 5;

/// The table of the right column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TopTableViewModel {
    /// The projects that spent the most tokens in the last 30 days.
    Projects(Vec<TopProjectRowViewModel>),
    /// The money of each model this month.
    Models(Vec<ModelRowViewModel>),
}

/// One project of the top projects table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TopProjectRowViewModel {
    /// The folder name of the project.
    pub(crate) name: String,
    /// How many sessions of the profile ran in it.
    pub(crate) sessions: String,
    /// The tokens it spent in the last 30 days, written short.
    pub(crate) tokens: String,
}

/// One model of the by model table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelRowViewModel {
    /// The model id, or that the files did not name it.
    pub(crate) model: String,
    /// Its input tokens, the cached ones too, written short.
    pub(crate) input: String,
    /// Its output tokens, written short.
    pub(crate) output: String,
    /// What they cost, or "no price".
    pub(crate) cost: String,
}

/// The projects of `spend` that spent the most tokens, the most first,
/// each with the number of `sessions` of the profile in it; `None` when no
/// project spent any.
pub(crate) fn top_projects(
    spend: &Spend,
    sessions: &[AgentSession],
) -> Option<TopTableViewModel> {
    let rows: Vec<TopProjectRowViewModel> = spend
        .projects()
        .iter()
        .filter(|project| !project.tokens().is_zero())
        .take(TOP_PROJECTS)
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
        .collect();

    (!rows.is_empty()).then_some(TopTableViewModel::Projects(rows))
}

/// The models of `money` this month, the most money first; `None` when
/// none spent anything.
pub(crate) fn by_model(money: &MoneyEstimate) -> Option<TopTableViewModel> {
    let rows: Vec<ModelRowViewModel> = money
        .this_month_by_model()
        .iter()
        .map(|model| {
            let tokens = model.tokens().tokens();
            ModelRowViewModel {
                model: model.tokens().model().map_or_else(
                    || i18n::t(Key::AgentsUnnamedModel).to_string(),
                    String::from,
                ),
                input: short_count(tokens.input() + tokens.cached()),
                output: short_count(tokens.output()),
                cost: model.cost().map_or_else(
                    || i18n::t(Key::AgentsNoPrice).to_string(),
                    dollars_text,
                ),
            }
        })
        .collect();

    (!rows.is_empty()).then_some(TopTableViewModel::Models(rows))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        CalendarDate, DaySpend, ModelTokens, ProjectTokens, TokenCount,
    };

    use super::*;
    use crate::testing::session;

    #[test]
    fn the_projects_that_spent_the_most_come_first_with_their_sessions() {
        let project = |folder: &str, input: u64| {
            ProjectTokens::new(
                PathBuf::from(folder),
                TokenCount::new(input, 0, 0),
            )
        };
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

        let table = top_projects(&spend, &sessions);

        let row =
            |name: &str, sessions: &str, tokens: &str| TopProjectRowViewModel {
                name: String::from(name),
                sessions: String::from(sessions),
                tokens: String::from(tokens),
            };
        assert_eq!(
            table,
            Some(TopTableViewModel::Projects(vec![
                row("p7", "2", "7K"),
                row("p6", "0", "6K"),
                row("p5", "0", "5K"),
                row("p4", "0", "4K"),
                row("p3", "0", "3K"),
            ]))
        );
        assert_eq!(top_projects(&Spend::default(), &sessions), None);
    }

    #[test]
    fn each_model_shows_its_tokens_and_its_cost_or_that_it_has_no_price() {
        let today = DaySpend::new(
            CalendarDate::new(2026, 11, 16),
            vec![
                ModelTokens::new(
                    Some("gpt-5"),
                    TokenCount::new(1_000_000, 200_000, 3_000),
                    4,
                ),
                ModelTokens::new(None, TokenCount::new(10, 0, 1), 1),
            ],
        );
        let spend = Spend::new(vec![today], Vec::new());
        let money = MoneyEstimate::new(&spend, &otty_agents::bundled_prices());

        let table = by_model(&money);

        assert_eq!(
            table,
            Some(TopTableViewModel::Models(vec![
                ModelRowViewModel {
                    model: String::from("gpt-5"),
                    input: String::from("1.2M"),
                    output: String::from("3K"),
                    cost: String::from("$1.31"),
                },
                ModelRowViewModel {
                    model: i18n::t(Key::AgentsUnnamedModel).to_string(),
                    input: String::from("10"),
                    output: String::from("1"),
                    cost: i18n::t(Key::AgentsNoPrice).to_string(),
                },
            ]))
        );
        let empty = MoneyEstimate::new(
            &Spend::default(),
            &otty_agents::bundled_prices(),
        );
        assert_eq!(by_model(&empty), None);
    }
}
