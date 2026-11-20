//! The limits of a profile, best source first (issue 15 of spec
//! 2026-11-19-agent-plugins): the provider part that serves the profile's
//! subscription login reads them with one request, before the agent's own
//! sources, which may start the agent's program.

use crate::account::{Account, AccountKind};
use crate::account_numbers::AccountNumbers;
use crate::agent::Agent;
use crate::error::{SourceError, Unavailable};
use crate::limits::{LimitSource, Limits};
use crate::plugin_provider::PluginProvider;
use crate::profile::Profile;
use crate::source_id::SourceId;
use crate::walk::{Answer, SourceFailure, walk};

/// The limits of `profile` from the first source that answers: the
/// provider part of `providers` that serves the subscription login
/// `agent` finds in the profile, then `sources`, the agent's own, best
/// first. A source is asked only when the ones before it failed. The ids
/// of the sources come with the answer, in the order they are walked.
///
/// When no source answered, the error holds the failure of each source in
/// order; it is empty when there were no sources.
pub fn read_profile_limits(
    agent: &dyn Agent,
    providers: &[PluginProvider],
    sources: &[Box<dyn LimitSource>],
    profile: &Profile,
) -> (Result<Answer<Limits>, Vec<SourceFailure>>, Vec<SourceId>) {
    let subscription = subscription_of(agent, providers, profile);

    let mut ids: Vec<SourceId> = subscription
        .iter()
        .map(|(provider, _)| provider.source())
        .collect();
    ids.extend(sources.iter().map(|source| source.id()));

    let from_provider = subscription.iter().map(|(provider, account)| {
        let limits = provider_limits(agent, provider, account, profile);
        (provider.source(), limits)
    });
    let from_agent = sources
        .iter()
        .map(|source| (source.id(), source.limits(profile)));

    (walk(from_provider.chain(from_agent)), ids)
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
/// finds for it in `profile`.
fn provider_limits(
    agent: &dyn Agent,
    provider: &PluginProvider,
    account: &Account,
    profile: &Profile,
) -> Result<Limits, SourceError> {
    let login = agent
        .found_login(profile, account)
        .ok_or(SourceError::NotAvailable(Unavailable::KeyGone))?;

    match provider.read(account, &login)? {
        AccountNumbers::Limits(answer) => Ok(answer.into_value()),
        _ => Err(SourceError::NotAvailable(Unavailable::NoAccountSource)),
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::agent::{
        Identity, JsonField, KeyPlace, Login, LoginKind,
        ProviderRef as PartProvider,
    };
    use otty_plugin_host::provider::{
        AccountFacts, AccountNumbers as PartNumbers,
    };

    use super::*;
    use crate::account::{AccountIdentity, AccountLogin, FoundIn};
    use crate::limits::{Billing, MockLimitSource, UsageWindows};
    use crate::plugin_provider::MockProviderPartCalls;
    use crate::provider::ProviderRef;
    use crate::source_id::SourceName;
    use crate::testing::{MockAgent, mock_entry, profile};

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
        calls
            .expect_read_numbers()
            .times(times)
            .returning(move |_, found| {
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
            });

        PluginProvider::made_up("anthropic", &["anthropic.com"], calls)
    }

    fn usage_screen(times: usize) -> Box<dyn LimitSource> {
        let mut source = MockLimitSource::new();
        source
            .expect_id()
            .return_const(SourceId::Named(SourceName::new("made-up /usage")));
        source.expect_limits().times(times).returning(|_| {
            Ok(Limits::new(
                Billing::UsageWindows(UsageWindows::default()),
                None,
            ))
        });
        Box::new(source)
    }

    #[test]
    fn the_provider_part_answers_first_and_the_agent_is_not_asked() {
        let agent = agent_with(vec![claude_login()]);
        let providers = [anthropic(Ok(()), 1)];
        let sources = [usage_screen(0)];

        let (limits, ids) = read_profile_limits(
            &agent,
            &providers,
            &sources,
            &profile("Example"),
        );

        let answer = limits.expect("the limits");
        assert_eq!(answer.source(), providers[0].source());
        assert_eq!(answer.value().plan(), Some("Max 20x"));
        assert_eq!(
            ids,
            [
                providers[0].source(),
                SourceId::Named(SourceName::new("made-up /usage"))
            ]
        );
    }

    #[test]
    fn when_the_provider_part_fails_the_agents_source_is_asked() {
        let agent = agent_with(vec![claude_login()]);
        let providers = [anthropic(Err(SourceError::KeyRefused), 1)];
        let sources = [usage_screen(1)];

        let (limits, _) = read_profile_limits(
            &agent,
            &providers,
            &sources,
            &profile("Example"),
        );

        let answer = limits.expect("the limits");
        assert_eq!(
            answer.source(),
            SourceId::Named(SourceName::new("made-up /usage"))
        );
        assert_eq!(answer.earlier_failures().len(), 1);
        assert_eq!(
            answer.earlier_failures()[0].error(),
            &SourceError::NotLoggedIn
        );
    }

    #[test]
    fn a_profile_with_no_login_a_part_serves_walks_only_its_agent() {
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
        let sources = [usage_screen(1)];

        let (limits, ids) = read_profile_limits(
            &agent,
            &providers,
            &sources,
            &profile("Example"),
        );

        assert!(limits.is_ok());
        assert_eq!(ids, [SourceId::Named(SourceName::new("made-up /usage"))]);
    }
}
