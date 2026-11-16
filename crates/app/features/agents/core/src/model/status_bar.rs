use otty_app_i18n::{self as i18n, Key};

use super::super::state::AgentsState;

/// The texts of the left side of the Agents mode status bar: the counts of
/// the projects and of their sessions, or that the profiles are read. The
/// frame adds the sessions button and the version on the right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusBarViewModel {
    /// The items, in order.
    pub(crate) items: Vec<String>,
}

impl StatusBarViewModel {
    /// Build the status bar texts for the current state.
    pub(in super::super) fn from_state(state: &AgentsState) -> Self {
        if !state.is_loaded() {
            return Self {
                items: vec![i18n::t(Key::AgentsLoading).to_string()],
            };
        }

        let projects = state.projects();
        let sessions = projects
            .iter()
            .map(|project| project.sessions().len())
            .sum();

        Self {
            items: vec![
                count(
                    Key::AgentsStatusOneProject,
                    Key::TplAgentsStatusProjects,
                    projects.len(),
                ),
                count(
                    Key::AgentsStatusOneSession,
                    Key::TplAgentsStatusSessions,
                    sessions,
                ),
            ],
        }
    }
}

/// `one` for a single item, else `many` with the count, such as "1
/// project" or "7 projects".
fn count(one: Key, many: Key, count: usize) -> String {
    if count == 1 {
        return i18n::t(one).to_string();
    }

    i18n::with_count(many, count as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, claude, connected, env, read, session,
        two_agents_with_sessions, with_profiles,
    };

    #[test]
    fn the_first_read_shows_that_it_loads() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(
            StatusBarViewModel::from_state(&state).items,
            [i18n::t(Key::AgentsLoading)]
        );
    }

    #[test]
    fn the_bar_counts_the_projects_and_their_sessions() {
        let state = two_agents_with_sessions();

        assert_eq!(
            StatusBarViewModel::from_state(&state).items,
            [
                i18n::with_count(Key::TplAgentsStatusProjects, 2),
                i18n::with_count(Key::TplAgentsStatusSessions, 3),
            ]
        );
    }

    #[test]
    fn one_project_and_one_session_are_counted_in_the_singular() {
        let mut state = with_profiles(&[(claude(), connected())]);
        read(&mut state, &claude(), vec![session("c1", "/work/otty", 1)]);

        assert_eq!(
            StatusBarViewModel::from_state(&state).items,
            [
                i18n::t(Key::AgentsStatusOneProject),
                i18n::t(Key::AgentsStatusOneSession),
            ]
        );
    }
}
