//! The login file of a Codex profile, `<config>/auth.json`, read the same
//! way to tell whether the profile is logged in and to send the usage
//! request.

use std::path::Path;

use serde::Deserialize;

use super::jwt::jwt_claims;
use crate::json_file::{filled, read_json};
use crate::key_fingerprint::{SeenKey, read_seen_key};

/// The parts of `<config>/auth.json` that tell about the login.
#[derive(Debug, Deserialize)]
pub(super) struct AuthFile {
    /// How Codex logs in: `apikey` or `chatgpt`. Older files have none.
    auth_mode: Option<String>,
    /// The key of a login with an API key, kept only as its fingerprint.
    #[serde(
        rename = "OPENAI_API_KEY",
        default,
        deserialize_with = "read_seen_key"
    )]
    api_key: Option<SeenKey>,
    tokens: Option<Tokens>,
}

impl AuthFile {
    /// Whether the file holds an API key.
    pub(super) fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    /// The API key of the file, when it holds one.
    pub(super) fn api_key(&self) -> Option<&SeenKey> {
        self.api_key.as_ref()
    }

    /// The tokens of a ChatGPT login, when the file holds any.
    pub(super) fn tokens(&self) -> Option<&Tokens> {
        self.tokens.as_ref()
    }

    /// Whether the account is paid for by an API key.
    ///
    /// The `auth_mode` decides when the file has one, as Codex itself reads
    /// it: older versions kept a key next to a ChatGPT login. A file with
    /// no mode is an API key login when it holds a key.
    pub(super) fn is_api_key(&self) -> bool {
        match &self.auth_mode {
            Some(mode) => mode.replace('_', "").eq_ignore_ascii_case("apikey"),
            None => self.has_api_key(),
        }
    }

    /// Whether the file holds an API key or the token of a ChatGPT login.
    pub(super) fn is_logged_in(&self) -> bool {
        let has_token = self
            .tokens
            .as_ref()
            .is_some_and(|tokens| tokens.access_token().is_some());

        self.has_api_key() || has_token
    }

    /// The tokens of a ChatGPT login, when the file holds an access token.
    pub(super) fn into_chatgpt_tokens(self) -> Option<Tokens> {
        self.tokens.filter(|tokens| tokens.access_token().is_some())
    }
}

/// The tokens of a ChatGPT login. The access token is read only to be
/// sent; it is never written, refreshed or kept after the request.
#[derive(Debug, Deserialize)]
pub(super) struct Tokens {
    /// A JWT that tells who logged in: the email, the account and the
    /// plan. Read only for those.
    id_token: Option<String>,
    access_token: Option<String>,
    /// The ChatGPT workspace of the login, sent as `ChatGPT-Account-Id`
    /// so a team login gets the limits of its workspace.
    account_id: Option<String>,
}

impl Tokens {
    /// The access token, when there is one that is not blank.
    pub(super) fn access_token(&self) -> Option<&str> {
        filled(self.access_token.as_deref())
    }

    /// The ChatGPT workspace, when the file names one.
    pub(super) fn account_id(&self) -> Option<&str> {
        filled(self.account_id.as_deref())
    }

    /// What the id token tells of the login; nothing when there is no id
    /// token or it is not a JWT.
    pub(super) fn id_claims(&self) -> IdClaims {
        filled(self.id_token.as_deref())
            .and_then(jwt_claims)
            .unwrap_or_default()
    }
}

/// What the id token of a ChatGPT login tells of it.
#[derive(Debug, Default, Deserialize)]
pub(super) struct IdClaims {
    email: Option<String>,
    #[serde(rename = "https://api.openai.com/auth")]
    auth: Option<ChatgptClaims>,
}

impl IdClaims {
    /// The email of the login.
    pub(super) fn email(&self) -> Option<&str> {
        filled(self.email.as_deref())
    }

    /// The ChatGPT workspace of the login.
    pub(super) fn account_id(&self) -> Option<&str> {
        let auth = self.auth.as_ref()?;

        filled(auth.chatgpt_account_id.as_deref())
    }

    /// The code of the plan, such as `plus` or `pro`.
    pub(super) fn plan_type(&self) -> Option<&str> {
        let auth = self.auth.as_ref()?;

        filled(auth.chatgpt_plan_type.as_deref())
    }
}

/// The ChatGPT part of the id token.
#[derive(Debug, Deserialize)]
struct ChatgptClaims {
    chatgpt_account_id: Option<String>,
    chatgpt_plan_type: Option<String>,
}

/// The login file of the profile in `config_dir`. `None` when it is
/// missing or broken.
pub(super) fn read_auth_file(config_dir: &Path) -> Option<AuthFile> {
    read_json(&config_dir.join("auth.json"))
}
