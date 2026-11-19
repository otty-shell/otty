//! What the tokens in the statistics dialog cost, and the statistics of
//! the selected project made from what the files of each profile told:
//! only a profile that pays with an API key has a cost (spec Decisions 9
//! and 14).

use otty_agents::{ModelTokens, MoneyTotal, PriceTable, Profile, ProjectStats};

use super::state::AgentsState;

/// What the tokens of a profile, or of several, cost in the statistics
/// dialog: only a profile that pays with an API key has a cost (spec
/// Decision 9), and the Cost column shows while one does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatsCost {
    /// No cost: the profile does not pay per token.
    Unpaid,
    /// The profile pays per token, but no prices were read.
    NoPrices,
    /// What the tokens cost at the list prices.
    Priced(MoneyTotal),
}

impl StatsCost {
    /// Whether there is a cost to show: the table has a Cost column.
    pub(crate) fn is_paid(&self) -> bool {
        !matches!(self, Self::Unpaid)
    }

    /// The cost of `models` of a profile that `pays_per_token`, at
    /// `prices`.
    pub(crate) fn of(
        models: &[ModelTokens],
        pays_per_token: bool,
        prices: Option<&PriceTable>,
    ) -> Self {
        if !pays_per_token {
            return Self::Unpaid;
        }

        match prices {
            Some(prices) => Self::Priced(MoneyTotal::of_models(models, prices)),
            None => Self::NoPrices,
        }
    }

    /// This cost and `other` together: a profile that does not pay adds
    /// nothing.
    fn plus(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unpaid, cost) | (cost, Self::Unpaid) => cost,
            (Self::Priced(one), Self::Priced(other)) => {
                Self::Priced(one + other)
            },
            _ => Self::NoPrices,
        }
    }
}

/// The statistics of the selected project, made from what the files of
/// each profile told: every profile together, the sessions of the project
/// list and what the tokens of the profiles that pay per token cost.
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

    /// What the tokens of the profiles that pay per token cost together.
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
        let prices = profiles.prices().table();
        let project = state.selected_project();

        let cost =
            parts
                .iter()
                .fold(StatsCost::Unpaid, |sum, (profile, part)| {
                    let cost = StatsCost::of(
                        part.models(),
                        profiles.is_api_key(profile),
                        prices,
                    );
                    sum.plus(cost)
                });

        Self {
            all: ProjectStats::merged(parts.iter().map(|(_, part)| part)),
            sessions: project.map_or(0, |project| project.sessions().len()),
            cost,
        }
    }
}
