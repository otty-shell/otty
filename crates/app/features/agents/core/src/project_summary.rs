//! What the tokens in the statistics dialog cost, and the statistics of
//! the selected project made from what the files of each profile told:
//! the tokens of every profile cost their money at API prices, on a
//! subscription too (spec 2026-11-23-accounts-apart-from-profiles,
//! Decision 3).

use otty_agents::{MoneyTotal, Profile, ProjectStats};

use super::state::AgentsState;

/// What the tokens of the profiles of a project cost in the statistics
/// dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatsCost {
    /// No prices were read.
    NoPrices,
    /// What the tokens cost at API prices, or as the agent wrote it.
    Priced(MoneyTotal),
}

/// The statistics of the selected project, made from what the files of
/// each profile told: every profile together, the sessions of the project
/// list and what the tokens of every profile cost.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProjectSummary {
    all: ProjectStats,
    sessions: usize,
    cost: StatsCost,
}

impl ProjectSummary {
    /// What every profile told together.
    pub(crate) fn all(&self) -> &ProjectStats {
        &self.all
    }

    /// How many sessions the project list has in the project.
    pub(crate) fn sessions(&self) -> usize {
        self.sessions
    }

    /// What the tokens of every profile cost together.
    pub(crate) fn cost(&self) -> StatsCost {
        self.cost
    }

    /// The summary of `parts`, what each profile's files told about the
    /// selected project, at the prices of `state`.
    pub(crate) fn new(
        state: &AgentsState,
        parts: &[(Profile, ProjectStats)],
    ) -> Self {
        let profiles = state.profiles();
        let project = state.selected_project();

        let cost = match profiles.prices().table() {
            Some(prices) => StatsCost::Priced(
                parts
                    .iter()
                    .map(|(_, part)| {
                        MoneyTotal::of_models(
                            part.models(),
                            prices,
                            state.registry(),
                        )
                    })
                    .fold(MoneyTotal::default(), |sum, part| sum + part),
            ),
            None => StatsCost::NoPrices,
        };

        Self {
            all: ProjectStats::merged(parts.iter().map(|(_, part)| part)),
            sessions: project.map_or(0, |project| project.sessions().len()),
            cost,
        }
    }
}
