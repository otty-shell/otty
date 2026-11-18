//! The writes of Agents mode: the config folder of a new profile and
//! `agents.json`. They touch files, so each runs on a thread of its own.
//! A config folder is never deleted.

use std::path::{Path, PathBuf};

use iced::Task;
use otty_agents::{CreateConfigDirError, SavedProfiles};
use otty_app_agents::AgentsIntent;
use otty_app_i18n::{self as i18n, Key};
use thiserror::Error;

use crate::background::on_own_thread;
use crate::profiles_file::{ProfilesFileError, write_saved};

/// Why the profiles could not be saved. Its text is in the user's
/// language, as the user is told it.
#[derive(Debug, Error)]
enum SaveError {
    /// The config folder of a new profile could not be created or used.
    #[error("{}", folder_problem(.0))]
    ConfigDir(#[from] CreateConfigDirError),
    /// `agents.json` could not be written.
    #[error(transparent)]
    File(#[from] ProfilesFileError),
}

/// Create `create_dir` when there is one, then write `saved` to
/// `agents.json` in `config_dir`, in the background. The answer comes back
/// as [`AgentsIntent::ProfilesSaved`] with the error ready to show, in the
/// user's language.
pub fn save_profiles(
    config_dir: &Path,
    saved: SavedProfiles,
    create_dir: Option<PathBuf>,
) -> Task<AgentsIntent> {
    let config_dir = config_dir.to_path_buf();

    on_own_thread(
        String::from("saving the agent profiles"),
        move || {
            save(&config_dir, &saved, create_dir.as_deref())
                .map_err(|err| format!("{err}"))
        },
        || Err(i18n::t(Key::AgentsSaveStopped).to_string()),
        AgentsIntent::ProfilesSaved,
    )
}

/// Create the folder of a new profile, if asked, and write the profiles.
/// When the folder can not be created, `agents.json` is not written.
fn save(
    config_dir: &Path,
    saved: &SavedProfiles,
    create_dir: Option<&Path>,
) -> Result<(), SaveError> {
    if let Some(folder) = create_dir {
        otty_agents::create_config_dir(folder).map_err(|err| {
            log::warn!("the config folder of a new profile: {err}");
            err
        })?;
    }

    Ok(write_saved(config_dir, saved)?)
}

/// The text that tells the user why the config folder of a new profile
/// could not be created or used.
fn folder_problem(error: &CreateConfigDirError) -> String {
    match error {
        CreateConfigDirError::NotAFolder(path) => i18n::filled(
            Key::TplAgentsNotAFolder,
            &[("path", &path.display().to_string())],
        ),
        CreateConfigDirError::NotEmpty(path) => i18n::filled(
            Key::TplAgentsFolderHasFiles,
            &[("path", &path.display().to_string())],
        ),
        CreateConfigDirError::Io { path, source } => i18n::filled(
            Key::TplAgentsFolderNotCreated,
            &[
                ("path", &path.display().to_string()),
                ("detail", &format!("{source}")),
            ],
        ),
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::{AgentId, Profile, known_agents};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::profiles_file::read_saved;
    use crate::testing::run;

    fn semrush(folder: &Path) -> SavedProfiles {
        SavedProfiles::default().with_added(Profile::added(
            AgentId::new("claude-code"),
            "Semrush",
            folder.to_path_buf(),
        ))
    }

    #[test]
    fn a_new_folder_is_created_and_the_profile_saved() {
        let home = TempDir::new("agents-save-new");
        let config_dir = home.path().join("otty");
        let folder = home.path().join(".claude-semrush");

        let result = save(&config_dir, &semrush(&folder), Some(&folder));

        assert!(result.is_ok(), "{result:?}");
        assert!(folder.is_dir());
        let (saved, problem) = read_saved(&config_dir, &known_agents());
        assert_eq!(problem, None);
        assert_eq!(saved, semrush(&folder));
    }

    #[test]
    fn a_folder_with_files_is_refused_and_nothing_is_saved() {
        let home = TempDir::new("agents-save-refused");
        let config_dir = home.path().join("otty");
        let folder = home.path().join(".claude-semrush");
        std::fs::create_dir_all(&folder).expect("the folder is made");
        std::fs::write(folder.join("settings.json"), "{}").expect("a file");

        let result = save(&config_dir, &semrush(&folder), Some(&folder));

        let error = result.expect_err("the folder has files");
        let told = i18n::filled(
            Key::TplAgentsFolderHasFiles,
            &[("path", &folder.display().to_string())],
        );
        assert_eq!(format!("{error}"), told);
        assert!(!config_dir.join("agents.json").exists());
        assert!(folder.join("settings.json").exists());
    }

    #[test]
    fn a_change_without_a_new_folder_only_writes_the_file() {
        let home = TempDir::new("agents-save-plain");
        let config_dir = home.path().join("otty");
        let folder = home.path().join("elsewhere");

        let result = save(&config_dir, &semrush(&folder), None);

        assert!(result.is_ok(), "{result:?}");
        assert!(!folder.exists());
        assert!(config_dir.join("agents.json").is_file());
    }

    #[test]
    fn the_save_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-save-task");

        let answers =
            run(save_profiles(home.path(), SavedProfiles::default(), None));

        assert_eq!(answers, [AgentsIntent::ProfilesSaved(Ok(()))]);
    }
}
