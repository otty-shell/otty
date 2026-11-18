mod age;
mod dashboard;
mod kept_answer;
mod left_panel;
mod limits;
mod main_area;
mod menus;
mod money;
mod numbers;
mod period;
mod project;
mod projects_section;
mod remove_dialog;
mod reset_time;
mod short_number;
mod sources;
mod stats_dialog;
mod status_bar;
mod tokens;
mod top_table;
mod user_need;
mod window_name;

pub(crate) use dashboard::{BillingViewModel, DashboardViewModel};
pub(crate) use left_panel::{
    AgentsSectionViewModel, LeftPanelViewModel, ProfileRowViewModel,
};
pub(crate) use limits::{CreditsViewModel, LimitsViewModel, UsageRowViewModel};
pub(crate) use main_area::MainAreaViewModel;
pub(crate) use menus::{
    FilterMenuViewModel, MenuViewModel, NewSessionMenuViewModel,
    ProfileMenuViewModel, SessionMenuViewModel,
};
pub(crate) use money::MoneyViewModel;
#[cfg(test)]
pub(crate) use period::DayBarViewModel;
pub(crate) use period::{DayBarsViewModel, TotalViewModel};
pub(crate) use project::ProjectViewModel;
#[cfg(test)]
pub(crate) use project::session_columns;
#[cfg(test)]
pub(crate) use projects_section::FilterChipViewModel;
pub(crate) use projects_section::{
    ProjectRowViewModel, ProjectSearchViewModel, ProjectsSectionViewModel,
};
pub(crate) use remove_dialog::RemoveDialogViewModel;
pub(crate) use sources::SourceRowViewModel;
#[cfg(test)]
pub(crate) use stats_dialog::{ModelRowViewModel, ProfileStatsRowViewModel};
pub(crate) use stats_dialog::{
    ModelsTableViewModel, ProfilesTableViewModel, ProjectStatsViewModel,
    SessionStatsViewModel, StatsBodyViewModel, StatsDialogViewModel,
};
pub(crate) use status_bar::StatusBarViewModel;
pub(crate) use tokens::TokensViewModel;
pub(crate) use top_table::{
    TOP_ROWS, TopModelRowViewModel, TopProjectRowViewModel, TopRowsViewModel,
    TopViewModel,
};

/// What stands for a number, a time or an amount that is not known.
const UNKNOWN: &str = "—";

/// One fact of a list, such as the Profile facts of a dashboard or the
/// facts of a session: its label and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FactViewModel {
    /// What the fact is, such as "Plan".
    pub(crate) label: String,
    /// Its value, such as "Max".
    pub(crate) value: String,
}

impl FactViewModel {
    /// The fact `label` with its `value`.
    pub(crate) fn new(label: &str, value: String) -> Self {
        Self {
            label: label.to_string(),
            value,
        }
    }
}
