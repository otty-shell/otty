//! The name an account is shown by until the user names it: its provider
//! and its login (spec Decisions 1 and 2).

use otty_agents::{Account, AccountLogin};
use otty_app_i18n::{self as i18n, Key};

/// The provider and the login of `account`, such as "OpenAI ·
/// you@example.com" or "Anthropic · key ••••a1f3"; a login with no email
/// shows its plan when the files name one.
pub(crate) fn account_name(account: &Account) -> String {
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
