//! OTTY's own login of a provider part (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 6): Add account →
//! "Log in with <agent>" makes an empty folder of OTTY's own,
//! `<plugins folder>/<plugin>/accounts/<id>/` (issue 13), runs the agent's
//! official login command there in a terminal tab, and then asks the
//! provider part for the accounts in that folder. OTTY never handles a
//! password or a login flow itself, and the folder is never a profile: no
//! session or project of it is read.
//!
//! The folder belongs to the plugin that made the login: it sits next to
//! the plugin's version folders, so an update of the plugin keeps it and
//! removing the plugin removes it. `agents.json` keeps each own login by
//! its id, the plugin, the provider part and the agent part; the folder is
//! always found from the plugin and the id.

use std::path::{Component, Path, PathBuf};

use crate::account::Account;
use crate::agent::AgentId;
use crate::command::AgentCommand;
use crate::error::{SourceError, Unavailable};
use crate::plugin_provider::PluginProvider;
use crate::provider::ProviderId;

/// The folder of a plugin's folder that holds the own logins its provider
/// parts made.
const ACCOUNTS_FOLDER: &str = "accounts";

/// One own login: which provider part made it, with which agent part, and
/// the id its folder is named by.
///
/// ```
/// use std::path::{Path, PathBuf};
///
/// use otty_agents::{AgentId, OwnLogin, ProviderId};
///
/// let login = OwnLogin::new(
///     "6f0c1e2a",
///     "example",
///     ProviderId::new("example-cloud"),
///     AgentId::new("coder"),
/// );
///
/// assert_eq!(
///     login.folder_in(Path::new("/data/otty/plugins")),
///     PathBuf::from("/data/otty/plugins/example/accounts/6f0c1e2a")
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnLogin {
    id: String,
    plugin: String,
    provider: ProviderId,
    agent: AgentId,
}

impl OwnLogin {
    /// The id, a plain name its folder is named by.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The id of the plugin whose provider part made it.
    pub fn plugin(&self) -> &str {
        &self.plugin
    }

    /// The id of the provider part that made it, such as `openai`.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// The agent part whose login command logged in there, such as
    /// `codex`.
    pub fn agent(&self) -> &AgentId {
        &self.agent
    }

    /// The own login `id` made by the provider part `provider` of the
    /// plugin `plugin`, logged in with the agent part `agent`.
    pub fn new(
        id: impl Into<String>,
        plugin: impl Into<String>,
        provider: ProviderId,
        agent: AgentId,
    ) -> Self {
        Self {
            id: id.into(),
            plugin: plugin.into(),
            provider,
            agent,
        }
    }

    /// Its folder: `<plugin>/accounts/<id>` in the folder of the installed
    /// plugins `plugins_folder`.
    pub fn folder_in(&self, plugins_folder: &Path) -> PathBuf {
        plugins_folder
            .join(&self.plugin)
            .join(ACCOUNTS_FOLDER)
            .join(&self.id)
    }

    /// Whether the provider part `provider` of the plugin `plugin` made
    /// it: only that part reads its folder.
    pub fn is_of(&self, plugin: &str, provider: &ProviderId) -> bool {
        self.plugin == plugin && self.provider == *provider
    }
}

/// Whether `folder` is inside the own login folders of a plugin in the
/// folder of the installed plugins `plugins_folder`, at
/// `<plugin>/accounts/` or below it. Such a folder can not be a profile:
/// Remove of an own login, and removing its plugin, delete it.
///
/// ```
/// use std::path::Path;
///
/// let plugins = Path::new("/data/otty/plugins");
///
/// assert!(otty_agents::is_own_login_folder(
///     Path::new("/data/otty/plugins/openai/accounts/6f0c1e2a"),
///     plugins,
/// ));
/// assert!(!otty_agents::is_own_login_folder(
///     Path::new("/home/user/.codex"),
///     plugins,
/// ));
/// ```
pub fn is_own_login_folder(folder: &Path, plugins_folder: &Path) -> bool {
    let Ok(inside) = folder.strip_prefix(plugins_folder) else {
        return false;
    };

    let mut parts = inside.components();
    let plugin = parts.next();
    let accounts = parts.next();
    matches!(plugin, Some(Component::Normal(_)))
        && accounts == Some(Component::Normal(ACCOUNTS_FOLDER.as_ref()))
}

