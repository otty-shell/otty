//! What was read of the accounts that tell their own numbers with their
//! own key (spec 2026-11-19, Decisions 5 and 7): the money of a key and
//! the limits of a subscription login no profile reads the limits of. Each
//! account is read once, with the key read again from where it was found;
//! the last good numbers stay, with their time, when a later read fails.

use std::time::SystemTime;

use otty_agents::{
    Account, AccountNumbers, Balance, Profile, SourceError, SourceFailure,
    SourceId,
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
    /// Why the latest read failed, when it did.
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

    /// The limits of a subscription login its provider part read, kept
    /// the way the limits of a profile are, so they are drawn the same
    /// way; `None` before any read.
    pub(crate) fn limits(&self) -> Option<ProfileLimits> {
        let mut limits = ProfileLimits::default();

        if let Some((AccountNumbers::Limits(answer), read_at)) = &self.last_good
        {
            // A part with no way to read the limits answers nothing.
            let read = match answer.value().is_nothing() {
                true => LimitsRead::NoSources,
                false => LimitsRead::Answered(answer.clone()),
            };
            limits.apply(read, *read_at);
        }
        if let Some(failure) = &self.failure {
            let read_at = self.reads.read_at()?;
            let read = LimitsRead::Failed(vec![SourceFailure::new(
                SourceId::ProviderPart,
                failure.clone(),
            )]);
            limits.apply(read, read_at);
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

#[cfg(test)]
impl OwnRead {
    /// Whether a read is on its way and when the latest one ended.
    fn reads(&self) -> &ReadState {
        &self.reads
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

    /// Start a read of each account of `wanted`, with the profiles it was
    /// found in, the first of which names the place of its key, that
    /// `which` picks and that has no read on its way, and return those.
    /// What was read of an account no longer wanted, such as one hidden,
    /// is forgotten.
    pub(crate) fn start(
        &mut self,
        wanted: Vec<(Account, Vec<Profile>)>,
        which: Which,
    ) -> Vec<(Account, Vec<Profile>)> {
        self.reads.retain(|read| {
            wanted
                .iter()
                .any(|(account, _)| account.is_same_account(&read.account))
        });

        let mut started = Vec::new();
        for (account, found_in) in wanted {
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
                started.push((account, found_in));
            }
        }

        started
    }

    /// Forget what was read of `account`, so it is read again at once,
    /// such as after the user gave a key a part asked for: of every copy
    /// of its login, whichever plugin reads it.
    pub(crate) fn forget(&mut self, account: &Account) {
        self.reads
            .retain(|read| !read.account.is_same_login(account));
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
    use otty_agents::Amount;

    use super::*;
    use crate::testing::{at, chatgpt_login, five_hour_limits};

    /// A made-up DeepSeek key found in the Claude Code folder.
    fn deepseek() -> Account {
        crate::testing::read_by_its_plugin(Account::new(
            otty_agents::ProviderRef::of_id("deepseek"),
            otty_agents::AccountIdentity::KeyFingerprint(
                otty_agents::KeyFingerprint::of("sk-made-up-deepseek"),
            ),
            otty_agents::AccountLogin::Key(None),
            otty_agents::AccountKind::Money,
            otty_agents::FoundIn::ProfileFolder("/home/user/.claude".into()),
        ))
    }

    /// The profiles the DeepSeek key is read in.
    fn in_claude() -> Vec<Profile> {
        vec![crate::testing::claude()]
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
        let wanted = vec![(deepseek(), in_claude())];

        let first = reads.start(wanted.clone(), Which::Every);
        let again = reads.start(wanted.clone(), Which::Every);
        reads.apply(loaded(deepseek(), Ok(left(110.0)), at(10)));
        let after = reads.start(wanted, Which::Stale(at(20)));

        assert_eq!(first, [(deepseek(), in_claude())]);
        assert!(again.is_empty(), "a read is on its way");
        assert!(after.is_empty(), "the read is fresh");
        let read = reads.of(&deepseek()).map(OwnRead::reads);
        assert!(read.is_some_and(|read| !read.is_reading()));
    }

    #[test]
    fn a_failed_read_keeps_the_last_good_numbers_and_says_why() {
        let mut reads = OwnReads::default();
        let _ = reads.start(vec![(deepseek(), in_claude())], Which::Every);
        reads.apply(loaded(deepseek(), Ok(left(110.0)), at(10)));
        let _ = reads.start(vec![(deepseek(), in_claude())], Which::Every);
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
        let mut reads = OwnReads::default();
        let _ = reads.start(vec![(deepseek(), in_claude())], Which::Every);

        let _ = reads.start(Vec::new(), Which::Every);
        reads.apply(loaded(deepseek(), Ok(left(1.0)), at(10)));

        assert_eq!(reads.of(&deepseek()), None);
    }

    #[test]
    fn the_limits_of_a_login_are_kept_as_the_limits_of_a_profile() {
        let mut reads = OwnReads::default();
        let found_in = vec![crate::testing::codex()];
        let _ = reads.start(vec![(chatgpt_login(), found_in)], Which::Every);
        let answer = crate::testing::limits_answer(vec![(
            otty_agents::SourceId::ProviderPart,
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
        let answered = reads.of(&chatgpt_login()).and_then(OwnRead::limits);
        reads.apply(loaded(
            chatgpt_login(),
            Err(crate::testing::login_expired()),
            at(20),
        ));
        let expired = reads.of(&chatgpt_login()).and_then(OwnRead::limits);

        assert!(
            answered
                .as_ref()
                .is_some_and(|limits| limits.last_good().is_some())
        );
        let expired = expired.expect("the limits were read");
        assert_eq!(expired.needs().len(), 1);
        assert!(expired.last_good().is_some(), "the last good limits stay");
    }
}
