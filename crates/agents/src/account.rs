//! An account: where the user pays at a provider, a subscription, a key
//! or an organization, as the provider part of one plugin found it (spec
//! 2026-11-23-accounts-apart-from-profiles, Decisions 5 and 8). The part
//! gives each account an id that stays the same between reads, and
//! decides itself whether two logins are one account. OTTY joins no
//! accounts: two plugins that read one provider give two accounts
//! (Decision 1).

use std::path::{Path, PathBuf};

use crate::agent::AgentId;
use crate::part_need::PartNeed;
use crate::provider::ProviderId;

/// What OTTY keeps the user's choices for an account by, its name and
/// whether it is hidden: the plugin whose provider part found it and the
/// part's id of the account, unique within the plugin.
///
/// ```
/// use otty_agents::AccountKey;
///
/// let key = AccountKey::new("example", "key:made-up");
///
/// assert_eq!(key.plugin(), "example");
/// assert_eq!(key.id(), "key:made-up");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountKey {
    plugin: String,
    id: String,
}

impl AccountKey {
    /// The id of the plugin, such as `openai`.
    pub fn plugin(&self) -> &str {
        &self.plugin
    }

    /// The part's id of the account, such as `chatgpt:<account id>`.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The account `id` of the plugin `plugin`.
    pub fn new(plugin: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            plugin: plugin.into(),
            id: id.into(),
        }
    }
}

/// What kind of account it is (spec Decision 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    /// A subscription with windows of limits or a pool of credits, such
    /// as a ChatGPT or a Claude plan.
    Subscription,
    /// A key paid per token: money left and spent.
    Key,
    /// An organization that pays for what its members use, whose bill
    /// may be read.
    Organization,
}

/// Where an account was found. OTTY's own logins join as another place
/// with issue 08.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoundIn {
    /// The config folder of a profile of an agent part of the same
    /// plugin, such as `~/.codex` of Codex.
    ProfileFolder {
        /// The agent part whose profile it is.
        agent: AgentId,
        /// The profile's config folder.
        folder: PathBuf,
    },
}

impl FoundIn {
    /// The profile folder `folder` of the agent `agent`.
    pub fn profile(agent: AgentId, folder: impl Into<PathBuf>) -> Self {
        Self::ProfileFolder {
            agent,
            folder: folder.into(),
        }
    }

    /// The config folder of the profile the account was found in.
    pub fn folder(&self) -> &Path {
        match self {
            Self::ProfileFolder { folder, .. } => folder,
        }
    }
}

/// One account at a provider, as one provider part found it.
///
/// The key or the token of the account is never part of it: the part
/// reads them where the agent keeps them, as handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    key: AccountKey,
    provider: ProviderId,
    name: String,
    kind: AccountKind,
    plan: Option<String>,
    found_in: Vec<FoundIn>,
    needs: Vec<PartNeed>,
}

impl Account {
    /// What OTTY keeps the user's choices for the account by.
    pub fn key(&self) -> &AccountKey {
        &self.key
    }

    /// The id of the plugin whose provider part found the account.
    pub fn plugin(&self) -> &str {
        self.key.plugin()
    }

    /// The id of the provider part that found the account, such as
    /// `openai`.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// What the part named the account, such as `you@example.com` or
    /// `key ••••a1f3`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// What kind of account it is.
    pub fn kind(&self) -> AccountKind {
        self.kind
    }

    /// The plan of a subscription, such as "Max 20x" or "Pro", when the
    /// part tells.
    pub fn plan(&self) -> Option<&str> {
        self.plan.as_deref()
    }

    /// Every place the account was found, in the order the part gave.
    pub fn found_in(&self) -> &[FoundIn] {
        &self.found_in
    }

    /// What the user must do for the account, as its part said when it
    /// found it, such as log in again.
    pub fn needs(&self) -> &[PartNeed] {
        &self.needs
    }

    /// Whether the account was found in the profile folder `folder` of the
    /// agent `agent`.
    pub fn is_found_in(&self, agent: &AgentId, folder: &Path) -> bool {
        self.found_in.iter().any(|place| match place {
            FoundIn::ProfileFolder {
                agent: found_by,
                folder: found_at,
            } => found_by == agent && found_at == folder,
        })
    }

    /// The account `id` of the plugin `plugin`, found by its provider
    /// part `provider` in `found_in`, named `name`, with no plan and no
    /// need.
    ///
    /// ```
    /// use otty_agents::{Account, AccountKind, AgentId, FoundIn, ProviderId};
    ///
    /// let account = Account::new(
    ///     "example",
    ///     ProviderId::new("example-cloud"),
    ///     "chatgpt:made-up",
    ///     "you@example.com",
    ///     AccountKind::Subscription,
    ///     vec![FoundIn::profile(AgentId::new("coder"), "/home/user/.coder")],
    /// )
    /// .with_plan(Some("Pro".into()));
    ///
    /// assert_eq!(account.key().id(), "chatgpt:made-up");
    /// assert_eq!(account.plan(), Some("Pro"));
    /// assert_eq!(account.found_in().len(), 1);
    /// ```
    pub fn new(
        plugin: impl Into<String>,
        provider: ProviderId,
        id: impl Into<String>,
        name: impl Into<String>,
        kind: AccountKind,
        found_in: Vec<FoundIn>,
    ) -> Self {
        Self {
            key: AccountKey::new(plugin, id),
            provider,
            name: name.into(),
            kind,
            plan: None,
            found_in,
            needs: Vec::new(),
        }
    }

    /// The same account with a plan, or none.
    pub fn with_plan(mut self, plan: Option<String>) -> Self {
        self.plan = plan;
        self
    }

    /// The same account with the needs its part gave for it.
    pub fn with_needs(mut self, needs: Vec<PartNeed>) -> Self {
        self.needs = needs;
        self
    }

    /// Whether `other` is this account, maybe read at another time: the
    /// same plugin and the same id.
    pub fn is_same_account(&self, other: &Account) -> bool {
        self.key == other.key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found_in(folder: &str) -> FoundIn {
        FoundIn::profile(AgentId::new("codex"), folder)
    }

    fn account(plugin: &str, id: &str) -> Account {
        Account::new(
            plugin,
            ProviderId::new("openai"),
            id,
            "you@example.com",
            AccountKind::Subscription,
            vec![found_in("/home/user/.codex")],
        )
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 1: one id of
    /// two plugins is two accounts.
    #[test]
    fn an_account_is_known_by_its_plugin_and_its_id() {
        let one = account("openai", "chatgpt:made-up");
        let renamed = Account::new(
            "openai",
            ProviderId::new("openai"),
            "chatgpt:made-up",
            "Another name",
            AccountKind::Subscription,
            Vec::new(),
        );

        assert!(one.is_same_account(&renamed));
        assert!(!one.is_same_account(&account("other", "chatgpt:made-up")));
        assert!(!one.is_same_account(&account("openai", "chatgpt:other")));
    }

    #[test]
    fn an_account_is_found_in_the_folder_of_its_agent_only() {
        let account = account("openai", "chatgpt:made-up");

        let codex = AgentId::new("codex");
        assert!(account.is_found_in(&codex, Path::new("/home/user/.codex")));
        assert!(
            !account.is_found_in(&codex, Path::new("/home/user/.codex-work"))
        );
        assert!(!account.is_found_in(
            &AgentId::new("other"),
            Path::new("/home/user/.codex")
        ));
    }
}
