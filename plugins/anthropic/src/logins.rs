//! The logins of a Claude Code profile folder, as the provider part
//! `anthropic` finds its accounts in them (spec
//! 2026-11-23-accounts-apart-from-profiles, Decisions 5 and 8). OTTY gives
//! the part the folders of the `claude-code` profiles, and the part reads
//! the login files there itself, with every key replaced by a handle.
//!
//! A folder can hold three logins:
//!
//! - A Claude login: the token of `.credentials.json` and the account in
//!   `oauthAccount` of `.claude.json`. It is a subscription known by the
//!   account's id, which stays the same when Claude Code renews the
//!   tokens, and named by its email. A login to an Anthropic Console
//!   organization that pays per token (`billingType` `prepaid`, no plan)
//!   is that organization: its bill is the organization's, so everyone
//!   logged in to it is one account.
//! - A Console key, `primaryApiKey` of `.claude.json`.
//! - A key in the `env` of `settings.json` (`ANTHROPIC_AUTH_TOKEN`, else
//!   `ANTHROPIC_API_KEY`). A key sent to another server
//!   (`ANTHROPIC_BASE_URL`, such as DeepSeek's) is no Anthropic account
//!   (spec scenario "Claude Code pointed at DeepSeek").
//!
//! A key is known by its fingerprint, so one key in two places is one
//! account; an admin key (`sk-ant-admin…`) is an organization whose bill
//! it reads.
//!
//! Whether a login expired is the part's call. The token of a Claude
//! login expires within hours and Claude Code renews it with its refresh
//! token on its next run, also in the `/usage` probe, so a token past its
//! time is no need. A token past its time with no refresh token beside it
//! can not be renewed: that login expired, and the account gets the need
//! that runs `claude` in its folder, or in OTTY's own login "Log in
//! again", Claude Code's login in that folder.

use otty_plugin_sdk::agents::{Account, AccountKind, LoginFolder, ProviderRef};
use otty_plugin_sdk::{Key, Need, clock};

use crate::claude_code::{claude_in_folder, claude_login_in};
use crate::config_files::{
    Credentials, GlobalConfig, KeyFile, OauthAccount, OauthLogin, Settings,
    read_credentials, read_global_configs, read_settings,
};
use crate::needs::{self, ADMIN_PREFIX};
use crate::provider::{ANTHROPIC, provider_of};

/// The agent part whose profiles hold the logins.
pub(crate) const CLAUDE_CODE: &str = "claude-code";

/// The host of Anthropic's API, as `ANTHROPIC_BASE_URL` may name it.
const ANTHROPIC_API_HOST: &str = "api.anthropic.com";

/// One login found in one folder: the account it is, found in that
/// folder only, and how the part reads it.
pub(crate) struct FolderLogin {
    account: Account,
    access: Access,
}

impl FolderLogin {
    /// The account of the login.
    pub(crate) fn account(&self) -> &Account {
        &self.account
    }

    /// The account and how the part reads it.
    pub(crate) fn into_parts(self) -> (Account, Access) {
        (self.account, self.access)
    }
}

/// How the part reads the account of a login.
pub(crate) enum Access {
    /// A Claude login, with its token when `.credentials.json` holds one
    /// (Claude Code may keep it elsewhere and still work), and whether
    /// that token is past its time, so Claude Code renews it on its next
    /// run.
    Claude {
        /// The handle of the login token.
        token: Option<Key>,
        /// Whether the token is past its `expiresAt`.
        is_past: bool,
    },
    /// An API key or an admin key.
    Key(Key),
}

/// The logins of the profile folder `folder`: the Claude login first,
/// then the Console key, then the key of the settings. OTTY's own login
/// folder is read as a profile folder. A folder of another agent part, or
/// with no login, gives none.
pub(crate) fn logins(folder: &LoginFolder) -> Vec<FolderLogin> {
    let Some(path) = claude_folder(folder) else {
        return Vec::new();
    };

    let configs = read_global_configs(path);
    let settings = read_settings(path);

    let claude = configs
        .iter()
        .find_map(|config| config.fields().oauth_account())
        .and_then(|account| claude_login(folder, path, account));
    let console = configs
        .into_iter()
        .find_map(console_key)
        .and_then(|key| key_login(folder, key));
    let settings = settings
        .and_then(settings_key)
        .and_then(|key| key_login(folder, key));

    [claude, console, settings].into_iter().flatten().collect()
}

