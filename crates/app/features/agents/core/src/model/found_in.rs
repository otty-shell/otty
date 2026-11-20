//! Where an account was found, as the Found by OTTY report and the account
//! page write it (spec 2026-11-19, Decisions 2 and 6): the agent's folder
//! from `~`, or "added by you".

use std::path::Path;

use otty_agents::FoundIn;
use otty_app_domain::folder_path::tilde_path;
use otty_app_i18n::{self as i18n, Key};

/// Where an account was found, `place`, with a folder written from `~`
/// when it is in the `home` folder.
pub(crate) fn found_in_text(place: &FoundIn, home: &Path) -> String {
    match place {
        FoundIn::ProfileFolder(folder) => tilde_path(folder, home),
        _ => i18n::t(Key::AgentsFoundInUser).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn an_account_is_found_in_a_folder_or_by_the_user() {
        let home = Path::new("/home/user");
        let folder = FoundIn::ProfileFolder(PathBuf::from("/home/user/.coder"));
        let elsewhere = FoundIn::ProfileFolder(PathBuf::from("/opt/coder"));

        assert_eq!(found_in_text(&folder, home), "~/.coder");
        assert_eq!(found_in_text(&elsewhere, home), "/opt/coder");
        assert_eq!(
            found_in_text(&FoundIn::AddedByUser, home),
            i18n::t(Key::AgentsFoundInUser)
        );
    }
}
