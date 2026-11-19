//! An account: one login or key at a provider, which owns the limits, the
//! credits, the money left and the bills (spec "Words", Decisions 2 and 5).

use std::path::PathBuf;

use crate::key_fingerprint::{KeyFingerprint, SeenKey};
use crate::provider::Provider;

/// What tells one account at a provider from another (spec Decision 2).
///
/// Two accounts are the same when they are at the same provider and have
/// the same identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AccountIdentity {
    /// The id of a login, such as the account id of a Claude login or of a
    /// ChatGPT login.
    AccountId(String),
    /// The id of an organization that pays for what its members use, such
    /// as an Anthropic Console organization.
    OrganizationId(String),
    /// The email of a login whose files name no id.
    Email(String),
    /// The fingerprint of a key.
    KeyFingerprint(KeyFingerprint),
}

/// How an account is shown, so the user recognizes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountLogin {
    /// The email of a login, such as `you@example.com`.
    Email(String),
    /// A key, shown by its last characters, such as `a1f3`; `None` for a
    /// key too short to show any of it.
    Key(Option<String>),
    /// A login whose files name no email.
    Unnamed,
}

/// What an account shows of its use (spec Decision 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    /// A subscription with windows of limits.
    Limits,
    /// A pool of credits.
    Credits,
    /// A key or an organization paid per token: money left and spent.
    Money,
}

/// Where an account was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoundIn {
    /// The config folder of a profile, such as `~/.codex`.
    ProfileFolder(PathBuf),
    /// The system keyring.
    SystemKeyring,
    /// The user added it.
    AddedByUser,
}

/// One login or key at a provider.
///
/// The key or the token of the account is never part of it: a key is known
/// only by its [`KeyFingerprint`], and the token stays in the agent's file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    provider: Provider,
    identity: AccountIdentity,
    login: AccountLogin,
    plan: Option<String>,
    kind: AccountKind,
    found_in: Vec<FoundIn>,
}

impl Account {
    /// The provider of the account.
    pub fn provider(&self) -> Provider {
        self.provider
    }

    /// What tells the account from the others of its provider.
    pub fn identity(&self) -> &AccountIdentity {
        &self.identity
    }

    /// How the account is shown.
    pub fn login(&self) -> &AccountLogin {
        &self.login
    }

    /// The plan of a subscription, such as "Max 20x" or "Pro", when the
    /// files name it.
    pub fn plan(&self) -> Option<&str> {
        self.plan.as_deref()
    }

    /// What the account shows of its use.
    pub fn kind(&self) -> AccountKind {
        self.kind
    }

    /// Every place the account was found, in the order they were found.
    pub fn found_in(&self) -> &[FoundIn] {
        &self.found_in
    }

    /// An account at `provider` found in `found_in`, with no plan.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_agents::{
    ///     Account, AccountIdentity, AccountKind, AccountLogin, FoundIn, Provider,
    /// };
    ///
    /// let account = Account::new(
    ///     Provider::OpenAI,
    ///     AccountIdentity::AccountId("made-up-account".into()),
    ///     AccountLogin::Email("you@example.com".into()),
    ///     AccountKind::Limits,
    ///     FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
    /// )
    /// .with_plan(Some("Pro".into()));
    ///
    /// assert_eq!(account.plan(), Some("Pro"));
    /// assert_eq!(account.found_in().len(), 1);
    /// ```
    pub fn new(
        provider: Provider,
        identity: AccountIdentity,
        login: AccountLogin,
        kind: AccountKind,
        found_in: FoundIn,
    ) -> Self {
        Self {
            provider,
            identity,
            login,
            plan: None,
            kind,
            found_in: vec![found_in],
        }
    }

    /// The same account with a plan, or none.
    pub fn with_plan(mut self, plan: Option<String>) -> Self {
        self.plan = plan;
        self
    }

    /// The account of the key `key` at `provider`, paid per token, found
    /// in `found_in`.
    pub(crate) fn of_key(
        provider: Provider,
        key: &SeenKey,
        found_in: FoundIn,
    ) -> Self {
        Self::new(
            provider,
            AccountIdentity::KeyFingerprint(key.fingerprint()),
            AccountLogin::Key(key.ending().map(String::from)),
            AccountKind::Money,
            found_in,
        )
    }

    /// Takes in `copy`, another copy of this account found elsewhere: its
    /// places, and its plan and email when this one has none.
    fn take_in(&mut self, copy: Account) {
        for place in copy.found_in {
            if !self.found_in.contains(&place) {
                self.found_in.push(place);
            }
        }

        if self.plan.is_none() {
            self.plan = copy.plan;
        }

        if self.login == AccountLogin::Unnamed {
            self.login = copy.login;
        }
    }
}

