//! The name an account is shown by: the user's own (spec 2026-11-19,
//! Decision 7), else its provider and its login (spec Decisions 1 and 2).

use otty_agents::{Account, AccountLogin, Registry, SavedProfiles};
use otty_app_i18n::{self as i18n, Key};

use super::super::state::provider_name;
use super::add_account_dialog::key_place_text;

/// The name of `account`: the one the user gave it in `saved`, the
/// content of `agents.json`, else its own ([`own_account_name`]), with
/// the name of its provider from `registry`.
pub(crate) fn account_name(
    registry: &Registry,
    saved: Option<&SavedProfiles>,
    account: &Account,
) -> String {
    match saved.and_then(|saved| saved.account_name(account)) {
        Some(name) => name.to_string(),
        None => own_account_name(registry, account),
    }
}

/// The provider and the login of `account`, such as "OpenAI ·
/// you@example.com", "Anthropic · key ••••a1f3" or, for an account the user
/// added, "DeepSeek · $DEEPSEEK_API_KEY"; a login with no email shows its
/// plan when the files name one. The provider is named as the entry of
/// `registry` that serves it names it, else by its id or host, such as
/// "api.deepseek.com · key ••••a1f3".
pub(crate) fn own_account_name(
    registry: &Registry,
    account: &Account,
) -> String {
    let login = match account.login() {
        AccountLogin::Email(email) => email.clone(),
        AccountLogin::Key(Some(last)) => {
            i18n::filled(Key::TplAgentsAccountKey, &[("last", last)])
        },
        AccountLogin::Key(None) => i18n::t(Key::AgentsAccountKey).to_string(),
        AccountLogin::KeyPlace(place) => key_place_text(place),
        _ => account
            .plan()
            .unwrap_or(i18n::t(Key::AgentsAccountLogin))
            .to_string(),
    };

    let provider = provider_name(registry, account.provider());

    format!("{provider} · {login}")
}
