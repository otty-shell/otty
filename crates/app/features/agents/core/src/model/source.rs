//! The Source of an account, as the account page and the Found by OTTY
//! report write it (spec 2026-11-23-accounts-apart-from-profiles,
//! "Account page"): where its provider part found it, such as "Found in
//! ~/.codex", with the folder written from `~`.

use std::path::Path;

use otty_agents::FoundIn;
use otty_app_domain::folder_path::tilde_path;
use otty_app_i18n::{self as i18n, Key};

/// The Source line of `place`, such as "Found in ~/.codex", with the
/// folder written from `~` when it is in the `home` folder.
pub(crate) fn source_text(place: &FoundIn, home: &Path) -> String {
    let folder = tilde_path(place.folder(), home);

    i18n::filled(Key::TplAgentsSourceFoundIn, &[("folder", &folder)])
}

#[cfg(test)]
mod tests {
    use otty_agents::AgentId;

    use super::*;

    #[test]
    fn a_source_is_a_folder_written_from_the_home_folder() {
        let home = Path::new("/home/user");
        let codex = || AgentId::new("codex");
        let folder = FoundIn::profile(codex(), "/home/user/.codex");
        let elsewhere = FoundIn::profile(codex(), "/opt/codex");
        let found_in = |folder: &str| {
            i18n::filled(Key::TplAgentsSourceFoundIn, &[("folder", folder)])
        };

        assert_eq!(source_text(&folder, home), found_in("~/.codex"));
        assert_eq!(source_text(&elsewhere, home), found_in("/opt/codex"));
    }
}
