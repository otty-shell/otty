//! The writes of Agents mode: the config folder of a new profile and
//! `agents.json`. They touch files, so each runs on a thread of its own.
//! A config folder is never deleted.

use std::path::{Path, PathBuf};

use iced::Task;
use otty_agents::SavedProfiles;
use otty_app_agents::AgentsIntent;

use crate::load::on_thread;
use crate::profiles_file::write_saved;

/// Create `create_dir` when there is one, then write `saved` to
/// `agents.json` in `config_dir`, in the background. The answer comes back
/// as [`AgentsIntent::ProfilesSaved`] with the error ready to show.
pub fn save_profiles(
    config_dir: &Path,
    saved: SavedProfiles,
    create_dir: Option<PathBuf>,
) -> Task<AgentsIntent> {
    let config_dir = config_dir.to_path_buf();

    Task::perform(
        on_thread(move || save(&config_dir, &saved, create_dir.as_deref())),
        |result| {
            let result =
                result.unwrap_or_else(|| Err(String::from("the save stopped")));
            AgentsIntent::ProfilesSaved(result)
        },
    )
}

/// Create the folder of a new profile, if asked, and write the profiles.
/// When the folder can not be created, `agents.json` is not written.
fn save(
    config_dir: &Path,
    saved: &SavedProfiles,
    create_dir: Option<&Path>,
) -> Result<(), String> {
    if let Some(folder) = create_dir {
        otty_agents::create_config_dir(folder).map_err(|err| {
            log::warn!("the config folder of a new profile: {err}");
            format!("{err}")
        })?;
    }

    write_saved(config_dir, saved)
}

#[cfg(test)]
mod tests {
    use otty_agents::{AgentId, Profile, known_agents};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::profiles_file::read_saved;

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

        assert_eq!(result, Ok(()));
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
        assert!(error.contains("already has files"), "{error}");
        assert!(!config_dir.join("agents.json").exists());
        assert!(folder.join("settings.json").exists());
    }

    #[test]
    fn a_change_without_a_new_folder_only_writes_the_file() {
        let home = TempDir::new("agents-save-plain");
        let config_dir = home.path().join("otty");
        let folder = home.path().join("elsewhere");

        let result = save(&config_dir, &semrush(&folder), None);

        assert_eq!(result, Ok(()));
        assert!(!folder.exists());
        assert!(config_dir.join("agents.json").is_file());
    }

    #[test]
    fn the_save_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-save-task");

        let answers = crate::load::tests::run(save_profiles(
            home.path(),
            SavedProfiles::default(),
            None,
        ));

        assert_eq!(answers, [AgentsIntent::ProfilesSaved(Ok(()))]);
    }
}
