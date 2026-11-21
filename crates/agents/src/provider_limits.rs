//! The limits of a profile (issues 15 and 19 of spec
//! 2026-11-19-agent-plugins). The parts may come from different plugins,
//! such as OpenCode with DeepSeek, so OTTY keeps one order rule: the
//! provider part that serves the profile's subscription login is asked
//! first, with one request, then the agent part, which may start the
//! agent's program. The first answer wins. Where each part reads its
//! numbers from is the part's business.

use crate::account::{Account, AccountKind};
use crate::account_numbers::AccountNumbers;
use crate::agent::Agent;
use crate::error::{SourceError, Unavailable};
use crate::limits::{LimitSource, Limits};
use crate::plugin_provider::PluginProvider;
use crate::profile::Profile;
use crate::refused_login::to_holding_agent;
use crate::source_id::SourceId;
use crate::walk::{Answer, SourceFailure, walk};

/// The limits of `profile` from the first part that answers: the
/// provider part of `providers` that serves the subscription login
/// `agent` finds in the profile ([`SourceId::ProviderPart`]), then
/// `agent_limits`, the agent part's own read ([`SourceId::AgentPart`]).
/// The agent part is asked only when the provider part failed or there
/// is none.
///
/// When no part answered, the error holds the failure of each part in
/// order; it is empty when neither reads limits for the profile.
pub fn read_profile_limits(
    agent: &dyn Agent,
    providers: &[PluginProvider],
    agent_limits: Option<&dyn LimitSource>,
    profile: &Profile,
) -> Result<Answer<Limits>, Vec<SourceFailure>> {
    let subscription = subscription_of(agent, providers, profile);

    let from_provider = subscription.iter().map(|(provider, account)| {
        let limits = provider_limits(agent, provider, account, profile);
        (SourceId::ProviderPart, limits)
    });
    let from_agent = agent_limits.into_iter().filter_map(|source| {
        let limits = source.limits(profile).transpose()?;
        Some((SourceId::AgentPart, limits))
    });

    walk(from_provider.chain(from_agent))
}

/// The subscription login `agent` finds in `profile` with the provider
/// part of `providers` that serves it; `None` when the profile has none,
/// or no part serves its provider.
fn subscription_of<'a>(
    agent: &dyn Agent,
    providers: &'a [PluginProvider],
    profile: &Profile,
) -> Option<(&'a PluginProvider, Account)> {
    agent
        .accounts(profile)
        .into_iter()
        .filter(|account| account.kind() != AccountKind::Money)
        .find_map(|account| {
            let provider =
                providers.iter().find(|part| part.serves(&account))?;
            Some((provider, account))
        })
}

