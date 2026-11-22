//! The name an account is shown by: the user's own (spec 2026-11-19,
//! Decision 7), else its provider and its login (spec Decisions 1 and 2).
//! The left panel, whose rows show the logo of the provider, names an
//! account by its login alone (issue 34 of spec 2026-11-19-agent-plugins).

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

/// The name of `account` in a row that already shows the logo of its
/// provider: the one the user gave it in `saved`, else its login without
/// the provider, such as "you@example.com", "key ••••a1f3" or "unknown",
/// with its plugin when two plugins read it, such as "you@example.com
/// (Example Fork)".
pub(crate) fn short_account_name(
    registry: &Registry,
    saved: Option<&SavedProfiles>,
    account: &Account,
) -> String {
    match saved.and_then(|saved| saved.account_name(account)) {
        Some(name) => name.to_string(),
        None => with_plugin(registry, account, login_text(account)),
    }
}

/// The provider and the login of `account`, such as "Example Cloud ·
/// you@example.com", "Example Cloud · key ••••a1f3" or, for an account the user
/// added, "Example Cloud · $EXAMPLE_API_KEY"; a login with no email shows its
/// plan when the files name one, else "unknown". The provider is named as
/// the entry of `registry` that serves it names it, else by its id or
/// host, such as "api.example.com · key ••••a1f3". When a part of another
/// plugin serves the provider id or a host of the part that reads the
/// account, its plugin follows, such as "Example Cloud · you@example.com
/// (Example Fork)".
pub(crate) fn own_account_name(
    registry: &Registry,
    account: &Account,
) -> String {
    let provider = provider_name(registry, account.provider());
    let login = login_text(account);

    with_plugin(registry, account, format!("{provider} · {login}"))
}

/// The login of `account` as text: its email, its key, the place of the
/// key the user added, else its plan, else "unknown".
fn login_text(account: &Account) -> String {
    match account.login() {
        AccountLogin::Email(email) => email.clone(),
        AccountLogin::Key(Some(last)) => {
            i18n::filled(Key::TplAgentsAccountKey, &[("last", last)])
        },
        AccountLogin::Key(None) => i18n::t(Key::AgentsAccountKey).to_string(),
        AccountLogin::KeyPlace(place) => key_place_text(place),
        _ => account
            .plan()
            .unwrap_or(i18n::t(Key::AgentsAccountUnknown))
            .to_string(),
    }
}

/// `name` with the plugin that reads `account` after it, when two plugins
/// read the account: a login or a host two plugins read is two accounts,
/// and each says its plugin (issues 29 and 31).
fn with_plugin(registry: &Registry, account: &Account, name: String) -> String {
    match registry.reader_of(account) {
        Some(reader) if registry.is_shared(reader) => i18n::filled(
            Key::TplAgentsAccountOfPlugin,
            &[("name", &name), ("plugin", reader.part().name())],
        ),
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{AccountIdentity, AccountKind, FoundIn, ProviderRef};

    use super::*;
    use crate::testing::registry;

    /// A ChatGPT login whose files name no email, such as one OpenCode
    /// keeps encrypted, with the plan `plan`.
    fn nameless_login(plan: Option<&str>) -> Account {
        Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::AccountId(String::from("made-up-account")),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder(PathBuf::from("/home/user/.opencode")),
        )
        .with_plan(plan.map(String::from))
        .with_read_by(Some(String::from("openai")))
    }

    /// Issue 34: a login that names no email, no key and no plan is
    /// "unknown"; one that names its plan shows the plan.
    #[test]
    fn a_login_with_nothing_known_is_named_unknown() {
        let registry = registry();

        let unknown = own_account_name(&registry, &nameless_login(None));
        let with_plan =
            own_account_name(&registry, &nameless_login(Some("Plus")));

        assert_eq!(unknown, "OpenAI · unknown");
        assert_eq!(with_plan, "OpenAI · Plus");
    }
}
