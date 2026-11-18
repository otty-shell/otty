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

use std::path::Path;

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

/// The last part of a project folder, such as `otty` for `/work/otty`; the
/// whole path when it has no last part, such as `/`.
fn folder_name(folder: &Path) -> String {
    match folder.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => folder.to_string_lossy().into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_is_named_after_its_last_part() {
        assert_eq!(folder_name(Path::new("/work/otty")), "otty");
        assert_eq!(folder_name(Path::new("/")), "/");
    }
}