/// The id of the account that pays for the answers Claude Code gets in the
/// profile folder `folder` now, an account paid per token (issue 22 of
/// spec 2026-11-23-accounts-apart-from-profiles): the key of the settings,
/// which Claude Code sends first, else the Console organization of its
/// login, else its Console key. Its Console key is only how Claude Code
/// reaches the organization, so the organization pays. `None` when a
/// Claude subscription pays, when the settings send Claude Code to
/// another server than Anthropic's, for an admin key, which runs no
/// session, and for OTTY's own login folder, whose sessions are the
/// `/usage` probe's.
pub(crate) fn paying_account(folder: &LoginFolder) -> Option<String> {
    let LoginFolder::Profile((agent, path)) = folder else {
        return None;
    };
    if agent != CLAUDE_CODE {
        return None;
    }
    let settings = read_settings(path);
    let provider = provider_of(settings.as_ref().map(KeyFile::fields))?;
    if !is_anthropic(&provider) {
        return None;
    }

    let settings_login = settings
        .and_then(settings_key)
        .and_then(|key| key_login(folder, key));
    if let Some(login) = settings_login {
        let account = login.account;
        return (account.kind == AccountKind::Key).then_some(account.id);
    }

    let configs = read_global_configs(path);
    let claude = configs
        .iter()
        .find_map(|config| config.fields().oauth_account())
        .and_then(|account| claude_login(folder, path, account));
    if let Some(login) = claude {
        let account = login.account;
        return (account.kind == AccountKind::Organization)
            .then_some(account.id);
    }

    let console = configs
        .into_iter()
        .find_map(console_key)
        .and_then(|key| key_login(folder, key))?;
    let account = console.account;
    (account.kind == AccountKind::Key).then_some(account.id)
}

/// The path of `folder` when it is a Claude Code config folder: a
/// profile folder of `claude-code`, or OTTY's own login folder, where
/// `claude auth login` ran with `CLAUDE_CONFIG_DIR` (spec
/// 2026-11-23-accounts-apart-from-profiles, Decision 6).
pub(crate) fn claude_folder(folder: &LoginFolder) -> Option<&str> {
    match folder {
        LoginFolder::Profile((agent, path)) if agent == CLAUDE_CODE => {
            Some(path)
        },
        LoginFolder::OwnLogin((_, path)) => Some(path),
        _ => None,
    }
}

/// The Claude login of the account `account` in the profile folder
/// `path`, given to the part as `place`, with the plan and the token of
/// its login file. A login whose files name neither an account id nor an
/// email can not be told from another, so it is none.
fn claude_login(
    place: &LoginFolder,
    path: &str,
    account: &OauthAccount,
) -> Option<FolderLogin> {
    let credentials = read_credentials(path);
    let login = credentials
        .as_ref()
        .and_then(|credentials| credentials.fields().login());
    let plan = login.and_then(OauthLogin::plan);
    let is_past = login.is_some_and(is_past_its_time);
    let has_lapsed = credentials
        .as_ref()
        .zip(login)
        .is_some_and(|(credentials, login)| has_lapsed(credentials, login));
    let token_marker = login.and_then(OauthLogin::token);
    let token =
        credentials.and_then(|credentials| credentials.into_key(token_marker));

    let organization = account
        .organization_id()
        .filter(|_| account.is_console() && plan.is_none());
    let (id, name, kind, plan) = match organization {
        Some(organization) => (
            format!("console:{organization}"),
            organization_name(account, organization),
            AccountKind::Organization,
            None,
        ),
        None => (
            format!("claude:{}", account.account_id().or(account.email())?),
            subscription_name(account),
            AccountKind::Subscription,
            plan,
        ),
    };

    let needs = match has_lapsed {
        true => vec![lapsed_login_need(place, path)],
        false => Vec::new(),
    };
    Some(FolderLogin {
        account: Account {
            id,
            name,
            kind,
            plan,
            found_in: vec![place.clone()],
            needs,
        },
        access: Access::Claude { token, is_past },
    })
}

