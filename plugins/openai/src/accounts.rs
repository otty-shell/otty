//! The login found in a Codex profile: the ChatGPT login or the API key of
//! its `auth.json`, named by where its key sits, never by the key.
//!
//! A ChatGPT login is told by its account id, from `tokens.account_id` or
//! from the id token; the id token, a JWT, also names the email and the
//! plan (`chatgpt_plan_type`). OTTY gives the plugin only the claims of
//! the id token, through its handle. An API key tells nothing of its
//! account: OTTY tells it by the key's fingerprint.

use otty_plugin_sdk::{Identity, JsonField, KeyPlace, Login, LoginKind};

use crate::auth_file::{ACCESS_TOKEN_FIELD, API_KEY_FIELD, AuthFile};
use crate::base_url::CodexConfig;
use crate::rate_limits::plan_name;

/// The logins of the profile in `folder`: the one its `auth.json` logs in
/// with, as its `auth_mode` says, or none.
///
/// A ChatGPT login whose files name no account id and no email can not be
/// told from the others, so it is none.
pub(crate) fn logins(folder: &str) -> Vec<Login> {
    let Some(auth) = AuthFile::read(folder) else {
        return Vec::new();
    };
    let config = CodexConfig::read(folder);

    let login = if auth.is_api_key() {
        api_key_login(&auth, &config)
    } else {
        chatgpt_login(&auth, &config)
    };

    login.into_iter().collect()
}

/// The place of the login of the profile in `folder` whose account id is
/// `account_id`, the account a session file names as its creator.
pub(crate) fn place_of_account(
    folder: &str,
    account_id: &str,
) -> Option<KeyPlace> {
    logins(folder)
        .into_iter()
        .find(|login| login.identity.account_id.as_deref() == Some(account_id))
        .map(|login| login.place)
}

/// The API key of `auth`, for the server the config sends it to.
fn api_key_login(auth: &AuthFile, config: &CodexConfig) -> Option<Login> {
    if !auth.has_api_key() {
        return None;
    }

    Some(Login {
        kind: LoginKind::ApiKey,
        for_whom: config.api_key_for(),
        place: json_field(auth.path(), API_KEY_FIELD),
        identity: Identity::default(),
        plan: None,
    })
}

/// The ChatGPT login of `auth`: told by its account id, else its email,
/// with the plan its id token names; its key is the access token.
fn chatgpt_login(auth: &AuthFile, config: &CodexConfig) -> Option<Login> {
    if !auth.has_access_token() {
        return None;
    }

    let claims = auth.id_claims();
    let account_id = auth.account_id().or(claims.account_id());
    let email = claims.email();
    if account_id.is_none() && email.is_none() {
        return None;
    }

    Some(Login {
        kind: LoginKind::Subscription,
        for_whom: config.chatgpt_login_for(),
        place: json_field(auth.path(), ACCESS_TOKEN_FIELD),
        identity: Identity {
            account_id: account_id.map(String::from),
            organization_id: None,
            email: email.map(String::from),
        },
        plan: claims.plan_type().and_then(plan_name),
    })
}

