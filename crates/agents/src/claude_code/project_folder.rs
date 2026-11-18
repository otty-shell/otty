//! The project folders of a Claude Code profile: the folders in
//! `<config>/projects`, one for each working folder Claude Code ran in, how
//! their names are made from the working folder and guessed back, and the
//! session files in them.
//!
//! Claude Code names the folder after the working folder with every
//! character that is not an ASCII letter or digit replaced by `-`, one for
//! each UTF-16 unit of it, so an emoji becomes `--`. A name longer than 200
//! is cut and ends with a hash OTTY does not compute.

use std::path::{Path, PathBuf};

use super::spend_file::read_spend;
use crate::error::SourceError;
use crate::file_tree::{files_in_tree, open_data_folder};
use crate::spend_files::FileSpend;

/// The longest folder name Claude Code keeps whole.
const LONGEST_FOLDER_NAME: usize = 200;

/// The `projects` folder of the profile in `config_dir`.
pub(super) fn projects_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("projects")
}

/// The project folders in `<config>/projects` of the profile in
/// `config_dir`.
///
/// A missing `projects` folder has none, and a file in its place is "not
/// available"; a `projects` folder that can not be read is "unreadable".
/// What is not a folder inside it is left out.
pub(super) fn project_dirs(
    config_dir: &Path,
) -> Result<Vec<PathBuf>, SourceError> {
    let Some(entries) = open_data_folder(&projects_dir(config_dir))? else {
        return Ok(Vec::new());
    };

    let dirs = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();

    Ok(dirs)
}

/// The `*.jsonl` files in `folder` and in the folders inside it, at any
/// depth. Unlike [`project_dirs`], it never fails: a `folder` that can not
/// be read has none.
pub(super) fn session_files_if_readable(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };

    files_in_tree(entries, |path| {
        path.extension().is_some_and(|ext| ext == "jsonl")
    })
}

/// The answers in the session file at `path` of the profile in
/// `config_dir`, under the project its records tell, else the one guessed
/// from the project folder it is in.
pub(super) fn read_session_spend(config_dir: &Path, path: &Path) -> FileSpend {
    let fallback = fallback_project(&projects_dir(config_dir), path);

    read_spend(path, &fallback)
}

/// The name of the project folder Claude Code gives `working_dir`. `None`
/// for a name Claude Code would cut.
pub(super) fn project_folder_name(working_dir: &Path) -> Option<String> {
    let mut name = String::new();

    for character in working_dir.to_string_lossy().chars() {
        if character.is_ascii_alphanumeric() {
            name.push(character);
        } else {
            name.extend(std::iter::repeat_n('-', character.len_utf16()));
        }
    }

    (name.len() <= LONGEST_FOLDER_NAME).then_some(name)
}

/// The working folder guessed from the name of its project folder.
///
/// The guess turns every `-` back into `/`, so it is right only for a
/// path with no other characters replaced. It is used only when no record
/// of a session tells the real folder.
pub(super) fn decoded_folder(project_dir: &Path) -> PathBuf {
    let name = project_dir
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();

    PathBuf::from(name.replace('-', "/"))
}

/// Whether the project folder `project_dir` holds the sessions of
/// `working_dir`: its name is the one Claude Code gives that folder, or one
/// the session list would guess back as that folder.
pub(super) fn is_project_dir_of(
    project_dir: &Path,
    working_dir: &Path,
) -> bool {
    let name = project_dir.file_name().and_then(|name| name.to_str());
    let is_named_after = name.is_some_and(|name| {
        project_folder_name(working_dir).as_deref() == Some(name)
    });

    is_named_after || decoded_folder(project_dir) == working_dir
}

/// The working folder of the session file at `path` when its records do
/// not tell: guessed from the name of its project folder right inside
/// `projects_dir`, the same way the session list guesses it.
fn fallback_project(projects_dir: &Path, path: &Path) -> PathBuf {
    let project_dir = path
        .strip_prefix(projects_dir)
        .ok()
        .and_then(|inside| inside.components().next())
        .map(|first| projects_dir.join(first))
        .unwrap_or_default();

    decoded_folder(&project_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TempDir, write_file};

    #[test]
    fn the_folder_name_replaces_every_character_but_letters_and_digits() {
        assert_eq!(
            project_folder_name(Path::new("/home/user/.cache/otty/probe")),
            Some(String::from("-home-user--cache-otty-probe"))
        );
        assert_eq!(
            project_folder_name(Path::new("/home/Jürgen/probe_2")),
            Some(String::from("-home-J-rgen-probe-2"))
        );
    }

    #[test]
    fn a_character_outside_the_basic_plane_takes_two_dashes() {
        assert_eq!(
            project_folder_name(Path::new("/home/user/🦀-otty")),
            Some(String::from("-home-user----otty"))
        );
    }

    #[test]
    fn a_folder_name_claude_code_would_shorten_is_not_guessed() {
        let long = format!("/home/user/{}", "a".repeat(200));

        assert_eq!(project_folder_name(Path::new(&long)), None);
    }

    #[test]
    fn a_project_folder_is_matched_by_its_name_or_by_the_guess() {
        let projects = Path::new("/home/user/.claude/projects");
        let otty = Path::new("/home/user/.otty");
        let crab = Path::new("/home/user/🦀-otty");

        assert!(is_project_dir_of(&projects.join("-home-user--otty"), otty));
        assert!(is_project_dir_of(
            &projects.join("-home-user----otty"),
            crab
        ));
        assert!(is_project_dir_of(
            &projects.join("-home-user-docs"),
            Path::new("/home/user/docs")
        ));
        assert!(!is_project_dir_of(&projects.join("-home-user-docs"), otty));
    }

    #[test]
    fn the_fallback_project_is_guessed_from_the_folder_inside_projects() {
        let projects = Path::new("/home/user/.claude/projects");
        let helper = projects.join("-home-user-otty/s1/subagents/agent.jsonl");

        assert_eq!(
            fallback_project(projects, &helper),
            PathBuf::from("/home/user/otty")
        );
    }

    #[test]
    fn the_project_folders_are_the_folders_inside_projects() {
        let temp = TempDir::new();
        let config_dir = temp.path().join(".claude-work");
        write_file(&config_dir.join("projects/-a/s1.jsonl"), "{}");
        write_file(&config_dir.join("projects/stray.jsonl"), "{}");

        let dirs = project_dirs(&config_dir);

        assert_eq!(dirs, Ok(vec![config_dir.join("projects/-a")]));
    }

    #[test]
    fn a_missing_projects_folder_has_no_project_folders() {
        let temp = TempDir::new();

        let dirs = project_dirs(&temp.path().join(".claude-gone"));

        assert_eq!(dirs, Ok(Vec::new()));
    }

    #[test]
    fn a_projects_path_that_is_not_a_folder_is_not_available() {
        let temp = TempDir::new();
        let config_dir = temp.path().join(".claude-work");
        write_file(&config_dir.join("projects"), "not a folder");

        let dirs = project_dirs(&config_dir);

        assert!(matches!(dirs, Err(SourceError::NotAvailable(_))));
    }
}
