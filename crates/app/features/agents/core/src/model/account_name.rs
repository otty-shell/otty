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

/// The provider and the login of `account`, such as "Example Cloud ·
/// you@example.com", "Example Cloud · key ••••a1f3" or, for an account the user
/// added, "Example Cloud · $EXAMPLE_API_KEY"; a login with no email shows its
/// plan when the files name one. The provider is named as the entry of
/// `registry` that serves it names it, else by its id or host, such as
/// "api.example.com · key ••••a1f3". When two plugins read the provider,
/// the plugin that reads the account follows, such as "Example Cloud ·
/// you@example.com (Example Fork)".
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
    let name = format!("{provider} · {login}");

    // A login two plugins read is two accounts: each says its plugin
    // (issue 29).
    let is_read_by_several = registry.readers_of(account.provider()).len() > 1;
    match registry.reader_of(account) {
        Some(reader) if is_read_by_several => i18n::filled(
            Key::TplAgentsAccountOfPlugin,
            &[("name", &name), ("plugin", reader.part().name())],
        ),
        _ => name,
    }
}
