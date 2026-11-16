//! Whether a Claude Code profile can be used: its program, its config
//! folder and its login.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::json_file::read_json;
use crate::profile::{Profile, ProfileStatus};

/// The parts of `<config>/.credentials.json` that tell about the login.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Credentials {
    claude_ai_oauth: Option<OauthCredentials>,
}

/// The login of a Claude account.
///
/// The token is only checked for being there: it is skipped while reading,
/// so it is never kept.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OauthCredentials {
    access_token: Option<IgnoredAny>,
    subscription_type: Option<String>,
    rate_limit_tier: Option<String>,
}

impl OauthCredentials {
    fn has_token(&self) -> bool {
        self.access_token.is_some()
    }

    /// The plan as Claude Code names it, such as "Pro" or "Max 20x".
    ///
    /// The plan is the subscription type, such as `max`. The tier, such as
    /// `default_claude_max_20x`, adds how much more the plan allows.
    fn plan(&self) -> Option<String> {
        let subscription = self.subscription_type.as_deref()?.trim();
        let mut letters = subscription.chars();
        let first = letters.next()?;
        let name = format!("{}{}", first.to_uppercase(), letters.as_str());

        match self.rate_limit_tier.as_deref().and_then(tier_multiplier) {
            Some(multiplier) => Some(format!("{name} {multiplier}")),
            None => Some(name),
        }
    }
}

/// The parts of Claude Code's global config file, `.claude.json`, that tell
/// about the login.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GlobalConfig {
    oauth_account: Option<OauthAccount>,
}

/// The account a profile is logged in with.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OauthAccount {
    email_address: Option<String>,
}

/// Whether `profile` is installed and logged in.
///
/// A profile is logged in when its credentials file holds a token, or when
/// its global config file names an account: the credentials file can be an
/// empty object while the token is kept elsewhere. The program is "not
/// found" only when the profile names a program path that does not exist;
/// looking for `claude` on `PATH` is left to the app.
pub(super) fn detect(profile: &Profile) -> ProfileStatus {
    let config_dir = profile.config_dir();

    if profile.program().is_some_and(|program| !program.exists()) {
        return ProfileStatus::ProgramNotFound;
    }
    if !config_dir.is_dir() {
        return ProfileStatus::ConfigDirNotFound;
    }

    let credentials =
        read_json::<Credentials>(&config_dir.join(".credentials.json"))
            .and_then(|credentials| credentials.claude_ai_oauth);
    let account = global_config_files(config_dir)
        .iter()
        .find_map(|path| read_json::<GlobalConfig>(path)?.oauth_account);

    let has_token = credentials
        .as_ref()
        .is_some_and(OauthCredentials::has_token);
    if !has_token && account.is_none() {
        return ProfileStatus::NotLoggedIn;
    }

    ProfileStatus::Connected {
        account: account.and_then(|account| account.email_address),
        plan: credentials.and_then(|credentials| credentials.plan()),
    }
}

/// Where the global config file of the profile in `config_dir` can be,
/// in the order to look.
///
/// With `CLAUDE_CONFIG_DIR` set, Claude Code keeps the file inside the
/// config folder. Without it, the config folder is `~/.claude` and the file
/// is `~/.claude.json`, next to the folder.
fn global_config_files(config_dir: &Path) -> Vec<PathBuf> {
    let mut files = vec![config_dir.join(".claude.json")];

    let is_default_folder =
        config_dir.file_name().is_some_and(|name| name == ".claude");
    if let Some(parent) = config_dir.parent()
        && is_default_folder
    {
        files.push(parent.join(".claude.json"));
    }

    files
}

