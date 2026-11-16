mod left_panel;
mod limits;
mod main_area;
mod profile_details;
mod remove_dialog;
mod status_bar;
mod tokens;

use std::path::Path;

#[cfg(test)]
pub(crate) use left_panel::HiddenProfileViewModel;
pub(crate) use left_panel::{
    HiddenProfilesViewModel, LeftPanelViewModel, ProfileRowViewModel,
    ProjectRowViewModel,
};
pub(crate) use limits::{LimitBarViewModel, LimitsViewModel};
pub(crate) use main_area::{
    MainAreaViewModel, ProfileChoiceViewModel, ProjectViewModel,
    SessionRowViewModel,
};
use otty_agents::SourceFailure;
pub(crate) use profile_details::ProfileDetailsViewModel;
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
