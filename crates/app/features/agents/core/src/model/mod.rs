mod account_name;
mod account_numbers;
mod account_page;
mod age;
mod found_in;
mod found_report;
mod kept_answer;
mod left_panel;
mod limits;
mod main_area;
mod menus;
mod money;
mod needs;
mod numbers;
mod overview;
mod profile_page;
mod project;
mod projects_section;
mod remove_dialog;
mod reset_time;
mod session_account;
mod short_number;
mod sources;
mod stats_dialog;
mod status_bar;
mod tokens;
mod usage_block;
mod usage_groups;
mod used_by;
mod user_need;
mod window_name;

pub(crate) use account_numbers::AccountNumbersViewModel;
pub(crate) use account_page::AccountPageViewModel;
pub(crate) use found_report::{
    FoundAccountRowViewModel, FoundProfileRowViewModel, FoundReportViewModel,
};
pub(crate) use left_panel::{
    AccountRowViewModel, AccountsSectionViewModel, AgentsSectionViewModel,
    AllRowViewModel, LeftPanelViewModel, ProfileRowViewModel,
};
pub(crate) use limits::{CreditsViewModel, LimitsViewModel, UsageRowViewModel};
pub(crate) use main_area::MainAreaViewModel;
pub(crate) use menus::{
    FilterMenuViewModel, MenuViewModel, NewSessionMenuViewModel,
    ProfileMenuViewModel, SessionMenuViewModel, UsageMeasureMenuViewModel,
    UsagePeriodMenuViewModel,
};
pub(crate) use money::MoneyViewModel;
#[cfg(test)]
pub(crate) use needs::NeedFixViewModel;
pub(crate) use needs::NeedViewModel;
pub(crate) use overview::{
    AccountTableRowViewModel, AllAccountsViewModel, AllProfilesViewModel,
    ProfileTableRowViewModel,
};
pub(crate) use profile_page::{PaysThroughViewModel, ProfilePageViewModel};
#[cfg(test)]
pub(crate) use project::session_columns;
pub(crate) use project::{
    ProjectInfoViewModel, ProjectProfileViewModel, ProjectViewModel,
};
#[cfg(test)]
pub(crate) use projects_section::FilterChipViewModel;
pub(crate) use projects_section::{
    ProjectRowViewModel, ProjectSearchViewModel, ProjectsSectionViewModel,
};
pub(crate) use remove_dialog::RemoveDialogViewModel;
pub(crate) use sources::SourceRowViewModel;
pub(crate) use stats_dialog::{
    ProjectStatsViewModel, SessionStatsViewModel, StatsBodyViewModel,
    StatsDialogViewModel,
};
pub(crate) use status_bar::StatusBarViewModel;
#[cfg(test)]
pub(crate) use usage_block::UsageBarViewModel;
pub(crate) use usage_block::{
    UsageBlockViewModel, UsageListsViewModel, UsageTableRowViewModel,
};
pub(crate) use used_by::UsedByViewModel;

/// What stands for a number, a time or an amount that is not known.
pub(crate) const UNKNOWN: &str = "—";

/// One fact of a list, such as the Profile facts of a profile page or the
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
