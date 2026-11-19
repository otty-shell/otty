//! The Account line of the statistics of a session (spec 2026-11-19,
//! Decisions 4 and 6): which account paid for the session, as far as the
//! agent tells it.

use otty_agents::{Account, ModelTokens, SavedProfiles};
use otty_app_i18n::{self as i18n, Key};

use super::UNKNOWN;
use super::account_name::account_name;

/// What the Account line of a session with the tokens `models` of a
/// profile with `profile_accounts` says, for each provider of its tokens
/// once: the account the agent wrote in the session (Codex), found among
/// `known_accounts`; the one account of that provider in the profile
/// (Claude Code); or "one of 2 OpenAI accounts" when the profile has
/// several and the agent does not write which one paid. "—" when no
/// account paid. An account is named as the user named it in `saved`.
pub(crate) fn session_account_text(
    models: &[ModelTokens],
    profile_accounts: &[Account],
    known_accounts: &[Account],
    saved: Option<&SavedProfiles>,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    for tokens in models {
        let line =
            account_line(tokens, profile_accounts, known_accounts, saved);
        if let Some(line) = line
            && !lines.contains(&line)
        {
            lines.push(line);
        }
    }

    if lines.is_empty() {
        return String::from(UNKNOWN);
    }
    lines.join(", ")
}

/// The account that paid for `tokens`, as far as the agent tells it;
/// `None` for tokens no account paid for.
fn account_line(
    tokens: &ModelTokens,
    profile_accounts: &[Account],
    known_accounts: &[Account],
    saved: Option<&SavedProfiles>,
) -> Option<String> {
    let provider = tokens.provider()?;

    if let Some(written) = tokens.account() {
        let account =
            known_accounts
                .iter()
                .chain(profile_accounts)
                .find(|account| {
                    account.provider() == provider
                        && account.identity() == written
                })?;
        return Some(i18n::filled(
            Key::TplAgentsAccountWritten,
            &[("account", &account_name(saved, account))],
        ));
    }

    let of_provider: Vec<&Account> = profile_accounts
        .iter()
        .filter(|account| account.provider() == provider)
        .collect();
    match of_provider.as_slice() {
        [] => None,
        [only] => Some(i18n::filled(
            Key::TplAgentsAccountOfProfile,
            &[("account", &account_name(saved, only))],
        )),
        several => Some(i18n::filled(
            Key::TplAgentsOneOfAccounts,
            &[
                ("count", &several.len().to_string()),
                ("provider", provider.name()),
            ],
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        AccountIdentity, AccountKind, AccountLogin, FoundIn, Provider,
        TokenCount,
    };
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::model::UNKNOWN;
    use crate::testing::{chatgpt_login, organization};

    /// A second made-up ChatGPT login, also found in the Codex folder.
    fn second_login() -> Account {
        Account::new(
            Provider::OpenAI,
            AccountIdentity::AccountId(String::from("made-up-second")),
            AccountLogin::Email(String::from("two@example.com")),
            AccountKind::Limits,
            FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
        )
    }

    /// Tokens of `model` sent to `provider`, paid by `account` when the
    /// agent wrote it.
    fn tokens(
        model: &str,
        provider: Option<Provider>,
        account: Option<&Account>,
    ) -> ModelTokens {
        ModelTokens::new(Some(model), TokenCount::new(10, 0, 0), 1)
            .with_provider(provider)
            .with_account(account.map(|account| account.identity().clone()))
    }

    /// Codex writes the account of each session.
    #[test]
    fn the_account_written_in_the_session_is_named_as_written() {
        let accounts = [chatgpt_login(), second_login()];
        let models = [tokens(
            "gpt-5",
            Some(Provider::OpenAI),
            Some(&second_login()),
        )];

        let text = session_account_text(&models, &accounts, &accounts, None);

        assert_eq!(
            text,
            i18n::filled(
                Key::TplAgentsAccountWritten,
                &[("account", "OpenAI · two@example.com")]
            )
        );
    }

    /// Claude Code does not write the account: the one account of the
    /// provider in the profile paid.
    #[test]
    fn the_one_account_of_the_provider_in_the_profile_paid() {
        let accounts = [organization()];
        let models =
            [tokens("claude-opus-4-5", Some(Provider::Anthropic), None)];

        let text = session_account_text(&models, &accounts, &accounts, None);

        assert_eq!(
            text,
            i18n::filled(
                Key::TplAgentsAccountOfProfile,
                &[("account", "Anthropic · dev@example.com")]
            )
        );
    }

    /// Spec Decision 4: the tokens of a profile with two accounts of the
    /// provider are not split when the agent does not write which one
    /// paid.
    #[test]
    fn two_accounts_of_the_provider_not_told_apart_are_counted() {
        let accounts = [chatgpt_login(), second_login()];
        let models = [tokens("gpt-5", Some(Provider::OpenAI), None)];

        let text = session_account_text(&models, &accounts, &accounts, None);

        assert_eq!(
            text,
            i18n::filled(
                Key::TplAgentsOneOfAccounts,
                &[("count", "2"), ("provider", "OpenAI")]
            )
        );
    }

    #[test]
    fn a_session_no_account_paid_for_has_a_dash() {
        let accounts = [organization()];
        let local = [tokens("qwen", None, None)];
        let elsewhere =
            [tokens("deepseek-chat", Some(Provider::DeepSeek), None)];

        assert_eq!(
            session_account_text(&local, &accounts, &accounts, None),
            UNKNOWN
        );
        assert_eq!(
            session_account_text(&elsewhere, &accounts, &accounts, None),
            UNKNOWN
        );
        assert_eq!(
            session_account_text(&[], &accounts, &accounts, None),
            UNKNOWN
        );
    }

    /// A session that used two providers names the account of each, once.
    #[test]
    fn each_provider_of_the_session_names_its_account_once() {
        let accounts = [organization(), chatgpt_login()];
        let models = [
            tokens("claude-opus-4-5", Some(Provider::Anthropic), None),
            tokens("gpt-5", Some(Provider::OpenAI), None),
            tokens("gpt-5-mini", Some(Provider::OpenAI), None),
        ];

        let text = session_account_text(&models, &accounts, &accounts, None);

        let anthropic = i18n::filled(
            Key::TplAgentsAccountOfProfile,
            &[("account", "Anthropic · dev@example.com")],
        );
        let openai = i18n::filled(
            Key::TplAgentsAccountOfProfile,
            &[("account", "OpenAI · you@example.com")],
        );
        assert_eq!(text, format!("{anthropic}, {openai}"));
    }
}
