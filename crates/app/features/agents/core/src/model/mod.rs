mod left_panel;
mod main_area;
mod profile_details;
mod remove_dialog;
mod status_bar;

use std::path::Path;

#[cfg(test)]
pub(crate) use left_panel::HiddenProfileViewModel;
pub(crate) use left_panel::{
    HiddenProfilesViewModel, LeftPanelViewModel, ProfileRowViewModel,
    ProjectRowViewModel,
};
pub(crate) use main_area::{
    MainAreaViewModel, ProfileChoiceViewModel, ProjectViewModel,
    SessionRowViewModel,
};
pub(crate) use profile_details::ProfileDetailsViewModel;
pub(crate) use remove_dialog::RemoveDialogViewModel;
pub(crate) use status_bar::StatusBarViewModel;

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
