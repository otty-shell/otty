//! The counting of tokens over the session files of one config folder, in
//! one pass, with what each file told kept between refreshes in a
//! [`FileMemory`].
//!
//! A file with the same stamp as on the last count is not read again. A
//! file that did not change since the first counted day is not opened at
//! all: it can hold no record of the counted days.
//!
//! Each file also tells how many records of the kind its reader reads it
//! saw and understood; when the newest files show the agent changed their
//! format, the count fails with [`SourceError::FormatChanged`] instead of
//! quietly giving fewer tokens (spec 2026-11-19, Decision 9).

use std::cmp::Reverse;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::account::AccountIdentity;
use crate::dollars::Dollars;
use crate::error::SourceError;
use crate::file_memory::{
    FileMemory, ReadFile, ReadFiles, try_read_if_changed,
};
use crate::file_stamp::FileStamp;
use crate::local_time::LocalNow;
use crate::provider::ProviderRef;
use crate::record_format::{RecordCounts, RecordFormat, format_change};
use crate::spend::Spend;
use crate::spend_tally::{first_counted_day, tally};
use crate::token_count::TokenCount;

/// One answer of the model found in a session file. Its project, provider
/// and account, when a plugin part names them for each answer, go before
/// the ones of its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CountedAnswer {
    identity: Option<u64>,
    at: SystemTime,
    model: Option<String>,
    tokens: TokenCount,
    cost: Option<Dollars>,
    project: Option<PathBuf>,
    provider: Option<ProviderRef>,
    account: Option<AccountIdentity>,
}

impl CountedAnswer {
    /// What tells this answer from the others, when the agent writes it.
    /// Two records with the same identity are the same answer.
    pub(crate) fn identity(&self) -> Option<u64> {
        self.identity
    }

    /// When the answer was written.
    pub(crate) fn at(&self) -> SystemTime {
        self.at
    }

    /// The model that answered, when the file names it.
    pub(crate) fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// The tokens of the answer.
    pub(crate) fn tokens(&self) -> TokenCount {
        self.tokens
    }

    /// What the answer cost, when the agent wrote it.
    pub(crate) fn cost(&self) -> Option<Dollars> {
        self.cost
    }

    /// The project folder of this answer, when named for it alone.
    pub(crate) fn project(&self) -> Option<&Path> {
        self.project.as_deref()
    }

    /// The provider that served this answer, when named for it alone.
    pub(crate) fn provider(&self) -> Option<&ProviderRef> {
        self.provider.as_ref()
    }

    /// The account that paid for this answer, when named for it alone.
    pub(crate) fn account(&self) -> Option<&AccountIdentity> {
        self.account.as_ref()
    }

    /// An answer written at `at` by `model`, with no identity.
    pub(crate) fn new(
        at: SystemTime,
        model: Option<&str>,
        tokens: TokenCount,
    ) -> Self {
        Self {
            identity: None,
            at,
            model: model.map(String::from),
            tokens,
            cost: None,
            project: None,
            provider: None,
            account: None,
        }
    }

    /// The same answer, which cost `cost` as the agent wrote it.
    pub(crate) fn with_cost(self, cost: Option<Dollars>) -> Self {
        Self { cost, ..self }
    }

    /// The same answer, given in the project folder `project`.
    pub(crate) fn with_project(self, project: Option<PathBuf>) -> Self {
        Self { project, ..self }
    }

    /// The same answer, served by `provider`.
    pub(crate) fn with_provider(self, provider: Option<ProviderRef>) -> Self {
        Self { provider, ..self }
    }

    /// The same answer, paid by `account`.
    pub(crate) fn with_account(self, account: Option<AccountIdentity>) -> Self {
        Self { account, ..self }
    }

    /// The same answer with the identity made of `parts`, such as the ids
    /// of the message and of the request. Only a short hash of the parts
    /// is kept, so many answers take little memory.
    pub(crate) fn with_identity(self, parts: &impl Hash) -> Self {
        let mut hasher = DefaultHasher::new();
        parts.hash(&mut hasher);

        Self {
            identity: Some(hasher.finish()),
            ..self
        }
    }
}