/// The limits of `account` read by `provider` with the login `agent`
/// finds for it in `profile`. A login the provider refused is the need of
/// `agent`, which holds it.
fn provider_limits(
    agent: &dyn Agent,
    provider: &PluginProvider,
    account: &Account,
    profile: &Profile,
) -> Result<Limits, SourceError> {
    let login = agent
        .found_login(profile, account)
        .ok_or(SourceError::NotAvailable(Unavailable::KeyGone))?;

    let numbers = provider
        .read(account, &login, &[])
        .map_err(|error| to_holding_agent(error, agent, profile, &login))?;

    match numbers {
        AccountNumbers::Limits(answer) => Ok(answer.into_value()),
        _ => Err(SourceError::NotAvailable(Unavailable::NoAccountSource)),
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::agents::agent::{
        Identity, JsonField, KeyPlace, Login, LoginKind,
        ProviderRef as PartProvider,
    };
    use otty_plugin_host::agents::provider::{
        AccountFacts, AccountNumbers as PartNumbers,
    };

    use super::*;
    use crate::account::{AccountIdentity, AccountLogin, FoundIn};
    use crate::command::AgentCommand;
    use crate::fixtures::login_need;
    use crate::limits::{Billing, MockLimitSource, UsageWindows};
    use crate::part_need::PartNeedAction;
    use crate::plugin_provider::MockProviderPartCalls;
    use crate::provider::ProviderRef;
    use crate::refused_login::RefusedLogin;
    use crate::testing::{MockAgent, made_up_part, mock_entry, profile};

    fn claude_login() -> Account {
        Account::new(
            ProviderRef::of_id("anthropic"),
            AccountIdentity::AccountId("made-up-account".into()),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder("/home/user/.config-Example".into()),
        )
    }

    fn login() -> Login {
        Login {
            kind: LoginKind::Subscription,
            for_whom: PartProvider::Id("anthropic".to_string()),
            place: KeyPlace::JsonField(JsonField {
                file: "/home/user/.config-Example/.credentials.json"
                    .to_string(),
                pointer: "/claudeAiOauth/accessToken".to_string(),
            }),
            identity: Identity {
                account_id: Some("made-up-account".to_string()),
                organization_id: None,
                email: None,
            },
            plan: Some("Max 20x".to_string()),
        }
    }

    fn agent_with(accounts: Vec<Account>) -> MockAgent {
        let mut agent = MockAgent::new();
        agent
            .expect_entry()
            .return_const(mock_entry("claude-code", "claude-code"));
        agent.expect_accounts().returning(move |_| accounts.clone());
        agent.expect_found_login().returning(|_, _| Some(login()));
        agent
    }

    fn anthropic(
        answer: Result<(), SourceError>,
        times: usize,
    ) -> PluginProvider {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().times(times).returning(
            move |_, found, _| {
                answer.clone()?;
                let facts = AccountFacts {
                    identity: found.identity.clone(),
                    plan: found.plan.clone(),
                };
                let numbers = PartNumbers {
                    windows: Vec::new(),
                    credits: None,
                    extra_usage: None,
                    left: None,
                    key_limit: None,
                    spent_this_month: None,
                    plan: None,
                };
                Ok((facts, numbers))
            },
        );

        PluginProvider::made_up("anthropic", &["anthropic.com"], calls)
    }

    /// The need of the provider part whose login expired.
    fn login_expired() -> SourceError {
        let need = crate::part_need::PartNeed::new(
            crate::testing::made_up_part("anthropic"),
            "login-expired",
            "The login expired",
            "Open the agent to log in again.",
            crate::part_need::PartNeedAction::OpenPage(
                "https://example.com/login".to_string(),
            ),
        );

        SourceError::NeedsUser(Box::new(need))
    }

    /// The agent part's read of the limits, asked `times` times, which
    /// answers with the plan "From the agent".
    fn usage_screen(times: usize) -> MockLimitSource {
        let mut source = MockLimitSource::new();
        source.expect_limits().times(times).returning(|_| {
            let limits = Limits::new(
                Billing::UsageWindows(UsageWindows::default()),
                Some("From the agent".to_string()),
            );
            Ok(Some(limits))
        });
        source
    }

    /// Issue 19: the provider part's answer wins over the agent part's,
    /// which is not asked.
    #[test]
    fn the_provider_part_answers_first_and_the_agent_part_is_not_asked() {
        let agent = agent_with(vec![claude_login()]);
        let providers = [anthropic(Ok(()), 1)];
        let screen = usage_screen(0);

        let limits = read_profile_limits(
            &agent,
            &providers,
            Some(&screen),
            &profile("Example"),
        );

        let answer = limits.expect("the limits");
        assert_eq!(answer.source(), SourceId::ProviderPart);
        assert_eq!(answer.value().plan(), Some("Max 20x"));
        assert!(answer.earlier_failures().is_empty());
    }

    #[test]
    fn when_the_provider_part_fails_the_agent_part_is_asked() {
        let agent = agent_with(vec![claude_login()]);
        let providers = [anthropic(Err(login_expired()), 1)];
        let screen = usage_screen(1);

        let limits = read_profile_limits(
            &agent,
            &providers,
            Some(&screen),
            &profile("Example"),
        );

        let answer = limits.expect("the limits");
        assert_eq!(answer.source(), SourceId::AgentPart);
        assert_eq!(answer.value().plan(), Some("From the agent"));
        assert_eq!(answer.earlier_failures().len(), 1);
        assert_eq!(
            answer.earlier_failures()[0].source(),
            SourceId::ProviderPart
        );
        assert_eq!(answer.earlier_failures()[0].error(), &login_expired());
    }

    #[test]
    fn a_profile_with_no_login_a_part_serves_asks_only_its_agent_part() {
        let mut key = claude_login();
        key = Account::new(
            key.provider().clone(),
            key.identity().clone(),
            AccountLogin::Key(None),
            AccountKind::Money,
            FoundIn::ProfileFolder("/home/user/.config-Example".into()),
        );
        let agent = agent_with(vec![key]);
        let providers = [anthropic(Ok(()), 0)];
        let screen = usage_screen(1);

        let limits = read_profile_limits(
            &agent,
            &providers,
            Some(&screen),
            &profile("Example"),
        );

        let answer = limits.expect("the limits");
        assert_eq!(answer.source(), SourceId::AgentPart);
    }

    /// An agent part that reads no limits is no failure: with no provider
    /// part either, nothing reads the limits of the profile.
    #[test]
    fn an_agent_part_that_reads_no_limits_leaves_no_failure() {
        let agent = agent_with(Vec::new());
        let mut no_limits = MockLimitSource::new();
        no_limits.expect_limits().times(1).returning(|_| Ok(None));

        let limits = read_profile_limits(
            &agent,
            &[],
            Some(&no_limits),
            &profile("Example"),
        );

        assert_eq!(limits, Err(Vec::new()));
    }

    #[test]
    fn a_failed_provider_part_and_no_agent_read_give_its_failure() {
        let agent = agent_with(vec![claude_login()]);
        let providers = [anthropic(Err(login_expired()), 1)];

        let limits =
            read_profile_limits(&agent, &providers, None, &profile("Example"));

        let failures = limits.expect_err("no part answers");
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].source(), SourceId::ProviderPart);
        assert_eq!(failures[0].error(), &login_expired());
    }

    /// A ChatGPT login only OpenCode keeps, in its own profile.
    fn chatgpt_in_opencode() -> Account {
        Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::AccountId("made-up-account".into()),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder("/home/user/.config-OpenCode".into()),
        )
    }

    /// The `openai` provider part, which refuses every login it reads
    /// with.
    fn refusing_openai() -> PluginProvider {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().times(1).returning(|_, _, _| {
            let refused = RefusedLogin::new(made_up_part("openai"));
            Err(SourceError::LoginRefused(Box::new(refused)))
        });

        PluginProvider::made_up("openai", &["openai.com"], calls)
    }

    /// OpenCode, which keeps the ChatGPT login, and logs in again with
    /// `opencode auth login` when `logs_in`.
    fn opencode(logs_in: bool) -> MockAgent {
        let mut agent = MockAgent::new();
        agent
            .expect_entry()
            .return_const(mock_entry("opencode", "opencode"));
        agent
            .expect_accounts()
            .returning(|_| vec![chatgpt_in_opencode()]);
        agent.expect_found_login().returning(|_, _| Some(login()));
        agent
            .expect_login_refused()
            .times(1)
            .returning(move |_, _| {
                let run = AgentCommand::new("opencode")
                    .with_arg("auth")
                    .with_arg("login");
                logs_in.then(|| login_need("opencode", "opencode", run))
            });
        agent
    }

    /// Issue 20: the `openai` provider part refuses a ChatGPT login only
    /// OpenCode keeps; the need comes from OpenCode's part, about its
    /// profile, and runs OpenCode's login, not Codex.
    #[test]
    fn a_login_refused_for_opencode_shows_the_need_of_opencode() {
        let agent = opencode(true);
        let providers = [refusing_openai()];
        let holder = profile("OpenCode");

        let limits = read_profile_limits(&agent, &providers, None, &holder);

        let failures = limits.expect_err("no part answers");
        let SourceError::NeedsUser(need) = failures[0].error() else {
            panic!("the need of OpenCode: {failures:?}");
        };
        assert_eq!(need.part().part(), "opencode");
        assert_eq!(need.profile(), Some(holder.config_dir()));
        assert_eq!(
            need.action(),
            &PartNeedAction::Run(
                AgentCommand::new("opencode")
                    .with_arg("auth")
                    .with_arg("login")
            )
        );
    }

    /// Issue 20: when the agent part that keeps the login says nothing,
    /// OTTY says itself that the provider refused the login of the
    /// profile.
    #[test]
    fn a_refused_login_with_no_need_of_its_agent_part_names_the_profile() {
        let agent = opencode(false);
        let providers = [refusing_openai()];
        let holder = profile("OpenCode");

        let limits = read_profile_limits(&agent, &providers, None, &holder);

        let failures = limits.expect_err("no part answers");
        let SourceError::LoginRefused(refused) = failures[0].error() else {
            panic!("OTTY's own refusal: {failures:?}");
        };
        assert_eq!(refused.profile(), Some(&holder));
        assert_eq!(refused.provider().part(), "openai");
    }
}
