//! "Your spend this month" of an Anthropic account paid per token (issue
//! 22 of spec 2026-11-23-accounts-apart-from-profiles): what the Claude
//! Code sessions of the folders it pays for cost this calendar month, at
//! API prices, kind by kind.
//!
//! Claude Code writes no account in its sessions, so a folder's answers
//! are counted for the account that pays for that folder now
//! (`logins::paying_account`): a folder that changed its login this month
//! counts its earlier answers for the new one too. Only this computer's
//! sessions of Claude Code count, not what other programs spent through
//! the same key. The answers are read the way the agent part reads them
//! (`spend_file.rs`) and priced at the price OTTY gives during
//! `numbers`.

use otty_plugin_sdk::agents::{
    Account, AccountKind, LoginFolder, Money, MonthSpend,
};
use otty_plugin_sdk::files;

use crate::claude_code::session_files_glob;
use crate::logins::paying_account;
use crate::provider::profile_provider;
use crate::spend_file;

/// What the Claude Code sessions of the profile folders `account` pays
/// for spent this month at API prices, in US dollars by kind; `None` for
/// a subscription, which shows its limits, for an account that pays for
/// no folder, and when no answer had a price.
pub(crate) fn session_spend(account: &Account) -> Option<Money> {
    if account.kind == AccountKind::Subscription {
        return None;
    }

    let folders: Vec<&str> = account
        .found_in
        .iter()
        .filter(|folder| {
            paying_account(folder).is_some_and(|id| id == account.id)
        })
        .filter_map(|folder| match folder {
            LoginFolder::Profile((_, path)) => Some(path.as_str()),
            _ => None,
        })
        .collect();
    if folders.is_empty() {
        return None;
    }

    let mut month = MonthSpend::this_month();
    for folder in folders {
        add_folder(&mut month, folder);
    }

    month.money()
}