/// What a Claude login in `place` at `path` that can not be renewed
/// needs: Claude Code run in a profile folder, which renews it or asks to
/// log in again, or for OTTY's own login "Log in again", Claude Code's
/// login in that folder (spec 2026-11-23-accounts-apart-from-profiles,
/// Decision 9).
fn lapsed_login_need(place: &LoginFolder, path: &str) -> Need {
    match place {
        LoginFolder::OwnLogin(_) => needs::log_in_again(claude_login_in(path)),
        _ => needs::login_expired(claude_in_folder(path)),
    }
}

/// Whether the Claude login `login` of `credentials` expired for good:
/// its token is past its time and no refresh token is there to renew it.
/// The agent part checks the same for the profile (`detect.rs`).
pub(crate) fn has_lapsed(
    credentials: &KeyFile<Credentials>,
    login: &OauthLogin,
) -> bool {
    is_past_its_time(login) && !credentials.holds_key(login.refresh_token())
}

/// Whether the token of the Claude login `login` is past its time.
fn is_past_its_time(login: &OauthLogin) -> bool {
    login
        .expires_at_ms()
        .is_some_and(|expires_at| expires_at <= clock::now_ms())
}

/// What the user knows a Claude subscription by: its email, then a comma
/// and the name of its organization when it is not the personal one
/// Claude makes for every account, `<email>'s Organization`, such as
/// "you@example.com, Example Org". OTTY puts the name in parentheses after
/// the provider, so a comma keeps them from nesting, and the organization
/// keeps no parentheses of its own.
fn subscription_name(account: &OauthAccount) -> String {
    let email = account.email().or(account.account_id()).unwrap_or_default();

    match account.organization_name() {
        Some(organization) if !organization.contains(email) => {
            let organization = without_parentheses(organization);
            format!("{email}, {organization}")
        },
        _ => email.to_string(),
    }
}

/// What the user knows a Console organization by: its name, with no
/// parentheses, else the email of the login, else its id.
fn organization_name(account: &OauthAccount, organization_id: &str) -> String {
    let name = account
        .organization_name()
        .or(account.email())
        .unwrap_or(organization_id);

    without_parentheses(name)
}

/// `name` with each part in parentheses set off by commas instead, such
/// as "Example Org, Claude API Platform" for Anthropic's "Example Org
/// (Claude API Platform)". OTTY names an account "Anthropic (…)", and
/// parentheses inside would nest (issue 21 of spec
/// 2026-11-23-accounts-apart-from-profiles). Every word stays, so two
/// organizations that differ only in the part in parentheses stay apart.
fn without_parentheses(name: &str) -> String {
    let parts: Vec<&str> = name
        .split(['(', ')'])
        .map(|part| part.trim().trim_matches(',').trim())
        .filter(|part| !part.is_empty())
        .collect();

    parts.join(", ")
}

/// The Console key of the global config file `config`, when it holds one.
fn console_key(config: KeyFile<GlobalConfig>) -> Option<Key> {
    let marker = config.fields().primary_api_key();

    config.into_key(marker)
}

/// The key the settings give Claude Code in their variables:
/// `ANTHROPIC_AUTH_TOKEN`, which Claude Code sends first, else
/// `ANTHROPIC_API_KEY`. None when the settings send Claude Code to a
/// server other than Anthropic's.
fn settings_key(settings: KeyFile<Settings>) -> Option<Key> {
    let fields = settings.fields();
    let provider = provider_of(Some(fields))?;
    if !is_anthropic(&provider) {
        return None;
    }

    let marker = match settings.holds_key(fields.auth_token()) {
        true => fields.auth_token(),
        false => fields.api_key(),
    };
    settings.into_key(marker)
}

/// Whether `provider` is Anthropic: no other server set, or Anthropic's
/// own API.
fn is_anthropic(provider: &ProviderRef) -> bool {
    match provider {
        ProviderRef::Id(id) => id == ANTHROPIC,
        ProviderRef::Host(host) => host == ANTHROPIC_API_HOST,
    }
}

