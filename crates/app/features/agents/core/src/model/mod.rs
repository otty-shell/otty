mod dashboard;
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
mod short_number;
mod sources;
mod status_bar;
mod tokens;
mod top_table;

use std::path::Path;

pub(crate) use dashboard::{BillingViewModel, DashboardViewModel};
#[cfg(test)]
pub(crate) use left_panel::HiddenProfileViewModel;
pub(crate) use left_panel::{
    AgentsSectionViewModel, HiddenProfilesViewModel, LeftPanelViewModel,
    ProfileRowViewModel,
};
pub(crate) use limits::{CreditsViewModel, LimitsViewModel, WindowViewModel};
pub(crate) use main_area::MainAreaViewModel;
pub(crate) use menus::{
    FilterMenuViewModel, MenuViewModel, NewSessionMenuViewModel,
    ProfileMenuViewModel, SessionMenuViewModel,
};
pub(crate) use money::MoneyViewModel;
use otty_agents::SourceFailure;
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
pub(crate) use status_bar::StatusBarViewModel;
pub(crate) use tokens::TokensViewModel;
pub(crate) use top_table::TopTableViewModel;
#[cfg(test)]
pub(crate) use top_table::{ModelRowViewModel, TopProjectRowViewModel};

/// The last part of a project folder, such as `otty` for `/work/otty`; the
/// whole path when it has no last part, such as `/`.
fn folder_name(folder: &Path) -> String {
    match folder.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => folder.to_string_lossy().into_owned(),
    }
}

/// Each source and why it failed, such as "files: not available: no
/// projects folder; server: not logged in".
fn failure_reasons(failures: &[SourceFailure]) -> String {
    let reasons: Vec<String> = failures
        .iter()
        .map(|failure| format!("{}: {}", failure.source(), failure.error()))
        .collect();

    reasons.join("; ")
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
