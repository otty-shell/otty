//! Who sells the model use an agent runs on (spec "Words"), as data: the
//! id of a provider, what an account names its provider by, and the entry
//! a provider has in the [`Registry`] (spec "Links in the interface").
//!
//! [`Registry`]: crate::Registry

use std::borrow::Cow;

use crate::account::AccountKind;
use crate::registry::Reader;

/// The prefix of a provider saved in `agents.json` by the host of its
/// server, such as `host:api.deepseek.com`.
const HOST_PREFIX: &str = "host:";

/// The id of a provider, such as `deepseek`.
///
/// It is the name the provider is saved under, so it never changes once a
/// provider is released; a new id is a new provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProviderId(String);

impl ProviderId {
    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A provider id from its text.
    ///
    /// ```
    /// use otty_agents::ProviderId;
    ///
    /// assert_eq!(ProviderId::new("deepseek").as_str(), "deepseek");
    /// ```
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// What an account or the tokens of a model name their provider by (spec
/// "Identity and links do not depend on a provider plugin"): the provider
/// id when the agent named one, else the host of the server the key is
/// for, in lower case, such as `api.deepseek.com`.
///
/// The [`Registry`] turns a host into the id of the provider that serves
/// it, when one does; with no such provider the account stays known by
/// its host.
///
/// [`Registry`]: crate::Registry
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProviderRef {
    /// A provider by its id.
    Id(ProviderId),
    /// The host of a server no id was named for.
    Host(String),
}

impl ProviderRef {
    /// The id, when the provider is named by one.
    pub fn as_id(&self) -> Option<&ProviderId> {
        match self {
            Self::Id(id) => Some(id),
            Self::Host(_) => None,
        }
    }

    /// The host, when the provider is named by the host of its server.
    pub fn as_host(&self) -> Option<&str> {
        match self {
            Self::Host(host) => Some(host),
            Self::Id(_) => None,
        }
    }

    /// The id or the host as text, such as `deepseek` or
    /// `api.deepseek.com`: what the app shows for a provider no entry
    /// names.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Id(id) => id.as_str(),
            Self::Host(host) => host,
        }
    }

    /// Whether this is the provider with the id `id`.
    ///
    /// ```
    /// use otty_agents::ProviderRef;
    ///
    /// assert!(ProviderRef::of_id("openai").is_id("openai"));
    /// assert!(!ProviderRef::of_id("openai").is_id("anthropic"));
    /// ```
    pub fn is_id(&self, id: &str) -> bool {
        self.as_id().is_some_and(|own| own.as_str() == id)
    }

    /// The provider with the id `id`.
    ///
    /// ```
    /// use otty_agents::ProviderRef;
    ///
    /// assert_eq!(ProviderRef::of_id("openai").as_str(), "openai");
    /// ```
    pub fn of_id(id: impl Into<String>) -> Self {
        Self::Id(ProviderId::new(id))
    }

    /// The provider of the server at `base_url`, by its host in lower
    /// case, such as `api.deepseek.com` of
    /// `https://api.deepseek.com/anthropic`. `None` when it is not an
    /// address of the web.
    ///
    /// ```
    /// use otty_agents::ProviderRef;
    ///
    /// let deepseek = ProviderRef::of_base_url("https://API.DeepSeek.com/x");
    ///
    /// assert_eq!(deepseek.unwrap().as_host(), Some("api.deepseek.com"));
    /// assert_eq!(ProviderRef::of_base_url("not a url"), None);
    /// ```
    pub fn of_base_url(base_url: &str) -> Option<Self> {
        host_of(base_url).map(Self::Host)
    }

    /// The text the provider is saved as in `agents.json`: its id, or
    /// `host:` and its host.
    pub(crate) fn to_saved(&self) -> String {
        match self {
            Self::Id(id) => id.as_str().to_string(),
            Self::Host(host) => format!("{HOST_PREFIX}{host}"),
        }
    }

    /// The provider saved as `text`; `None` for an empty one.
    pub(crate) fn from_saved(text: &str) -> Option<Self> {
        let text = text.trim();

        let reference = match text.strip_prefix(HOST_PREFIX) {
            Some(host) => Self::Host(host.trim().to_ascii_lowercase()),
            None => Self::of_id(text),
        };

        (!reference.as_str().is_empty()).then_some(reference)
    }
}