/// The account of the key `key` found in `place`: known by the key's
/// fingerprint and named by its end, an organization for an admin key.
fn key_login(place: &LoginFolder, key: Key) -> Option<FolderLogin> {
    let fingerprint = key.fingerprint().ok()?;
    let is_admin = key.has_prefix(ADMIN_PREFIX).unwrap_or(false);

    let kind = match is_admin {
        true => AccountKind::Organization,
        false => AccountKind::Key,
    };
    Some(FolderLogin {
        account: Account {
            id: format!("key:{fingerprint}"),
            name: short_key(&fingerprint),
            kind,
            plan: None,
            found_in: vec![place.clone()],
            needs: Vec::new(),
        },
        access: Access::Key(key),
    })
}

/// A key shown by the end of its fingerprint, such as `key ••••a1f3`: the
/// key itself is never read.
fn short_key(fingerprint: &str) -> String {
    let start = fingerprint.len().saturating_sub(4);

    format!("key ••••{}", &fingerprint[start..])
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::NeedAction;
    use otty_plugin_sdk::test_host::TestHost;

    use super::*;
    use crate::profiles::CONFIG_VARIABLE;
    use crate::testing::{as_provider_in, host, login_folder, path};

    /// A time after the made-up tokens of the fixtures expired, which
    /// expire on 2026-11-16, and after the one of `.claude-expired`.
    const AFTER_EXPIRY_MS: i64 = 1_800_000_000_000;

    /// The accounts of the logins in the folder `name` of `host`, read in
    /// a call given that folder.
    fn accounts_of(host: &TestHost, name: &str) -> Vec<Account> {
        let folder = login_folder(host, name);

        as_provider_in(host, std::slice::from_ref(&folder), || {
            logins(&folder)
                .into_iter()
                .map(|login| login.into_parts().0)
                .collect()
        })
    }

    fn accounts_in(name: &str) -> Vec<Account> {
        accounts_of(&host(), name)
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 8: a Claude
    /// login is a subscription known by its account and named by its
    /// email and its organization, with its plan.
    #[test]
    fn a_claude_login_is_a_subscription_named_by_its_email() {
        let host = host();

        let found = accounts_of(&host, ".claude-work");

        assert_eq!(
            found,
            [Account {
                id: "claude:00000000-0000-4000-8000-00000000000c".to_string(),
                name: "own@example.com, Example Org".to_string(),
                kind: AccountKind::Subscription,
                plan: Some("Max 20x".to_string()),
                found_in: vec![login_folder(&host, ".claude-work")],
                needs: Vec::new(),
            }]
        );
    }

    /// The default folder `~/.claude` keeps its global config file next
    /// to it, `~/.claude.json`, outside the folder. A provider part may
    /// not declare a key file outside its login folders yet, so the
    /// manifest can not give the part that file.
    #[test]
    fn the_default_folder_finds_the_account_next_to_it() {
        let found = accounts_in(".claude");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "dev@example.com, Example Org");
        assert_eq!(found[0].plan.as_deref(), Some("Max 20x"));
    }

    /// Claude makes every account a personal organization named after
    /// its email; that name tells nothing more.
    #[test]
    fn a_personal_organization_leaves_the_email_alone() {
        let found = accounts_in(".claude-expired");

        assert_eq!(found[0].name, "old@example.com");
    }

    /// Decision 8: the id stays when Claude Code renews the tokens.
    #[test]
    fn the_id_stays_the_same_after_the_tokens_are_renewed() {
        let before = accounts_in(".claude-work");
        let after = accounts_in(".claude-renewed");

        assert_eq!(before[0].id, after[0].id);
    }

    /// The bill of a Console organization belongs to the organization, so
    /// everyone logged in to it is one account.
    #[test]
    fn a_prepaid_login_is_its_organization() {
        let found = accounts_in(".claude-prepaid");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "console:00000000-0000-4000-8000-0000000000dd");
        assert_eq!(found[0].name, "Example Org");
        assert_eq!(found[0].kind, AccountKind::Organization);
        assert_eq!(found[0].plan, None);
    }

    /// The account of a login whose organization Anthropic names
    /// `organization`.
    fn account_of(organization: &str) -> OauthAccount {
        let account = serde_json::json!({
            "emailAddress": "dev@example.com",
            "organizationName": organization,
        });

        serde_json::from_value(account).expect("an account")
    }

    /// Issue 21 of spec 2026-11-23-accounts-apart-from-profiles: OTTY puts
    /// the name in parentheses after the provider, so the parentheses of
    /// the organization's own name become commas and never nest.
    #[test]
    fn an_organization_is_named_with_no_parentheses() {
        for (given, name) in [
            (
                "Example Org (Claude API Platform)",
                "Example Org, Claude API Platform",
            ),
            ("Example Org, (Team) Ltd", "Example Org, Team, Ltd"),
            ("(Example Org)", "Example Org"),
            ("Example Org", "Example Org"),
        ] {
            let account = account_of(given);

            assert_eq!(organization_name(&account, "org"), name, "{given}");
        }
    }

    /// Issue 21: the organization after the email of a subscription keeps
    /// no parentheses either.
    #[test]
    fn a_subscription_is_named_with_no_parentheses() {
        let account = account_of("Example Org (Claude Team)");

        let name = subscription_name(&account);

        assert_eq!(name, "dev@example.com, Example Org, Claude Team");
    }

    #[test]
    fn a_prepaid_login_with_a_plan_is_a_subscription() {
        let found = accounts_in(".claude-prepaid-max");

        assert_eq!(found[0].id, "claude:00000000-0000-4000-8000-00000000000d");
        assert_eq!(found[0].kind, AccountKind::Subscription);
    }

    #[test]
    fn a_login_with_no_id_is_known_by_its_email() {
        let found = accounts_in(".claude-email");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "claude:dev@example.com");
        assert_eq!(found[0].name, "dev@example.com");
    }

    /// A token with no account in the files can not be told apart after
    /// it is renewed, and a folder with no login has no account; neither
    /// gives a need here: a missing login is the agent part's need.
    #[test]
    fn a_folder_with_no_account_in_its_files_gives_none() {
        assert_eq!(accounts_in(".claude-token"), []);
        assert_eq!(accounts_in(".claude-nothing"), []);
        assert_eq!(accounts_in(".claude-no-token"), []);
        assert_eq!(accounts_in(".claude-broken"), []);
    }

    #[test]
    fn a_console_key_is_a_key_named_by_its_fingerprint() {
        let found = accounts_in(".claude-console");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, AccountKind::Key);
        assert!(found[0].id.starts_with("key:"), "{}", found[0].id);
        assert!(found[0].name.starts_with("key ••••"), "{}", found[0].name);
        assert_eq!(
            found[0].name.chars().count(),
            "key ••••a1f3".chars().count()
        );
    }

    /// The same key as the Console key of `.claude-console` is the same
    /// account.
    #[test]
    fn an_anthropic_key_of_the_settings_is_a_key() {
        let in_settings = accounts_in(".claude-api");
        let in_console = accounts_in(".claude-console");

        assert_eq!(in_settings.len(), 1);
        assert_eq!(in_settings[0].kind, AccountKind::Key);
        assert_eq!(in_settings[0].id, in_console[0].id);
    }

    /// Spec scenario "Claude Code pointed at DeepSeek": a key the settings
    /// send to another server is no Anthropic account.
    #[test]
    fn a_key_for_another_server_is_no_account() {
        assert_eq!(accounts_in(".claude-deepseek"), []);
        assert_eq!(accounts_in(".claude-gateway"), []);
        assert_eq!(accounts_in(".claude-local"), []);
    }

    #[test]
    fn empty_null_and_blank_keys_are_none() {
        assert_eq!(accounts_in(".claude-nullkey"), []);
        assert_eq!(accounts_in(".claude-blank"), []);
    }

    #[test]
    fn an_admin_key_is_an_organization() {
        let found = accounts_in(".claude-admin");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, AccountKind::Organization);
    }

    #[test]
    fn a_login_and_a_key_in_one_folder_are_two_accounts() {
        let kinds: Vec<AccountKind> = accounts_in(".claude-mixed")
            .iter()
            .map(|account| account.kind)
            .collect();

        assert_eq!(kinds, [AccountKind::Subscription, AccountKind::Key]);
    }

    /// Decision 9: a token past its time with no refresh token can not be
    /// renewed; the account needs `claude` in its folder.
    #[test]
    fn an_expired_login_needs_claude_in_its_folder() {
        let host = host().with_now_ms(AFTER_EXPIRY_MS);
        let folder = path(&host, ".claude-expired");

        let found = accounts_of(&host, ".claude-expired");

        assert_eq!(found[0].needs.len(), 1);
        let need = &found[0].needs[0];
        assert_eq!(need.id, "login-expired");
        let NeedAction::Run(command) = &need.action else {
            panic!("a command: {need:?}");
        };
        assert_eq!(command.program, "claude");
        assert_eq!(command.env, [(CONFIG_VARIABLE.to_string(), folder)]);
    }

    /// Issue 09 of spec 2026-11-23-accounts-apart-from-profiles, Decision
    /// 9: OTTY's own login that can not be renewed needs "Log in again",
    /// which runs Claude Code's login in that folder.
    #[test]
    fn an_expired_own_login_needs_log_in_again() {
        let host = host().with_now_ms(AFTER_EXPIRY_MS);
        let folder = path(&host, ".claude-expired");
        let own = LoginFolder::OwnLogin(("a1".to_string(), folder.clone()));

        let found = as_provider_in(&host, std::slice::from_ref(&own), || {
            logins(&own)
                .into_iter()
                .map(|login| login.into_parts().0)
                .collect::<Vec<_>>()
        });

        assert_eq!(found[0].needs.len(), 1);
        let need = &found[0].needs[0];
        assert_eq!(need.id, "log-in-again");
        let NeedAction::Run(command) = &need.action else {
            panic!("a command: {need:?}");
        };
        assert_eq!(command.args, ["auth", "login", "--claudeai"]);
        assert_eq!(command.env, [(CONFIG_VARIABLE.to_string(), folder)]);
    }

    /// Claude Code renews a token past its time with its refresh token.
    #[test]
    fn a_login_with_a_refresh_token_needs_nothing() {
        let host = host().with_now_ms(AFTER_EXPIRY_MS);

        let found = accounts_of(&host, ".claude-work");

        assert_eq!(found[0].needs, []);
    }

    #[test]
    fn a_login_before_its_time_needs_nothing() {
        let host = host().with_now_ms(1_700_000_000_000);

        let found = accounts_of(&host, ".claude-expired");

        assert_eq!(found[0].needs, []);
    }

    /// Decision 6: OTTY's own login folder, where `claude auth login` ran
    /// with `CLAUDE_CONFIG_DIR`, holds its Claude login as a profile
    /// folder outside the default one does, and the account names it.
    #[test]
    fn an_own_login_folder_holds_its_claude_login() {
        let host = host();
        let own = LoginFolder::OwnLogin((
            "6f0c1e2a".to_string(),
            path(&host, ".claude-work"),
        ));

        let found = as_provider_in(&host, std::slice::from_ref(&own), || {
            logins(&own)
                .into_iter()
                .map(|login| login.into_parts().0)
                .collect::<Vec<_>>()
        });

        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].name, "own@example.com, Example Org");
        assert_eq!(found[0].found_in, [own]);
    }

    #[test]
    fn a_folder_of_another_agent_part_gives_none() {
        let host = host();
        let folder =
            LoginFolder::Profile(("other".to_string(), path(&host, ".claude")));

        let found = as_provider_in(&host, &[], || logins(&folder));

        assert!(found.is_empty());
    }

    #[test]
    fn no_key_or_token_reaches_the_plugin() {
        let host = host();
        let names = [".claude", ".claude-mixed", ".claude-console"];

        let found: Vec<Account> = names
            .iter()
            .flat_map(|name| accounts_of(&host, name))
            .collect();

        let text = format!("{found:?}");
        for secret in [
            "made-up-access-token",
            "made-up-refresh-token",
            "sk-ant-made-up",
        ] {
            assert!(!text.contains(secret), "{secret} in {text}");
        }
    }

    #[test]
    fn a_key_is_named_by_the_end_of_its_fingerprint() {
        assert_eq!(short_key("27c8d30e"), "key ••••d30e");
    }
}
