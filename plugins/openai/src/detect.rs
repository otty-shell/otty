//! Whether a Codex profile can be used: its config folder and its login.

use otty_plugin_sdk::{Billing, Connection, ProfileStatus, files};

use crate::auth_file::AuthFile;
use crate::glob::escaped;

/// Whether the profile in `folder` is logged in.
///
/// A profile is logged in when its `auth.json` holds an API key or the
/// token of a ChatGPT login; a blank one counts as none. An API key login
/// is billed by its key; for a ChatGPT login the limits tell the billing.
/// The account and the plan are not read here.
pub(crate) fn status(folder: &str) -> ProfileStatus {
    if !folder_exists(folder) {
        return ProfileStatus::ConfigFolderNotFound;
    }

    let auth = AuthFile::read(folder);
    let Some(auth) = auth.filter(AuthFile::is_logged_in) else {
        return ProfileStatus::NotLoggedIn;
    };

    let billing = auth.is_api_key().then_some(Billing::ApiKey);

    ProfileStatus::Connected(Connection {
        account: None,
        plan: None,
        billing,
    })
}

/// Whether the config folder `folder` holds any file. A folder is never
/// listed itself, so an empty folder counts as missing.
fn folder_exists(folder: &str) -> bool {
    let folder = escaped(folder);
    let has_files =
        |glob: String| files::list(&glob).is_ok_and(|found| !found.is_empty());

    has_files(format!("{folder}/*")) || has_files(format!("{folder}/**"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    /// A connected profile whose billing comes from its limits.
    fn connected() -> ProfileStatus {
        ProfileStatus::Connected(Connection {
            account: None,
            plan: None,
            billing: None,
        })
    }

    /// A connected profile paid for by an API key.
    fn api_key() -> ProfileStatus {
        ProfileStatus::Connected(Connection {
            account: None,
            plan: None,
            billing: Some(Billing::ApiKey),
        })
    }

    fn status_of(folder: &str) -> ProfileStatus {
        let host = host();
        let folder = path(&host, folder);

        as_part(&host, || status(&folder))
    }

    #[test]
    fn a_chatgpt_login_is_connected() {
        assert_eq!(status_of(".codex"), connected());
    }

    #[test]
    fn an_api_key_login_is_an_api_key_profile() {
        assert_eq!(status_of(".codex-work"), api_key());
    }

    #[test]
    fn a_key_with_no_auth_mode_is_an_api_key_profile() {
        assert_eq!(status_of(".codex-nomode"), api_key());
    }

    /// Older Codex versions kept a key next to the ChatGPT login; the
    /// `auth_mode` then says how the account is paid for, as Codex itself
    /// reads it.
    #[test]
    fn the_auth_mode_wins_over_a_key_kept_next_to_a_chatgpt_login() {
        assert_eq!(status_of(".codex-both"), connected());
    }

    #[test]
    fn an_auth_file_with_no_key_and_no_token_is_not_logged_in() {
        for folder in
            [".codex-loggedout", ".codex-emptyauth", ".codex-notokens"]
        {
            assert_eq!(
                status_of(folder),
                ProfileStatus::NotLoggedIn,
                "{folder}"
            );
        }
    }

    /// A blank text is no text: the host finds no key in it.
    #[test]
    fn a_blank_key_or_token_is_not_logged_in() {
        assert_eq!(status_of(".codex-blank"), ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn a_missing_or_broken_auth_file_is_not_logged_in() {
        assert_eq!(status_of(".codex_old"), ProfileStatus::NotLoggedIn);
        assert_eq!(status_of(".codex-broken"), ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn a_missing_config_folder_is_not_found() {
        assert_eq!(
            status_of(".codex-gone"),
            ProfileStatus::ConfigFolderNotFound
        );
    }
}
