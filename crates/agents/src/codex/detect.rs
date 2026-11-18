//! Whether a Codex profile can be used: its program, its config folder and
//! its login.

use super::auth_file::{AuthFile, read_auth_file};
use crate::billing::BillingKind;
use crate::profile::{Profile, ProfileStatus};

/// Whether `profile` is installed and logged in.
///
/// A profile is logged in when its `auth.json` holds an API key or the
/// token of a ChatGPT login; a blank one counts as none. An API key login is billed by its key; for a
/// ChatGPT login the limits answer tells the billing. The account and the
/// plan are not read yet. Looking for `codex` on `PATH` is left to the
/// app.
pub(super) fn detect(profile: &Profile) -> ProfileStatus {
    let config_dir = profile.config_dir();

    if !config_dir.is_dir() {
        return ProfileStatus::ConfigDirNotFound;
    }

    let auth = read_auth_file(config_dir);
    let Some(auth) = auth.filter(AuthFile::is_logged_in) else {
        return ProfileStatus::NotLoggedIn;
    };

    let billing = auth.is_api_key().then_some(BillingKind::ApiKey);

    ProfileStatus::Connected {
        account: None,
        plan: None,
        billing,
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::testing::{TempDir, codex_profile, write_file};

    const CHATGPT_AUTH: &str = r#"{
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": {
            "id_token": "made-up-id-token",
            "access_token": "made-up-access-token",
            "refresh_token": "made-up-refresh-token",
            "account_id": "00000000-0000-4000-8000-000000000000"
        },
        "last_refresh": "2026-11-16T10:00:00.000Z"
    }"#;

    const API_KEY_AUTH: &str = r#"{
        "auth_mode": "apikey",
        "OPENAI_API_KEY": "sk-made-up-key",
        "tokens": null,
        "last_refresh": null
    }"#;

    const LOGGED_OUT_AUTH: &str = r#"{
        "OPENAI_API_KEY": null,
        "tokens": {"access_token": "", "account_id": null}
    }"#;

    /// A connected profile whose billing comes from its limits.
    fn connected() -> ProfileStatus {
        ProfileStatus::Connected {
            account: None,
            plan: None,
            billing: None,
        }
    }

    /// A connected profile paid for by an API key.
    fn api_key() -> ProfileStatus {
        ProfileStatus::Connected {
            account: None,
            plan: None,
            billing: Some(BillingKind::ApiKey),
        }
    }

    fn config_dir(temp: &TempDir) -> PathBuf {
        let dir = temp.path().join(".codex-work");
        std::fs::create_dir_all(&dir).expect("create config folder");
        dir
    }

    fn detect_in(config_dir: &Path) -> ProfileStatus {
        detect(&codex_profile(config_dir))
    }

    #[test]
    fn a_chatgpt_login_is_connected() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(&dir.join("auth.json"), CHATGPT_AUTH);

        assert_eq!(detect_in(&dir), connected());
    }

    #[test]
    fn an_api_key_login_is_an_api_key_profile() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(&dir.join("auth.json"), API_KEY_AUTH);

        assert_eq!(detect_in(&dir), api_key());
    }

    #[test]
    fn a_key_with_no_auth_mode_is_an_api_key_profile() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(
            &dir.join("auth.json"),
            r#"{"OPENAI_API_KEY": "sk-made-up-key"}"#,
        );

        assert_eq!(detect_in(&dir), api_key());
    }

    /// Older Codex versions kept a key next to the ChatGPT login; the
    /// `auth_mode` then says how the account is paid for, as Codex itself
    /// reads it.
    #[test]
    fn the_auth_mode_wins_over_a_key_kept_next_to_a_chatgpt_login() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        let both = CHATGPT_AUTH.replace(
            r#""OPENAI_API_KEY": null"#,
            r#""OPENAI_API_KEY": "sk-made-up-key""#,
        );
        write_file(&dir.join("auth.json"), &both);

        assert_eq!(detect_in(&dir), connected());
    }

    #[test]
    fn an_auth_file_with_no_key_and_no_token_is_not_logged_in() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);

        for auth in [LOGGED_OUT_AUTH, "{}", r#"{"tokens": {}}"#] {
            write_file(&dir.join("auth.json"), auth);

            assert_eq!(detect_in(&dir), ProfileStatus::NotLoggedIn, "{auth}");
        }
    }

    /// A blank text is no text: the usage request does not send a blank
    /// token, so a blank key or token does not make the profile logged in
    /// either.
    #[test]
    fn a_blank_key_or_token_is_not_logged_in() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(
            &dir.join("auth.json"),
            r#"{"OPENAI_API_KEY": "  ", "tokens": {"access_token": " "}}"#,
        );

        assert_eq!(detect_in(&dir), ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn a_missing_or_broken_auth_file_is_not_logged_in() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);

        let missing = detect_in(&dir);
        write_file(&dir.join("auth.json"), "{\"tokens\": ");
        let broken = detect_in(&dir);

        assert_eq!(missing, ProfileStatus::NotLoggedIn);
        assert_eq!(broken, ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn a_missing_config_folder_is_not_found() {
        let temp = TempDir::new();

        let status = detect_in(&temp.path().join(".codex-gone"));

        assert_eq!(status, ProfileStatus::ConfigDirNotFound);
    }
}
