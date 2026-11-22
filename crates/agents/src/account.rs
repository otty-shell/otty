//! An account: one login or key at a provider, which owns the limits, the
//! credits, the money left and the bills (spec "Words", Decisions 2 and 5),
//! as one plugin reads it. Every provider part that serves the account
//! reads it, so a login two plugins serve is two accounts, one per plugin,
//! and the user hides the one they do not want (spec
//! 2026-11-19-agent-plugins, issue 29). A key known by its host is read by
//! every part that serves the host, whatever its provider id (issue 31).

use std::path::PathBuf;

use crate::given_key::KeyRef;
use crate::key_fingerprint::KeyFingerprint;
use crate::provider::{ProviderEntry, ProviderRef};
use crate::registry::Registry;
use crate::spend::ModelTokens;

/// What tells one account at a provider from another (spec Decision 2).
///
/// Two accounts are the same when they are at the same provider and have
/// the same identity (spec 2026-11-19-agent-plugins, "Identity and links do
/// not depend on a provider plugin").
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AccountIdentity {
    /// The id of a login, such as the account id of a subscription login.
    AccountId(String),
    /// The id of an organization that pays for what its members use, such
    /// as an organization of a provider's console.
    OrganizationId(String),
    /// The email of a login whose files name no id.
    Email(String),
    /// The fingerprint of a key.
    KeyFingerprint(KeyFingerprint),
    /// An account the user added by hand, known by where its key is kept:
    /// the file or the environment variable, never the key (spec
    /// 2026-11-19, Decision 7).
    AddedKey(KeyRef),
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
    KeyPlace(KeyRef),
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
    /// The config folder of a profile, such as `~/.coder`.
    ProfileFolder(PathBuf),
    /// The user added it.
    AddedByUser,
}

/// One login or key at a provider, as one plugin reads it.
///
/// The key or the token of the account is never part of it: a key is known
/// only by its [`KeyFingerprint`], and the token stays in the agent's file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    provider: ProviderRef,
    identity: AccountIdentity,
    read_by: Option<String>,
    login: AccountLogin,
    plan: Option<String>,
    kind: AccountKind,
    found_in: Vec<FoundIn>,
}

impl Account {
    /// The provider of the account: its id, or the host of the server its
    /// key is for when no id was named and no entry serves that host.
    pub fn provider(&self) -> &ProviderRef {
        &self.provider
    }

    /// What tells the account from the others of its provider.
    pub fn identity(&self) -> &AccountIdentity {
        &self.identity
    }

