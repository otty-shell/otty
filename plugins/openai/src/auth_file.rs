//! The login file of a Codex profile, `<config>/auth.json`. It holds keys,
//! so the host gives it with every key replaced by a handle: the plugin
//! learns only whether a key is there, never the key.

use otty_plugin_sdk::{JsonWithKeys, files};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// The name of the login file in the config folder.
const AUTH_FILE: &str = "auth.json";

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
    #[serde(default, deserialize_with = "key_marker")]
    access_token: Option<KeyMarker>,
}

/// The login file of a profile, read through the host.
#[derive(Debug)]
pub(crate) struct AuthFile {
    fields: AuthFields,
    document: JsonWithKeys,
}

impl AuthFile {
    /// The login file of the profile in `folder`; `None` when it is
    /// missing, broken, of another shape or not allowed.
    pub(crate) fn read(folder: &str) -> Option<Self> {
        let document =
            files::json_with_keys(&format!("{folder}/{AUTH_FILE}")).ok()?;
        let fields = serde_json::from_str(document.json()).ok()?;

        Some(Self { fields, document })
    }

    /// Whether the file holds an API key or the token of a ChatGPT login.
    pub(crate) fn is_logged_in(&self) -> bool {
        let token = self
            .fields
            .tokens
            .as_ref()
            .and_then(|tokens| tokens.access_token);

        self.has_api_key() || self.holds_key(token)
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
    fn has_api_key(&self) -> bool {
        self.holds_key(self.fields.api_key)
    }

    /// Whether `marker` stands for a key that is there. The host finds no
    /// key in a blank field, so a blank key is none.
    fn holds_key(&self, marker: Option<KeyMarker>) -> bool {
        let Some(KeyMarker(index)) = marker else {
            return false;
        };

        self.document
            .key(index)
            .is_some_and(|key| key.fingerprint().is_ok())
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
