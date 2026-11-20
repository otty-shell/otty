//! The account found in a Codex profile: the ChatGPT login or the API key
//! of its `auth.json`.
//!
//! A ChatGPT login is told by its account id, from `tokens.account_id` or
//! from the id token; the id token, a JWT, also names the email and the
//! plan (`chatgpt_plan_type`). An API key is turned into its fingerprint at
//! once and never kept.

use super::auth_file::{Tokens, read_auth_file};
use super::rate_limits::plan_name;
use crate::account::{
    Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
};
use crate::built_in::OPENAI;
use crate::profile::Profile;
use crate::provider::ProviderRef;

/// The accounts found in the files of `profile`: the one its `auth.json`
/// logs in with, as its `auth_mode` says, or none.
///
/// A ChatGPT login whose files name no account id and no email can not be
/// told from the others, so it is no account.
pub(super) fn accounts(profile: &Profile) -> Vec<Account> {
    let config_dir = profile.config_dir();
    let found_in = FoundIn::ProfileFolder(config_dir.to_path_buf());
    let Some(auth) = read_auth_file(config_dir) else {
        return Vec::new();
    };

    let account = if auth.is_api_key() {
        auth.api_key().map(|key| {
            Account::of_key(ProviderRef::of_id(OPENAI), key, found_in)
        })
    } else {
        auth.tokens()
            .and_then(|tokens| chatgpt_account(tokens, found_in))
    };

    account.into_iter().collect()
}

/// The account of the ChatGPT login with `tokens`: told by its account id,
/// else its email, with the plan its id token names.
fn chatgpt_account(tokens: &Tokens, found_in: FoundIn) -> Option<Account> {
    let claims = tokens.id_claims();
    let account_id = tokens.account_id().or(claims.account_id());

    let identity = match (account_id, claims.email()) {
        (Some(id), _) => AccountIdentity::AccountId(id.to_string()),
        (None, Some(email)) => AccountIdentity::Email(email.to_string()),
        (None, None) => return None,
    };
    let login = match claims.email() {
        Some(email) => AccountLogin::Email(email.to_string()),
        None => AccountLogin::Unnamed,
    };
    let plan = claims.plan_type().and_then(plan_name);

    Some(
        Account::new(
            ProviderRef::of_id(OPENAI),
            identity,
            login,
            AccountKind::Limits,
            found_in,
        )
        .with_plan(plan),
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::key_fingerprint::KeyFingerprint;
    use crate::testing::{TempDir, codex_profile, made_up_jwt, write_file};

    const ACCOUNT_ID: &str = "00000000-0000-4000-8000-0000000000c1";
    const KEY: &str = "sk-proj-made-up-key-0000-77c2";

    /// The id token of a ChatGPT login on the plan `plan`.
    fn id_token(plan: &str) -> String {
        made_up_jwt(&format!(
            r#"{{
                "email": "dev@example.com",
                "exp": 1794823200,
                "https://api.openai.com/auth": {{
                    "chatgpt_plan_type": "{plan}",
                    "chatgpt_account_id": "{ACCOUNT_ID}",
                    "chatgpt_user_id": "user-made-up"
                }}
            }}"#
        ))
    }

    /// An `auth.json` of a ChatGPT login with `tokens` as its tokens.
    fn chatgpt_auth(tokens: &str) -> String {
        format!(
            r#"{{
                "auth_mode": "chatgpt",
                "OPENAI_API_KEY": null,
                "tokens": {tokens},
                "last_refresh": "2026-11-16T10:00:00.000Z"
            }}"#
        )
    }

    fn config_dir(temp: &TempDir) -> PathBuf {
        let dir = temp.path().join(".codex-work");
        std::fs::create_dir_all(&dir).expect("create config folder");
        dir
    }

    fn accounts_in(config_dir: &Path) -> Vec<Account> {
        accounts(&codex_profile(config_dir))
    }

    #[test]
    fn a_chatgpt_login_is_an_openai_account_with_its_plan() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        let tokens = format!(
            r#"{{
                "id_token": "{}",
                "access_token": "made-up-access-token",
                "refresh_token": "made-up-refresh-token",
                "account_id": "{ACCOUNT_ID}"
            }}"#,
            id_token("pro")
        );
        write_file(&dir.join("auth.json"), &chatgpt_auth(&tokens));

        let expected = Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::AccountId(ACCOUNT_ID.into()),
            AccountLogin::Email("dev@example.com".into()),
            AccountKind::Limits,
            FoundIn::ProfileFolder(dir.clone()),
        )
        .with_plan(Some("Pro 200".into()));
        assert_eq!(accounts_in(&dir), [expected]);
    }

    #[test]
    fn the_account_id_can_come_from_the_id_token() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        let tokens = format!(r#"{{"id_token": "{}"}}"#, id_token("plus"));
        write_file(&dir.join("auth.json"), &chatgpt_auth(&tokens));

        let found = accounts_in(&dir);

        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].identity(),
            &AccountIdentity::AccountId(ACCOUNT_ID.into())
        );
        assert_eq!(found[0].plan(), Some("Plus"));
    }

    /// The id token can be missing or not a JWT; the account id of the
    /// tokens still tells the account.
    #[test]
    fn a_login_with_no_readable_id_token_has_no_email_and_no_plan() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        let tokens = format!(
            r#"{{"id_token": "made-up-opaque", "account_id": "{ACCOUNT_ID}"}}"#
        );
        write_file(&dir.join("auth.json"), &chatgpt_auth(&tokens));

        let expected = Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::AccountId(ACCOUNT_ID.into()),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder(dir.clone()),
        );
        assert_eq!(accounts_in(&dir), [expected]);
    }

    #[test]
    fn an_api_key_login_is_an_openai_key() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        write_file(
            &dir.join("auth.json"),
            &format!(
                r#"{{"auth_mode": "apikey", "OPENAI_API_KEY": "{KEY}", "tokens": null}}"#
            ),
        );

        let expected = Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::KeyFingerprint(KeyFingerprint::of(KEY)),
            AccountLogin::Key(Some("77c2".into())),
            AccountKind::Money,
            FoundIn::ProfileFolder(dir.clone()),
        );
        assert_eq!(accounts_in(&dir), [expected]);
    }

    #[test]
    fn a_missing_or_logged_out_auth_file_has_no_account() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);

        let missing = accounts_in(&dir);
        write_file(
            &dir.join("auth.json"),
            r#"{"OPENAI_API_KEY": null, "tokens": {"access_token": ""}}"#,
        );
        let logged_out = accounts_in(&dir);

        assert_eq!(missing, []);
        assert_eq!(logged_out, []);
    }

    #[test]
    fn no_key_or_token_is_in_the_debug_output() {
        let temp = TempDir::new();
        let dir = config_dir(&temp);
        let token = id_token("team");
        let tokens = format!(
            r#"{{"id_token": "{token}", "access_token": "made-up-access-token"}}"#
        );
        write_file(&dir.join("auth.json"), &chatgpt_auth(&tokens));
        let chatgpt = accounts_in(&dir);
        write_file(
            &dir.join("auth.json"),
            &format!(r#"{{"OPENAI_API_KEY": "{KEY}"}}"#),
        );
        let api_key = accounts_in(&dir);

        let debug = format!("{chatgpt:?} {api_key:?}");

        assert_eq!(chatgpt.len() + api_key.len(), 2);
        assert!(!debug.contains(KEY), "{debug}");
        assert!(!debug.contains(&token), "{debug}");
        assert!(!debug.contains("made-up-access-token"), "{debug}");
    }
}