/// The multiplier at the end of a rate limit tier, such as `20x` in
/// `default_claude_max_20x`.
fn tier_multiplier(tier: &str) -> Option<&str> {
    let last_part = tier.rsplit('_').next()?;
    let digits = last_part.strip_suffix('x')?;

    let is_number =
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit());

    is_number.then_some(last_part)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::testing::{TempDir, claude_profile, write_file};

    const TOKEN_CREDENTIALS: &str = r#"{
        "claudeAiOauth": {
            "accessToken": "made-up-access-token",
            "refreshToken": "made-up-refresh-token",
            "expiresAt": 1794823200000,
            "scopes": ["user:inference"],
            "subscriptionType": "max",
            "rateLimitTier": "default_claude_max_20x"
        }
    }"#;

    const ACCOUNT_CONFIG: &str = r#"{
        "numStartups": 12,
        "oauthAccount": {
            "accountUuid": "00000000-0000-4000-8000-000000000000",
            "emailAddress": "dev@example.com",
            "organizationName": "Example Org"
        },
        "projects": {}
    }"#;

    fn connected(account: Option<&str>, plan: Option<&str>) -> ProfileStatus {
        ProfileStatus::Connected {
            account: account.map(String::from),
            plan: plan.map(String::from),
        }
    }

    fn config_dir(temp: &TempDir, name: &str) -> PathBuf {
        let dir = temp.path().join(name);
        std::fs::create_dir_all(&dir).expect("create config folder");
        dir
    }

    fn detect_in(config_dir: &Path) -> ProfileStatus {
        detect(&claude_profile(config_dir))
    }

    #[test]
    fn a_profile_with_a_token_is_connected_with_its_plan() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), TOKEN_CREDENTIALS);

        assert_eq!(detect_in(&dir), connected(None, Some("Max 20x")));
    }

    #[test]
    fn the_account_comes_from_the_global_config_file() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), TOKEN_CREDENTIALS);
        write_file(&dir.join(".claude.json"), ACCOUNT_CONFIG);

        assert_eq!(
            detect_in(&dir),
            connected(Some("dev@example.com"), Some("Max 20x"))
        );
    }

    #[test]
    fn an_empty_credentials_file_with_an_account_is_connected() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), "{}");
        write_file(&dir.join(".claude.json"), ACCOUNT_CONFIG);

        assert_eq!(detect_in(&dir), connected(Some("dev@example.com"), None));
    }

    #[test]
    fn the_default_folder_finds_the_global_config_file_next_to_it() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude");
        write_file(&dir.join(".credentials.json"), "{}");
        write_file(&temp.path().join(".claude.json"), ACCOUNT_CONFIG);

        assert_eq!(detect_in(&dir), connected(Some("dev@example.com"), None));
    }

    #[test]
    fn the_global_config_file_in_the_folder_comes_first() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude");
        let own_config = ACCOUNT_CONFIG.replace("dev@", "own@");
        write_file(&dir.join(".claude.json"), &own_config);
        write_file(&temp.path().join(".claude.json"), ACCOUNT_CONFIG);

        assert_eq!(detect_in(&dir), connected(Some("own@example.com"), None));
    }

    #[test]
    fn another_folder_does_not_look_for_the_global_config_file_next_to_it() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), "{}");
        write_file(&temp.path().join(".claude.json"), ACCOUNT_CONFIG);

        assert_eq!(detect_in(&dir), ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn an_empty_credentials_file_is_not_logged_in() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), "{}");
        write_file(&dir.join(".claude.json"), r#"{"numStartups": 1}"#);

        assert_eq!(detect_in(&dir), ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn broken_files_are_not_logged_in() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), "{\"claudeAiOauth\": ");
        write_file(&dir.join(".claude.json"), "not json");

        assert_eq!(detect_in(&dir), ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn a_missing_config_folder_is_not_found() {
        let temp = TempDir::new();

        let status = detect_in(&temp.path().join(".claude-gone"));

        assert_eq!(status, ProfileStatus::ConfigDirNotFound);
    }

    #[test]
    fn a_program_path_that_does_not_exist_is_not_found() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), TOKEN_CREDENTIALS);
        let profile = claude_profile(&dir)
            .with_program(temp.path().join("bin").join("claude"));

        assert_eq!(detect(&profile), ProfileStatus::ProgramNotFound);
    }

    #[test]
    fn a_program_path_that_exists_is_accepted() {
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        write_file(&dir.join(".credentials.json"), TOKEN_CREDENTIALS);
        let program = temp.path().join("bin").join("claude");
        write_file(&program, "");
        let profile = claude_profile(&dir).with_program(program);

        assert_eq!(detect(&profile), connected(None, Some("Max 20x")));
    }

    #[test]
    fn the_plan_reads_like_the_name_claude_code_shows() {
        let credentials = |subscription: &str, tier: &str| {
            format!(
                r#"{{"claudeAiOauth": {{"accessToken": "made-up",
                "subscriptionType": "{subscription}",
                "rateLimitTier": "{tier}"}}}}"#
            )
        };
        let temp = TempDir::new();
        let dir = config_dir(&temp, ".claude-work");
        let cases = [
            ("pro", "default_claude_pro", "Pro"),
            ("max", "default_claude_max_5x", "Max 5x"),
            ("team", "default_raven", "Team"),
        ];

        for (subscription, tier, plan) in cases {
            write_file(
                &dir.join(".credentials.json"),
                &credentials(subscription, tier),
            );

            assert_eq!(detect_in(&dir), connected(None, Some(plan)), "{tier}");
        }
    }
}