/// What one session file told: the project folder it ran in, the provider
/// and the account of the session when the agent writes them, the answers
/// of the model in it, and how well its records were understood.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FileSpend {
    project: Option<PathBuf>,
    provider: Option<ProviderRef>,
    account: Option<AccountIdentity>,
    answers: Vec<CountedAnswer>,
    records: RecordCounts,
}

impl FileSpend {
    /// The project folder of the session, when known.
    pub(crate) fn project(&self) -> Option<&Path> {
        self.project.as_deref()
    }

    /// The provider of the session, when the agent writes it.
    pub(crate) fn provider(&self) -> Option<&ProviderRef> {
        self.provider.as_ref()
    }

    /// The account of the session, when the agent writes it.
    pub(crate) fn account(&self) -> Option<&AccountIdentity> {
        self.account.as_ref()
    }

    /// The answers of the model, in the order of the file.
    pub(crate) fn answers(&self) -> &[CountedAnswer] {
        &self.answers
    }

    /// The records of the kind the reader reads that it saw in the file,
    /// and the ones it understood.
    pub(crate) fn records(&self) -> &RecordCounts {
        &self.records
    }

    /// What a file told: its `project` and its `answers`.
    pub(crate) fn new(
        project: Option<PathBuf>,
        answers: Vec<CountedAnswer>,
    ) -> Self {
        Self {
            project,
            provider: None,
            account: None,
            answers,
            records: RecordCounts::default(),
        }
    }

    /// The same file, of a session served by `provider`.
    pub(crate) fn with_provider(
        mut self,
        provider: Option<ProviderRef>,
    ) -> Self {
        self.provider = provider;
        self
    }

    /// The same file, of a session paid by `account`.
    pub(crate) fn with_account(
        mut self,
        account: Option<AccountIdentity>,
    ) -> Self {
        self.account = account;
        self
    }

    /// The same file, whose reader saw and understood `records`.
    pub(crate) fn with_records(mut self, records: RecordCounts) -> Self {
        self.records = records;
        self
    }
}

/// The tokens spent in the session files `paths` of `config_dir` at `now`.
/// The answers of a file that names no provider are counted under
/// `profile_provider`.
///
/// A file is read with `read_file` only when it changed since the last
/// count kept in `memory`, and not at all when it did not change since the
/// first counted day. A file that is gone is skipped and forgotten.
///
/// When the newest files show that the agent changed the `format` of its
/// files, the count fails with [`SourceError::FormatChanged`]. When
/// `read_file` fails, so does the count: the files read before it are
/// kept, and the one that failed is read again next time.
pub(crate) fn count_spend(
    memory: &FileMemory<FileSpend>,
    config_dir: &Path,
    paths: Vec<PathBuf>,
    now: &LocalNow,
    profile_provider: Option<&ProviderRef>,
    format: RecordFormat,
    mut read_file: impl FnMut(&Path) -> Result<FileSpend, SourceError>,
) -> Result<Spend, SourceError> {
    let counted_since = now.start_of(first_counted_day(now));

    let mut previous = memory.take(config_dir);
    let mut counted = ReadFiles::new();
    for path in paths {
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        let stamp = FileStamp::of(&metadata);
        if stamp
            .changed_at()
            .is_some_and(|changed_at| changed_at < counted_since)
        {
            continue;
        }

        match try_read_if_changed(&mut previous, &path, stamp, &mut read_file) {
            Ok(file) => {
                counted.insert(path, file);
            },
            Err(error) => {
                counted.extend(previous);
                memory.keep(config_dir, counted);
                return Err(error);
            },
        }
    }

    let change = format_change(format, newest_first(&counted));
    let spend = tally(
        counted.values().map(|file| file.content()),
        now,
        profile_provider,
    );
    memory.keep(config_dir, counted);

    match change {
        Some(change) => Err(SourceError::FormatChanged(Box::new(change))),
        None => Ok(spend),
    }
}

