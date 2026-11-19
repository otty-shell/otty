//! An account: one login or key at a provider, which owns the limits, the
//! credits, the money left and the bills (spec "Words", Decisions 2 and 5).

use std::path::PathBuf;

use crate::admin_key::AdminKeyRef;
use crate::key_fingerprint::{KeyFingerprint, SeenKey};
use crate::provider::Provider;
use crate::spend::ModelTokens;

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
    /// An account the user added by hand, known by where its key is kept:
    /// the file or the environment variable, never the key (spec
    /// 2026-11-19, Decision 7).
    AddedKey(AdminKeyRef),
}

/// How an account is shown, so the user recognizes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountLogin {
    /// The email of a login, such as `you@example.com`.
    Email(String),
    /// A key, shown by its last characters, such as `a1f3`; `None` for a
    /// key too short to show any of it.
    Key(Option<String>),
    /// A key the user added, shown by the file or the variable it is kept
    /// in.
    KeyPlace(AdminKeyRef),
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

    /// The id of the organization when the account is one: an account an
    /// admin key reads the bill of (spec Decision 7). `None` for a login of
    /// a person and for a key.
    pub fn organization_id(&self) -> Option<&str> {
        match &self.identity {
            AccountIdentity::OrganizationId(id) => Some(id),
            _ => None,
        }
    }

    /// Where the key of an account the user added is kept; `None` for an
    /// account OTTY found.
    pub fn added_key(&self) -> Option<&AdminKeyRef> {
        match &self.identity {
            AccountIdentity::AddedKey(key) => Some(key),
            _ => None,
        }
    }

    /// Whether the user's choices for the account, its name and whether it
    /// is hidden, can be kept in `agents.json`: it is known by an id, an
    /// organization, an email or, for an account the user added, the place
    /// of its key. A key OTTY found is known only by its fingerprint, which
    /// is kept in memory only, so it is never saved.
    pub fn can_be_saved(&self) -> bool {
        !matches!(self.identity, AccountIdentity::KeyFingerprint(_))
    }

    /// The page of the provider where the user sees the account's own
    /// usage, opened in the browser: the plan's page for a subscription,
    /// the console's usage page for a key or an organization.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_agents::{
    ///     Account, AccountIdentity, AccountKind, AccountLogin, FoundIn, Provider,
    /// };
    ///
    /// let login = Account::new(
    ///     Provider::OpenAI,
    ///     AccountIdentity::AccountId("made-up-account".into()),
    ///     AccountLogin::Unnamed,
    ///     AccountKind::Limits,
    ///     FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
    /// );
    ///
    /// assert_eq!(
    ///     login.usage_page(),
    ///     "https://chatgpt.com/codex/settings/usage"
    /// );
    /// ```
    pub fn usage_page(&self) -> &'static str {
        let is_plan = self.kind != AccountKind::Money;

        match self.provider {
            Provider::Anthropic if is_plan => {
                "https://claude.ai/settings/usage"
            },
            Provider::Anthropic => {
                "https://console.anthropic.com/settings/usage"
            },
            Provider::OpenAI if is_plan => {
                "https://chatgpt.com/codex/settings/usage"
            },
            Provider::OpenAI => "https://platform.openai.com/usage",
            Provider::DeepSeek => "https://platform.deepseek.com/usage",
            Provider::OpenRouter => "https://openrouter.ai/activity",
            Provider::Moonshot => "https://platform.moonshot.ai/console",
            Provider::Kilo => "https://app.kilo.ai/usage",
        }
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

    /// The account the user added at `provider` with its key kept at `key`
    /// (spec 2026-11-19, Decision 7): paid per token, found "added by
    /// you". For Anthropic and OpenAI it is an organization whose bill the
    /// key, an admin key, reads.
    ///
    /// ```
    /// use otty_agents::{Account, AdminKeyRef, FoundIn, Provider};
    ///
    /// let key = AdminKeyRef::EnvVar("DEEPSEEK_API_KEY".into());
    /// let account = Account::added(Provider::DeepSeek, key.clone());
    ///
    /// assert_eq!(account.added_key(), Some(&key));
    /// assert_eq!(account.found_in(), [FoundIn::AddedByUser]);
    /// assert!(account.can_be_saved());
    /// ```
    pub fn added(provider: Provider, key: AdminKeyRef) -> Self {
        Self::new(
            provider,
            AccountIdentity::AddedKey(key.clone()),
            AccountLogin::KeyPlace(key),
            AccountKind::Money,
            FoundIn::AddedByUser,
        )
    }

    /// The same account with a plan, or none.
    pub fn with_plan(mut self, plan: Option<String>) -> Self {
        self.plan = plan;
        self
    }

    /// Whether `other` is this account, found in the same or another
    /// place: the same provider and the same identity (spec Decision 2).
    pub fn is_same_account(&self, other: &Account) -> bool {
        self.provider == other.provider && self.identity == other.identity
    }

    /// Whether this account paid `tokens` of a profile with the accounts
    /// `profile_accounts` (spec Decision 4): the agent wrote so, as Codex
    /// does per session, or the tokens went to its provider and the profile
    /// has no other account there. The tokens of a profile with several
    /// accounts of one provider that does not write which one paid are not
    /// split between them, and count for none.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_agents::{
    ///     Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
    ///     ModelTokens, Provider, TokenCount,
    /// };
    ///
    /// let login = |id: &str| {
    ///     Account::new(
    ///         Provider::OpenAI,
    ///         AccountIdentity::AccountId(id.into()),
    ///         AccountLogin::Unnamed,
    ///         AccountKind::Limits,
    ///         FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
    ///     )
    /// };
    /// let tokens = ModelTokens::new(Some("gpt-5"), TokenCount::new(10, 0, 0), 1)
    ///     .with_provider(Some(Provider::OpenAI));
    ///
    /// assert!(login("one").pays_for(&tokens, &[login("one")]));
    /// assert!(!login("one").pays_for(&tokens, &[login("one"), login("two")]));
    /// ```
    pub fn pays_for(
        &self,
        tokens: &ModelTokens,
        profile_accounts: &[Account],
    ) -> bool {
        if tokens.provider() != Some(self.provider) {
            return false;
        }

        if let Some(written) = tokens.account() {
            return *written == self.identity;
        }

        let mut of_provider = profile_accounts
            .iter()
            .filter(|found| found.provider == self.provider);

        match (of_provider.next(), of_provider.next()) {
            (Some(only), None) => only.is_same_account(self),
            _ => false,
        }
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
        let same = merged
            .iter_mut()
            .find(|kept| kept.is_same_account(&account));

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

    fn organization(id: &str, folder: &str) -> Account {
        Account::new(
            Provider::Anthropic,
            AccountIdentity::OrganizationId(id.into()),
            AccountLogin::Email("dev@example.com".into()),
            AccountKind::Money,
            FoundIn::ProfileFolder(folder.into()),
        )
    }

    #[test]
    fn copies_found_in_two_places_are_the_same_account() {
        let codex = chatgpt("made-up-account", "/home/user/.codex");
        let work = chatgpt("made-up-account", "/home/user/.codex-work");
        let other = chatgpt("made-up-other", "/home/user/.codex");
        let same_id_elsewhere = Account::new(
            Provider::Anthropic,
            AccountIdentity::AccountId("made-up-account".into()),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder("/home/user/.claude".into()),
        );

        assert!(codex.is_same_account(&work));
        assert!(!codex.is_same_account(&other));
        assert!(!codex.is_same_account(&same_id_elsewhere));
    }

    /// Spec Decision 7: only an organization takes an admin key, which
    /// reads its bill.
    #[test]
    fn only_an_organization_has_an_organization_id() {
        let organization = organization("made-up-org", "/home/user/.claude");
        let login = chatgpt("made-up-account", "/home/user/.codex");
        let key = key(Provider::Anthropic, "sk-made-up", "/home/user/.claude");

        assert_eq!(organization.organization_id(), Some("made-up-org"));
        assert_eq!(login.organization_id(), None);
        assert_eq!(key.organization_id(), None);
    }

    /// Spec 2026-11-19, Decision 7: the user's choices are kept by an id,
    /// an organization or an email, never by the fingerprint of a key.
    #[test]
    fn only_an_account_with_an_id_an_organization_or_an_email_can_be_saved() {
        let email = Account::new(
            Provider::Anthropic,
            AccountIdentity::Email("me@example.com".into()),
            AccountLogin::Email("me@example.com".into()),
            AccountKind::Limits,
            FoundIn::ProfileFolder("/home/user/.claude".into()),
        );
        let organization = Account::new(
            Provider::Anthropic,
            AccountIdentity::OrganizationId("made-up-org".into()),
            AccountLogin::Unnamed,
            AccountKind::Money,
            FoundIn::ProfileFolder("/home/user/.claude".into()),
        );

        let added = Account::added(
            Provider::DeepSeek,
            AdminKeyRef::File("/home/user/.config/deepseek-key".into()),
        );

        assert!(chatgpt("made-up-account", "/home/user/.codex").can_be_saved());
        assert!(email.can_be_saved());
        assert!(organization.can_be_saved());
        assert!(added.can_be_saved());
        assert!(
            !key(Provider::DeepSeek, "sk-made-up-key", "/home/user/.claude")
                .can_be_saved()
        );
    }

    /// Spec 2026-11-19, Decision 7: an account added by hand is known by
    /// where its key is kept, so two places are two accounts, and it is
    /// never one with a key OTTY found.
    #[test]
    fn an_added_account_is_known_by_the_place_of_its_key() {
        let in_file = || {
            Account::added(
                Provider::DeepSeek,
                AdminKeyRef::File("/home/user/.config/deepseek-key".into()),
            )
        };
        let in_variable = Account::added(
            Provider::DeepSeek,
            AdminKeyRef::EnvVar("DEEPSEEK_API_KEY".into()),
        );
        let found = key(Provider::DeepSeek, "sk-made-up", "/home/user/.claude");

        let accounts =
            merge_accounts(vec![in_file(), in_variable, in_file(), found]);

        assert_eq!(accounts.len(), 3);
        assert_eq!(accounts[0].kind(), AccountKind::Money);
        assert_eq!(accounts[0].found_in(), [FoundIn::AddedByUser]);
        assert_eq!(
            accounts[0].login(),
            &AccountLogin::KeyPlace(AdminKeyRef::File(
                "/home/user/.config/deepseek-key".into()
            ))
        );
        assert_eq!(accounts[2].added_key(), None);
    }

    /// Open usage page: a subscription opens the page of its plan, a key
    /// or an organization the usage page of the provider's console.
    #[test]
    fn each_account_opens_the_usage_page_of_its_provider() {
        let claude_max = Account::new(
            Provider::Anthropic,
            AccountIdentity::AccountId("made-up-account".into()),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder("/home/user/.claude".into()),
        );
        let cases = [
            (claude_max, "https://claude.ai/settings/usage"),
            (
                key(
                    Provider::Anthropic,
                    "sk-ant-made-up",
                    "/home/user/.claude",
                ),
                "https://console.anthropic.com/settings/usage",
            ),
            (
                chatgpt("made-up-account", "/home/user/.codex"),
                "https://chatgpt.com/codex/settings/usage",
            ),
            (
                key(Provider::OpenAI, "sk-made-up", "/home/user/.codex"),
                "https://platform.openai.com/usage",
            ),
            (
                key(Provider::DeepSeek, "sk-made-up", "/home/user/.claude"),
                "https://platform.deepseek.com/usage",
            ),
            (
                key(Provider::OpenRouter, "sk-made-up", "/home/user/.claude"),
                "https://openrouter.ai/activity",
            ),
            (
                key(Provider::Moonshot, "sk-made-up", "/home/user/.claude"),
                "https://platform.moonshot.ai/console",
            ),
            (
                key(Provider::Kilo, "sk-made-up", "/home/user/.claude"),
                "https://app.kilo.ai/usage",
            ),
        ];

        for (account, page) in cases {
            assert_eq!(account.usage_page(), page, "{account:?}");
        }
    }
}
