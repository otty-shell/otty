//! The table in the right column of a profile's dashboard: "Top projects,
//! 30 days", the same for every billing (spec Decisions 6 and 12).

use otty_agents::{AgentSession, Spend};

use super::folder_name;
use super::numbers::short_count;

/// How many projects the top projects table lists.
const TOP_PROJECTS: usize = 5;

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

/// The projects of `spend` that spent the most tokens, the most first,
/// each with the number of `sessions` of the profile in it; `None` when no
/// project spent any.
pub(crate) fn top_projects(
    spend: &Spend,
    sessions: &[AgentSession],
) -> Option<Vec<TopProjectRowViewModel>> {
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

    (!rows.is_empty()).then_some(rows)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{ProjectTokens, TokenCount};

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
            Some(vec![
                row("p7", "2", "7K"),
                row("p6", "0", "6K"),
                row("p5", "0", "5K"),
                row("p4", "0", "4K"),
                row("p3", "0", "3K"),
            ])
        );
        assert_eq!(top_projects(&Spend::default(), &sessions), None);
    }
}
