//! What was read of each account (spec
//! 2026-11-23-accounts-apart-from-profiles, Decisions 1, 5 and 8): its
//! numbers, the limits of a subscription or the money of a key or an
//! organization, and the bill of an organization. The provider part that
//! found the account reads them, in the folders it found it in; no profile
//! takes part. Each account is read once, and the last good numbers stay,
//! with their time, when a later read fails.

use std::time::SystemTime;

use otty_agents::{
    Account, AccountNumbers, Balance, SourceError, SourceFailure, SourceId,
    Unavailable,
};

use super::bill::AccountBill;
use super::freshness::{ReadState, Which};
use super::limits::AccountLimits;
use super::types::{LimitsRead, LoadedAccountNumbers, LoadedBill};

/// What was read of one account.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AccountRead {
    account: Account,
    last_good: Option<(AccountNumbers, SystemTime)>,
    failure: Option<SourceError>,
    reads: ReadState,
    may_start_programs: bool,
    waits_for_programs: bool,
    bill: AccountBill,
}

impl AccountRead {
    /// Why the latest read of the numbers failed, when it did.
    pub(crate) fn failure(&self) -> Option<&SourceError> {
        self.failure.as_ref()
    }

    /// The last good money left or key limit and when it was read.
    pub(crate) fn balance(&self) -> Option<(&Balance, SystemTime)> {
        match &self.last_good {
            Some((AccountNumbers::Balance(balance), read_at)) => {
                Some((balance, *read_at))
            },
            _ => None,
        }
    }

    /// The limits of a subscription its provider part read; `None` before
    /// any read that told of them. A part with no way to read the limits,
    /// or no part that reads the account, is nothing to show.
    pub(crate) fn limits(&self) -> Option<AccountLimits> {
        let mut limits = AccountLimits::default();

        if let Some((AccountNumbers::Limits(answer), read_at)) = &self.last_good
        {
            let read = match answer.value().is_nothing() {
                true => LimitsRead::NoSources,
                false => LimitsRead::Answered(answer.clone()),
            };
            limits.apply(read, *read_at);
        }
        if let Some(failure) = &self.failure {
            let read_at = self.reads.read_at()?;
            let read = match failure {
                SourceError::NotAvailable(Unavailable::NoAccountSource) => {
                    LimitsRead::NoSources
                },
                _ => LimitsRead::Failed(vec![SourceFailure::new(
                    SourceId::ProviderPart,
                    failure.clone(),
                )]),
            };
            limits.apply(read, read_at);
        }

        limits.read_at().map(|_| limits)
    }

    /// What was read of the bill of the organization the account is.
    pub(crate) fn bill(&self) -> &AccountBill {
        &self.bill
    }

    /// Nothing read yet of `account`.
    fn new(account: Account) -> Self {
        Self {
            account,
            last_good: None,
            failure: None,
            reads: ReadState::default(),
            may_start_programs: true,
            waits_for_programs: false,
            bill: AccountBill::default(),
        }
    }

    /// Whether the numbers need a read for `which`: unlike
    /// [`ReadState::is_wanted`], numbers that wait for the agent's program
    /// are stale whatever their time.
    fn is_wanted(&self, which: Which) -> bool {
        match which {
            Which::WaitingForPrograms => self.waits_for_programs,
            Which::Stale(_) => {
                self.waits_for_programs || self.reads.is_wanted(which)
            },
            Which::Every => true,
        }
    }

    /// Start a read, which may start the agent's program when
    /// `may_start_programs`; `false` when one is on its way.
    fn start(&mut self, may_start_programs: bool) -> bool {
        if !self.reads.start() {
            return false;
        }

        self.may_start_programs = may_start_programs;
        true
    }

