//! Whether a Codex profile can be used: its config folder and its login.

use otty_plugin_sdk::agents::{Billing, Connection, ProfileStatus};
use otty_plugin_sdk::{Command, files};

use crate::auth_file::AuthFile;
use crate::glob::escaped;
use crate::needs;

/// Whether the profile in `folder` is logged in, as [`connection`] tells.
/// A profile nobody is logged in with needs the user to run `login`,
/// Codex's login command.
pub(crate) fn status(folder: &str, login: Command) -> ProfileStatus {
    if !folder_exists(folder) {
        return ProfileStatus::ConfigFolderNotFound;
    }

    match connection(folder) {
        Some(connection) => ProfileStatus::Connected(connection),
        None => ProfileStatus::NotLoggedIn(needs::not_logged_in(login)),
    }
}

/// The login of the profile in `folder`; `None` when nobody is logged in.
///
/// A profile is logged in when its `auth.json` holds an API key or the
/// token of a ChatGPT login; a blank one counts as none. An API key login
/// is billed by its key; for a ChatGPT login the limits tell the billing.
/// The account and the plan are not read here.
pub(crate) fn connection(folder: &str) -> Option<Connection> {
    let auth = AuthFile::read(folder).filter(AuthFile::is_logged_in)?;

    let billing = auth.is_api_key().then_some(Billing::ApiKey);

    Some(Connection {
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
    use otty_plugin_sdk::Command;

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
        let login = Command {
            program: "codex".to_string(),
            args: vec!["login".to_string()],
            env: Vec::new(),
        };

        as_part(&host, || status(&folder, login))
    }

    /// Whether nobody is logged in with the config folder `folder`.
    fn is_not_logged_in(folder: &str) -> bool {
        matches!(status_of(folder), ProfileStatus::NotLoggedIn(_))
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
            assert!(is_not_logged_in(folder), "{folder}");
        }
    }

    /// A blank text is no text: the host finds no key in it.
    #[test]
    fn a_blank_key_or_token_is_not_logged_in() {
        assert!(is_not_logged_in(".codex-blank"));
    }

    #[test]
    fn a_missing_or_broken_auth_file_is_not_logged_in() {
        assert!(is_not_logged_in(".codex_old"));
        assert!(is_not_logged_in(".codex-broken"));
    }

    #[test]
    fn a_missing_config_folder_is_not_found() {
        assert_eq!(
            status_of(".codex-gone"),
            ProfileStatus::ConfigFolderNotFound
        );
    }
}
