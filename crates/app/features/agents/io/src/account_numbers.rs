//! The read of the numbers of one account with its own key (spec
//! 2026-11-19, Decision 5): the money left, the limit of a key, or the
//! limits of a subscription login no profile reads. It reads the key again
//! from where it was found, asks the provider's server with it and drops
//! it, so it runs on a thread of its own.

use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{Account, Agent, GivenKey, PluginProvider, Profile};
use otty_app_agents::{AgentList, AgentsIntent, LoadedAccountNumbers};

use crate::background::{on_own_thread, read_stopped};
use crate::plugin_parts::ProviderList;
use crate::shell_variable::shell_variable;

/// Read the numbers of `account` in the background, with the key of its
/// login read again from the files of the first profile of `found_in`
/// through its agent of `agents`, and the keys the user gave for it
/// (`given`), read from their files or variables, a variable as the
/// user's login shell at `shell` sees it. The provider part of
/// `providers` that reads the account reads it with a handle to each key,
/// any way it can, told of every profile of `found_in`; its programs
/// start only when `starts_programs`. A key is used for this read only and
/// never logged. The answer comes back as
/// [`AgentsIntent::AccountNumbersLoaded`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_agents::{
///     Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
///     ProviderRef,
/// };
/// use otty_app_agents::AgentList;
///
/// let account = Account::new(
///     ProviderRef::of_id("example-cloud"),
///     AccountIdentity::AccountId(String::from("made-up-account")),
///     AccountLogin::Unnamed,
///     AccountKind::Limits,
///     FoundIn::ProfileFolder(PathBuf::from("/home/user/.coder")),
/// );
/// let agents: AgentList = Arc::from(Vec::new());
/// let providers: otty_app_agents_io::ProviderList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_account_numbers(
///     &agents,
///     &providers,
///     account,
///     Vec::new(),
///     Vec::new(),
///     true,
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_account_numbers(
    agents: &AgentList,
    providers: &ProviderList,
    account: Account,
    found_in: Vec<Profile>,
    given: Vec<GivenKey>,
    starts_programs: bool,
    shell: &str,
) -> Task<AgentsIntent> {
    let task = format!(
        "reading the numbers of a {} account",
        account.provider().as_str()
    );
    let agents = Arc::clone(agents);
    let providers = Arc::clone(providers);
    let fallback = account.clone();
    let shell = shell.to_string();

    on_own_thread(
        task,
        move || {
            read_account_numbers(
                &agents,
                &providers,
                account,
                &found_in,
                &given,
                starts_programs,
                |name| shell_variable(&shell, name),
            )
        },
        move || LoadedAccountNumbers {
            account: fallback,
            numbers: Err(read_stopped()),
            read_at: SystemTime::now(),
        },
        AgentsIntent::AccountNumbersLoaded,
    )
}

/// The numbers of `account`, read with the key of its login in the first
/// profile of `found_in` and the keys the user gave for it (`given`),
/// starting programs only when `starts_programs`; `env_var` gives the
/// value of an environment variable.
fn read_account_numbers(
    agents: &[Box<dyn Agent>],
    providers: &[PluginProvider],
    account: Account,
    found_in: &[Profile],
    given: &[GivenKey],
    starts_programs: bool,
    env_var: impl Fn(&str) -> Option<String>,
) -> LoadedAccountNumbers {
    let numbers = otty_agents::read_account_numbers(
        agents,
        providers,
        &account,
        found_in,
        given,
        starts_programs,
        env_var,
    );

    LoadedAccountNumbers {
        account,
        numbers,
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::Path;

    use otty_agents::{
        AccountIdentity, AccountKind, AccountLogin, AgentId, FoundIn,
        ProviderRef, SourceError, Unavailable,
    };
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{providers_with_plugins, run};

    /// An Anthropic organization found in the Claude Code folder under
    /// `home`, read by the Anthropic plugin, and that folder as a profile.
    fn organization_in(home: &Path) -> (Account, Profile) {
        let folder = home.join(".claude");
        let account = Account::new(
            ProviderRef::of_id("anthropic"),
            AccountIdentity::OrganizationId(String::from("made-up-org")),
            AccountLogin::Unnamed,
            AccountKind::Money,
            FoundIn::ProfileFolder(folder.clone()),
        )
        .with_read_by(Some(String::from("anthropic")));
        let profile = Profile::default_for(
            AgentId::new("claude-code"),
            "Claude Code",
            folder,
        );

        (account, profile)
    }

    /// A login no agent part finds any more is not available: nothing is
    /// sent and no test reaches the network.
    #[test]
    fn a_login_that_is_not_there_is_not_available_and_nothing_is_sent() {
        let folder = TempDir::new("agents-numbers-no-login");
        let (account, profile) = organization_in(folder.path());
        let before = SystemTime::now();

        let loaded = read_account_numbers(
            &[],
            &providers_with_plugins(folder.path()),
            account.clone(),
            &[profile],
            &[],
            true,
            |_| None,
        );

        assert_eq!(loaded.account, account);
        assert!(loaded.read_at >= before, "the read tells when it ended");
        assert_eq!(
            loaded.numbers,
            Err(SourceError::NotAvailable(Unavailable::KeyGone))
        );
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let folder = TempDir::new("agents-numbers-task");
        let (account, profile) = organization_in(folder.path());
        let agents: AgentList = Arc::from(Vec::new());

        let providers = providers_with_plugins(folder.path());
        let answers = run(load_account_numbers(
            &agents,
            &providers,
            account.clone(),
            vec![profile],
            Vec::new(),
            true,
            "/no/such/shell",
        ));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::AccountNumbersLoaded(loaded)]
                if loaded.account == account && loaded.numbers.is_err()
        ));
    }
}
