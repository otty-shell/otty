//! The accounts the provider part `openai` finds in the Codex profile
//! folders OTTY gives it (spec 2026-11-23-accounts-apart-from-profiles,
//! Decisions 5 and 8). The `auth.json` of each folder logs in with a
//! ChatGPT login or an API key, as its `auth_mode` says; the part reads it
//! with every key replaced by a handle.
//!
//! A ChatGPT login is a subscription known by its account id, from
//! `tokens.account_id` or the id token, so the account keeps its id after
//! Codex renews the tokens. The id token, a JWT, also names the email and
//! the plan (`chatgpt_plan_type`); OTTY gives the plugin only its claims.
//! An API key tells nothing of its account: it is known by its
//! fingerprint. One account found in two folders is one account with two
//! places. A login Codex sends to a gateway (`base_url.rs`) is for the
//! gateway's host and is no OpenAI account.
//!
//! Whether a login expired is the plugin's call (Decision 9): a ChatGPT
//! login whose access token says it expired gets the need to run
//! `codex login` in its folder.

use otty_plugin_sdk::agents::{
    Account, AccountKind, Found, LoginFolder, ProviderRef,
};
use otty_plugin_sdk::{Key, Need};

use crate::auth_file::AuthFile;
use crate::base_url::{CodexConfig, OPENAI};
use crate::needs::{self, ADMIN_PREFIX};
use crate::rate_limits::plan_name;
use crate::token_expiry::has_expired;

/// The agent part whose profiles hold the logins the part reads.
pub(crate) const CODEX: &str = "codex";

/// The ChatGPT login of a folder.
pub(crate) struct ChatgptLogin {
    auth: AuthFile,
    account_id: String,
    email: Option<String>,
    plan: Option<String>,
}

impl ChatgptLogin {
    /// The ChatGPT workspace of the login, which the usage request names.
    pub(crate) fn account_id(&self) -> &str {
        &self.account_id
    }

    /// The handle of the access token.
    pub(crate) fn token(&self) -> Option<&Key> {
        self.auth.access_token()
    }

    /// The id of the account: the same in every folder and after the
    /// tokens are renewed.
    fn id(&self) -> String {
        format!("chatgpt:{}", self.account_id)
    }

    /// The email of the login, else the end of its account id.
    fn name(&self) -> String {
        match &self.email {
            Some(email) => email.clone(),
            None => format!("ChatGPT ••••{}", last_four(&self.account_id)),
        }
    }

    /// Whether the access token says it expired.
    fn has_expired(&self) -> bool {
        self.token().is_some_and(has_expired)
    }
}

/// The API key of a folder.
pub(crate) struct KeyLogin {
    auth: AuthFile,
    fingerprint: String,
    is_admin: bool,
}

impl KeyLogin {
    /// The handle of the key.
    pub(crate) fn key(&self) -> Option<&Key> {
        self.auth.api_key()
    }

    /// Whether the key is an admin key, which reads the bill of its
    /// organization.
    pub(crate) fn is_admin(&self) -> bool {
        self.is_admin
    }

    /// The id of the account: the key's fingerprint.
    fn id(&self) -> String {
        format!("key:{}", self.fingerprint)
    }

    /// The key shown by the end of its fingerprint, such as
    /// `key ••••a1f3`: the key itself is never read.
    fn name(&self) -> String {
        format!("key ••••{}", last_four(&self.fingerprint))
    }
}

/// The OpenAI login of one Codex profile folder.
pub(crate) enum FolderLogin {
    /// A ChatGPT login, paid for by a subscription.
    Chatgpt(ChatgptLogin),
    /// An API key, paid per token.
    Key(KeyLogin),
}

impl FolderLogin {
    /// The login of `auth.json` in `folder`, the one its `auth_mode`
    /// says Codex logs in with; `None` when there is none, it is for a
    /// gateway, or it names no account it can be known by.
    pub(crate) fn read(folder: &str) -> Option<Self> {
        let auth = AuthFile::read(folder)?;
        let config = CodexConfig::read(folder);

        match auth.is_api_key() {
            true => key_login(auth, &config).map(Self::Key),
            false => chatgpt_login(auth, &config).map(Self::Chatgpt),
        }
    }

    /// The id of the account the login is of, such as
    /// `chatgpt:<account id>` or `key:<fingerprint>`.
    pub(crate) fn id(&self) -> String {
        match self {
            Self::Chatgpt(login) => login.id(),
            Self::Key(login) => login.id(),
        }
    }

