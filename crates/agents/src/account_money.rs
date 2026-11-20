//! The money of an account paid per token (spec Decisions 4 and 5): its
//! bill when one was read, else an estimate over the tokens of the
//! profiles that pay through it.

use std::collections::BTreeMap;

use jiff::civil::Date;

use crate::account::Account;
use crate::bill::Bill;
use crate::money_estimate::MoneyEstimate;
use crate::price_table::PriceTable;
use crate::registry::Registry;
use crate::spend::{DaySpend, ModelTokens, Spend};

/// The money of an account, for the "Spent this month" of its page: the
/// bill of the account as its provider counts it, or an estimate at list
/// prices, which the screen marks with "~".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountMoney {
    /// The bill of the account.
    Bill(Bill),
    /// The estimate over the tokens of the profiles that pay through the
    /// account.
    Estimate(MoneyEstimate),
}

impl AccountMoney {
    /// The money of `account`: its `bill` when one was read, else the
    /// estimate at `prices` over the tokens of `profiles`, each the tokens
    /// a profile spent with the accounts found in it.
    ///
    /// A token counts for the account when the agent wrote that the
    /// account paid it, as Codex does per session, or when it went to the
    /// account's provider and the profile has no other account of that
    /// provider. The tokens of a profile with several accounts of one
    /// provider that does not write which one paid are not split between
    /// them, and count for none. A provider named by its host is the one
    /// of `registry` that serves it.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_agents::{
    ///     Account, AccountIdentity, AccountKind, AccountLogin, AccountMoney,
    ///     DaySpend, Dollars, FoundIn, KeyFingerprint, ModelPrice, ModelTokens,
    ///     PriceTable, ProviderRef, Registry, Spend, TokenCount, TokenPrices,
    /// };
    ///
    /// let key = Account::new(
    ///     ProviderRef::of_id("deepseek"),
    ///     AccountIdentity::KeyFingerprint(KeyFingerprint::of("sk-made-up")),
    ///     AccountLogin::Key(None),
    ///     AccountKind::Money,
    ///     FoundIn::ProfileFolder(PathBuf::from("/home/user/.claude")),
    /// );
    /// let today = DaySpend::new(
    ///     jiff::civil::date(2026, 11, 16),
    ///     vec![
    ///         ModelTokens::new(
    ///             Some("deepseek-chat"),
    ///             TokenCount::new(1_000_000, 0, 0),
    ///             1,
    ///         )
    ///         .with_provider(Some(ProviderRef::of_id("deepseek"))),
    ///     ],
    /// );
    /// let spend = Spend::new(vec![today], Vec::new());
    /// let prices = PriceTable::new(vec![ModelPrice::new(
    ///     "deepseek-chat",
    ///     TokenPrices {
    ///         input: Dollars::from_micro_dollars(270_000),
    ///         ..TokenPrices::default()
    ///     },
    /// )]);
    ///
    /// let accounts = [key.clone()];
    /// let money = AccountMoney::of(
    ///     &key,
    ///     None,
    ///     &[(&spend, &accounts)],
    ///     &prices,
    ///     &Registry::default(),
    /// );
    ///
    /// let AccountMoney::Estimate(estimate) = money else {
    ///     panic!("no bill was read");
    /// };
    /// assert_eq!(estimate.today().cost().micro_dollars(), 270_000);
    /// ```
    pub fn of(
        account: &Account,
        bill: Option<&Bill>,
        profiles: &[(&Spend, &[Account])],
        prices: &PriceTable,
        registry: &Registry,
    ) -> Self {
        if let Some(bill) = bill {
            return Self::Bill(bill.clone());
        }

        let spend = spend_of_account(account, profiles, registry);

        Self::Estimate(MoneyEstimate::new(&spend, prices))
    }
}

