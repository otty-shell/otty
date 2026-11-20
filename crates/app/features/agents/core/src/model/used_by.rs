//! "Used by" on an account page (spec 2026-11-19, Decisions 4 and 6): a
//! chip for each profile the account was found in, with the tokens it sent
//! today through the account's provider. A profile with several accounts
//! of that provider whose agent does not write which one served a message
//! is not split: its chip says how many accounts it uses, with no number.

use otty_agents::{
    Account, DaySpend, Profile, ProviderRef, Registry, TokenCount,
};
use otty_app_i18n::{self as i18n, Key};

use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::entry_icon::EntryIcon;
use super::numbers::short_count;

/// One chip of "Used by"; a press opens the profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsedByViewModel {
    /// The profile, sent when the chip is pressed.
    pub(crate) profile: Profile,
    /// The icon of its agent.
    pub(crate) icon: EntryIcon,
    /// The profile name.
    pub(crate) name: String,
    /// The tokens it sent today through the account ("1.2M today"), "uses
    /// 2 OpenAI accounts", or "no tokens yet"; `None` before its tokens
    /// were counted.
    pub(crate) usage: Option<String>,
}

/// The chips of the profiles `account` was found in, in their order.
pub(crate) fn used_by(
    state: &AgentsState,
    account: &Account,
) -> Vec<UsedByViewModel> {
    state
        .profiles()
        .found_with(account)
        .map(|shown| {
            let profile = shown.profile();
            UsedByViewModel {
                profile: profile.clone(),
                icon: state.agent_icon(profile.agent()),
                name: profile.name().to_string(),
                usage: usage(state, shown, account),
            }
        })
        .collect()
}

/// What `shown` sent through `account`: "uses 2 OpenAI accounts" when it
/// has several accounts of the provider and its agent does not write which
/// one paid, else the tokens the account paid today, or "no tokens yet"
/// when it paid none in the days counted. `None` before a count.
fn usage(
    state: &AgentsState,
    shown: &ProfileState,
    account: &Account,
) -> Option<String> {
    let registry = state.registry();
    let provider = account.provider();
    let days = shown
        .spend()
        .last_good()
        .map(|(spend, _)| spend.last_30_days_by_day());

    if let Some(count) = untold_accounts(registry, shown, provider) {
        return Some(i18n::filled(
            Key::TplAgentsUsesAccounts,
            &[
                ("count", &count.to_string()),
                ("provider", state.provider_name(provider)),
            ],
        ));
    }

    let paid: Vec<TokenCount> = days?
        .iter()
        .map(|day| paid_on(registry, day, account, shown.accounts()))
        .collect();
    if paid.iter().all(TokenCount::is_zero) {
        return Some(i18n::t(Key::AgentsNoTokensYet).to_string());
    }

    let today = paid.last().copied().unwrap_or_default();
    Some(i18n::filled(
        Key::TplAgentsAmountToday,
        &[("amount", &short_count(today.total()))],
    ))
}

/// How many accounts of `provider` the profile `shown` has, when there
/// are several and its agent does not write which one paid its tokens:
/// those tokens are then not split between them (spec Decision 4). `None`
/// when it has one account there, or its agent writes the account.
pub(super) fn untold_accounts(
    registry: &Registry,
    shown: &ProfileState,
    provider: &ProviderRef,
) -> Option<usize> {
    let of_provider = shown
        .accounts()
        .iter()
        .filter(|found| found.provider() == provider)
        .count();
    let days = shown
        .spend()
        .last_good()
        .map(|(spend, _)| spend.last_30_days_by_day());

    let writes_accounts = days
        .unwrap_or_default()
        .iter()
        .flat_map(DaySpend::models)
        .any(|tokens| {
            let is_of_provider = tokens.provider().is_some_and(|served| {
                registry.is_same_provider(served, provider)
            });
            is_of_provider && tokens.account().is_some()
        });

    (of_provider > 1 && !writes_accounts).then_some(of_provider)
}

