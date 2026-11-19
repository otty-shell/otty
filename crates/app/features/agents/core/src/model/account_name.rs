//! The name an account is shown by: the user's own (spec 2026-11-19,
//! Decision 7), else its provider and its login (spec Decisions 1 and 2).

use otty_agents::{Account, AccountLogin, SavedProfiles};
use otty_app_i18n::{self as i18n, Key};

/// The name of `account`: the one the user gave it in `saved`, the
/// content of `agents.json`, else its own ([`own_account_name`]).
pub(crate) fn account_name(
    saved: Option<&SavedProfiles>,
    account: &Account,
) -> String {
    match saved.and_then(|saved| saved.account_name(account)) {
        Some(name) => name.to_string(),
        None => own_account_name(account),
    }
}

/// The provider and the login of `account`, such as "OpenAI ·
/// you@example.com" or "Anthropic · key ••••a1f3"; a login with no email
/// shows its plan when the files name one.
pub(crate) fn own_account_name(account: &Account) -> String {
    let login = match account.login() {
        AccountLogin::Email(email) => email.clone(),
        AccountLogin::Key(Some(last)) => {
            i18n::filled(Key::TplAgentsAccountKey, &[("last", last)])
        },
        AccountLogin::Key(None) => i18n::t(Key::AgentsAccountKey).to_string(),
        _ => account
            .plan()
            .unwrap_or(i18n::t(Key::AgentsAccountLogin))
            .to_string(),
    };

    format!("{} · {login}", account.provider().name())
}