/// The tokens of `profiles` that `account` paid, day by day: the days of
/// every profile, so today is the latest of them.
fn spend_of_account(
    account: &Account,
    profiles: &[(&Spend, &[Account])],
    registry: &Registry,
) -> Spend {
    let mut days: BTreeMap<Date, Vec<ModelTokens>> = BTreeMap::new();

    for (spend, accounts) in profiles {
        for day in spend.days() {
            let paid = day
                .models()
                .iter()
                .filter(|tokens| account.pays_for(tokens, accounts, registry))
                .cloned();
            days.entry(day.date()).or_default().extend(paid);
        }
    }

    let days = days
        .into_iter()
        .map(|(date, models)| DaySpend::new(date, models))
        .collect();

    Spend::new(days, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AccountIdentity;
    use crate::bill::DayCost;
    use crate::price_table::ModelPrice;
    use crate::provider::ProviderRef;
    use crate::testing::{dollars, key_account, organization, token_prices};
    use crate::token_count::TokenCount;

    /// A dollar in micro-dollars.
    const DOLLAR: u64 = 1_000_000;

    /// Made-up prices: `m` costs $1 per million input tokens.
    fn prices() -> PriceTable {
        PriceTable::new(vec![ModelPrice::new(
            "m",
            token_prices(DOLLAR, 0, 0, 0),
        )])
    }

    /// `millions` of input tokens of `m` sent to `provider`, paid by
    /// `account` when the agent wrote it.
    fn tokens(
        millions: u64,
        provider: ProviderRef,
        account: Option<AccountIdentity>,
    ) -> ModelTokens {
        ModelTokens::new(Some("m"), TokenCount::new(millions * DOLLAR, 0, 0), 1)
            .with_provider(Some(provider))
            .with_account(account)
    }

    /// The tokens `models` spent today, 2026-11-16.
    fn today(models: Vec<ModelTokens>) -> Spend {
        Spend::new(
            vec![DaySpend::new(jiff::civil::date(2026, 11, 16), models)],
            Vec::new(),
        )
    }

    fn estimate_today(money: AccountMoney) -> u64 {
        match money {
            AccountMoney::Estimate(estimate) => {
                estimate.today().cost().micro_dollars()
            },
            AccountMoney::Bill(_) => panic!("no bill was read"),
        }
    }

    #[test]
    fn the_bill_is_the_money_of_an_organization_when_it_was_read() {
        let organization = organization("made-up-org");
        let bill = Bill::new(vec![DayCost::new(
            jiff::civil::date(2026, 11, 16),
            dollars(12),
        )]);
        let spend =
            today(vec![tokens(5, ProviderRef::of_id("anthropic"), None)]);
        let accounts = [organization.clone()];

        let money = AccountMoney::of(
            &organization,
            Some(&bill),
            &[(&spend, &accounts)],
            &prices(),
            &Registry::default(),
        );

        assert_eq!(money, AccountMoney::Bill(bill));
    }

    /// Two profiles use one DeepSeek key: its estimate adds up the tokens
    /// of both.
    #[test]
    fn the_estimate_is_summed_over_the_profiles_that_pay_through_it() {
        let key =
            key_account(ProviderRef::of_id("deepseek"), "sk-made-up-deepseek");
        let home = today(vec![tokens(2, ProviderRef::of_id("deepseek"), None)]);
        let work = today(vec![
            tokens(3, ProviderRef::of_id("deepseek"), None),
            tokens(7, ProviderRef::of_id("anthropic"), None),
        ]);
        let accounts = [key.clone()];

        let money = AccountMoney::of(
            &key,
            None,
            &[(&home, &accounts), (&work, &accounts)],
            &prices(),
            &Registry::default(),
        );

        assert_eq!(estimate_today(money), 5 * DOLLAR);
    }

    /// Spec Decision 4: a profile with two accounts of one provider that
    /// does not write which one paid is not split; tokens whose account is
    /// written count for that account only.
    #[test]
    fn tokens_are_not_split_when_the_agent_does_not_tell_the_account() {
        let one = key_account(ProviderRef::of_id("openai"), "sk-made-up-one");
        let two = key_account(ProviderRef::of_id("openai"), "sk-made-up-two");
        let spend = today(vec![
            tokens(4, ProviderRef::of_id("openai"), None),
            tokens(
                1,
                ProviderRef::of_id("openai"),
                Some(two.identity().clone()),
            ),
        ]);
        let accounts = [one.clone(), two.clone()];

        let of_one = AccountMoney::of(
            &one,
            None,
            &[(&spend, &accounts)],
            &prices(),
            &Registry::default(),
        );
        let of_two = AccountMoney::of(
            &two,
            None,
            &[(&spend, &accounts)],
            &prices(),
            &Registry::default(),
        );

        assert_eq!(estimate_today(of_one), 0);
        assert_eq!(estimate_today(of_two), DOLLAR);
    }

    #[test]
    fn today_is_the_latest_day_of_the_profiles() {
        let key =
            key_account(ProviderRef::of_id("deepseek"), "sk-made-up-deepseek");
        let yesterday = Spend::new(
            vec![DaySpend::new(
                jiff::civil::date(2026, 11, 15),
                vec![tokens(2, ProviderRef::of_id("deepseek"), None)],
            )],
            Vec::new(),
        );
        let now = today(vec![tokens(3, ProviderRef::of_id("deepseek"), None)]);
        let accounts = [key.clone()];

        let money = AccountMoney::of(
            &key,
            None,
            &[(&yesterday, &accounts), (&now, &accounts)],
            &prices(),
            &Registry::default(),
        );

        let AccountMoney::Estimate(estimate) = money else {
            panic!("no bill was read");
        };
        assert_eq!(estimate.today().cost().micro_dollars(), 3 * DOLLAR);
        assert_eq!(estimate.last_7_days().cost().micro_dollars(), 5 * DOLLAR);
    }
}