    /// Take the answer of a read that ended at `read_at`: numbers replace
    /// the last good ones, a failure keeps them and says why. A read that
    /// could not start the agent's program and gave nothing waits for it:
    /// the provider part may read by asking the agent, such as with
    /// `claude /usage`, once the program may start (issue 29 of spec
    /// 2026-11-19-agent-plugins).
    fn apply(
        &mut self,
        numbers: Result<AccountNumbers, SourceError>,
        read_at: SystemTime,
    ) {
        let gave_something = match &numbers {
            Ok(AccountNumbers::Limits(answer)) => !answer.value().is_nothing(),
            Ok(AccountNumbers::Balance(_)) => true,
            Err(_) => false,
        };

        self.reads.end_at(read_at);
        self.waits_for_programs = !gave_something && !self.may_start_programs;

        match numbers {
            Ok(numbers) => {
                self.last_good = Some((numbers, read_at));
                self.failure = None;
            },
            Err(failure) => self.failure = Some(failure),
        }
    }
}

#[cfg(test)]
impl AccountRead {
    /// Whether a read is on its way and when the latest one ended.
    fn reads(&self) -> &ReadState {
        &self.reads
    }
}

/// What was read of every account.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AccountReads {
    reads: Vec<AccountRead>,
}

impl AccountReads {
    /// What was read of `account`, if it was asked for.
    pub(crate) fn of(&self, account: &Account) -> Option<&AccountRead> {
        self.reads
            .iter()
            .find(|read| read.account.is_same_account(account))
    }

    /// Start a read of the numbers of each account of `wanted` that
    /// `which` picks and that has no read on its way, and return those.
    /// The read may start the agent's program when `may_start_programs`.
    /// What was read of an account no longer wanted, such as one hidden,
    /// is forgotten.
    pub(crate) fn start(
        &mut self,
        wanted: Vec<Account>,
        which: Which,
        may_start_programs: bool,
    ) -> Vec<Account> {
        self.reads.retain(|read| {
            wanted
                .iter()
                .any(|account| account.is_same_account(&read.account))
        });

        let mut started = Vec::new();
        for account in wanted {
            let read = self.read_mut(&account);
            if read.is_wanted(which) && read.start(may_start_programs) {
                started.push(account);
            }
        }

        started
    }

    /// Start a read of the bill of each organization of `wanted` that
    /// `which` picks and that has no read of its bill on its way, and
    /// return those.
    pub(crate) fn start_bills(
        &mut self,
        wanted: Vec<Account>,
        which: Which,
    ) -> Vec<Account> {
        let mut started = Vec::new();

        for organization in wanted {
            let bill = &mut self.read_mut(&organization).bill;
            if bill.reads().is_wanted(which) && bill.start_reading() {
                started.push(organization);
            }
        }

        started
    }

    /// Forget what was read of `account`, so it is read again at once,
    /// such as after the user gave a key a part asked for.
    pub(crate) fn forget(&mut self, account: &Account) {
        self.reads
            .retain(|read| !read.account.is_same_account(account));
    }

    /// Take the answer of the read of the numbers of one account; an
    /// answer of an account no longer read is dropped.
    pub(crate) fn apply(&mut self, loaded: LoadedAccountNumbers) {
        let Some(read) = self.asked_mut(&loaded.account) else {
            return;
        };

        read.apply(loaded.numbers, loaded.read_at);
    }

    /// Take the answer of the read of the bill of one organization; an
    /// answer of an organization no longer read is dropped.
    pub(crate) fn apply_bill(&mut self, loaded: LoadedBill) {
        let Some(read) = self.asked_mut(&loaded.organization) else {
            return;
        };

        read.bill.apply(loaded.bill, loaded.read_at);
    }

    /// What was read of `account`, if it was asked for, to change it.
    fn asked_mut(&mut self, account: &Account) -> Option<&mut AccountRead> {
        self.reads
            .iter_mut()
            .find(|read| read.account.is_same_account(account))
    }

