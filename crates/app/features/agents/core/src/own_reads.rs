//! What was read of the accounts that tell their own numbers with their
//! own key (spec 2026-11-19, Decisions 5 and 7): the money of a key, the
//! bill of an organization the user added, and the limits of a
//! subscription login no profile reads the limits of. Each account is read once, with
//! the key read again from where it was found; the last good numbers stay,
//! with their time, when a later read fails.

use std::time::SystemTime;

use otty_agents::{
    Account, AccountNumbers, Balance, Bill, KeyPlace, SourceError,
    SourceFailure, SourceId,
};

use super::freshness::{ReadState, Which};
use super::limits::ProfileLimits;
use super::types::{LimitsRead, LoadedAccountNumbers};

/// What was read of one account with its own key.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OwnRead {
    account: Account,
    last_good: Option<(AccountNumbers, SystemTime)>,
    failure: Option<SourceError>,
    reads: ReadState,
}

impl OwnRead {
    /// The account read.
    pub(crate) fn account(&self) -> &Account {
        &self.account
    }

    /// Why the latest read failed, when it did.
    pub(crate) fn failure(&self) -> Option<&SourceError> {
        self.failure.as_ref()
    }

    /// Whether a read is on its way and when the latest one ended.
    pub(crate) fn reads(&self) -> &ReadState {
        &self.reads
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

    /// The last good bill and when it was read.
    pub(crate) fn bill(&self) -> Option<(&Bill, SystemTime)> {
        match &self.last_good {
            Some((AccountNumbers::Bill(bill), read_at)) => {
                Some((bill, *read_at))
            },
            _ => None,
        }
    }

    /// The limits of a subscription login read by `source`, kept the way
    /// the limits of a profile are, so they are drawn the same way; `None`
    /// before any read.
    pub(crate) fn limits(&self, source: SourceId) -> Option<ProfileLimits> {
        let mut limits = ProfileLimits::default();

        if let Some((AccountNumbers::Limits(answer), read_at)) = &self.last_good
        {
            let read = LimitsRead::Answered(answer.clone());
            limits.apply(read, vec![source], *read_at);
        }
        if let Some(failure) = &self.failure {
            let read_at = self.reads.read_at()?;
            let read = LimitsRead::Failed(vec![SourceFailure::new(
                source,
                failure.clone(),
            )]);
            limits.apply(read, vec![source], read_at);
        }

        limits.reads().read_at().map(|_| limits)
    }

    /// Nothing read yet of `account`.
    fn new(account: Account) -> Self {
        Self {
            account,
            last_good: None,
            failure: None,
            reads: ReadState::default(),
        }
    }

    /// Take the answer of a read that ended at `read_at`: numbers replace
    /// the last good ones, a failure keeps them and says why.
    fn apply(
        &mut self,
        numbers: Result<AccountNumbers, SourceError>,
        read_at: SystemTime,
    ) {
        self.reads.end_at(read_at);

        match numbers {
            Ok(numbers) => {
                self.last_good = Some((numbers, read_at));
                self.failure = None;
            },
            Err(failure) => self.failure = Some(failure),
        }
    }
}

/// What was read of every account that tells its own numbers.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct OwnReads {
    reads: Vec<OwnRead>,
}

impl OwnReads {
    /// What was read of `account`, if it was asked for.
    pub(crate) fn of(&self, account: &Account) -> Option<&OwnRead> {
        self.reads
            .iter()
            .find(|read| read.account.is_same_account(account))
    }

    /// Whether a read is on its way for any account.
    pub(crate) fn is_reading_any(&self) -> bool {
        self.reads.iter().any(|read| read.reads.is_reading())
    }

    /// Start a read of each account of `wanted`, with the place its key
    /// is read again from, that `which` picks and that has no read on its
    /// way, and return those. What was read of an account no longer
    /// wanted, such as one removed or hidden, is forgotten.
    pub(crate) fn start(
        &mut self,
        wanted: Vec<(Account, KeyPlace)>,
        which: Which,
    ) -> Vec<(Account, KeyPlace)> {
        self.reads.retain(|read| {
            wanted
                .iter()
                .any(|(account, _)| account.is_same_account(&read.account))
        });

        let mut started = Vec::new();
        for (account, place) in wanted {
            let position = self
                .reads
                .iter()
                .position(|read| read.account.is_same_account(&account));
            let read = match position {
                Some(position) => &mut self.reads[position],
                None => {
                    self.reads.push(OwnRead::new(account.clone()));
                    let last = self.reads.len() - 1;
                    &mut self.reads[last]
                },
            };

            if read.reads.is_wanted(which) && read.reads.start() {
                started.push((account, place));
            }
        }

        started
    }