/// What Add account offers for a provider (spec 2026-11-19, Decision 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddAccountKind {
    /// A key with money left on it, such as a DeepSeek key.
    Key,
    /// The admin key of an organization, which reads its bill, such as an
    /// Anthropic Console organization.
    AdminKey,
}

/// What the app shows and needs to know of a provider (spec "Links in the
/// interface"): its id, name and icon, the pages of its usage, the hosts
/// of its servers, what Add account offers for it, and what reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderEntry {
    id: ProviderId,
    name: String,
    icon: Option<Cow<'static, [u8]>>,
    usage_page: Option<String>,
    plan_usage_page: Option<String>,
    hosts: Vec<String>,
    add_account: Option<AddAccountKind>,
    reader: Reader,
}

impl ProviderEntry {
    /// The id of the provider.
    pub fn id(&self) -> &ProviderId {
        &self.id
    }

    /// The name shown to the user, such as "OpenRouter".
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The SVG of its icon, in one color the app tints; `None` when it has
    /// none, and the app draws the first letter of its name.
    pub fn icon(&self) -> Option<&Cow<'static, [u8]>> {
        self.icon.as_ref()
    }

    /// The domains of its servers, such as `deepseek.com`. A server is the
    /// provider's when its host is one of them or ends with `.` and one.
    pub fn hosts(&self) -> &[String] {
        &self.hosts
    }

    /// What Add account offers for the provider; `None` when it is not
    /// offered.
    pub fn add_account(&self) -> Option<AddAccountKind> {
        self.add_account
    }

    /// What reads the provider's accounts.
    pub fn reader(&self) -> &Reader {
        &self.reader
    }

    /// The page of the provider where the user sees the use of an account
    /// of `kind`, opened in the browser: the plan's page for a
    /// subscription, the console's usage page for a key or an
    /// organization. `None` when the provider names none.
    pub fn usage_page(&self, kind: AccountKind) -> Option<&str> {
        let plan = match kind {
            AccountKind::Money => None,
            _ => self.plan_usage_page.as_deref(),
        };

        plan.or(self.usage_page.as_deref())
    }

    /// Whether the provider serves `host`: it is one of its domains or
    /// ends with `.` and one.
    ///
    /// ```
    /// use otty_agents::{ProviderEntry, ProviderId, Reader};
    ///
    /// let deepseek = ProviderEntry::new(
    ///     ProviderId::new("deepseek"),
    ///     "DeepSeek",
    ///     Reader::BuiltIn,
    /// )
    /// .with_hosts(["deepseek.com"]);
    ///
    /// assert!(deepseek.serves_host("api.deepseek.com"));
    /// assert!(!deepseek.serves_host("notdeepseek.com"));
    /// ```
    pub fn serves_host(&self, host: &str) -> bool {
        self.hosts.iter().any(|domain| is_host_of(host, domain))
    }

    /// An entry with no icon, no usage page, no host, and not offered by
    /// Add account.
    pub fn new(
        id: ProviderId,
        name: impl Into<String>,
        reader: Reader,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            icon: None,
            usage_page: None,
            plan_usage_page: None,
            hosts: Vec::new(),
            add_account: None,
            reader,
        }
    }

    /// The same entry with the SVG `icon`.
    pub fn with_icon(mut self, icon: impl Into<Cow<'static, [u8]>>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// The same entry whose servers are at `hosts`, in lower case.
    pub fn with_hosts<'a>(
        mut self,
        hosts: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        self.hosts = hosts
            .into_iter()
            .map(|host| host.trim().to_ascii_lowercase())
            .filter(|host| !host.is_empty())
            .collect();
        self
    }

    /// The same entry whose usage page is `page`: the page of a key or an
    /// organization, and of a subscription too unless
    /// [`ProviderEntry::with_plan_usage_page`] names another.
    pub fn with_usage_page(mut self, page: impl Into<String>) -> Self {
        self.usage_page = Some(page.into());
        self
    }

    /// The same entry whose subscriptions show their use on `page`.
    pub fn with_plan_usage_page(mut self, page: impl Into<String>) -> Self {
        self.plan_usage_page = Some(page.into());
        self
    }

    /// The same entry offered by Add account as `kind`.
    pub fn with_add_account(mut self, kind: AddAccountKind) -> Self {
        self.add_account = Some(kind);
        self
    }
}

