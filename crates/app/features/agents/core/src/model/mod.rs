mod left_panel;
mod limits;
mod main_area;
mod menus;
mod profile_details;
mod projects_section;
mod remove_dialog;
mod short_number;
mod status_bar;
mod tokens;

use std::path::Path;

#[cfg(test)]
pub(crate) use left_panel::HiddenProfileViewModel;
pub(crate) use left_panel::{
    AgentsSectionViewModel, HiddenProfilesViewModel, LeftPanelViewModel,
    ProfileRowViewModel,
};
pub(crate) use limits::{LimitBarViewModel, LimitsViewModel};
pub(crate) use main_area::{
    MainAreaViewModel, ProfileChoiceViewModel, ProjectViewModel,
    SessionRowViewModel,
};
pub(crate) use menus::{
    FilterMenuViewModel, MenuViewModel, NewSessionMenuViewModel,
};
use otty_agents::SourceFailure;
pub(crate) use profile_details::ProfileDetailsViewModel;
#[cfg(test)]
pub(crate) use projects_section::FilterChipViewModel;
pub(crate) use projects_section::{
    ProjectRowViewModel, ProjectSearchViewModel, ProjectsSectionViewModel,
};
pub(crate) use remove_dialog::RemoveDialogViewModel;
pub(crate) use status_bar::StatusBarViewModel;
pub(crate) use tokens::TokensViewModel;

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