    /// Take the answer of the read of one account; an answer of an account
    /// no longer read is dropped.
    pub(crate) fn apply(&mut self, loaded: LoadedAccountNumbers) {
        let Some(read) = self
            .reads
            .iter_mut()
            .find(|read| read.account.is_same_account(&loaded.account))
        else {
            return;
        };

        read.apply(loaded.numbers, loaded.read_at);
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::{AdminKeyRef, Amount, ProviderRef};

    use super::*;
    use crate::testing::{at, chatgpt_login, five_hour_limits};

    fn deepseek() -> Account {
        Account::added(
            ProviderRef::of_id("deepseek"),
            AdminKeyRef::EnvVar(String::from("DEEPSEEK_API_KEY")),
        )
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
        let mut reads = OwnReads::default();
        let wanted = vec![(deepseek(), KeyPlace::Added)];

        let first = reads.start(wanted.clone(), Which::Every);
        let again = reads.start(wanted.clone(), Which::Every);
        reads.apply(loaded(deepseek(), Ok(left(110.0)), at(10)));
        let after = reads.start(wanted, Which::Stale(at(20)));

        assert_eq!(first, [(deepseek(), KeyPlace::Added)]);
        assert!(again.is_empty(), "a read is on its way");
        assert!(after.is_empty(), "the read is fresh");
        assert!(!reads.is_reading_any());
    }

    #[test]
    fn a_failed_read_keeps_the_last_good_numbers_and_says_why() {
        let mut reads = OwnReads::default();
        let _ = reads.start(vec![(deepseek(), KeyPlace::Added)], Which::Every);
        reads.apply(loaded(deepseek(), Ok(left(110.0)), at(10)));
        let _ = reads.start(vec![(deepseek(), KeyPlace::Added)], Which::Every);
        reads.apply(loaded(deepseek(), Err(SourceError::KeyRefused), at(20)));

        let read = reads.of(&deepseek()).expect("the account was read");

        assert_eq!(
            read.balance().map(|(balance, at)| (balance.clone(), at)),
            Some((Balance::new(Amount::new(110.0, "CNY"), None), at(10)))
        );
        assert_eq!(read.failure(), Some(&SourceError::KeyRefused));
        assert_eq!(read.reads().read_at(), Some(at(20)));
    }

    /// A removed account is no longer read, and a late answer of it is
    /// dropped.
    #[test]
    fn an_account_no_longer_wanted_is_forgotten() {
        let mut reads = OwnReads::default();
        let _ = reads.start(vec![(deepseek(), KeyPlace::Added)], Which::Every);

        let _ = reads.start(Vec::new(), Which::Every);
        reads.apply(loaded(deepseek(), Ok(left(1.0)), at(10)));

        assert_eq!(reads.of(&deepseek()), None);
    }

    #[test]
    fn the_limits_of_a_login_are_kept_as_the_limits_of_a_profile() {
        let mut reads = OwnReads::default();
        let place = KeyPlace::ProfileFiles(crate::testing::codex());
        let _ = reads.start(vec![(chatgpt_login(), place)], Which::Every);
        let answer = crate::testing::limits_answer(vec![(
            crate::testing::named_source("ChatGPT usage API"),
            Ok(five_hour_limits(42.0)),
        )]);
        let LimitsRead::Answered(answer) = answer else {
            panic!("an answer");
        };
        reads.apply(loaded(
            chatgpt_login(),
            Ok(AccountNumbers::Limits(answer)),
            at(10),
        ));
        let source = crate::testing::named_source("ChatGPT usage API");
        let answered = reads
            .of(&chatgpt_login())
            .and_then(|read| read.limits(source));
        reads.apply(loaded(
            chatgpt_login(),
            Err(SourceError::NotLoggedIn),
            at(20),
        ));
        let expired = reads
            .of(&chatgpt_login())
            .and_then(|read| read.limits(source));

        assert!(
            answered
                .as_ref()
                .is_some_and(|limits| limits.last_good().is_some())
        );
        let expired = expired.expect("the limits were read");
        assert!(expired.is_login_expired());
        assert!(expired.last_good().is_some(), "the last good limits stay");
    }
}