    /// What was read of `account`, made empty when it was never asked
    /// for.
    fn read_mut(&mut self, account: &Account) -> &mut AccountRead {
        let position = self
            .reads
            .iter()
            .position(|read| read.account.is_same_account(account));

        match position {
            Some(position) => &mut self.reads[position],
            None => {
                self.reads.push(AccountRead::new(account.clone()));
                let last = self.reads.len() - 1;
                &mut self.reads[last]
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::Amount;

    use super::*;
    use crate::testing::{
        at, bill_of, chatgpt_login, five_hour_limits, limits_numbers,
        organization,
    };

    /// A made-up DeepSeek key found in the Claude Code folder.
    fn deepseek() -> Account {
        crate::testing::found_key("deepseek")
    }

    fn left(value: f64) -> AccountNumbers {
        AccountNumbers::Balance(Balance::new(Amount::new(value, "CNY"), None))
    }

    fn loaded(
        account: Account,
        numbers: Result<AccountNumbers, SourceError>,
        read_at: SystemTime,
    ) -> LoadedAccountNumbers {
        LoadedAccountNumbers {
            account,
            numbers,
            read_at,
        }
    }

    #[test]
    fn an_account_is_read_once_until_its_read_ends() {
        let mut reads = AccountReads::default();
        let wanted = vec![deepseek()];

        let first = reads.start(wanted.clone(), Which::Every, true);
        let again = reads.start(wanted.clone(), Which::Every, true);
        reads.apply(loaded(deepseek(), Ok(left(110.0)), at(10)));
        let after = reads.start(wanted, Which::Stale(at(20)), true);

        assert_eq!(first, [deepseek()]);
        assert!(again.is_empty(), "a read is on its way");
        assert!(after.is_empty(), "the read is fresh");
        let read = reads.of(&deepseek()).map(AccountRead::reads);
        assert!(read.is_some_and(|read| !read.is_reading()));
    }

    #[test]
    fn a_failed_read_keeps_the_last_good_numbers_and_says_why() {
        let mut reads = AccountReads::default();
        let _ = reads.start(vec![deepseek()], Which::Every, true);
        reads.apply(loaded(deepseek(), Ok(left(110.0)), at(10)));
        let _ = reads.start(vec![deepseek()], Which::Every, true);
        reads.apply(loaded(
            deepseek(),
            Err(crate::testing::refused_key_need()),
            at(20),
        ));

        let read = reads.of(&deepseek()).expect("the account was read");

        assert_eq!(
            read.balance().map(|(balance, at)| (balance.clone(), at)),
            Some((Balance::new(Amount::new(110.0, "CNY"), None), at(10)))
        );
        assert_eq!(read.failure(), Some(&crate::testing::refused_key_need()));
        assert_eq!(read.reads().read_at(), Some(at(20)));
    }

    /// An account no longer wanted is no longer read, and a late answer of
    /// it is dropped.
    #[test]
    fn an_account_no_longer_wanted_is_forgotten() {
        let mut reads = AccountReads::default();
        let _ = reads.start(vec![deepseek()], Which::Every, true);

        let _ = reads.start(Vec::new(), Which::Every, true);
        reads.apply(loaded(deepseek(), Ok(left(1.0)), at(10)));

        assert_eq!(reads.of(&deepseek()), None);
    }

    #[test]
    fn the_limits_of_a_login_stay_when_a_later_read_fails() {
        let mut reads = AccountReads::default();
        let _ = reads.start(vec![chatgpt_login()], Which::Every, true);
        reads.apply(loaded(
            chatgpt_login(),
            Ok(limits_numbers(five_hour_limits(42.0))),
            at(10),
        ));
        let answered = reads.of(&chatgpt_login()).and_then(AccountRead::limits);
        reads.apply(loaded(
            chatgpt_login(),
            Err(crate::testing::login_expired()),
            at(20),
        ));
        let expired = reads.of(&chatgpt_login()).and_then(AccountRead::limits);

        assert!(
            answered
                .as_ref()
                .is_some_and(|limits| limits.last_good().is_some())
        );
        let expired = expired.expect("the limits were read");
        assert_eq!(expired.needs().len(), 1);
        assert!(expired.last_good().is_some(), "the last good limits stay");
    }

    /// No part that reads the account, or a key, has no limits to draw.
    #[test]
    fn an_account_no_part_reads_or_a_key_has_no_limits_to_draw() {
        let mut reads = AccountReads::default();
        let wanted = vec![chatgpt_login(), deepseek()];
        let _ = reads.start(wanted, Which::Every, true);
        reads.apply(loaded(
            chatgpt_login(),
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource)),
            at(10),
        ));
        reads.apply(loaded(deepseek(), Ok(left(1.0)), at(10)));

        let unread = reads.of(&chatgpt_login()).and_then(AccountRead::limits);
        let key = reads.of(&deepseek()).and_then(AccountRead::limits);

        assert!(unread.is_some_and(|limits| !limits.has_sources()));
        assert_eq!(key, None);
    }

    /// A read while Agents mode was away did not start the agent's
    /// program; when it gave nothing, the program may answer on screen,
    /// so the numbers wait for it although the read is fresh.
    #[test]
    fn a_read_that_gave_nothing_without_programs_waits_for_them() {
        let mut reads = AccountReads::default();
        let now = at(10_000);
        let wanted = || vec![chatgpt_login()];
        let stopped =
            || Err(SourceError::NotAvailable(Unavailable::ReadStopped));

        let _ = reads.start(wanted(), Which::Every, false);
        reads.apply(loaded(chatgpt_login(), stopped(), now));
        let waits = reads.start(wanted(), Which::Stale(now), true);
        reads.apply(loaded(chatgpt_login(), stopped(), now));
        let tried = reads.start(wanted(), Which::WaitingForPrograms, true);

        assert_eq!(waits, [chatgpt_login()]);
        assert!(tried.is_empty(), "on screen the programs had their chance");
    }

    /// Issue 29: an answer with nothing in it, read without the
    /// program, waits for it too; a real answer does not.
    #[test]
    fn only_an_empty_answer_without_programs_waits_for_them() {
        let mut reads = AccountReads::default();
        let nothing = otty_agents::Limits::new(
            otty_agents::Billing::UsageWindows(
                otty_agents::UsageWindows::default(),
            ),
            None,
        );

        let _ = reads.start(vec![chatgpt_login()], Which::Every, false);
        reads.apply(loaded(
            chatgpt_login(),
            Ok(limits_numbers(nothing)),
            at(10),
        ));
        let waits =
            reads.start(vec![chatgpt_login()], Which::WaitingForPrograms, true);
        reads.apply(loaded(
            chatgpt_login(),
            Ok(limits_numbers(five_hour_limits(10.0))),
            at(20),
        ));
        let _ = reads.start(vec![chatgpt_login()], Which::Every, false);
        reads.apply(loaded(
            chatgpt_login(),
            Ok(limits_numbers(five_hour_limits(10.0))),
            at(30),
        ));
        let answered =
            reads.start(vec![chatgpt_login()], Which::WaitingForPrograms, true);

        assert_eq!(waits, [chatgpt_login()]);
        assert!(answered.is_empty(), "the part needed no program");
    }

    #[test]
    fn the_bill_of_an_organization_is_read_once_and_kept_with_it() {
        let mut reads = AccountReads::default();
        let wanted = || vec![organization()];

        let first = reads.start_bills(wanted(), Which::Every);
        let again = reads.start_bills(wanted(), Which::Every);
        reads.apply_bill(LoadedBill {
            organization: organization(),
            given: Vec::new(),
            bill: Ok(bill_of(1, 2)),
            read_at: at(10),
        });
        let bill = reads.of(&organization()).map(AccountRead::bill);

        assert_eq!(first, [organization()]);
        assert!(again.is_empty(), "a read is on its way");
        assert_eq!(bill.and_then(AccountBill::answer), Some(&bill_of(1, 2)));
    }
}
