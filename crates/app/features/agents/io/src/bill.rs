//! The read of the real bill of one organization, through one profile that
//! pays through it (spec Decision 9). It reads the keys the user gave for
//! the organization from their files or their environment variables,
//! hands the provider part of the organization's provider a handle to
//! each and to the login of the profile, and drops them, so it runs on a
//! thread of its own.

use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{Account, Agent, GivenKey, PluginProvider, Profile};
use otty_app_agents::{AgentList, AgentsIntent, LoadedBill};

use crate::background::{on_own_thread, read_stopped};
use crate::plugin_parts::ProviderList;
use crate::shell_variable::shell_variable;

/// Read the bill of `organization`, which `profile` pays through, in the
/// background, with the provider part of `providers` that serves it, the
/// login the agent part of `agents` found it by in the profile and the
/// keys the user gave for it (`given`). Each given key is read from its
/// file, or from its variable as the user's login shell at `shell` sees
/// it, so a variable set in `~/.zshrc` is found when OTTY was started from
/// the desktop. The keys are used for this read only and never logged.
/// The answer comes back as [`AgentsIntent::BillLoaded`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_agents::{
///     Account, AccountIdentity, AccountKind, AccountLogin, AgentId, FoundIn,
///     Profile, ProviderRef,
/// };
/// use otty_app_agents::AgentList;
///
/// let profile = Profile::default_for(
///     AgentId::new("claude-code"),
///     "Coder",
///     PathBuf::from("/home/user/.coder"),
/// );
/// let organization = Account::new(
///     ProviderRef::of_id("anthropic"),
///     AccountIdentity::OrganizationId("made-up-org".into()),
///     AccountLogin::Unnamed,
///     AccountKind::Money,
///     FoundIn::ProfileFolder(PathBuf::from("/home/user/.coder")),
/// );
/// let agents: AgentList = Arc::from(Vec::new());
/// let providers: otty_app_agents_io::ProviderList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_bill(
///     &agents,
///     &providers,
///     profile,
///     organization,
///     Vec::new(),
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_bill(
    agents: &AgentList,
    providers: &ProviderList,
    profile: Profile,
    organization: Account,
    given: Vec<GivenKey>,
    shell: &str,
) -> Task<AgentsIntent> {
    let task = format!("reading the bill through {}", profile.name());
    let agents = Arc::clone(agents);
    let providers = Arc::clone(providers);
    let fallback = (profile.clone(), given.clone());
    let shell = shell.to_string();

    on_own_thread(
        task,
        move || {
            read_bill(
                &agents,
                &providers,
                profile,
                &organization,
                given,
                SystemTime::now(),
                |name| shell_variable(&shell, name),
            )
        },
        move || LoadedBill {
            profile: fallback.0,
            given: fallback.1,
            bill: Err(read_stopped()),
            read_at: SystemTime::now(),
        },
        AgentsIntent::BillLoaded,
    )
}

/// The bill up to `now` of `organization`, which `profile` pays through,
/// read with the keys of `given`; `env_var` gives the value of an
/// environment variable.
fn read_bill(
    agents: &[Box<dyn Agent>],
    providers: &[PluginProvider],
    profile: Profile,
    organization: &Account,
    given: Vec<GivenKey>,
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> LoadedBill {
    let bill = otty_agents::read_bill(
        agents,
        providers,
        organization,
        &profile,
        &given,
        now,
        env_var,
    );

    LoadedBill {
        profile,
        given,
        bill,
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::Path;

    use otty_agents::{
        AccountIdentity, AccountKind, AccountLogin, AgentId, FoundIn, KeyRef,
        ProviderRef, SourceError, Unavailable,
    };
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{fake_shell, providers_with_plugins, run};

    /// No environment variable is set: no test reads the real environment.
    fn no_variables(_: &str) -> Option<String> {
        None
    }

    /// A Coder API key profile in a made-up folder.
    fn claude_api(home: &Path) -> Profile {
        Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · API",
            home.join(".claude-api"),
        )
    }

    /// A made-up organization at `provider`, found in `home`.
    fn organization(home: &Path, provider: &str) -> Account {
        Account::new(
            ProviderRef::of_id(provider),
            AccountIdentity::OrganizationId(String::from("made-up-org")),
            AccountLogin::Unnamed,
            AccountKind::Money,
            FoundIn::ProfileFolder(home.join(".claude-api")),
        )
    }

    fn given(key: KeyRef) -> Vec<GivenKey> {
        vec![GivenKey::new("admin", key)]
    }

    /// Issue 15 of spec 2026-11-19-agent-plugins: with no provider part
    /// that serves the organization, nothing is read or sent.
    #[test]
    fn a_provider_with_no_part_has_no_bill() {
        let home = TempDir::new("agents-bill-no-part");
        let key = KeyRef::EnvVar(String::from("OTTY_TEST_ORG_KEY"));
        let profile = claude_api(home.path());
        let before = SystemTime::now();

        let loaded = read_bill(
            &[],
            &providers_with_plugins(home.path()),
            profile.clone(),
            &organization(home.path(), "example-cloud"),
            given(key.clone()),
            SystemTime::now(),
            |_| Some(String::from("sk-made-up-org-key")),
        );

        assert_eq!(loaded.profile, profile);
        assert_eq!(loaded.given, given(key));
        assert!(loaded.read_at >= before, "the read tells when it ended");
        assert_eq!(
            loaded.bill,
            Err(SourceError::NotAvailable(Unavailable::NoBill))
        );
    }

    /// The login of the profile is gone, so nothing is read: no agent
    /// finds it, and no given key is looked at.
    #[test]
    fn a_profile_whose_login_is_gone_reads_no_bill() {
        let home = TempDir::new("agents-bill-no-login");
        let key = KeyRef::EnvVar(String::from("OTTY_TEST_ORG_KEY"));

        let loaded = read_bill(
            &[],
            &providers_with_plugins(home.path()),
            claude_api(home.path()),
            &organization(home.path(), "anthropic"),
            given(key),
            SystemTime::now(),
            no_variables,
        );

        assert_eq!(
            loaded.bill,
            Err(SourceError::NotAvailable(Unavailable::KeyGone))
        );
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-bill-task");
        let shell = fake_shell(&home);
        let profile = claude_api(home.path());
        let agents: AgentList = Arc::from(Vec::new());
        let providers = providers_with_plugins(home.path());

        let answers = run(load_bill(
            &agents,
            &providers,
            profile.clone(),
            organization(home.path(), "anthropic"),
            given(KeyRef::File(home.path().join("no-such-org-key"))),
            &shell,
        ));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::BillLoaded(loaded)]
                if loaded.profile == profile && loaded.bill.is_err()
        ));
    }
}