/// The record counts of the `files`, the file changed last first.
fn newest_first(files: &ReadFiles<FileSpend>) -> Vec<&RecordCounts> {
    let mut files: Vec<&ReadFile<FileSpend>> = files.values().collect();
    files.sort_by_key(|file| Reverse(file.stamp().changed_at()));

    files.iter().map(|file| file.content().records()).collect()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::testing::{
        NOW, TempDir, at, set_file_time, utc_now, write_file,
    };

    const DAY: u64 = 24 * 3600;

    const FORMAT: RecordFormat = RecordFormat::new("session file", "2.1.0");

    /// A made-up config folder with session files whose content is the
    /// number of input tokens of one answer written now.
    struct Folder {
        temp: TempDir,
        memory: FileMemory<FileSpend>,
        opened: RefCell<Vec<String>>,
    }

    impl Folder {
        fn new() -> Self {
            Self {
                temp: TempDir::new(),
                memory: FileMemory::default(),
                opened: RefCell::new(Vec::new()),
            }
        }

        fn path(&self, name: &str) -> PathBuf {
            self.temp.path().join(name)
        }

        fn write(&self, name: &str, tokens: u64, changed: u64) {
            write_file(&self.path(name), &tokens.to_string());
            set_file_time(&self.path(name), at(changed));
        }

        /// Counts the files `names` and says which were opened.
        fn count(&self, names: &[&str]) -> (Spend, Vec<String>) {
            let spend = self
                .try_count(names)
                .expect("the format of the files is known");

            (spend, self.opened.borrow().clone())
        }

        /// Counts the files `names`, which may show a changed format.
        fn try_count(&self, names: &[&str]) -> Result<Spend, SourceError> {
            let paths = names.iter().map(|name| self.path(name)).collect();
            self.opened.borrow_mut().clear();

            count_spend(
                &self.memory,
                self.temp.path(),
                paths,
                &utc_now(),
                None,
                FORMAT,
                |path| Ok(self.read(path)),
            )
        }

        /// What a file tells: one answer of as many input tokens as its
        /// content says, and ten records understood; or, for the content
        /// `changed`, ten records not understood.
        fn read(&self, path: &Path) -> FileSpend {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.opened.borrow_mut().push(name);

            let content = std::fs::read_to_string(path).unwrap_or_default();
            let mut records = RecordCounts::default();
            if content.trim() == "changed" {
                for _ in 0..10 {
                    records.add_not_understood("message.usage", "", None);
                }
                return FileSpend::default().with_records(records);
            }

            let tokens = content.trim().parse().unwrap_or_default();
            let answer = CountedAnswer::new(
                at(NOW),
                Some("m"),
                TokenCount::new(tokens, 0, 0),
            );
            for _ in 0..10 {
                records.add_understood();
            }
            FileSpend::new(None, vec![answer]).with_records(records)
        }
    }

    #[test]
    fn every_file_is_read_once_and_added_up() {
        let folder = Folder::new();
        folder.write("a.jsonl", 10, NOW);
        folder.write("b.jsonl", 20, NOW - DAY);

        let (spend, mut opened) = folder.count(&["a.jsonl", "b.jsonl"]);

        opened.sort();
        assert_eq!(opened, ["a.jsonl", "b.jsonl"]);
        assert_eq!(spend.today(), TokenCount::new(30, 0, 0));
    }

    #[test]
    fn a_file_that_did_not_change_is_not_read_again() {
        let folder = Folder::new();
        folder.write("a.jsonl", 10, NOW);
        folder.write("b.jsonl", 20, NOW);

        let _ = folder.count(&["a.jsonl", "b.jsonl"]);
        folder.write("a.jsonl", 70, NOW);
        let (spend, opened) = folder.count(&["a.jsonl", "b.jsonl"]);

        assert!(opened.is_empty());
        assert_eq!(spend.today(), TokenCount::new(30, 0, 0));
    }

    #[test]
    fn a_file_that_changed_is_read_again_and_a_gone_one_is_forgotten() {
        let folder = Folder::new();
        folder.write("a.jsonl", 10, NOW);
        folder.write("b.jsonl", 20, NOW);

        let _ = folder.count(&["a.jsonl", "b.jsonl"]);
        folder.write("a.jsonl", 100, NOW);
        std::fs::remove_file(folder.path("b.jsonl")).expect("remove b");
        let (spend, opened) = folder.count(&["a.jsonl", "b.jsonl"]);

        assert_eq!(opened, ["a.jsonl"]);
        assert_eq!(spend.today(), TokenCount::new(100, 0, 0));
    }

    #[test]
    fn a_file_not_changed_in_the_counted_days_is_not_opened() {
        let folder = Folder::new();
        let first_day_start = NOW - 10 * 3600 - 29 * DAY;
        folder.write("old.jsonl", 10, first_day_start - 1);
        folder.write("edge.jsonl", 20, first_day_start);

        let (spend, opened) = folder.count(&["old.jsonl", "edge.jsonl"]);

        assert_eq!(opened, ["edge.jsonl"]);
        assert_eq!(spend.today(), TokenCount::new(20, 0, 0));
    }

    #[test]
    fn each_config_folder_keeps_its_own_files() {
        let folder = Folder::new();
        folder.write("a.jsonl", 10, NOW);
        let other_dir = folder.path("other-config");

        let _ = folder.count(&["a.jsonl"]);
        let empty = count_spend(
            &folder.memory,
            &other_dir,
            Vec::new(),
            &utc_now(),
            None,
            FORMAT,
            |path| Ok(folder.read(path)),
        )
        .expect("no file is no change");
        let (again, opened) = folder.count(&["a.jsonl"]);

        assert!(empty.today().is_zero());
        assert!(opened.is_empty());
        assert_eq!(again.today(), TokenCount::new(10, 0, 0));
    }

    /// Spec 2026-11-19, Decision 9: the newest file holds records that are
    /// not understood, so the count fails instead of giving fewer tokens;
    /// an older file like it does not.
    #[test]
    fn a_newest_file_not_understood_fails_the_count() {
        let folder = Folder::new();
        folder.write("old.jsonl", 10, NOW - DAY);
        write_file(&folder.path("new.jsonl"), "changed");
        set_file_time(&folder.path("new.jsonl"), at(NOW));

        let changed = folder.try_count(&["old.jsonl", "new.jsonl"]);
        set_file_time(&folder.path("new.jsonl"), at(NOW - 2 * DAY));
        let old_change = folder.try_count(&["old.jsonl", "new.jsonl"]);

        let Err(SourceError::FormatChanged(change)) = changed else {
            panic!("the newest file is not understood: {changed:?}");
        };
        assert_eq!((change.seen(), change.understood()), (10, 0));
        let spend = old_change.expect("the newest file is understood");
        assert_eq!(spend.today(), TokenCount::new(10, 0, 0));
    }

    #[test]
    fn the_identity_is_the_same_for_the_same_parts_only() {
        let answer = CountedAnswer::new(at(NOW), None, TokenCount::default());

        let first = answer.clone().with_identity(&("msg_1", "req_1"));
        let same = answer.clone().with_identity(&("msg_1", "req_1"));
        let other = answer.clone().with_identity(&("msg_1", "req_2"));

        assert_eq!(answer.identity(), None);
        assert_eq!(first.identity(), same.identity());
        assert_ne!(first.identity(), other.identity());
    }
    /// A plugin part that traps fails the count; the file it trapped on is
    /// read again next time, the files read before it are not.
    #[test]
    fn a_read_that_failed_fails_the_count_and_is_read_again() {
        let folder = Folder::new();
        folder.write("a.jsonl", 10, NOW);
        folder.write("b.jsonl", 20, NOW);
        let paths = || vec![folder.path("a.jsonl"), folder.path("b.jsonl")];
        let opened = RefCell::new(Vec::new());
        let failing = |path: &Path| {
            opened.borrow_mut().push(path.to_path_buf());
            if path.ends_with("b.jsonl") {
                return Err(SourceError::NotLoggedIn);
            }
            Ok(folder.read(path))
        };

        let failed = count_spend(
            &folder.memory,
            folder.temp.path(),
            paths(),
            &utc_now(),
            None,
            FORMAT,
            failing,
        );
        opened.borrow_mut().clear();
        let (spend, _) = folder.count(&["a.jsonl", "b.jsonl"]);

        assert_eq!(failed, Err(SourceError::NotLoggedIn));
        assert_eq!(folder.opened.borrow().clone(), ["b.jsonl"]);
        assert_eq!(spend.today(), TokenCount::new(30, 0, 0));
    }
}