/// Adds the answers of this month in the session files of the profile
/// folder `folder` to `month`. A file changed before the month started
/// holds none, and a file that can not be read counts nothing.
fn add_folder(month: &mut MonthSpend, folder: &str) {
    let provider = profile_provider(folder);
    let Ok(files) = files::list(&session_files_glob(folder)) else {
        return;
    };

    for file in files {
        if file.changed_ms < month.since_ms() {
            continue;
        }
        let Ok(answers) =
            spend_file::read_answers(folder, &file.path, provider.as_ref())
        else {
            continue;
        };
        for answer in &answers.answers {
            month.add(answer);
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::{
        ModelPrice, PricedCost, Provider, TokenPrices,
    };
    use otty_plugin_sdk::test_host::TestHost;

    use super::*;
    use crate::Anthropic;
    use crate::testing::{
        PROVIDER, as_provider_in, host, login_folder, own_login_folder,
    };

    /// 2026-11-20 at 12:00 UTC.
    const NOV_20_NOON: i64 = 1_795_176_000_000;

    /// 2026-12-15 at 12:00 UTC.
    const DEC_15_NOON: i64 = 1_797_336_000_000;

    /// A dollar in millionths.
    const DOLLAR: u64 = 1_000_000;

    /// The host at `now_ms` in UTC, where Claude Sonnet 4.5 costs $3 per
    /// million input tokens, $3.75 written to the cache, $0.30 read from
    /// it and $15 of output.
    fn priced_host_at(now_ms: i64) -> TestHost {
        let price = ModelPrice {
            standard: TokenPrices {
                input: 3 * DOLLAR,
                cached: 3 * DOLLAR / 10,
                cache_written: 3_750_000,
                output: 15 * DOLLAR,
            },
            long_prompt: None,
        };

        host()
            .with_now_ms(now_ms)
            .with_local_offset_minutes(0)
            .with_price("claude-sonnet-4-5", None, price)
    }

    /// The accounts the part finds in the config folders `names`.
    fn accounts_in(host: &TestHost, names: &[&str]) -> Vec<Account> {
        let folders: Vec<_> =
            names.iter().map(|name| login_folder(host, name)).collect();

        as_provider_in(host, &folders, || {
            Anthropic::accounts(PROVIDER, &folders).accounts
        })
    }

    /// The one account of `kind` the part finds in the folders `names`.
    fn account_of(
        host: &TestHost,
        names: &[&str],
        kind: AccountKind,
    ) -> Account {
        accounts_in(host, names)
            .into_iter()
            .find(|account| account.kind == kind)
            .expect("the account")
    }

    /// What `account` spent this month, read in a call given its folders.
    fn spend_of(host: &TestHost, account: &Account) -> Option<Money> {
        as_provider_in(host, &account.found_in, || session_spend(account))
    }

    /// The key of the settings of `.claude-spend-key` pays for its
    /// sessions: this month's answers, each once, the helper agents' too,
    /// priced kind by kind; the answer of October is left out.
    #[test]
    fn a_key_spends_what_its_sessions_cost_this_month() {
        let host = priced_host_at(NOV_20_NOON);
        let key = account_of(&host, &[".claude-spend-key"], AccountKind::Key);

        let spent = spend_of(&host, &key).expect("the spend of the key");

        // 1 000 input not written to the cache and 500 of a helper agent at
        // $3, 2 000 written at $3.75, 3 000 read at $0.30, 400 and 100 out
        // at $15 per million tokens.
        assert_eq!(
            spent.by_kind,
            Some(PricedCost {
                total: 20_400,
                input: 4_500,
                cache_written: 7_500,
                cached: 900,
                output: 7_500,
            })
        );
        assert!((spent.amount - 0.0204).abs() < 1e-9, "{}", spent.amount);
        assert_eq!(spent.currency, "USD");
    }

    /// The numbers of the key carry its spend as the money spent this
    /// month, marked at API prices by its kinds.
    #[test]
    fn the_numbers_of_a_key_carry_its_spend() {
        let host = priced_host_at(NOV_20_NOON);
        let key = account_of(&host, &[".claude-spend-key"], AccountKind::Key);

        let numbers = as_provider_in(&host, &key.found_in, || {
            Anthropic::numbers(PROVIDER, &key, &[])
        })
        .expect("the numbers");

        let spent = numbers.spent_this_month.expect("the spend of the key");
        assert_eq!(spent.by_kind.map(|cost| cost.total), Some(20_400));
        assert!(host.requests().is_empty(), "nothing is sent for a key");
    }

    /// In December nothing of November is spent this month.
    #[test]
    fn the_answers_of_another_month_are_left_out() {
        let host = priced_host_at(DEC_15_NOON);
        let key = account_of(&host, &[".claude-spend-key"], AccountKind::Key);

        let spent = spend_of(&host, &key).expect("the spend of the key");

        assert_eq!(spent.by_kind.map(|cost| cost.total), Some(0));
        assert_eq!(spent.amount, 0.0);
    }

    /// A Console organization pays for the sessions of the folders it is
    /// logged in to, but for a folder whose settings send Claude Code to
    /// DeepSeek: those answers went to another provider. Its Console key
    /// is only how Claude Code reaches the organization, so it shows no
    /// spend of its own.
    #[test]
    fn an_organization_spends_what_went_to_anthropic_only() {
        let host = priced_host_at(NOV_20_NOON);
        let folders = [".claude-org-spend", ".claude-org-deepseek"];
        let organization =
            account_of(&host, &folders, AccountKind::Organization);
        let console_key = account_of(&host, &folders, AccountKind::Key);

        let spent = spend_of(&host, &organization).expect("the spend");
        let of_the_key = spend_of(&host, &console_key);

        // 100 input at $3 and 10 output at $15 per million tokens.
        assert_eq!(spent.by_kind.map(|cost| cost.total), Some(450));
        assert_eq!(organization.found_in.len(), 2);
        assert_eq!(of_the_key, None);
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles: a subscription shows
    /// its limits, not this.
    #[test]
    fn a_subscription_has_no_spend() {
        let host = priced_host_at(NOV_20_NOON);
        let subscription =
            account_of(&host, &[".claude-account"], AccountKind::Subscription);

        assert_eq!(spend_of(&host, &subscription), None);
    }

    /// The sessions of OTTY's own login are never read: they are the
    /// `/usage` probe's.
    #[test]
    fn an_own_login_has_no_spend() {
        let host = priced_host_at(NOV_20_NOON);
        let account = Account {
            id: "key:made-up".to_string(),
            name: "key ••••made".to_string(),
            kind: AccountKind::Key,
            plan: None,
            found_in: vec![own_login_folder(&host)],
            needs: Vec::new(),
        };

        assert_eq!(spend_of(&host, &account), None);
    }

    /// Before OTTY read its price list no answer has a price, so the spend
    /// is not known.
    #[test]
    fn with_no_price_the_spend_is_not_known() {
        let host = host().with_now_ms(NOV_20_NOON);
        let key = account_of(&host, &[".claude-spend-key"], AccountKind::Key);

        assert_eq!(spend_of(&host, &key), None);
    }
}
