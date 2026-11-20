//! The login file of a Codex profile, `<config>/auth.json`. It holds keys,
//! so the host gives it with every key replaced by a handle: the plugin
//! learns whether a key is there and the claims of a token, never the key.

use otty_plugin_sdk::{JsonWithKeys, Key, files};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// The name of the login file in the config folder.
const AUTH_FILE: &str = "auth.json";

/// Where the API key of a login with a key sits in the file.
pub(crate) const API_KEY_FIELD: &str = "/OPENAI_API_KEY";

/// Where the access token of a ChatGPT login sits in the file.
pub(crate) const ACCESS_TOKEN_FIELD: &str = "/tokens/access_token";

/// Where the host put a key: `{"$key": n}`, the index of its handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KeyMarker(usize);

/// The parts of `auth.json` that tell about the login.
#[derive(Debug, Deserialize)]
struct AuthFields {
    /// How Codex logs in: `apikey` or `chatgpt`. Older files have none.
    auth_mode: Option<String>,
    /// The key of a login with an API key.
    #[serde(
        rename = "OPENAI_API_KEY",
        default,
        deserialize_with = "key_marker"
    )]
    api_key: Option<KeyMarker>,
    tokens: Option<Tokens>,
}

/// The tokens of a ChatGPT login.
#[derive(Debug, Deserialize)]
struct Tokens {
    /// A JWT that tells who logged in: the email, the account and the
    /// plan. Only its claims are read.
    #[serde(default, deserialize_with = "key_marker")]
    id_token: Option<KeyMarker>,
    #[serde(default, deserialize_with = "key_marker")]
    access_token: Option<KeyMarker>,
    /// The ChatGPT workspace of the login. It is no key, so the host
    /// leaves it as it is.
    account_id: Option<String>,
}

/// What the id token of a ChatGPT login tells of it.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct IdClaims {
    email: Option<String>,
    #[serde(rename = "https://api.openai.com/auth")]
    auth: Option<ChatgptClaims>,
}

impl IdClaims {
    /// The email of the login.
    pub(crate) fn email(&self) -> Option<&str> {
        filled(self.email.as_deref())
    }

    /// The ChatGPT workspace of the login.
    pub(crate) fn account_id(&self) -> Option<&str> {
        let auth = self.auth.as_ref()?;

        filled(auth.chatgpt_account_id.as_deref())
    }

    /// The code of the plan, such as `plus` or `pro`.
    pub(crate) fn plan_type(&self) -> Option<&str> {
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

/// The login file of a profile, read through the host.
#[derive(Debug)]
pub(crate) struct AuthFile {
    path: String,
    fields: AuthFields,
    document: JsonWithKeys,
}

impl AuthFile {
    /// The path of the file, as its key places name it.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// The login file of the profile in `folder`; `None` when it is
    /// missing, broken, of another shape or not allowed.
    pub(crate) fn read(folder: &str) -> Option<Self> {
        let path = format!("{folder}/{AUTH_FILE}");
        let document = files::json_with_keys(&path).ok()?;
        let fields = serde_json::from_str(document.json()).ok()?;

        Some(Self {
            path,
            fields,
            document,
        })
    }

    /// Whether the file holds an API key or the token of a ChatGPT login.
    pub(crate) fn is_logged_in(&self) -> bool {
        self.has_api_key() || self.has_access_token()
    }

    /// Whether the account is paid for by an API key.
    ///
    /// The `auth_mode` decides when the file has one, as Codex itself reads
    /// it: older versions kept a key next to a ChatGPT login. A file with
    /// no mode is an API key login when it holds a key.
    pub(crate) fn is_api_key(&self) -> bool {
        match &self.fields.auth_mode {
            Some(mode) => mode.replace('_', "").eq_ignore_ascii_case("apikey"),
            None => self.has_api_key(),
        }
    }

    /// Whether the file holds an API key.
    pub(crate) fn has_api_key(&self) -> bool {
        self.key(self.fields.api_key).is_some()
    }

    /// Whether the file holds the access token of a ChatGPT login.
    pub(crate) fn has_access_token(&self) -> bool {
        self.key(self.tokens().and_then(|tokens| tokens.access_token))
            .is_some()
    }

    /// The ChatGPT workspace the tokens name, when they name one.
    pub(crate) fn account_id(&self) -> Option<&str> {
        filled(self.tokens()?.account_id.as_deref())
    }

    /// What the id token tells of the login; nothing when there is no id
    /// token or it is not a JWT.
    pub(crate) fn id_claims(&self) -> IdClaims {
        let token = self.key(self.tokens().and_then(|tokens| tokens.id_token));

        token
            .and_then(|token| token.jwt_claims().ok().flatten())
            .and_then(|claims| serde_json::from_str(&claims).ok())
            .unwrap_or_default()
    }

    fn tokens(&self) -> Option<&Tokens> {
        self.fields.tokens.as_ref()
    }

    /// The key that `marker` stands for, when it is there. The host finds
    /// no key in a blank field, so a blank key is none.
    fn key(&self, marker: Option<KeyMarker>) -> Option<&Key> {
        let KeyMarker(index) = marker?;

        self.document
            .key(index)
            .filter(|key| key.fingerprint().is_ok())
    }
}

/// The marker of a key field: `{"$key": n}` where the host put a key;
/// anything else, such as `null`, is no key.
fn key_marker<'de, D>(deserializer: D) -> Result<Option<KeyMarker>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    let index = value
        .get("$key")
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok());

    Ok(index.map(KeyMarker))
}

/// `text` when it is not blank.
fn filled(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    /// The issue's test host check: the plugin gets `auth.json` with every
    /// token and key replaced by a handle, and reads the claims of the id
    /// token through it.
    #[test]
    fn the_plugin_never_gets_the_text_of_a_token() {
        let host = host();
        let folder = path(&host, ".codex-plus");

        let auth = as_part(&host, || AuthFile::read(&folder)).expect("read");
        let claims = as_part(&host, || auth.id_claims());

        let json = auth.document.json();
        for secret in ["e30.", "made-up-refresh-token", "c2ln"] {
            assert!(!json.contains(secret), "{secret} in {json}");
        }
        assert!(json.contains("\"$key\""), "{json}");
        assert_eq!(claims.email(), Some("dev@example.com"));
        assert_eq!(claims.plan_type(), Some("pro"));
    }

    #[test]
    fn the_account_id_of_the_tokens_is_no_key_and_is_read_as_text() {
        let host = host();
        let folder = path(&host, ".codex-plus");

        let auth = as_part(&host, || AuthFile::read(&folder)).expect("read");

        assert_eq!(
            auth.account_id(),
            Some("00000000-0000-4000-8000-0000000000c1")
        );
        assert!(as_part(&host, || auth.has_access_token()));
        assert!(!as_part(&host, || auth.has_api_key()));
    }
}
