//! Whether a Codex profile can be used: its program, its config folder and
//! its login.

use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::billing::BillingKind;
use crate::json_file::{is_filled_text, read_json};
use crate::profile::{Profile, ProfileStatus};

/// The parts of `<config>/auth.json` that tell about the login.
///
/// The key and the token are only checked for being there: they are
/// skipped while reading, so they are never kept.
#[derive(Debug, Deserialize)]
struct Auth {
    /// How Codex logs in: `apikey` or `chatgpt`. Older files have none.
    auth_mode: Option<String>,
    #[serde(rename = "OPENAI_API_KEY")]
    api_key: Option<IgnoredAny>,
    tokens: Option<Tokens>,
}

impl Auth {
    /// Whether the account is paid for by an API key.
    ///
    /// The `auth_mode` decides when the file has one, as Codex itself reads
    /// it: older versions kept a key next to a ChatGPT login. A file with
    /// no mode is an API key login when it holds a key.
    fn is_api_key(&self) -> bool {
        match &self.auth_mode {
            Some(mode) => mode.replace('_', "").eq_ignore_ascii_case("apikey"),
            None => self.api_key.is_some(),
        }
    }

    /// Whether the file holds an API key or the token of a ChatGPT login.
    fn is_logged_in(&self) -> bool {
        let has_token = self
            .tokens
            .as_ref()
            .is_some_and(|tokens| tokens.has_access_token);

        self.api_key.is_some() || has_token
    }
}

/// The tokens of a ChatGPT login.
#[derive(Debug, Deserialize)]
struct Tokens {
    #[serde(
        rename = "access_token",
        default,
        deserialize_with = "is_filled_text"
    )]
    has_access_token: bool,
}

/// Whether `profile` is installed and logged in.
///
/// A profile is logged in when its `auth.json` holds an API key or the
/// token of a ChatGPT login. An API key login is billed by its key; for a
/// ChatGPT login the limits answer tells the billing. The account and the
/// plan are not read yet.
/// The program is "not found" only when the profile names a program path
/// that does not exist; looking for `codex` on `PATH` is left to the app.
pub(super) fn detect(profile: &Profile) -> ProfileStatus {
    let config_dir = profile.config_dir();

    if profile.program().is_some_and(|program| !program.exists()) {
        return ProfileStatus::ProgramNotFound;
    }
    if !config_dir.is_dir() {
        return ProfileStatus::ConfigDirNotFound;
    }

    let auth = read_json::<Auth>(&config_dir.join("auth.json"));
    let Some(auth) = auth.filter(Auth::is_logged_in) else {
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

    #[test]
    fn a_program_path_that_does_not_exist_is_not_found() {
        let temp = TempDir::new();
        let profile = codex_profile(&temp.path().join(".codex-gone"))
            .with_program(temp.path().join("bin").join("codex"));

        assert_eq!(detect(&profile), ProfileStatus::ProgramNotFound);
    }

    #[test]
    fn a_program_path_that_exists_is_accepted() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(&dir.join("auth.json"), CHATGPT_AUTH);
        let program = temp.path().join("bin").join("codex");
        write_file(&program, "");
        let profile = codex_profile(&dir).with_program(program);

        assert_eq!(detect(&profile), connected());
    }
}
