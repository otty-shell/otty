//! The read of the real bill of one organization (spec Decision 9). The
//! provider part that found the organization reads it, in the folders it
//! found it in, with the keys the user gave for it read from their files
//! or their environment variables and handed over as handles, then
//! dropped, so it runs on a thread of its own.

use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{Account, GivenKey, PluginProvider};
use otty_app_agents::{AgentsIntent, LoadedBill};

use crate::background::{on_own_thread, read_stopped};
use crate::plugin_parts::ProviderList;
use crate::shell_variable::shell_variable;

/// Read the bill of `organization` in the background, by the provider
/// part of `providers` that found it, with the keys the user gave for it
/// (`given`). Each given key is read from
/// its file, or from its variable as the user's login shell at `shell`
/// sees it, so a variable set in `~/.zshrc` is found when OTTY was started
/// from the desktop. The keys are used for this read only and never
/// logged. The answer comes back as [`AgentsIntent::BillLoaded`].
///
/// # Example
///
/// ```
/// use std::sync::Arc;
///
/// use otty_agents::{Account, AccountKind, ProviderId};
///
/// let organization = Account::new(
///     "anthropic",
///     ProviderId::new("anthropic"),
///     "org:made-up-org",
///     "Made-up org",
///     AccountKind::Organization,
///     Vec::new(),
/// );
/// let providers: otty_app_agents_io::ProviderList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_bill(
///     &providers,
///     organization,
///     Vec::new(),
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_bill(
    providers: &ProviderList,
    organization: Account,
    given: Vec<GivenKey>,
    shell: &str,
) -> Task<AgentsIntent> {
    let task = format!(
        "reading the bill of a {} organization",
        organization.provider().as_str()
    );
    let providers = Arc::clone(providers);
    let fallback = (organization.clone(), given.clone());
    let shell = shell.to_string();

    on_own_thread(
        task,
        move || {
            read_bill(
                &providers,
                organization,
                given,
                SystemTime::now(),
                |name| shell_variable(&shell, name),
            )
        },
        move || LoadedBill {
            organization: fallback.0,
            given: fallback.1,
            bill: Err(read_stopped()),
            read_at: SystemTime::now(),
        },
        AgentsIntent::BillLoaded,
    )
}

/// The bill up to `now` of `organization`, with the keys of `given`;
/// `env_var` gives the value of an environment variable.
fn read_bill(
    providers: &[PluginProvider],
    organization: Account,
    given: Vec<GivenKey>,
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> LoadedBill {
    let bill =
        otty_agents::read_bill(providers, &organization, &given, now, env_var);

    LoadedBill {
        organization,
        given,
        bill,
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::Path;

    use otty_agents::{
        AccountKind, AgentId, FoundIn, KeyRef, ProviderId, SourceError,
        Unavailable,
    };
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{fake_shell, providers_with_plugins, run};

    /// A made-up organization the provider part `provider` of the plugin
    /// of the same id found in `home`.
    fn organization(home: &Path, provider: &str) -> Account {
        Account::new(
            provider,
            ProviderId::new(provider),
            "org:made-up-org",
            "Made-up org",
            AccountKind::Organization,
            vec![FoundIn::profile(
                AgentId::new("claude-code"),
                home.join(".claude-api"),
            )],
        )
    }

    fn given(key: KeyRef) -> Vec<GivenKey> {
        vec![GivenKey::new("admin", key)]
    }

    /// Issue 15 of spec 2026-11-19-agent-plugins: with no provider part
    /// that found the organization, nothing is read or sent.
    #[test]
    fn a_provider_with_no_part_has_no_bill() {
        let home = TempDir::new("agents-bill-no-part");
        let key = KeyRef::EnvVar(String::from("OTTY_TEST_ORG_KEY"));
        let organization = organization(home.path(), "example-cloud");
        let before = SystemTime::now();

        let loaded = read_bill(
            &providers_with_plugins(home.path()),
            organization.clone(),
            given(key.clone()),
            SystemTime::now(),
            |_| Some(String::from("sk-made-up-org-key")),
        );

        assert_eq!(loaded.organization, organization);
        assert_eq!(loaded.given, given(key));
        assert!(loaded.read_at >= before, "the read tells when it ended");
        assert_eq!(
            loaded.bill,
            Err(SourceError::NotAvailable(Unavailable::NoBill))
        );
    }

    /// A given key that is not where the user said fails the read before
    /// the part is asked.
    #[test]
    fn a_missing_given_key_reads_no_bill() {
        let home = TempDir::new("agents-bill-no-key");
        let key = KeyRef::EnvVar(String::from("OTTY_TEST_ORG_KEY"));

        let loaded = read_bill(
            &providers_with_plugins(home.path()),
            organization(home.path(), "anthropic"),
            given(key.clone()),
            SystemTime::now(),
            |_| None,
        );

        assert_eq!(
            loaded.bill,
            Err(SourceError::NotAvailable(Unavailable::KeyMissing(key)))
        );
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-bill-task");
        let shell = fake_shell(&home);
        let organization = organization(home.path(), "anthropic");
        let providers = providers_with_plugins(home.path());

        let answers = run(load_bill(
            &providers,
            organization.clone(),
            given(KeyRef::File(home.path().join("no-such-org-key"))),
            &shell,
        ));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::BillLoaded(loaded)]
                if loaded.organization == organization && loaded.bill.is_err()
        ));
    }
}