    /// The account of the login, found in `folder` at `path`.
    fn account(&self, folder: &LoginFolder, path: &str) -> Account {
        let (name, kind, plan) = match self {
            Self::Chatgpt(login) => {
                (login.name(), AccountKind::Subscription, login.plan.clone())
            },
            Self::Key(login) if login.is_admin => {
                (login.name(), AccountKind::Organization, None)
            },
            Self::Key(login) => (login.name(), AccountKind::Key, None),
        };

        Account {
            id: self.id(),
            name,
            kind,
            plan,
            found_in: vec![folder.clone()],
            needs: self.needs(folder, path),
        }
    }

    /// What the user must do for the login in `folder` at `path`: log in
    /// again when its token expired. In OTTY's own login an expired token
    /// is no need: the part reads there with `codex app-server`, which
    /// renews its own login (spec 2026-11-23-accounts-apart-from-profiles,
    /// Decision 7).
    fn needs(&self, folder: &LoginFolder, path: &str) -> Vec<Need> {
        if let LoginFolder::OwnLogin(_) = folder {
            return Vec::new();
        }

        match self {
            Self::Chatgpt(login) if login.has_expired() => {
                vec![needs::login_expired(path)]
            },
            _ => Vec::new(),
        }
    }
}

/// The accounts whose logins sit in `folders`, the Codex profile folders
/// of one call, in the order they were given. A folder with no OpenAI
/// login gives nothing: the agent part says when nobody is logged in.
pub(crate) fn accounts(folders: &[LoginFolder]) -> Found {
    let mut accounts: Vec<Account> = Vec::new();

    for folder in folders {
        let Some(path) = codex_folder(folder) else {
            continue;
        };
        let Some(login) = FolderLogin::read(path) else {
            continue;
        };

        let id = login.id();
        match accounts.iter_mut().find(|known| known.id == id) {
            Some(known) if known.found_in.contains(folder) => {},
            Some(known) => {
                known.found_in.push(folder.clone());
                known.needs.extend(login.needs(folder, path));
            },
            None => accounts.push(login.account(folder, path)),
        }
    }

    Found {
        accounts,
        needs: Vec::new(),
    }
}

/// The Codex config folders `account` was found in: its profile folders
/// and OTTY's own login folders.
pub(crate) fn codex_folders(account: &Account) -> Vec<&str> {
    account.found_in.iter().filter_map(codex_folder).collect()
}

/// The folders of OTTY's own logins `account` was found in (spec
/// 2026-11-23-accounts-apart-from-profiles, Decision 6).
pub(crate) fn own_login_folders(account: &Account) -> Vec<&str> {
    account
        .found_in
        .iter()
        .filter_map(|folder| match folder {
            LoginFolder::OwnLogin((_, path)) => Some(path.as_str()),
            _ => None,
        })
        .collect()
}

/// Whether the Codex profile folder `folder` holds no login at all, of
/// OpenAI or of a gateway.
pub(crate) fn has_no_login(folder: &str) -> bool {
    AuthFile::read(folder).is_none_or(|auth| !auth.is_logged_in())
}

/// The path of `folder` when it is a Codex config folder: a profile
/// folder of Codex, or OTTY's own login folder, where `codex login` ran
/// (spec 2026-11-23-accounts-apart-from-profiles, Decision 6).
fn codex_folder(folder: &LoginFolder) -> Option<&str> {
    match folder {
        LoginFolder::Profile((agent, path)) if agent == CODEX => Some(path),
        LoginFolder::OwnLogin((_, path)) => Some(path),
        _ => None,
    }
}

/// The API key of `auth`, when the config sends it to OpenAI.
fn key_login(auth: AuthFile, config: &CodexConfig) -> Option<KeyLogin> {
    if !is_openai(&config.api_key_for()) {
        return None;
    }

    let key = auth.api_key()?;
    let fingerprint = key.fingerprint().ok()?;
    let is_admin = key.has_prefix(ADMIN_PREFIX).unwrap_or(false);

    Some(KeyLogin {
        auth,
        fingerprint,
        is_admin,
    })
}

/// The ChatGPT login of `auth`, when the config sends it to OpenAI and it
/// names its account.
fn chatgpt_login(auth: AuthFile, config: &CodexConfig) -> Option<ChatgptLogin> {
    if !is_openai(&config.chatgpt_login_for()) || !auth.has_access_token() {
        return None;
    }

    let account_id = auth.chatgpt_account_id()?;
    let claims = auth.id_claims();
    let email = claims.email().map(String::from);
    let plan = claims.plan_type().and_then(plan_name);

    Some(ChatgptLogin {
        auth,
        account_id,
        email,
        plan,
    })
}

