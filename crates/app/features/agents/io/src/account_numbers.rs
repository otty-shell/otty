//! The read of the numbers of one account with its own key (spec
//! 2026-11-19, Decision 5): the money left, the limit of a key, the bill of
//! an organization the user added, or the limits of a ChatGPT login no
//! profile reads. It reads the key again from where it was found, asks
//! the provider's server with it and drops it, so it runs on a thread of
//! its own.

use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{Account, Agent, KeyPlace, PluginProvider};
use otty_app_agents::{AgentList, AgentsIntent, LoadedAccountNumbers};

use crate::background::{on_own_thread, read_stopped};
use crate::plugin_parts::ProviderList;
use crate::shell_variable::shell_variable;

/// Read the numbers of `account` in the background, with its key read
/// again from `place`: the files of a profile through its agent of
/// `agents`, the system keyring, or the file or variable the user gave, a
/// variable as the user's login shell at `shell` sees it. The provider
/// part of `providers` that serves the account reads it with a handle to
/// the key, else the reader built into OTTY. The key is used for this
/// read only and never logged. The answer comes back as
/// [`AgentsIntent::AccountNumbersLoaded`].
///
/// # Example
///
/// ```
/// use std::sync::Arc;
///
/// use otty_agents::{Account, AdminKeyRef, KeyPlace, ProviderRef};
/// use otty_app_agents::AgentList;
///
/// let account = Account::added(
///     ProviderRef::of_id("deepseek"),
///     AdminKeyRef::EnvVar(String::from("DEEPSEEK_API_KEY")),
/// );
/// let agents: AgentList = Arc::from(otty_agents::known_agents());
/// let providers: otty_app_agents_io::ProviderList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_account_numbers(
///     &agents,
///     &providers,
///     account,
///     KeyPlace::Added,
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_account_numbers(
    agents: &AgentList,
    providers: &ProviderList,
    account: Account,
    place: KeyPlace,
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
                &place,
                SystemTime::now(),
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

/// The numbers of `account` up to `now`, read with its key from `place`;
/// `env_var` gives the value of an environment variable.
fn read_account_numbers(
    agents: &[Box<dyn Agent>],
    providers: &[PluginProvider],
    account: Account,
    place: &KeyPlace,
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> LoadedAccountNumbers {
    let numbers = otty_agents::read_account_numbers(
        agents, providers, &account, place, now, env_var,
    );

    LoadedAccountNumbers {
        account,
        numbers,
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use otty_agents::{
        AdminKeyRef, ProviderRef, SourceError, Unavailable, known_agents,
    };
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::run;

    /// A DeepSeek account added with a key file that is not there: nothing
    /// is sent and no test reaches the network.
    fn missing_key(folder: &TempDir) -> (Account, AdminKeyRef) {
        let key = AdminKeyRef::File(folder.path().join("no-such-key"));
        let account =
            Account::added(ProviderRef::of_id("deepseek"), key.clone());

        (account, key)
    }

    #[test]
    fn a_key_that_is_not_there_is_not_available_and_nothing_is_sent() {
        let folder = TempDir::new("agents-numbers-no-key");
        let (account, key) = missing_key(&folder);
        let before = SystemTime::now();

        let loaded = read_account_numbers(
            &known_agents(),
            &[],
            account.clone(),
            &KeyPlace::Added,
            SystemTime::now(),
            |_| None,
        );

        assert_eq!(loaded.account, account);
        assert!(loaded.read_at >= before, "the read tells when it ended");
        assert_eq!(
            loaded.numbers,
            Err(SourceError::NotAvailable(Unavailable::AdminKeyMissing(key)))
        );
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let folder = TempDir::new("agents-numbers-task");
        let (account, _) = missing_key(&folder);
        let agents: AgentList = Arc::from(known_agents());

        let providers: ProviderList = Arc::from(Vec::new());
        let answers = run(load_account_numbers(
            &agents,
            &providers,
            account.clone(),
            KeyPlace::Added,
            "/no/such/shell",
        ));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::AccountNumbersLoaded(loaded)]
                if loaded.account == account && loaded.numbers.is_err()
        ));
    }
}
