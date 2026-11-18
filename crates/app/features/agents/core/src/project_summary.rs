//! What the tokens in the statistics dialog cost, and the statistics of
//! the selected project made from what the files of each profile told:
//! only a profile that pays with an API key has a cost (spec Decisions 9
//! and 14).

use otty_agents::{
    ModelTokens, MoneyTotal, PriceTable, Profile, ProjectStats, TokenCount,
};

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
/// each profile told: every profile together, and each profile with its
/// sessions in the project list, its tokens and their cost.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProjectSummary<'a> {
    all: ProjectStats,
    sessions: usize,
    cost: StatsCost,
    profiles: Vec<ProfileSummary<'a>>,
}

impl<'a> ProjectSummary<'a> {
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

    /// Each profile, in the order of the profiles.
    pub(crate) fn profiles(&self) -> &[ProfileSummary<'a>] {
        &self.profiles
    }

    /// The summary of `parts`, what each profile's files told about the
    /// selected project, at the prices of `state`.
    pub(crate) fn new(
        state: &AgentsState,
        parts: &'a [(Profile, ProjectStats)],
    ) -> Self {
        let profiles = state.profiles();
        let prices = profiles.prices().table();
        let project = state.selected_project();
        let sessions_of = |profile: &Profile| {
            project.map_or(0, |project| {
                project
                    .sessions()
                    .iter()
                    .filter(|entry| entry.profile() == profile)
                    .count()
            })
        };

        let rows: Vec<ProfileSummary<'a>> = parts
            .iter()
            .map(|(profile, part)| ProfileSummary {
                profile,
                sessions: sessions_of(profile),
                tokens: part.tokens(),
                cost: StatsCost::of(
                    part.models(),
                    profiles.is_api_key(profile),
                    prices,
                ),
            })
            .collect();
        let cost = rows
            .iter()
            .fold(StatsCost::Unpaid, |sum, row| sum.plus(row.cost));

        Self {
            all: ProjectStats::merged(parts.iter().map(|(_, part)| part)),
            sessions: project.map_or(0, |project| project.sessions().len()),
            cost,
            profiles: rows,
        }
    }
}

/// One profile of a project in its statistics.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfileSummary<'a> {
    profile: &'a Profile,
    sessions: usize,
    tokens: TokenCount,
    cost: StatsCost,
}

impl ProfileSummary<'_> {
    /// The profile.
    pub(crate) fn profile(&self) -> &Profile {
        self.profile
    }

    /// How many of its sessions the project list has in the project.
    pub(crate) fn sessions(&self) -> usize {
        self.sessions
    }

    /// The tokens its files tell.
    pub(crate) fn tokens(&self) -> TokenCount {
        self.tokens
    }

    /// What they cost.
    pub(crate) fn cost(&self) -> StatsCost {
        self.cost
    }
}