/// Whether `host` is `domain` or ends with `.` and `domain`.
fn is_host_of(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|start| start.ends_with('.'))
}

/// The host of `url` in lower case, such as `api.deepseek.com` of
/// `https://api.deepseek.com/anthropic`; `None` when it is not an address
/// of the web.
fn host_of(url: &str) -> Option<String> {
    let url = url.trim().to_ascii_lowercase();
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;

    let host = rest.split(['/', ':', '?', '#']).next().unwrap_or_default();

    (!host.is_empty()).then(|| host.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deepseek_entry() -> ProviderEntry {
        ProviderEntry::new(
            ProviderId::new("deepseek"),
            "DeepSeek",
            Reader::BuiltIn,
        )
        .with_hosts(["deepseek.com"])
    }

    #[test]
    fn a_base_url_names_the_host_of_its_server_in_lower_case() {
        let cases = [
            ("https://api.anthropic.com", "api.anthropic.com"),
            ("https://api.deepseek.com/anthropic", "api.deepseek.com"),
            ("https://openrouter.ai/api", "openrouter.ai"),
            ("http://localhost:4000", "localhost"),
            (" HTTPS://API.DeepSeek.com ", "api.deepseek.com"),
        ];

        for (base_url, host) in cases {
            assert_eq!(
                ProviderRef::of_base_url(base_url),
                Some(ProviderRef::Host(host.to_string())),
                "{base_url}"
            );
        }
    }

    #[test]
    fn what_is_not_an_address_of_the_web_names_no_server() {
        for base_url in ["not a url", "", "https://", "ftp://example.com"] {
            assert_eq!(ProviderRef::of_base_url(base_url), None, "{base_url}");
        }
    }

    /// Today's `PROVIDER_DOMAINS` rule: a server is a provider's when its
    /// host is one of the provider's domains or ends with `.` and one.
    #[test]
    fn a_provider_serves_its_domains_and_their_subdomains_only() {
        let deepseek = deepseek_entry();

        assert!(deepseek.serves_host("deepseek.com"));
        assert!(deepseek.serves_host("api.deepseek.com"));
        for host in ["deepseek.com.example.com", "notdeepseek.com", "com"] {
            assert!(!deepseek.serves_host(host), "{host}");
        }
    }

    #[test]
    fn a_provider_is_saved_by_its_id_or_by_host_and_read_back() {
        let by_id = ProviderRef::of_id("deepseek");
        let by_host = ProviderRef::Host(String::from("api.deepseek.com"));

        assert_eq!(by_id.to_saved(), "deepseek");
        assert_eq!(by_host.to_saved(), "host:api.deepseek.com");
        assert_eq!(ProviderRef::from_saved("deepseek"), Some(by_id));
        assert_eq!(
            ProviderRef::from_saved("host:API.deepseek.com"),
            Some(by_host)
        );
        assert_eq!(ProviderRef::from_saved(""), None);
        assert_eq!(ProviderRef::from_saved("host:"), None);
    }

    /// Open usage page: a subscription opens the page of its plan, a key
    /// or an organization the usage page of the provider's console.
    #[test]
    fn a_subscription_opens_its_plan_page_and_a_key_the_console() {
        let anthropic = ProviderEntry::new(
            ProviderId::new("anthropic"),
            "Anthropic",
            Reader::BuiltIn,
        )
        .with_usage_page("https://console.anthropic.com/settings/usage")
        .with_plan_usage_page("https://claude.ai/settings/usage");
        let deepseek = deepseek_entry()
            .with_usage_page("https://platform.deepseek.com/usage");

        assert_eq!(
            anthropic.usage_page(AccountKind::Limits),
            Some("https://claude.ai/settings/usage")
        );
        assert_eq!(
            anthropic.usage_page(AccountKind::Money),
            Some("https://console.anthropic.com/settings/usage")
        );
        assert_eq!(
            deepseek.usage_page(AccountKind::Limits),
            Some("https://platform.deepseek.com/usage")
        );
        assert_eq!(deepseek_entry().usage_page(AccountKind::Money), None);
    }
}
