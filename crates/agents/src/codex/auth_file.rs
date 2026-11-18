//! The login file of a Codex profile, `<config>/auth.json`, read the same
//! way to tell whether the profile is logged in and to send the usage
//! request.

use std::path::Path;

use serde::Deserialize;

use crate::json_file::{filled, is_filled_text, read_json};

/// The parts of `<config>/auth.json` that tell about the login.
#[derive(Debug, Deserialize)]
pub(super) struct AuthFile {
    /// How Codex logs in: `apikey` or `chatgpt`. Older files have none.
    auth_mode: Option<String>,
    /// The key of a login with an API key. Only checked for being there
    /// and never kept.
    #[serde(
        rename = "OPENAI_API_KEY",
        default,
        deserialize_with = "is_filled_text"
    )]
    has_api_key: bool,
    tokens: Option<Tokens>,
}

impl AuthFile {
    /// Whether the file holds an API key.
    pub(super) fn has_api_key(&self) -> bool {
        self.has_api_key
    }

    /// Whether the account is paid for by an API key.
    ///
    /// The `auth_mode` decides when the file has one, as Codex itself reads
    /// it: older versions kept a key next to a ChatGPT login. A file with
    /// no mode is an API key login when it holds a key.
    pub(super) fn is_api_key(&self) -> bool {
        match &self.auth_mode {
            Some(mode) => mode.replace('_', "").eq_ignore_ascii_case("apikey"),
            None => self.has_api_key,
        }
    }

    /// Whether the file holds an API key or the token of a ChatGPT login.
    pub(super) fn is_logged_in(&self) -> bool {
        let has_token = self
            .tokens
            .as_ref()
            .is_some_and(|tokens| tokens.access_token().is_some());

        self.has_api_key || has_token
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
}

/// The login file of the profile in `config_dir`. `None` when it is
/// missing or broken.
pub(super) fn read_auth_file(config_dir: &Path) -> Option<AuthFile> {
    read_json(&config_dir.join("auth.json"))
}
