//! The read of the real bill of one organization, through one profile that
//! pays through it (spec Decision 9). It reads the admin key from its file
//! or its environment variable, hands the provider part of the
//! organization's provider a handle to it and drops it, so it runs on a
//! thread of its own.

use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{AdminKeyRef, PluginProvider, Profile, ProviderRef};
use otty_app_agents::{AgentsIntent, LoadedBill};

use crate::background::{on_own_thread, read_stopped};
use crate::plugin_parts::ProviderList;
use crate::shell_variable::shell_variable;

/// Read the bill of the organization at `provider` that `profile` pays
/// through in the background, with the provider part of `providers` that
/// serves it and the organization's admin key kept at `admin_key`. The
/// key is read from its file, or from its variable as the user's login
/// shell at `shell` sees it, so a variable set in `~/.zshrc` is found when
/// OTTY was started from the desktop. The key is used for this read only
/// and never logged. The answer comes back as
/// [`AgentsIntent::BillLoaded`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_agents::{AdminKeyRef, AgentId, Profile, ProviderRef};
///
/// let profile = Profile::default_for(
///     AgentId::new("claude-code"),
///     "Coder",
///     PathBuf::from("/home/user/.coder"),
/// );
/// let admin_key = AdminKeyRef::EnvVar(String::from("ANTHROPIC_ADMIN_KEY"));
/// let providers: otty_app_agents_io::ProviderList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_bill(
///     &providers,
///     profile,
///     ProviderRef::of_id("anthropic"),
///     admin_key,
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_bill(
    providers: &ProviderList,
    profile: Profile,
    provider: ProviderRef,
    admin_key: AdminKeyRef,
    shell: &str,
) -> Task<AgentsIntent> {
    let task = format!("reading the bill through {}", profile.name());
    let providers = Arc::clone(providers);
    let fallback = (profile.clone(), admin_key.clone());
    let shell = shell.to_string();

    on_own_thread(
        task,
        move || {
            read_bill(
                &providers,
                profile,
                &provider,
                admin_key,
                SystemTime::now(),
                |name| shell_variable(&shell, name),
            )
        },
        move || LoadedBill {
            profile: fallback.0,
            admin_key: fallback.1,
            bill: Err(read_stopped()),
            read_at: SystemTime::now(),
        },
        AgentsIntent::BillLoaded,
    )
}

/// The bill up to `now` of the organization at `provider` that `profile`
/// pays through, read with its admin key kept at `admin_key`; `env_var`
/// gives the value of an environment variable.
fn read_bill(
    providers: &[PluginProvider],
    profile: Profile,
    provider: &ProviderRef,
    admin_key: AdminKeyRef,
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> LoadedBill {
    let bill =
        otty_agents::read_bill(providers, provider, &admin_key, now, env_var);

    LoadedBill {
        profile,
        admin_key,
        bill,
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::Path;

    use otty_agents::{AdminKeyRef, AgentId, SourceError, Unavailable};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{fake_shell, providers_with_plugins, run};

    /// No environment variable is set: no test reads the real environment.
    fn no_variables(_: &str) -> Option<String> {
        None
    }

    fn anthropic() -> ProviderRef {
        ProviderRef::of_id("anthropic")
    }

    /// A Coder API key profile in a made-up folder.
    fn claude_api(home: &Path) -> Profile {
        Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · API",
            home.join(".claude-api"),
        )
    }

    #[test]
    fn a_key_file_that_is_missing_is_not_available_and_nothing_is_sent() {
        let home = TempDir::new("agents-bill-no-file");
        let key_file = home.path().join("no-such-admin-key");
        let key = AdminKeyRef::File(key_file);
        let profile = claude_api(home.path());

        let before = SystemTime::now();

        let loaded = read_bill(
            &providers_with_plugins(home.path()),
            profile.clone(),
            &anthropic(),
            key.clone(),
            SystemTime::now(),
            no_variables,
        );

        assert_eq!(loaded.profile, profile);
        assert_eq!(loaded.admin_key, key);
        assert!(loaded.read_at >= before, "the read tells when it ended");
        assert_eq!(
            loaded.bill,
            Err(SourceError::NotAvailable(Unavailable::AdminKeyMissing(key)))
        );
    }

    /// Issue 15 of spec 2026-11-19-agent-plugins: with no provider part
    /// that reads bills for the provider, nothing is read or sent.
    #[test]
    fn a_provider_with_no_part_has_no_bill() {
        let home = TempDir::new("agents-bill-no-part");
        let key = AdminKeyRef::EnvVar(String::from("OTTY_TEST_ADMIN_KEY"));

        let loaded = read_bill(
            &providers_with_plugins(home.path()),
            claude_api(home.path()),
            &ProviderRef::of_id("deepseek"),
            key,
            SystemTime::now(),
            |_| Some(String::from("sk-made-up-admin-key")),
        );

        assert_eq!(
            loaded.bill,
            Err(SourceError::NotAvailable(Unavailable::NoBill))
        );
    }

    #[test]
    fn an_unset_variable_is_not_available() {
        let home = TempDir::new("agents-bill-no-variable");
        let key = AdminKeyRef::EnvVar(String::from("OTTY_TEST_ADMIN_KEY"));
        let profile = claude_api(home.path());

        let loaded = read_bill(
            &providers_with_plugins(home.path()),
            profile,
            &anthropic(),
            key.clone(),
            SystemTime::now(),
            no_variables,
        );

        assert_eq!(
            loaded.bill,
            Err(SourceError::NotAvailable(Unavailable::AdminKeyMissing(key)))
        );
    }

    #[test]
    fn the_variable_is_read_through_the_login_shell() {
        let home = TempDir::new("agents-bill-shell-variable");
        let shell = fake_shell(&home);
        let key =
            AdminKeyRef::EnvVar(String::from("OTTY_TEST_NO_SUCH_ADMIN_KEY"));
        let profile = claude_api(home.path());
        let providers = providers_with_plugins(home.path());

        let answers = run(load_bill(
            &providers,
            profile,
            anthropic(),
            key.clone(),
            &shell,
        ));

        let starts = std::fs::read_to_string(home.path().join("starts"))
            .map_or(0, |text| text.lines().count());
        assert_eq!(starts, 1, "the shell was asked for the variable");
        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::BillLoaded(loaded)]
                if loaded.bill == Err(SourceError::NotAvailable(
                    Unavailable::AdminKeyMissing(key)
                ))
        ));
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-bill-task");
        let key_file = home.path().join("no-such-admin-key");
        let profile = claude_api(home.path());
        let providers = providers_with_plugins(home.path());

        let answers = run(load_bill(
            &providers,
            profile.clone(),
            anthropic(),
            AdminKeyRef::File(key_file),
            "/no/such/shell",
        ));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::BillLoaded(loaded)]
                if loaded.profile == profile && loaded.bill.is_err()
        ));
    }
}
