//! The login file of a Claude Code profile, `<config>/.credentials.json`,
//! read the same way to tell whether the profile is logged in and to send
//! the usage request.

use std::path::Path;
use std::time::SystemTime;

use serde::Deserialize;

use super::plan::plan_name;
use crate::json_file::{filled, read_json};
use crate::timestamp::time_from_millis;

/// The scope a token needs to read the usage.
const PROFILE_SCOPE: &str = "user:profile";

/// The parts of `<config>/.credentials.json` that tell about the login.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Credentials {
    claude_ai_oauth: Option<OauthLogin>,
}

/// The login of a Claude account. The token is read only to be sent; it is
/// never written, refreshed or kept after the request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OauthLogin {
    access_token: Option<String>,
    /// When the token expires, in milliseconds since 1970.
    expires_at: Option<f64>,
    scopes: Option<Vec<String>>,
    subscription_type: Option<String>,
    rate_limit_tier: Option<String>,
}

impl OauthLogin {
    /// The token, when there is one that is not blank.
    pub(super) fn token(&self) -> Option<&str> {
        filled(self.access_token.as_deref())
    }

    /// Whether the token expired before `now`. A token with no expiry
    /// time is tried: the server says whether it is still good.
    pub(super) fn is_expired(&self, now: SystemTime) -> bool {
        let expires_at = self.expires_at.and_then(time_from_millis);

        expires_at.is_some_and(|expires_at| expires_at <= now)
    }

    /// Whether the token may read the usage. A token made only to run
    /// the agent, such as one from `claude setup-token`, has no
    /// `user:profile` scope, and the server refuses it.
    pub(super) fn may_read_usage(&self) -> bool {
        match &self.scopes {
            Some(scopes) => scopes.iter().any(|scope| scope == PROFILE_SCOPE),
            None => true,
        }
    }

    /// The plan as Claude Code names it, such as "Pro" or "Max 20x".
    pub(super) fn plan(&self) -> Option<String> {
        plan_name(
            self.subscription_type.as_deref(),
            self.rate_limit_tier.as_deref(),
        )
    }
}

/// The login of a Claude account in the login file of the profile in
/// `config_dir`. `None` when the file is missing, broken or holds no such
/// login.
pub(super) fn read_login(config_dir: &Path) -> Option<OauthLogin> {
    read_json::<Credentials>(&config_dir.join(".credentials.json"))
        .and_then(|credentials| credentials.claude_ai_oauth)
}
