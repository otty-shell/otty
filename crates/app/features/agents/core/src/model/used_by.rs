//! "Used by" on an account page (spec 2026-11-19, Decisions 4 and 6): a
//! chip for each profile the account was found in, with the tokens it sent
//! today through the account's provider. A profile with several accounts
//! of that provider whose agent does not write which one served a message
//! is not split: its chip says how many accounts it uses, with no number.

use otty_agents::{Account, DaySpend, Profile, Provider, TokenCount};
use otty_app_i18n::{self as i18n, Key};

use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::numbers::short_count;

/// One chip of "Used by"; a press opens the profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsedByViewModel {
    /// The profile, sent when the chip is pressed.
    pub(crate) profile: Profile,
    /// The name of its agent's icon.
    pub(crate) icon: &'static str,
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
                icon: state.icon_of(profile.agent()),
                name: profile.name().to_string(),
                usage: usage(shown, account),
            }
        })
        .collect()
}

/// What `shown` sent through `account`: "uses 2 OpenAI accounts" when it
/// has several accounts of the provider and its agent does not write which
/// one paid, else the tokens the account paid today, or "no tokens yet"
/// when it paid none in the days counted. `None` before a count.
fn usage(shown: &ProfileState, account: &Account) -> Option<String> {
    let provider = account.provider();
    let days = shown
        .spend()
        .last_good()
        .map(|(spend, _)| spend.last_30_days_by_day());

    if let Some(count) = untold_accounts(shown, provider) {
        return Some(i18n::filled(
            Key::TplAgentsUsesAccounts,
            &[("count", &count.to_string()), ("provider", provider.name())],
        ));
    }

    let paid: Vec<TokenCount> = days?
        .iter()
        .map(|day| paid_on(day, account, shown.accounts()))
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
    shown: &ProfileState,
    provider: Provider,
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
            tokens.provider() == Some(provider) && tokens.account().is_some()
        });

    (of_provider > 1 && !writes_accounts).then_some(of_provider)
}

/// The tokens of `day` that `account` paid, in a profile with the accounts
/// `profile_accounts`.
fn paid_on(
    day: &DaySpend,
    account: &Account,
    profile_accounts: &[Account],
) -> TokenCount {
    day.models()
        .iter()
        .filter(|tokens| account.pays_for(tokens, profile_accounts))
        .fold(TokenCount::default(), |sum, tokens| sum + tokens.tokens())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        AccountIdentity, AccountKind, AccountLogin, FoundIn, ModelTokens,
        Provider, SavedProfiles, Spend,
    };

    use super::*;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, at, chatgpt_login, claude, claude_work, codex, codex_work,
        connected, organization, spend_loaded, with_accounts,
    };
    use crate::types::SpendCount;

    /// Tokens of `model` sent to `provider`, `today` input tokens today,
    /// 2026-11-16, and 5 000 five days before; the agent wrote the
    /// `account` that paid them, or none.
    fn spend(
        provider: Provider,
        model: &str,
        today: u64,
        account: Option<AccountIdentity>,
    ) -> Spend {
        let tokens = |input: u64| {
            ModelTokens::new(Some(model), TokenCount::new(input, 0, 0), 1)
                .with_provider(Some(provider))
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
            Provider::OpenAI,
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
            spend(Provider::Anthropic, "claude-sonnet-4-5", 1_200_000, None),
        );
        counted(
            &mut state,
            &claude_work(),
            spend(Provider::DeepSeek, "deepseek-chat", 9_000, None),
        );

        let chips = used_by(&state, &organization());

        assert_eq!(
            chips,
            vec![
                UsedByViewModel {
                    profile: claude(),
                    icon: "claude-code",
                    name: String::from("Claude Code"),
                    usage: today("1.2M"),
                },
                UsedByViewModel {
                    profile: claude_work(),
                    icon: "claude-code",
                    name: String::from("Claude Code · Work"),
                    usage: Some(i18n::t(Key::AgentsNoTokensYet).to_string()),
                },
            ]
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
            spend(Provider::OpenAI, "gpt-5", 30_000, None),
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
                Provider::OpenAI,
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