/// Whether `provider` is OpenAI itself.
fn is_openai(provider: &ProviderRef) -> bool {
    matches!(provider, ProviderRef::Id(id) if id == OPENAI)
}

/// The last four characters of `text`.
fn last_four(text: &str) -> String {
    let count = text.chars().count();

    text.chars().skip(count.saturating_sub(4)).collect()
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{Command, NeedAction};

    use super::*;
    use crate::testing::{
        ACCOUNT, PROVIDER, TEN, codex_folder, host, own_folder, path,
    };

    /// What the part finds in the Codex profile folders `names`, in a
    /// call given them.
    fn found_in(host: &TestHost, names: &[&str]) -> Found {
        let folders: Vec<LoginFolder> =
            names.iter().map(|name| codex_folder(host, name)).collect();

        host.as_provider_in(PROVIDER, &folders, || accounts(&folders))
    }

    /// The one account the part finds in the Codex profile folder `name`.
    fn account_in(host: &TestHost, name: &str) -> Account {
        let mut found = found_in(host, &[name]).accounts;

        assert_eq!(found.len(), 1, "{name}: {found:?}");
        found.remove(0)
    }

    /// The fingerprint the host gives the made-up key `value`.
    fn fingerprint_of(host: &TestHost, value: &str) -> String {
        let key = host.key(value);

        host.as_provider(PROVIDER, || key.fingerprint())
            .expect("a fingerprint")
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 8: a ChatGPT
    /// login is a subscription named by its email, with its plan, and one
    /// account logged in to two folders is one account with two places.
    #[test]
    fn one_chatgpt_account_in_two_folders_is_one_subscription() {
        let host = host();

        let found = found_in(&host, &[".codex-plus", ".codex-renewed"]);

        assert_eq!(
            found.accounts,
            [Account {
                id: format!("chatgpt:{ACCOUNT}"),
                name: "dev@example.com".to_string(),
                kind: AccountKind::Subscription,
                plan: Some("Pro 200".to_string()),
                found_in: vec![
                    codex_folder(&host, ".codex-plus"),
                    codex_folder(&host, ".codex-renewed"),
                ],
                needs: Vec::new(),
            }]
        );
        assert!(found.needs.is_empty());
    }

    /// Decision 8: OTTY keeps the user's name and Hide by the id, so the
    /// account keeps it after Codex renews the tokens of `auth.json`.
    #[test]
    fn an_account_keeps_its_id_after_its_tokens_are_renewed() {
        let host = host();

        let before = account_in(&host, ".codex-plus");
        let after = account_in(&host, ".codex-renewed");

        assert_eq!(before.id, after.id);
    }

    #[test]
    fn two_chatgpt_accounts_are_two_subscriptions() {
        let host = host();

        let found = found_in(&host, &[".codex-plus", ".codex-team"]);

        let names: Vec<&str> = found
            .accounts
            .iter()
            .map(|account| account.name.as_str())
            .collect();
        assert_eq!(names, ["dev@example.com", "dev@team.example.com"]);
        assert_ne!(found.accounts[0].id, found.accounts[1].id);
        assert_eq!(found.accounts[1].plan.as_deref(), Some("Business"));
    }

    /// An API key is a key known by its fingerprint and named by its end;
    /// the same key in two folders is one account.
    #[test]
    fn an_api_key_is_a_key_named_by_its_fingerprint() {
        let host = host();
        let fingerprint = fingerprint_of(&host, "sk-made-up-key");
        let end = last_four(&fingerprint);

        let found = found_in(&host, &[".codex-work", ".codex-nomode"]);

        assert_eq!(
            found.accounts,
            [Account {
                id: format!("key:{fingerprint}"),
                name: format!("key ••••{end}"),
                kind: AccountKind::Key,
                plan: None,
                found_in: vec![
                    codex_folder(&host, ".codex-work"),
                    codex_folder(&host, ".codex-nomode"),
                ],
                needs: Vec::new(),
            }]
        );
    }

    #[test]
    fn an_admin_key_is_an_organization() {
        let host = host();

        let account = account_in(&host, ".codex-admin");

        assert_eq!(account.kind, AccountKind::Organization);
        assert!(account.id.starts_with("key:"), "{}", account.id);
    }

    /// Older versions of Codex kept a key next to a ChatGPT login: the
    /// mode tells which one Codex logs in with.
    #[test]
    fn the_mode_of_the_file_picks_the_login() {
        let host = host();

        let account = account_in(&host, ".codex-both");

        assert_eq!(account.kind, AccountKind::Subscription);
    }

    /// The id token can be missing or not a JWT: the account id of the
    /// tokens still tells the account, and names it by its end.
    #[test]
    fn a_login_with_no_readable_id_token_is_named_by_its_account() {
        let host = host();

        let account = account_in(&host, ".codex");

        assert_eq!(account.id, "chatgpt:00000000-0000-4000-8000-000000000000");
        assert_eq!(account.name, "ChatGPT ••••0000");
        assert_eq!(account.plan, None);
    }

    /// A login Codex sends to a gateway, the ChatGPT login of
    /// `chatgpt_base_url` or the key of a provider of the user's config,
    /// is for the gateway's host: no OpenAI account.
    #[test]
    fn a_login_behind_a_gateway_is_no_account() {
        let host = host();

        let found = found_in(&host, &[".codex-gateway", ".codex-proxy"]);

        assert!(found.accounts.is_empty(), "{found:?}");
        assert!(found.needs.is_empty());
    }

    /// Decision 9: a folder nobody is logged in with gives no account and
    /// no need here; that need is the agent part's.
    #[test]
    fn a_folder_with_no_login_gives_no_account_and_no_need() {
        let host = host();

        let found = found_in(
            &host,
            &[
                ".codex-gone",
                ".codex-empty",
                ".codex-broken",
                ".codex-loggedout",
                ".codex-blank",
                ".codex-emptyauth",
                ".codex-notokens",
                ".codex-nameless",
            ],
        );

        assert!(found.accounts.is_empty(), "{found:?}");
        assert!(found.needs.is_empty());
    }

    /// Decision 9: the files show the token expired, so the account needs
    /// `codex login` in the folder it was found in, in the language of
    /// OTTY.
    #[test]
    fn an_expired_login_needs_codex_login_in_its_folder() {
        let host = host().with_now_ms(TEN).with_language("ru");
        let folder = path(&host, ".codex-expired");

        let account = account_in(&host, ".codex-expired");

        assert_eq!(account.needs.len(), 1);
        let need = &account.needs[0];
        assert_eq!(need.id, "login-expired");
        assert_eq!(need.title, "Вход истёк");
        assert_eq!(
            need.action,
            NeedAction::Run(Command {
                program: "codex".to_string(),
                args: vec!["login".to_string()],
                env: vec![("CODEX_HOME".to_string(), folder)],
            })
        );
    }

    #[test]
    fn a_fresh_login_needs_nothing() {
        let host = host().with_now_ms(TEN);

        let account = account_in(&host, ".codex-plus");

        assert!(account.needs.is_empty());
    }

    /// Decision 6: OTTY's own login folder, where `codex login` ran, holds
    /// an account as a Codex profile folder does.
    #[test]
    fn an_own_login_folder_holds_its_account() {
        let host = host();
        let own = own_folder(&host, "6f0c1e2a", ".codex-plus");

        let found =
            host.as_provider_in(PROVIDER, std::slice::from_ref(&own), || {
                accounts(std::slice::from_ref(&own))
            });

        assert_eq!(found.accounts.len(), 1, "{found:?}");
        assert_eq!(found.accounts[0].name, "dev@example.com");
        assert_eq!(found.accounts[0].found_in, [own]);
    }

    /// Decision 8: one ChatGPT account in `~/.codex` and in OTTY's own
    /// login is one account with both places.
    #[test]
    fn one_account_in_a_profile_and_an_own_login_is_one_account() {
        let host = host();
        let folders = [
            codex_folder(&host, ".codex-plus"),
            own_folder(&host, "6f0c1e2a", ".codex-renewed"),
        ];

        let found =
            host.as_provider_in(PROVIDER, &folders, || accounts(&folders));

        assert_eq!(found.accounts.len(), 1, "{found:?}");
        assert_eq!(found.accounts[0].found_in, folders);
    }

    /// Decision 5: the part reads only the profiles of its own plugin's
    /// agent part.
    #[test]
    fn a_folder_of_another_agent_part_is_not_read() {
        let host = host();
        let folder = path(&host, ".codex-plus");
        let folders = [LoginFolder::Profile(("opencode".to_string(), folder))];

        let found =
            host.as_provider_in(PROVIDER, &folders, || accounts(&folders));

        assert!(found.accounts.is_empty());
    }

    #[test]
    fn no_key_or_token_reaches_an_account() {
        let host = host();

        let found = found_in(
            &host,
            &[".codex", ".codex-plus", ".codex-work", ".codex-admin"],
        );

        let text = format!("{found:?}");
        for secret in [
            "e30.",
            "made-up-access-token",
            "made-up-refresh-token",
            "sk-made-up",
            "sk-admin-made-up",
        ] {
            assert!(!text.contains(secret), "{secret} in {text}");
        }
    }
}