/// The agent's official login command for `login`, pointed at its folder
/// in `plugins_folder`, as the provider part of `providers` that made it
/// gives it. It blocks.
///
/// # Errors
///
/// [`Unavailable::NoLogIn`] when no part of `providers` made it or the
/// part gives no command; a failure of the part.
pub fn log_in_command(
    providers: &[PluginProvider],
    login: &OwnLogin,
    plugins_folder: &Path,
) -> Result<AgentCommand, SourceError> {
    let no_log_in = || SourceError::NotAvailable(Unavailable::NoLogIn);
    let provider = providers
        .iter()
        .find(|part| part.made(login))
        .ok_or_else(no_log_in)?;

    provider
        .log_in(&login.folder_in(plugins_folder))?
        .ok_or_else(no_log_in)
}

/// The accounts the provider part of `providers` that made `login` finds
/// in its folder in `plugins_folder` alone, as after its login tab closed.
/// Only the accounts found in that folder count. It blocks.
///
/// # Errors
///
/// [`Unavailable::NoAccountSource`] when no part of `providers` made it;
/// a failure of the part.
pub fn own_login_accounts(
    providers: &[PluginProvider],
    login: &OwnLogin,
    plugins_folder: &Path,
) -> Result<Vec<Account>, SourceError> {
    let provider = providers
        .iter()
        .find(|part| part.made(login))
        .ok_or(SourceError::NotAvailable(Unavailable::NoAccountSource))?;

    let (mut accounts, _) = provider.accounts(
        &[],
        &[],
        std::slice::from_ref(login),
        plugins_folder,
    )?;

    accounts.retain(|account| account.is_found_in_own_login(login.id()));
    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::agents::provider::LoginFolder;

    use super::*;
    use crate::account::{AccountKind, FoundIn};
    use crate::plugin_provider::MockProviderPartCalls;

    const PLUGINS_FOLDER: &str = "/home/user/.local/share/otty/plugins";

    fn own_login() -> OwnLogin {
        OwnLogin::new(
            "6f0c1e2a",
            "example",
            ProviderId::new("example-cloud"),
            AgentId::new("coder"),
        )
    }

    fn own_folder() -> PathBuf {
        PathBuf::from(
            "/home/user/.local/share/otty/plugins/example/accounts/6f0c1e2a",
        )
    }

    /// The account the part finds in OTTY's own login folder.
    fn own_account() -> Account {
        Account::new(
            "example",
            ProviderId::new("example-cloud"),
            "key:made-up",
            "dev@example.com",
            AccountKind::Key,
            vec![FoundIn::own_login("6f0c1e2a", own_folder())],
        )
    }

    fn example_cloud(calls: MockProviderPartCalls) -> PluginProvider {
        PluginProvider::made_up("example-cloud", &["coder"], calls)
    }

    /// Issue 13 of spec 2026-11-23-accounts-apart-from-profiles: the
    /// folder of an own login belongs to the plugin that made it,
    /// `<plugins folder>/<plugin>/accounts/<id>/`, next to its version
    /// folders.
    #[test]
    fn the_folder_of_an_own_login_is_under_its_plugin() {
        assert_eq!(
            own_login().folder_in(Path::new(PLUGINS_FOLDER)),
            own_folder()
        );
    }

    /// Issue 13: a folder inside `<plugins folder>/*/accounts/` is an own
    /// login folder, or the folder that holds them; a version folder of a
    /// plugin, the plugins folder itself and a folder elsewhere are not.
    #[test]
    fn a_folder_inside_the_accounts_of_a_plugin_is_an_own_login_folder() {
        let plugins = Path::new(PLUGINS_FOLDER);
        let inside = [
            "/home/user/.local/share/otty/plugins/openai/accounts",
            "/home/user/.local/share/otty/plugins/openai/accounts/6f0c1e2a",
            "/home/user/.local/share/otty/plugins/openai/accounts/a1/.codex",
        ];
        let outside = [
            "/home/user/.local/share/otty/plugins",
            "/home/user/.local/share/otty/plugins/openai",
            "/home/user/.local/share/otty/plugins/openai/0.2.13",
            "/home/user/.local/share/otty/plugins/accounts",
            "/home/user/.local/share/otty/accounts/6f0c1e2a",
            "/home/user/.codex",
        ];

        for folder in inside {
            assert!(
                is_own_login_folder(Path::new(folder), plugins),
                "{folder}"
            );
        }
        for folder in outside {
            assert!(
                !is_own_login_folder(Path::new(folder), plugins),
                "{folder}"
            );
        }
    }

    #[test]
    fn an_own_login_belongs_to_the_part_that_made_it() {
        let login = own_login();

        assert!(login.is_of("example", &ProviderId::new("example-cloud")));
        assert!(!login.is_of("other", &ProviderId::new("example-cloud")));
        assert!(!login.is_of("example", &ProviderId::new("other-cloud")));
    }

    /// Decision 6: the part that made the own login gives the agent's own
    /// login command pointed at its folder.
    #[test]
    fn the_part_gives_the_login_command_for_the_folder() {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_log_in()
            .withf(|folder| folder == own_folder())
            .times(1)
            .returning(|folder| {
                let home = folder.display().to_string();
                Ok(Some(
                    AgentCommand::new("coder")
                        .with_arg("login")
                        .with_env("CODER_HOME", home),
                ))
            });

        let command = log_in_command(
            &[example_cloud(calls)],
            &own_login(),
            Path::new(PLUGINS_FOLDER),
        );

        assert_eq!(
            command,
            Ok(AgentCommand::new("coder").with_arg("login").with_env(
                "CODER_HOME",
                "/home/user/.local/share/otty/plugins/example/accounts/6f0c1e2a"
            ))
        );
    }

    /// A part that is gone, or that gives no command, can not log in.
    #[test]
    fn with_no_part_or_no_command_there_is_no_login() {
        let mut silent = MockProviderPartCalls::new();
        silent.expect_log_in().returning(|_| Ok(None));

        let no_part =
            log_in_command(&[], &own_login(), Path::new(PLUGINS_FOLDER));
        let no_command = log_in_command(
            &[example_cloud(silent)],
            &own_login(),
            Path::new(PLUGINS_FOLDER),
        );

        let refused = Err(SourceError::NotAvailable(Unavailable::NoLogIn));
        assert_eq!(no_part, refused);
        assert_eq!(no_command, refused);
    }

    /// Decision 6: after the login tab closes, the part that made the own
    /// login is asked about its folder alone.
    #[test]
    fn the_accounts_of_an_own_login_come_from_its_folder_alone() {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_accounts()
            .withf(|folders| {
                matches!(folders, [LoginFolder::OwnLogin((id, folder))]
                    if id == "6f0c1e2a"
                        && *folder == own_folder().display().to_string())
            })
            .times(1)
            .returning(|_| Ok((vec![own_account()], Vec::new())));

        let found = own_login_accounts(
            &[example_cloud(calls)],
            &own_login(),
            Path::new(PLUGINS_FOLDER),
        );

        assert_eq!(found, Ok(vec![own_account()]));
    }

    /// An account the part finds there that names another folder only is
    /// not the own login's account.
    #[test]
    fn only_an_account_found_in_the_own_folder_counts() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_accounts().returning(|_| {
            let elsewhere = Account::new(
                "example",
                ProviderId::new("example-cloud"),
                "key:other",
                "other@example.com",
                AccountKind::Key,
                vec![FoundIn::profile(AgentId::new("coder"), "/home/u/.c")],
            );
            Ok((vec![elsewhere], Vec::new()))
        });

        let found = own_login_accounts(
            &[example_cloud(calls)],
            &own_login(),
            Path::new(PLUGINS_FOLDER),
        );

        assert_eq!(found, Ok(Vec::new()));
    }
}
