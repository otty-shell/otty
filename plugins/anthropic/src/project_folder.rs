//! The project folders of a Claude Code profile: the folders in
//! `<config>/projects`, one for each working folder Claude Code ran in, how
//! their names are made from the working folder and guessed back.
//!
//! Claude Code names the folder after the working folder with every
//! character that is not an ASCII letter or digit replaced by `-`, one for
//! each UTF-16 unit of it, so an emoji becomes `--`. A name longer than 200
//! is cut and ends with a hash the plugin does not compute.

use std::path::Path;

/// The longest folder name Claude Code keeps whole.
const LONGEST_FOLDER_NAME: usize = 200;

/// The `projects` folder of the profile in `folder`.
pub(crate) fn projects_folder(folder: &str) -> String {
    format!("{folder}/projects")
}

/// The name of the project folder Claude Code gives `working_folder`.
/// `None` for a name Claude Code would cut.
pub(crate) fn project_folder_name(working_folder: &str) -> Option<String> {
    let mut name = String::new();

    for character in working_folder.chars() {
        if character.is_ascii_alphanumeric() {
            name.push(character);
        } else {
            name.extend(std::iter::repeat_n('-', character.len_utf16()));
        }
    }

    (name.len() <= LONGEST_FOLDER_NAME).then_some(name)
}

/// The names a project folder of `working_folder` can have: the one
/// Claude Code gives it, and the one [`decoded_folder`] would guess back
/// as that folder, which a folder with a `-` in its path has not.
pub(crate) fn project_folder_names(working_folder: &str) -> Vec<String> {
    let mut names: Vec<String> =
        project_folder_name(working_folder).into_iter().collect();

    let guessed_back = working_folder.replace('/', "-");
    if !working_folder.contains('-') && !names.contains(&guessed_back) {
        names.push(guessed_back);
    }

    names
}

/// The working folder guessed from the name of its project folder `name`.
///
/// The guess turns every `-` back into `/`, so it is right only for a
/// path with no other characters replaced. It is used only when no record
/// of a session tells the real folder.
pub(crate) fn decoded_folder(name: &str) -> String {
    name.replace('-', "/")
}

/// The working folder of the session file at `path` when its records do
/// not tell: guessed from the name of its project folder right inside
/// `projects`, the `projects` folder of its profile.
pub(crate) fn fallback_project(projects: &str, path: &str) -> String {
    let name = Path::new(path)
        .strip_prefix(projects)
        .ok()
        .and_then(|inside| inside.components().next())
        .map(|first| first.as_os_str().to_string_lossy().into_owned())
        .unwrap_or_default();

    decoded_folder(&name)
}

/// The name of the folder `path` is right in, such as the project folder
/// of a session file.
pub(crate) fn parent_name(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folder_name_replaces_every_character_but_letters_and_digits() {
        assert_eq!(
            project_folder_name("/home/dev/.cache/otty/probe").as_deref(),
            Some("-home-dev--cache-otty-probe")
        );
        assert_eq!(
            project_folder_name("/home/Jürgen/probe_2").as_deref(),
            Some("-home-J-rgen-probe-2")
        );
    }

    #[test]
    fn a_character_outside_the_basic_plane_takes_two_dashes() {
        assert_eq!(
            project_folder_name("/home/dev/🦀-otty").as_deref(),
            Some("-home-dev----otty")
        );
    }

    #[test]
    fn a_folder_name_claude_code_would_shorten_is_not_made() {
        let long = format!("/home/dev/{}", "a".repeat(200));

        assert_eq!(project_folder_name(&long), None);
        assert_eq!(project_folder_names(&long), [long.replace('/', "-")]);
    }

    #[test]
    fn a_project_folder_is_found_by_its_name_or_by_the_guess() {
        assert_eq!(
            project_folder_names("/home/dev/.otty"),
            ["-home-dev--otty", "-home-dev-.otty"]
        );
        assert_eq!(project_folder_names("/home/dev/docs"), ["-home-dev-docs"]);
        assert_eq!(
            project_folder_names("/home/dev/my-app"),
            ["-home-dev-my-app"]
        );
    }

    #[test]
    fn the_fallback_project_is_guessed_from_the_folder_inside_projects() {
        let projects = "/home/dev/.claude/projects";
        let helper = format!("{projects}/-home-dev-otty/s1/subagents/a.jsonl");

        assert_eq!(fallback_project(projects, &helper), "/home/dev/otty");
        assert_eq!(
            parent_name(&format!("{projects}/-home-dev-otty/s1.jsonl")),
            "-home-dev-otty"
        );
    }
}