/// The field `pointer` of the JSON file `file` as a key place.
fn json_field(file: &str, pointer: &str) -> KeyPlace {
    KeyPlace::JsonField(JsonField {
        file: file.to_string(),
        pointer: pointer.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::{ProviderRef, keys};

    use super::*;
    use crate::base_url::OPENAI;
    use crate::testing::{ACCOUNT, as_part, host, path};

    fn logins_of(folder: &str) -> Vec<Login> {
        let host = host();
        let folder = path(&host, folder);

        as_part(&host, || logins(&folder))
    }

    fn openai() -> ProviderRef {
        ProviderRef::Id(OPENAI.to_string())
    }

    fn place(folder: &str, pointer: &str) -> KeyPlace {
        let host = host();

        json_field(&path(&host, &format!("{folder}/auth.json")), pointer)
    }

    #[test]
    fn a_chatgpt_login_is_an_openai_account_with_its_plan() {
        assert_eq!(
            logins_of(".codex-plus"),
            [Login {
                kind: LoginKind::Subscription,
                for_whom: openai(),
                place: place(".codex-plus", ACCESS_TOKEN_FIELD),
                identity: Identity {
                    account_id: Some(ACCOUNT.to_string()),
                    organization_id: None,
                    email: Some("dev@example.com".to_string()),
                },
                plan: Some("Pro 200".to_string()),
            }]
        );
    }

    #[test]
    fn the_account_id_can_come_from_the_id_token() {
        let found = logins_of(".codex-idtoken");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].identity.account_id.as_deref(), Some(ACCOUNT));
        assert_eq!(found[0].plan.as_deref(), Some("Plus"));
    }

    /// The id token can be missing or not a JWT; the account id of the
    /// tokens still tells the account.
    #[test]
    fn a_login_with_no_readable_id_token_has_no_email_and_no_plan() {
        assert_eq!(
            logins_of(".codex"),
            [Login {
                kind: LoginKind::Subscription,
                for_whom: openai(),
                place: place(".codex", ACCESS_TOKEN_FIELD),
                identity: Identity {
                    account_id: Some(
                        "00000000-0000-4000-8000-000000000000".to_string()
                    ),
                    organization_id: None,
                    email: None,
                },
                plan: None,
            }]
        );
    }

    #[test]
    fn a_login_whose_files_name_no_account_and_no_email_is_none() {
        assert_eq!(logins_of(".codex-nameless"), []);
    }

    #[test]
    fn an_api_key_login_is_an_openai_key() {
        assert_eq!(
            logins_of(".codex-work"),
            [Login {
                kind: LoginKind::ApiKey,
                for_whom: openai(),
                place: place(".codex-work", API_KEY_FIELD),
                identity: Identity::default(),
                plan: None,
            }]
        );
        assert_eq!(logins_of(".codex-nomode")[0].kind, LoginKind::ApiKey);
    }

    /// Older versions of Codex kept a key next to a ChatGPT login: the
    /// mode tells which one Codex logs in with.
    #[test]
    fn the_mode_of_the_file_picks_the_login() {
        let found = logins_of(".codex-both");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, LoginKind::Subscription);
    }

    #[test]
    fn a_login_behind_a_gateway_is_for_the_gateway() {
        let found = logins_of(".codex-gateway");

        assert_eq!(
            found[0].for_whom,
            ProviderRef::Host("gateway.example.com".to_string())
        );
        assert_eq!(found[0].plan.as_deref(), Some("Business"));
    }

    #[test]
    fn a_key_sent_to_a_server_of_the_users_config_is_for_its_host() {
        assert_eq!(
            logins_of(".codex-proxy"),
            [Login {
                kind: LoginKind::ApiKey,
                for_whom: ProviderRef::Host("llm.example.com".to_string()),
                place: place(".codex-proxy", API_KEY_FIELD),
                identity: Identity::default(),
                plan: None,
            }]
        );
    }

    #[test]
    fn a_missing_broken_or_logged_out_auth_file_has_no_login() {
        for folder in [
            ".codex-gone",
            ".codex-broken",
            ".codex-loggedout",
            ".codex-blank",
            ".codex-emptyauth",
            ".codex-notokens",
        ] {
            assert_eq!(logins_of(folder), [], "{folder}");
        }
    }

    #[test]
    fn the_place_of_an_account_is_the_place_of_its_login() {
        let host = host();
        let folder = path(&host, ".codex-plus");

        let own = as_part(&host, || place_of_account(&folder, ACCOUNT));
        let other = as_part(&host, || {
            place_of_account(&folder, "00000000-0000-4000-8000-0000000000ff")
        });

        assert_eq!(own, Some(place(".codex-plus", ACCESS_TOKEN_FIELD)));
        assert_eq!(other, None);
    }

    #[test]
    fn no_key_or_token_reaches_the_plugin() {
        let found: Vec<Login> = [
            ".codex",
            ".codex-plus",
            ".codex-work",
            ".codex-proxy",
            ".codex-gateway",
        ]
        .iter()
        .flat_map(|folder| logins_of(folder))
        .collect();

        let text = format!("{found:?}");
        for secret in [
            "e30.",
            "made-up-access-token",
            "made-up-refresh-token",
            "sk-made-up",
        ] {
            assert!(!text.contains(secret), "{secret} in {text}");
        }
    }

    #[test]
    fn the_key_of_a_login_opens_as_a_handle() {
        let host = host();
        let found = logins_of(".codex-plus");

        let claims =
            as_part(&host, || keys::open(&found[0].place)?.jwt_claims());

        assert_eq!(
            claims,
            Ok(Some(
                r#"{"exp":4102444800,"sub":"made-up-access"}"#.to_string()
            ))
        );
    }
}