/// The accounts of all profiles with each account once (spec Decision 2):
/// the accounts at the same provider with the same identity are one, found
/// in every place of them.
///
/// The accounts keep the order they come in. A merged account keeps what
/// the first of its copies tells, with the plan and the email filled in
/// from the others when the first has none.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_agents::{
///     Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
///     KeyFingerprint, Provider, merge_accounts,
/// };
///
/// let key_in = |folder: &str| {
///     Account::new(
///         Provider::DeepSeek,
///         AccountIdentity::KeyFingerprint(KeyFingerprint::of("sk-made-up")),
///         AccountLogin::Key(None),
///         AccountKind::Money,
///         FoundIn::ProfileFolder(PathBuf::from(folder)),
///     )
/// };
///
/// let accounts = merge_accounts(vec![key_in("/a"), key_in("/b")]);
///
/// assert_eq!(accounts.len(), 1);
/// assert_eq!(accounts[0].found_in().len(), 2);
/// ```
pub fn merge_accounts(accounts: Vec<Account>) -> Vec<Account> {
    let mut merged: Vec<Account> = Vec::new();

    for account in accounts {
        let same = merged.iter_mut().find(|kept| {
            kept.provider == account.provider
                && kept.identity == account.identity
        });

        match same {
            Some(kept) => kept.take_in(account),
            None => merged.push(account),
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn chatgpt(account_id: &str, folder: &str) -> Account {
        Account::new(
            Provider::OpenAI,
            AccountIdentity::AccountId(account_id.into()),
            AccountLogin::Email(format!("{account_id}@example.com")),
            AccountKind::Limits,
            FoundIn::ProfileFolder(folder.into()),
        )
    }

    fn key(provider: Provider, key: &str, folder: &str) -> Account {
        Account::new(
            provider,
            AccountIdentity::KeyFingerprint(KeyFingerprint::of(key)),
            AccountLogin::Key(None),
            AccountKind::Money,
            FoundIn::ProfileFolder(folder.into()),
        )
    }

    fn folders(account: &Account) -> Vec<&Path> {
        account
            .found_in()
            .iter()
            .filter_map(|place| match place {
                FoundIn::ProfileFolder(folder) => Some(folder.as_path()),
                _ => None,
            })
            .collect()
    }

    /// Owner's machine: one ChatGPT login in Codex and in a second Codex
    /// folder is one account.
    #[test]
    fn the_same_login_in_two_folders_is_one_account() {
        let accounts = merge_accounts(vec![
            chatgpt("made-up-account", "/home/user/.codex"),
            chatgpt("made-up-account", "/home/user/.codex-work"),
        ]);

        assert_eq!(accounts.len(), 1);
        assert_eq!(
            folders(&accounts[0]),
            [
                Path::new("/home/user/.codex"),
                Path::new("/home/user/.codex-work")
            ]
        );
    }

    #[test]
    fn two_different_chatgpt_logins_stay_two() {
        let accounts = merge_accounts(vec![
            chatgpt("made-up-one", "/home/user/.codex"),
            chatgpt("made-up-two", "/home/user/.codex-work"),
        ]);

        assert_eq!(accounts.len(), 2);
        assert_eq!(
            accounts[0].identity(),
            &AccountIdentity::AccountId("made-up-one".into())
        );
        assert_eq!(
            accounts[1].identity(),
            &AccountIdentity::AccountId("made-up-two".into())
        );
    }

    #[test]
    fn the_same_key_in_two_places_is_one_account() {
        let accounts = merge_accounts(vec![
            key(
                Provider::DeepSeek,
                "sk-made-up-key-0001",
                "/home/user/.claude",
            ),
            key(
                Provider::DeepSeek,
                "sk-made-up-key-0002",
                "/home/user/.claude",
            ),
            key(
                Provider::DeepSeek,
                "sk-made-up-key-0001",
                "/home/user/.claude-deepseek",
            ),
        ]);

        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].found_in().len(), 2);
        assert_eq!(accounts[1].found_in().len(), 1);
    }

    #[test]
    fn the_same_identity_at_two_providers_is_two_accounts() {
        let accounts = merge_accounts(vec![
            key(Provider::DeepSeek, "sk-made-up-key", "/home/user/.claude"),
            key(Provider::Moonshot, "sk-made-up-key", "/home/user/.claude"),
        ]);

        assert_eq!(accounts.len(), 2);
    }

    #[test]
    fn a_place_found_twice_is_listed_once() {
        let accounts = merge_accounts(vec![
            chatgpt("made-up-account", "/home/user/.codex"),
            chatgpt("made-up-account", "/home/user/.codex"),
        ]);

        assert_eq!(folders(&accounts[0]), [Path::new("/home/user/.codex")]);
    }

    #[test]
    fn a_merged_account_fills_in_the_plan_and_the_email_it_lacks() {
        let unnamed = Account::new(
            Provider::OpenAI,
            AccountIdentity::AccountId("made-up-account".into()),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder("/home/user/.codex".into()),
        );
        let named = chatgpt("made-up-account", "/home/user/.codex-work")
            .with_plan(Some("Pro".into()));

        let accounts = merge_accounts(vec![unnamed, named]);

        assert_eq!(accounts[0].plan(), Some("Pro"));
        assert_eq!(
            accounts[0].login(),
            &AccountLogin::Email("made-up-account@example.com".into())
        );
    }
}
