//! "Your spend this month" of an OpenAI API key found in Codex folders
//! (issue 22 of spec 2026-11-23-accounts-apart-from-profiles): what the
//! Codex sessions of the folders it pays for cost this calendar month, at
//! API prices, kind by kind.
//!
//! Each Codex session names the provider that served it
//! (`model_provider`), but not the login it used, so a folder's answers
//! served by OpenAI are counted for the key that folder logs in with now
//! (owner, 2026-11-25): a folder that changed its login this month counts
//! its earlier answers for the new one too. Only this computer's sessions
//! of Codex count, not what other programs spent through the same key.
//! The answers are read the way the agent part reads them
//! (`spend_file.rs`) and priced at the price OTTY gives during `numbers`.

use otty_plugin_sdk::agents::{
    Account, AccountKind, LoginFolder, Money, MonthSpend, ProviderRef,
};
use otty_plugin_sdk::files;

use crate::accounts::{CODEX, FolderLogin};
use crate::base_url::OPENAI;
use crate::glob::escaped;
use crate::sessions::session_file_globs;
use crate::spend_file::read_profile_answers;

/// What the Codex sessions of the profile folders the API key `account`
/// logs in with now spent this month at API prices, in US dollars by
/// kind; `None` for a subscription, which shows its limits, for an admin
/// key, which runs no session, for a key no folder logs in with, and when
/// no answer had a price.
pub(crate) fn key_spend(account: &Account) -> Option<Money> {
    if account.kind != AccountKind::Key {
        return None;
    }

    let folders: Vec<&str> = account
        .found_in
        .iter()
        .filter_map(|folder| match folder {
            LoginFolder::Profile((agent, path)) if agent == CODEX => {
                Some(path.as_str())
            },
            _ => None,
        })
        .filter(|path| {
            FolderLogin::read(path).is_some_and(|login| {
                matches!(login, FolderLogin::Key(_)) && login.id() == account.id
            })
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

/// Adds the answers of this month served by OpenAI in the session files
/// of the Codex folder `folder` to `month`. A file changed before the
/// month started holds none, and a file that can not be read counts
/// nothing.
fn add_folder(month: &mut MonthSpend, folder: &str) {
    let openai = ProviderRef::Id(OPENAI.to_string());

    for glob in session_file_globs(&escaped(folder), "*") {
        let Ok(files) = files::list(&glob) else {
            continue;
        };
        for file in files {
            if file.changed_ms < month.since_ms() {
                continue;
            }
            let Ok(answers) = read_profile_answers(&file.path) else {
                continue;
            };
            for answer in &answers.answers {
                if answer.provider.as_ref() == Some(&openai) {
                    month.add(answer);
                }
            }
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
    use crate::OpenAi;
    use crate::testing::{PROVIDER, codex_folder, host, own_folder};

    /// 2026-11-20 at 12:00 UTC.
    const NOV_20_NOON: i64 = 1_795_176_000_000;

    /// The host at `now_ms` in UTC, where gpt-5-codex costs $1.25 per
    /// million input tokens, $0.125 read from the cache and $10 of output.
    fn priced_host_at(now_ms: i64) -> TestHost {
        let price = ModelPrice {
            standard: TokenPrices {
                input: 1_250_000,
                cached: 125_000,
                cache_written: 0,
                output: 10_000_000,
            },
            long_prompt: None,
        };

        host()
            .with_now_ms(now_ms)
            .with_local_offset_minutes(0)
            .with_price("gpt-5-codex", None, price)
    }

    /// The account the part finds in the Codex folder `name`.
    fn account_in(host: &TestHost, name: &str) -> Account {
        let folders = [codex_folder(host, name)];

        host.as_provider_in(PROVIDER, &folders, || {
            OpenAi::accounts(PROVIDER, &folders).accounts
        })
        .pop()
        .expect("the account")
    }

    /// What `account` spent this month, read in a call given its folders.
    fn spend_of(host: &TestHost, account: &Account) -> Option<Money> {
        host.as_provider_in(PROVIDER, &account.found_in, || key_spend(account))
    }

    /// Owner, 2026-11-25: a key found in a Codex folder pays for that
    /// folder's sessions served by OpenAI. Each answer counts once, also
    /// when its session is archived too; the answer of October and the
    /// session served by a local model are left out.
    #[test]
    fn a_key_spends_what_its_codex_sessions_cost_this_month() {
        let host = priced_host_at(NOV_20_NOON);
        let key = account_in(&host, ".codex-spend-key");

        let spent = spend_of(&host, &key).expect("the spend of the key");

        // 600 input not read from the cache at $1.25, 400 read at $0.125
        // and 100 out at $10 per million tokens.
        assert_eq!(key.kind, AccountKind::Key);
        assert_eq!(
            spent.by_kind,
            Some(PricedCost {
                total: 1_800,
                input: 750,
                cache_written: 0,
                cached: 50,
                output: 1_000,
            })
        );
        assert!((spent.amount - 0.0018).abs() < 1e-9, "{}", spent.amount);
        assert_eq!(spent.currency, "USD");
    }

    /// The numbers of a key carry its spend; nothing is sent for it.
    #[test]
    fn the_numbers_of_a_key_carry_its_spend() {
        let host = priced_host_at(NOV_20_NOON);
        let key = account_in(&host, ".codex-spend-key");

        let numbers = host
            .as_provider_in(PROVIDER, &key.found_in, || {
                OpenAi::numbers(PROVIDER, &key, &[])
            })
            .expect("the numbers");

        let spent = numbers.spent_this_month.expect("the spend of the key");
        assert_eq!(spent.by_kind.map(|cost| cost.total), Some(1_800));
        assert!(host.requests().is_empty(), "nothing is sent for a key");
    }

    /// A ChatGPT subscription shows its limits, not this, and an admin key
    /// runs no session.
    #[test]
    fn a_subscription_and_an_admin_key_have_no_spend() {
        let host = priced_host_at(NOV_20_NOON);
        let subscription = account_in(&host, ".codex-plus");
        let admin = account_in(&host, ".codex-admin");

        assert_eq!(spend_of(&host, &subscription), None);
        assert_eq!(spend_of(&host, &admin), None);
    }

    /// The session files of OTTY's own login are never read.
    #[test]
    fn an_own_login_has_no_spend() {
        let host = priced_host_at(NOV_20_NOON);
        let mut key = account_in(&host, ".codex-spend-key");
        key.found_in = vec![own_folder(&host, "6f0c1e2a", ".codex-spend-key")];

        assert_eq!(spend_of(&host, &key), None);
    }
}
