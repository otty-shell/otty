//! Where an account was found, as the Found by OTTY report and the account
//! page write it (spec 2026-11-19, Decisions 2 and 6): the agent's folder
//! from `~`.

use std::path::Path;

use otty_agents::FoundIn;
use otty_app_domain::folder_path::tilde_path;

/// Where an account was found, `place`, with a folder written from `~`
/// when it is in the `home` folder.
pub(crate) fn found_in_text(place: &FoundIn, home: &Path) -> String {
    let FoundIn::ProfileFolder(folder) = place;

    tilde_path(folder, home)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn an_account_is_found_in_a_folder_written_from_the_home_folder() {
        let home = Path::new("/home/user");
        let folder = FoundIn::ProfileFolder(PathBuf::from("/home/user/.coder"));
        let elsewhere = FoundIn::ProfileFolder(PathBuf::from("/opt/coder"));

        assert_eq!(found_in_text(&folder, home), "~/.coder");
        assert_eq!(found_in_text(&elsewhere, home), "/opt/coder");
    }
}
