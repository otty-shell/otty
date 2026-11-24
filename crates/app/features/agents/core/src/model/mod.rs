mod account_name;
mod account_numbers;
mod account_page;
mod add_account_dialog;
mod age;
mod breakdown;
mod entry_icon;
mod found_report;
mod kept_answer;
mod key_dialog;
mod keyring_dialog;
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
mod read_by;
mod remove_dialog;
mod rename_dialog;
mod reset_time;
mod short_number;
mod source;
mod stats_dialog;
mod status_bar;
mod tokens;
mod usage_block;
mod usage_groups;
mod user_need;
mod window_name;

pub(crate) use account_name::{account_name, own_account_name};
pub(crate) use account_numbers::AccountNumbersViewModel;
pub(crate) use account_page::AccountPageViewModel;
pub(crate) use add_account_dialog::AddAccountDialogViewModel;
#[cfg(test)]
pub(crate) use add_account_dialog::{
    LogInChoiceViewModel, PasteKeyChoiceViewModel, ProviderOptionViewModel,
};
pub(crate) use breakdown::{Breakdown, BreakdownLine};
pub(crate) use entry_icon::EntryIcon;
pub(crate) use found_report::{
    FoundAccountRowViewModel, FoundProfileRowViewModel, FoundReportViewModel,
};
pub(crate) use key_dialog::KeyDialogViewModel;
pub(crate) use keyring_dialog::KeyringDialogViewModel;
pub(crate) use left_panel::{
    AccountRowViewModel, AccountsSectionViewModel, AgentsSectionViewModel,
    AllRowViewModel, LeftPanelViewModel, ProfileRowViewModel,
};
pub(crate) use limits::{CreditsViewModel, LimitsViewModel, UsageRowViewModel};
pub(crate) use main_area::MainAreaViewModel;
pub(crate) use menus::{
    AccountMenuViewModel, FilterMenuViewModel, HiddenListMenuViewModel,
    MenuViewModel, NewSessionMenuViewModel, ProfileMenuViewModel,
    SessionMenuViewModel, UsagePeriodMenuViewModel,
};
#[cfg(test)]
pub(crate) use money::ShownMoney;
pub(crate) use money::{MoneyKind, MoneyViewModel};
pub(crate) use needs::NeedViewModel;
#[cfg(test)]
pub(crate) use needs::{NeedFixViewModel, account_needs, profile_needs};
pub(crate) use overview::{
    AccountTableRowViewModel, AllAccountsViewModel, AllProfilesViewModel,
    ProfileTableRowViewModel,
};
pub(crate) use profile_page::ProfilePageViewModel;
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
pub(crate) use rename_dialog::RenameDialogViewModel;
#[cfg(test)]
pub(crate) use short_number::AccountNumber;
pub(crate) use stats_dialog::{
    ProjectStatsViewModel, SessionStatsViewModel, StatsBodyViewModel,
    StatsDialogViewModel,
};
pub(crate) use status_bar::StatusBarViewModel;
pub(crate) use usage_block::{
    PeriodPicker, UsageBarViewModel, UsageBlockViewModel,
    UsageTableRowViewModel,
};

/// What stands for a number, a time or an amount that is not known.
pub(crate) const UNKNOWN: &str = "—";

/// What stands for money while the list prices it is counted at are not
/// read yet.
pub(crate) const NOT_READ_YET: &str = "…";

/// One fact of a list, such as the Profile facts of a profile page or the
/// facts of a session: its label and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FactViewModel {
    /// What the fact is, such as "Plan".
    pub(crate) label: String,
    /// Its value, such as "Max".
    pub(crate) value: String,
    /// The plugin its value links to in Plugins mode, as "Read by"
    /// does.
    pub(crate) plugin: Option<String>,
}

impl FactViewModel {
    /// The fact `label` with its `value`.
    pub(crate) fn new(label: &str, value: String) -> Self {
        Self {
            label: label.to_string(),
            value,
            plugin: None,
        }
    }

    /// The fact `label` with its `value`, which opens the row of the
    /// plugin `plugin` in Plugins mode.
    pub(crate) fn linked(label: &str, value: String, plugin: String) -> Self {
        Self {
            label: label.to_string(),
            value,
            plugin: Some(plugin),
        }
    }
}