    /// The id of the plugin whose provider part reads the account, such as
    /// `example`; `None` when no installed plugin reads it. Its sessions
    /// and their estimated money still count.
    pub fn read_by(&self) -> Option<&str> {
        self.read_by.as_deref()
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
    pub fn added_key(&self) -> Option<&KeyRef> {
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

    /// An account at `provider` found in `found_in`, with no plan.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_agents::{
    ///     Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
    ///     ProviderRef,
    /// };
    ///
    /// let account = Account::new(
    ///     ProviderRef::of_id("other-cloud"),
    ///     AccountIdentity::AccountId("made-up-account".into()),
    ///     AccountLogin::Email("you@example.com".into()),
    ///     AccountKind::Limits,
    ///     FoundIn::ProfileFolder(PathBuf::from("/home/user/.coder")),
    /// )
    /// .with_plan(Some("Pro".into()));
    ///
    /// assert_eq!(account.plan(), Some("Pro"));
    /// assert_eq!(account.found_in().len(), 1);
    /// ```
    pub fn new(
        provider: ProviderRef,
        identity: AccountIdentity,
        login: AccountLogin,
        kind: AccountKind,
        found_in: FoundIn,
    ) -> Self {
        Self {
            provider,
            identity,
            read_by: None,
            login,
            plan: None,
            kind,
            found_in: vec![found_in],
        }
    }

    /// The account the user added at `provider` with its key kept at `key`
    /// (spec 2026-11-19, Decision 7): paid per token, found "added by
    /// you". For a provider whose part offers Add account with an admin
    /// key it is an organization whose bill the key reads.
    ///
    /// ```
    /// use otty_agents::{Account, FoundIn, KeyRef, ProviderRef};
    ///
    /// let key = KeyRef::EnvVar("EXAMPLE_API_KEY".into());
    /// let account =
    ///     Account::added(ProviderRef::of_id("example-cloud"), key.clone());
    ///
    /// assert_eq!(account.added_key(), Some(&key));
    /// assert_eq!(account.found_in(), [FoundIn::AddedByUser]);
    /// assert!(account.can_be_saved());
    /// ```
    pub fn added(provider: ProviderRef, key: KeyRef) -> Self {
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

    /// The same account read by the provider part of the plugin `plugin`,
    /// or by none.
    ///
    /// ```
    /// use otty_agents::{Account, KeyRef, ProviderRef};
    ///
    /// let added = Account::added(
    ///     ProviderRef::of_id("example-cloud"),
    ///     KeyRef::EnvVar("EXAMPLE_API_KEY".into()),
    /// );
    ///
    /// assert_eq!(added.read_by(), None);
    /// assert_eq!(
    ///     added.with_read_by(Some("example".into())).read_by(),
    ///     Some("example")
    /// );
    /// ```
    pub fn with_read_by(mut self, plugin: Option<String>) -> Self {
        self.read_by = plugin;
        self
    }

    /// Whether `other` is this account, found in the same or another
    /// place: the same provider and the same identity (spec Decision 2),
    /// read by the same plugin (issue 29).
    pub fn is_same_account(&self, other: &Account) -> bool {
        self.is_same_login(other) && self.read_by == other.read_by
    }

    /// Whether `other` is the same login or key as this account, whichever
    /// plugin reads each: the same provider and the same identity.
    pub fn is_same_login(&self, other: &Account) -> bool {
        self.provider == other.provider && self.identity == other.identity
    }

    /// Whether this account paid `tokens` of a profile with the accounts
    /// `profile_accounts` (spec Decision 4): the agent wrote so for the
    /// session, or the tokens went to its provider and the profile
    /// has no other login there. The tokens of a profile with several
    /// logins of one provider that does not write which one paid are not
    /// split between them, and count for none. The copies of one login
    /// that several plugins read each paid them (issue 29). A provider
    /// named by its host is each provider whose part in `registry` serves
    /// the host (issue 31).
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_agents::{
    ///     Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
    ///     ModelTokens, ProviderRef, Registry, TokenCount,
    /// };
    ///
    /// let registry = Registry::default();
    /// let login = |id: &str| {
    ///     Account::new(
    ///         ProviderRef::of_id("other-cloud"),
    ///         AccountIdentity::AccountId(id.into()),
    ///         AccountLogin::Unnamed,
    ///         AccountKind::Limits,
    ///         FoundIn::ProfileFolder(PathBuf::from("/home/user/.coder")),
    ///     )
    /// };
    /// let tokens = ModelTokens::new(Some("gpt-5"), TokenCount::new(10, 0, 0), 1)
    ///     .with_provider(Some(ProviderRef::of_id("other-cloud")));
    ///
    /// assert!(login("one").pays_for(&tokens, &[login("one")], &registry));
    /// assert!(!login("one").pays_for(
    ///     &tokens,
    ///     &[login("one"), login("two")],
    ///     &registry
    /// ));
    /// ```
    pub fn pays_for(
        &self,
        tokens: &ModelTokens,
        profile_accounts: &[Account],
        registry: &Registry,
    ) -> bool {
        let is_of_provider = tokens.provider().is_some_and(|provider| {
            registry.is_same_provider(provider, &self.provider)
        });
        if !is_of_provider {
            return false;
        }

        if let Some(written) = tokens.account() {
            return *written == self.identity;
        }

        let mut of_provider = profile_accounts.iter().filter(|found| {
            registry.is_same_provider(&found.provider, &self.provider)
        });

        let Some(first) = of_provider.next() else {
            return false;
        };
        first.is_same_login(self)
            && of_provider.all(|other| other.is_same_login(first))
    }

    /// One copy of the account for each provider part of `registry` that
    /// reads it, under the id of that part and read by its plugin: every
    /// part with its provider id or, for a key known by its host, every
    /// part that serves the host, whatever its id (issue 31); for an
    /// account the user added only those that offer Add account. An
    /// account already read by a plugin, or one no part reads, stays as it
    /// is.
    fn read_by_each(self, registry: &Registry) -> Vec<Account> {
        if self.read_by.is_some() {
            return vec![self];
        }

        let is_added = self.added_key().is_some();
        let readers: Vec<&ProviderEntry> = registry
            .readers_of(&self.provider)
            .into_iter()
            .filter(|entry| !is_added || entry.can_add_by_hand())
            .collect();

        if readers.is_empty() {
            return vec![self];
        }
        readers
            .into_iter()
            .map(|entry| {
                let mut copy = self.clone();
                copy.provider = ProviderRef::Id(entry.id().clone());
                copy.read_by = Some(entry.part().plugin().to_string());
                copy
            })
            .collect()
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
/// in every place of them, for each plugin of `registry` that reads it
/// (issue 29). An account known by the host of its server is read by every
/// part that serves the host, each copy under the id of its part (issue
/// 31), so a key found for a host is one account with the same key found
/// for one of those ids, and joins no copy of another id; with no such
/// part it stays known by its host, read by no plugin.
///
/// The accounts keep the order they come in, the copies of one login side
/// by side in the order of the registry. A merged account keeps what
/// the first of its copies tells, with the plan and the email filled in
/// from the others when the first has none.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_agents::{
///     Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
///     KeyFingerprint, ProviderRef, Registry, merge_accounts,
/// };
///
/// let key_in = |folder: &str| {
///     Account::new(
///         ProviderRef::Host("api.example.com".into()),
///         AccountIdentity::KeyFingerprint(KeyFingerprint::of("sk-made-up")),
///         AccountLogin::Key(None),
///         AccountKind::Money,
///         FoundIn::ProfileFolder(PathBuf::from(folder)),
///     )
/// };
///
/// let accounts =
///     merge_accounts(vec![key_in("/a"), key_in("/b")], &Registry::default());
///
/// assert_eq!(accounts.len(), 1);
/// assert_eq!(accounts[0].found_in().len(), 2);
/// ```
pub fn merge_accounts(
    accounts: Vec<Account>,
    registry: &Registry,
) -> Vec<Account> {
    let read = accounts
        .into_iter()
        .flat_map(|account| account.read_by_each(registry))
        .collect();

    join_copies(read)
}

/// `accounts` with the copies of each account made one, as
/// [`merge_accounts`] does, with each provider named as it is: what a
/// reader of this crate does with the accounts of one profile.
pub(crate) fn join_copies(accounts: Vec<Account>) -> Vec<Account> {
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
            ProviderRef::of_id("openai"),
            AccountIdentity::AccountId(account_id.into()),
            AccountLogin::Email(format!("{account_id}@example.com")),
            AccountKind::Limits,
            FoundIn::ProfileFolder(folder.into()),
        )
    }

    fn key(provider: ProviderRef, key: &str, folder: &str) -> Account {
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

    /// Owner's machine: one ChatGPT login in Coder and in a second Coder
    /// folder is one account.
    #[test]
    fn the_same_login_in_two_folders_is_one_account() {
        let accounts = join_copies(vec![
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
        let accounts = join_copies(vec![
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
        let accounts = join_copies(vec![
            key(
                ProviderRef::of_id("deepseek"),
                "sk-made-up-key-0001",
                "/home/user/.claude",
            ),
            key(
                ProviderRef::of_id("deepseek"),
                "sk-made-up-key-0002",
                "/home/user/.claude",
            ),
            key(
                ProviderRef::of_id("deepseek"),
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
        let accounts = join_copies(vec![
            key(
                ProviderRef::of_id("deepseek"),
                "sk-made-up-key",
                "/home/user/.claude",
            ),
            key(
                ProviderRef::of_id("example-cloud"),
                "sk-made-up-key",
                "/home/user/.claude",
            ),
        ]);

        assert_eq!(accounts.len(), 2);
    }

    #[test]
    fn a_place_found_twice_is_listed_once() {
        let accounts = join_copies(vec![
            chatgpt("made-up-account", "/home/user/.codex"),
            chatgpt("made-up-account", "/home/user/.codex"),
        ]);

        assert_eq!(folders(&accounts[0]), [Path::new("/home/user/.codex")]);
    }

    #[test]
    fn a_merged_account_fills_in_the_plan_and_the_email_it_lacks() {
        let unnamed = Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::AccountId("made-up-account".into()),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder("/home/user/.codex".into()),
        );
        let named = chatgpt("made-up-account", "/home/user/.codex-work")
            .with_plan(Some("Pro".into()));

        let accounts = join_copies(vec![unnamed, named]);

        assert_eq!(accounts[0].plan(), Some("Pro"));
        assert_eq!(
            accounts[0].login(),
            &AccountLogin::Email("made-up-account@example.com".into())
        );
    }

    fn organization(id: &str, folder: &str) -> Account {
        Account::new(
            ProviderRef::of_id("anthropic"),
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
            ProviderRef::of_id("anthropic"),
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
        let key = key(
            ProviderRef::of_id("anthropic"),
            "sk-made-up",
            "/home/user/.claude",
        );

        assert_eq!(organization.organization_id(), Some("made-up-org"));
        assert_eq!(login.organization_id(), None);
        assert_eq!(key.organization_id(), None);
    }

    /// Spec 2026-11-19, Decision 7: the user's choices are kept by an id,
    /// an organization or an email, never by the fingerprint of a key.
    #[test]
    fn only_an_account_with_an_id_an_organization_or_an_email_can_be_saved() {
        let email = Account::new(
            ProviderRef::of_id("anthropic"),
            AccountIdentity::Email("me@example.com".into()),
            AccountLogin::Email("me@example.com".into()),
            AccountKind::Limits,
            FoundIn::ProfileFolder("/home/user/.claude".into()),
        );
        let organization = Account::new(
            ProviderRef::of_id("anthropic"),
            AccountIdentity::OrganizationId("made-up-org".into()),
            AccountLogin::Unnamed,
            AccountKind::Money,
            FoundIn::ProfileFolder("/home/user/.claude".into()),
        );

        let added = Account::added(
            ProviderRef::of_id("deepseek"),
            KeyRef::File("/home/user/.config/deepseek-key".into()),
        );

        assert!(chatgpt("made-up-account", "/home/user/.codex").can_be_saved());
        assert!(email.can_be_saved());
        assert!(organization.can_be_saved());
        assert!(added.can_be_saved());
        assert!(
            !key(
                ProviderRef::of_id("deepseek"),
                "sk-made-up-key",
                "/home/user/.claude"
            )
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
                ProviderRef::of_id("deepseek"),
                KeyRef::File("/home/user/.config/deepseek-key".into()),
            )
        };
        let in_variable = Account::added(
            ProviderRef::of_id("deepseek"),
            KeyRef::EnvVar("DEEPSEEK_API_KEY".into()),
        );
        let found = key(
            ProviderRef::of_id("deepseek"),
            "sk-made-up",
            "/home/user/.claude",
        );

        let accounts =
            join_copies(vec![in_file(), in_variable, in_file(), found]);

        assert_eq!(accounts.len(), 3);
        assert_eq!(accounts[0].kind(), AccountKind::Money);
        assert_eq!(accounts[0].found_in(), [FoundIn::AddedByUser]);
        assert_eq!(
            accounts[0].login(),
            &AccountLogin::KeyPlace(KeyRef::File(
                "/home/user/.config/deepseek-key".into()
            ))
        );
        assert_eq!(accounts[2].added_key(), None);
    }

    fn deepseek_host() -> ProviderRef {
        ProviderRef::Host(String::from("api.deepseek.com"))
    }

    /// A registry with the DeepSeek entry its plugin puts in it.
    fn registry() -> Registry {
        let part = otty_plugin_host::PluginPart::new(
            "deepseek", "DeepSeek", "0.1.0", "", "deepseek",
        );
        let deepseek = crate::provider::ProviderEntry::new(
            crate::provider::ProviderId::new("deepseek"),
            "DeepSeek",
            part,
        )
        .with_hosts(["deepseek.com"]);

        Registry::new(Vec::new(), vec![deepseek])
    }

    /// Spec "What the UI shows": a key for a host no entry serves is an
    /// account known by that host.
    #[test]
    fn a_key_for_a_host_no_entry_serves_stays_known_by_the_host() {
        let registry = registry()
            .without_provider(&crate::provider::ProviderId::new("deepseek"));

        let accounts = merge_accounts(
            vec![key(deepseek_host(), "sk-made-up", "/home/user/.claude")],
            &registry,
        );

        assert_eq!(accounts[0].provider(), &deepseek_host());
    }

    /// Spec "Identity and links do not depend on a provider plugin": the
    /// same key found in two agents is one account, with an entry for its
    /// host or without one.
    #[test]
    fn the_same_key_in_two_agents_is_one_account_with_or_without_an_entry() {
        let claude = key(deepseek_host(), "sk-made-up", "/home/user/.claude");
        let opencode_by_host = key(
            deepseek_host(),
            "sk-made-up",
            "/home/user/.local/share/opencode",
        );
        let opencode_by_id = key(
            ProviderRef::of_id("deepseek"),
            "sk-made-up",
            "/home/user/.local/share/opencode",
        );
        let without = registry()
            .without_provider(&crate::provider::ProviderId::new("deepseek"));

        let with_entry =
            merge_accounts(vec![claude.clone(), opencode_by_id], &registry());
        let without_entry =
            merge_accounts(vec![claude, opencode_by_host], &without);

        assert_eq!(with_entry.len(), 1);
        assert_eq!(with_entry[0].provider(), &ProviderRef::of_id("deepseek"));
        assert_eq!(with_entry[0].found_in().len(), 2);
        assert_eq!(without_entry.len(), 1);
        assert_eq!(without_entry[0].provider(), &deepseek_host());
        assert_eq!(without_entry[0].found_in().len(), 2);
    }

    /// A registry with two plugins that both serve the provider
    /// `anthropic`, and one that offers Add account for it.
    fn two_anthropic_readers() -> Registry {
        let entry = |plugin: &str, add_by_hand: bool| {
            let part = otty_plugin_host::PluginPart::new(
                plugin,
                plugin,
                "0.1.0",
                "",
                "anthropic",
            );
            let entry = crate::provider::ProviderEntry::new(
                crate::provider::ProviderId::new("anthropic"),
                "Anthropic",
                part,
            )
            .with_hosts(["anthropic.com"]);
            match add_by_hand {
                true => entry.with_add_by_hand(),
                false => entry,
            }
        };

        Registry::new(
            Vec::new(),
            vec![entry("anthropic", true), entry("anthropic-fork", false)],
        )
    }

    fn claude_login(folder: &str) -> Account {
        Account::new(
            ProviderRef::of_id("anthropic"),
            AccountIdentity::AccountId("made-up-account".into()),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder(folder.into()),
        )
    }

    /// Issue 29: two plugins that serve one provider each read its
    /// accounts, so one login found in two profiles is two accounts, one
    /// per plugin, each found in both profiles; an account the user added
    /// is read by each plugin that offers Add account for its provider.
    #[test]
    fn each_plugin_that_serves_an_account_reads_its_own_copy() {
        let added = Account::added(
            ProviderRef::of_id("anthropic"),
            KeyRef::EnvVar("ANTHROPIC_ADMIN_KEY".into()),
        );

        let accounts = merge_accounts(
            vec![
                claude_login("/home/user/.claude"),
                claude_login("/home/user/.claude-work"),
                added,
            ],
            &two_anthropic_readers(),
        );

        let readers: Vec<Option<&str>> =
            accounts.iter().map(Account::read_by).collect();
        assert_eq!(
            readers,
            [Some("anthropic"), Some("anthropic-fork"), Some("anthropic")]
        );
        assert_eq!(accounts[0].found_in().len(), 2);
        assert_eq!(accounts[1].found_in().len(), 2);
        assert!(!accounts[0].is_same_account(&accounts[1]));
        assert!(accounts[0].is_same_login(&accounts[1]));
    }

    /// A registry where the plugin `cloud` serves `example.com` as the
    /// provider `cloud`, and the plugin `fork` serves it as `cloud-fork`.
    fn one_host_two_ids() -> Registry {
        let entry = |plugin: &str, id: &str| {
            let part = otty_plugin_host::PluginPart::new(
                plugin, plugin, "0.1.0", "", id,
            );
            crate::provider::ProviderEntry::new(
                crate::provider::ProviderId::new(id),
                id,
                part,
            )
            .with_hosts(["example.com"])
        };

        Registry::new(
            Vec::new(),
            vec![entry("cloud", "cloud"), entry("fork", "cloud-fork")],
        )
    }

    /// Issue 31: a key for a host two plugins serve under two provider ids
    /// is read by both, one account per plugin under its id. The same key
    /// found for the id `cloud` joins the copy of `cloud` only: joining
    /// stays inside one provider id.
    #[test]
    fn a_host_two_plugins_serve_under_two_ids_is_read_by_both() {
        let host = ProviderRef::Host(String::from("api.example.com"));
        let by_host = key(host, "sk-made-up", "/home/user/.claude");
        let by_id = key(
            ProviderRef::of_id("cloud"),
            "sk-made-up",
            "/home/user/.local/share/opencode",
        );

        let accounts =
            merge_accounts(vec![by_host, by_id], &one_host_two_ids());

        let read: Vec<(&str, Option<&str>, usize)> = accounts
            .iter()
            .map(|account| {
                (
                    account.provider().as_str(),
                    account.read_by(),
                    account.found_in().len(),
                )
            })
            .collect();
        assert_eq!(
            read,
            [("cloud", Some("cloud"), 2), ("cloud-fork", Some("fork"), 1)]
        );
        assert!(!accounts[0].is_same_login(&accounts[1]));
    }

    /// Issue 31: tokens a profile sent to the host are paid by each copy
    /// of its key, whichever provider id serves the host.
    #[test]
    fn each_copy_of_a_key_for_a_host_pays_for_its_tokens() {
        let host = ProviderRef::Host(String::from("api.example.com"));
        let registry = one_host_two_ids();
        let accounts = merge_accounts(
            vec![key(host.clone(), "sk-made-up", "/home/user/.claude")],
            &registry,
        );
        let tokens = ModelTokens::new(
            Some("made-up-model"),
            crate::token_count::TokenCount::new(10, 0, 0),
            1,
        )
        .with_provider(Some(host));

        assert_eq!(accounts.len(), 2);
        assert!(accounts[0].pays_for(&tokens, &accounts, &registry));
        assert!(accounts[1].pays_for(&tokens, &accounts, &registry));
    }

    /// Issue 29: an account no installed plugin reads is still one
    /// account, read by none; merged again it stays as it is.
    #[test]
    fn an_account_no_plugin_reads_is_read_by_none() {
        let accounts = merge_accounts(
            vec![claude_login("/home/user/.claude")],
            &Registry::default(),
        );
        let again = merge_accounts(accounts.clone(), &Registry::default());

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].read_by(), None);
        assert_eq!(again, accounts);
    }

    /// Issue 29: the tokens of a profile go to the login that paid them
    /// whichever plugin reads it: each copy of one login pays for them.
    #[test]
    fn each_copy_of_the_only_login_of_a_profile_pays_for_its_tokens() {
        let registry = two_anthropic_readers();
        let accounts =
            merge_accounts(vec![claude_login("/home/user/.claude")], &registry);
        let tokens = ModelTokens::new(
            Some("claude-sonnet-4-5"),
            crate::token_count::TokenCount::new(10, 0, 0),
            1,
        )
        .with_provider(Some(ProviderRef::of_id("anthropic")));

        assert_eq!(accounts.len(), 2);
        assert!(accounts[0].pays_for(&tokens, &accounts, &registry));
        assert!(accounts[1].pays_for(&tokens, &accounts, &registry));
    }

    /// The tokens of a profile whose server is named by its host are paid
    /// by its key, whether or not an entry serves the host.
    #[test]
    fn tokens_for_a_host_are_paid_by_the_key_of_the_provider_of_the_host() {
        let tokens = ModelTokens::new(
            Some("deepseek-chat"),
            crate::token_count::TokenCount::new(10, 0, 0),
            1,
        )
        .with_provider(Some(deepseek_host()));
        let found = key(deepseek_host(), "sk-made-up", "/home/user/.claude");
        let registry = registry();
        let resolved = merge_accounts(vec![found.clone()], &registry);
        let without = registry
            .clone()
            .without_provider(&crate::provider::ProviderId::new("deepseek"));

        assert!(resolved[0].pays_for(&tokens, &resolved, &registry));
        assert!(found.pays_for(
            &tokens,
            std::slice::from_ref(&found),
            &without
        ));
    }
}
