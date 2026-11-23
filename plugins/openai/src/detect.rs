//! Whether a Codex profile can be used: its config folder and whether a
//! login is there. Whose login it is, the agent part does not say: the
//! provider part `openai` finds the account (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 2).
//!
//! Whether the login expired is the plugin's call (Decision 9). The part
//! checks what the provider part checks for the account found in the
//! folder: a ChatGPT login whose access token says, by its `exp` claim,
//! that it expired. Then the profile gets the same need as the account,
//! `codex login` in the folder, so acting on either fixes both. An API
//! key tells no expiry, so a profile that logs in with one is found.

use otty_plugin_sdk::agents::ProfileStatus;
use otty_plugin_sdk::{Command, files};

use crate::auth_file::AuthFile;
use crate::glob::escaped;
use crate::needs;
use crate::token_expiry::has_expired;

/// Whether the profile in `folder` can be used: found when its
/// `auth.json` holds an API key or the token of a ChatGPT login, else not
/// logged in with the need to run `login`, Codex's login command. A blank
/// key or token counts as none. A ChatGPT login that expired is not
/// logged in either, with the need to log in again in `folder`.
pub(crate) fn status(folder: &str, login: Command) -> ProfileStatus {
    if !folder_exists(folder) {
        return ProfileStatus::ConfigFolderNotFound;
    }

    let auth = AuthFile::read(folder).filter(AuthFile::is_logged_in);

    match auth {
        None => ProfileStatus::NotLoggedIn(needs::not_logged_in(login)),
        Some(auth) if has_expired_login(&auth) => {
            ProfileStatus::NotLoggedIn(needs::login_expired(folder))
        },
        Some(_) => ProfileStatus::Found,
    }
}

/// Whether `auth` logs in with a ChatGPT login whose access token says it
/// expired.
fn has_expired_login(auth: &AuthFile) -> bool {
    !auth.is_api_key() && auth.access_token().is_some_and(has_expired)
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
    use crate::testing::{TEN, as_part, host, path};

    /// The status of the config folder `folder` at a time the made-up
    /// tokens of the fixtures are good, but the one of `.codex-expired`.
    fn status_of(folder: &str) -> ProfileStatus {
        let host = host().with_now_ms(TEN);
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

    /// Decision 2: a ChatGPT login and an API key are both found, with no
    /// word on whose login it is or how it is paid for.
    #[test]
    fn a_chatgpt_login_or_a_key_is_found() {
        for folder in [".codex", ".codex-work", ".codex-nomode", ".codex-both"]
        {
            assert_eq!(status_of(folder), ProfileStatus::Found, "{folder}");
        }
    }

    /// "No login": an `auth.json` with no key and no token.
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