/// The tokens of `day` that `account` paid, in a profile with the accounts
/// `profile_accounts`.
fn paid_on(
    registry: &Registry,
    day: &DaySpend,
    account: &Account,
    profile_accounts: &[Account],
) -> TokenCount {
    day.models()
        .iter()
        .filter(|tokens| account.pays_for(tokens, profile_accounts, registry))
        .fold(TokenCount::default(), |sum, tokens| sum + tokens.tokens())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        AccountIdentity, AccountKind, AccountLogin, FoundIn, ModelTokens,
        ProviderRef, SavedProfiles, Spend,
    };

    use super::*;
    use crate::model::ProfilePageViewModel;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, at, chatgpt_login, claude, claude_work, codex, codex_work,
        connected, deepseek_key_by_host, organization,
        registry_without_deepseek, spend_loaded, with_accounts,
        with_accounts_in,
    };
    use crate::types::SpendCount;

    /// Tokens of `model` sent to `provider`, `today` input tokens today,
    /// 2026-11-16, and 5 000 five days before; the agent wrote the
    /// `account` that paid them, or none.
    fn spend(
        provider: ProviderRef,
        model: &str,
        today: u64,
        account: Option<AccountIdentity>,
    ) -> Spend {
        let tokens = |input: u64| {
            ModelTokens::new(Some(model), TokenCount::new(input, 0, 0), 1)
                .with_provider(Some(provider.clone()))
                .with_account(account.clone())
        };

        Spend::new(
            vec![
                DaySpend::new(
                    jiff::civil::date(2026, 11, 11),
                    vec![tokens(5_000)],
                ),
                DaySpend::new(
                    jiff::civil::date(2026, 11, 16),
                    vec![tokens(today)],
                ),
            ],
            Vec::new(),
        )
    }

    fn counted(state: &mut AgentsState, profile: &Profile, spend: Spend) {
        let _ = reduce(
            state,
            spend_loaded(profile, SpendCount::Counted(spend), at(10)),
        );
    }

    /// A second made-up ChatGPT login, also found in the Codex folder.
    fn second_login() -> Account {
        Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::AccountId(String::from("made-up-second")),
            AccountLogin::Email(String::from("me@example.org")),
            AccountKind::Limits,
            FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
        )
    }

    fn today(amount: &str) -> Option<String> {
        Some(i18n::filled(
            Key::TplAgentsAmountToday,
            &[("amount", amount)],
        ))
    }

    /// Each profile of the organization: Claude Code sent 1.2M tokens to
    /// Anthropic today, Claude Code · Work sent tokens only to another
    /// provider, and Codex was not counted yet.
    #[test]
    fn each_profile_shows_its_tokens_today_through_the_provider() {
        let mut state = with_accounts(
            &[
                (claude(), api_key(), vec![organization()]),
                (claude_work(), api_key(), vec![organization()]),
                (codex(), connected(), vec![chatgpt_login()]),
            ],
            SavedProfiles::default(),
        );
        counted(
            &mut state,
            &claude(),
            spend(
                ProviderRef::of_id("anthropic"),
                "claude-sonnet-4-5",
                1_200_000,
                None,
            ),
        );
        counted(
            &mut state,
            &claude_work(),
            spend(ProviderRef::of_id("deepseek"), "deepseek-chat", 9_000, None),
        );

        let chips = used_by(&state, &organization());

        assert_eq!(
            chips,
            vec![
                UsedByViewModel {
                    profile: claude(),
                    icon: crate::testing::agent_icon("claude-code"),
                    name: String::from("Claude Code"),
                    usage: today("1.2M"),
                },
                UsedByViewModel {
                    profile: claude_work(),
                    icon: crate::testing::agent_icon("claude-code"),
                    name: String::from("Claude Code · Work"),
                    usage: Some(i18n::t(Key::AgentsNoTokensYet).to_string()),
                },
            ]
        );
    }

    /// Spec 2026-11-19-agent-plugins, "Identity and links do not depend on
    /// a provider plugin": with no DeepSeek entry the key behind Claude
    /// Code's base URL is listed by its host, still used by Claude Code
    /// with the tokens it sent there, and Claude Code still pays through
    /// it, with a letter icon.
    #[test]
    fn used_by_and_pays_through_stay_with_no_entry_for_the_provider() {
        let key = deepseek_key_by_host();
        let mut state = with_accounts_in(
            registry_without_deepseek(),
            &[(claude(), api_key(), vec![key.clone()])],
            SavedProfiles::default(),
        );
        let host = ProviderRef::Host(String::from("api.deepseek.com"));
        counted(
            &mut state,
            &claude(),
            spend(host, "deepseek-chat", 9_000, None),
        );

        let chips = used_by(&state, &key);
        let shown = state.profiles().get(&claude()).expect("shown");
        let page = ProfilePageViewModel::of(&state, shown, at(20));

        assert_eq!(
            chips,
            vec![UsedByViewModel {
                profile: claude(),
                icon: crate::testing::agent_icon("claude-code"),
                name: String::from("Claude Code"),
                usage: today("9K"),
            }]
        );
        assert_eq!(page.pays_through.len(), 1);
        assert_eq!(page.pays_through[0].account, key);
        assert_eq!(
            page.pays_through[0].icon,
            EntryIcon::Letter(String::from("api.deepseek.com"))
        );
    }

    #[test]
    fn a_profile_not_counted_yet_has_no_number() {
        let state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        );

        let chips = used_by(&state, &chatgpt_login());

        assert_eq!(chips.len(), 1);
        assert_eq!(chips[0].usage, None);
    }

    /// Spec 2026-11-19, Decision 4: two ChatGPT logins in one profile
    /// whose agent does not write which one served a message are not
    /// split.
    #[test]
    fn two_accounts_of_one_provider_not_told_apart_are_not_split() {
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login(), second_login()])],
            SavedProfiles::default(),
        );
        counted(
            &mut state,
            &codex(),
            spend(ProviderRef::of_id("openai"), "gpt-5", 30_000, None),
        );

        let chips = used_by(&state, &chatgpt_login());

        assert_eq!(
            chips[0].usage,
            Some(i18n::filled(
                Key::TplAgentsUsesAccounts,
                &[("count", "2"), ("provider", "OpenAI")]
            ))
        );
    }

    /// Codex writes the account of each session, so its tokens are told
    /// apart even with two logins in the profile.
    #[test]
    fn tokens_that_name_their_account_count_for_it_alone() {
        let mut state = with_accounts(
            &[
                (codex(), connected(), vec![chatgpt_login(), second_login()]),
                (codex_work(), connected(), vec![chatgpt_login()]),
            ],
            SavedProfiles::default(),
        );
        counted(
            &mut state,
            &codex(),
            spend(
                ProviderRef::of_id("openai"),
                "gpt-5",
                30_000,
                Some(second_login().identity().clone()),
            ),
        );

        let first = used_by(&state, &chatgpt_login());
        let second = used_by(&state, &second_login());

        assert_eq!(
            first[0].usage,
            Some(i18n::t(Key::AgentsNoTokensYet).to_string())
        );
        assert_eq!(first[1].profile, codex_work());
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].usage, today("30K"));
    }
}
