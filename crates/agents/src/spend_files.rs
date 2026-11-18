//! The counting of tokens over the session files of one config folder, in
//! one pass, with what each file told kept between refreshes in a
//! [`FileMemory`].
//!
//! A file with the same stamp as on the last count is not read again. A
//! file that did not change since the first counted day is not opened at
//! all: it can hold no record of the counted days.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::file_memory::{FileMemory, ReadFiles, read_if_changed};
use crate::file_stamp::FileStamp;
use crate::local_time::LocalNow;
use crate::spend::Spend;
use crate::spend_tally::{first_counted_day, tally};
use crate::token_count::TokenCount;

/// One answer of the model found in a session file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CountedAnswer {
    identity: Option<u64>,
    at: SystemTime,
    model: Option<String>,
    tokens: TokenCount,
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
        }
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

/// What one session file told: the project folder it ran in and the
/// answers of the model in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FileSpend {
    project: Option<PathBuf>,
    answers: Vec<CountedAnswer>,
}

impl FileSpend {
    /// The project folder of the session, when known.
    pub(crate) fn project(&self) -> Option<&Path> {
        self.project.as_deref()
    }

    /// The answers of the model, in the order of the file.
    pub(crate) fn answers(&self) -> &[CountedAnswer] {
        &self.answers
    }

    /// What a file told: its `project` and its `answers`.
    pub(crate) fn new(
        project: Option<PathBuf>,
        answers: Vec<CountedAnswer>,
    ) -> Self {
        Self { project, answers }
    }
}

/// The tokens spent in the session files `paths` of `config_dir` at `now`.
///
/// A file is read with `read_file` only when it changed since the last
/// count kept in `cache`, and not at all when it did not change since the
/// first counted day. A file that is gone is skipped and forgotten.
pub(crate) fn count_spend(
    cache: &FileMemory<FileSpend>,
    config_dir: &Path,
    paths: Vec<PathBuf>,
    now: &LocalNow,
    read_file: impl Fn(&Path) -> FileSpend,
) -> Spend {
    let counted_since = now.start_of(first_counted_day(now));

    let mut previous = cache.take(config_dir);
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

        let file = read_if_changed(&mut previous, &path, stamp, &read_file);
        counted.insert(path, file);
    }

    let spend = tally(counted.values().map(|file| file.content()), now);
    cache.keep(config_dir, counted);

    spend
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::testing::{
        NOW, TempDir, at, set_file_time, utc_now, write_file,
    };

    const DAY: u64 = 24 * 3600;

    /// A made-up config folder with session files whose content is the
    /// number of input tokens of one answer written now.
    struct Folder {
        temp: TempDir,
        cache: FileMemory<FileSpend>,
        opened: RefCell<Vec<String>>,
    }

    impl Folder {
        fn new() -> Self {
            Self {
                temp: TempDir::new(),
                cache: FileMemory::default(),
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
            let paths = names.iter().map(|name| self.path(name)).collect();
            self.opened.borrow_mut().clear();

            let spend = count_spend(
                &self.cache,
                self.temp.path(),
                paths,
                &utc_now(),
                |path| self.read(path),
            );

            (spend, self.opened.borrow().clone())
        }

        fn read(&self, path: &Path) -> FileSpend {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.opened.borrow_mut().push(name);

            let tokens = std::fs::read_to_string(path)
                .ok()
                .and_then(|text| text.trim().parse().ok())
                .unwrap_or_default();
            let answer = CountedAnswer::new(
                at(NOW),
                Some("m"),
                TokenCount::new(tokens, 0, 0),
            );
            FileSpend::new(None, vec![answer])
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
            &folder.cache,
            &other_dir,
            Vec::new(),
            &utc_now(),
            |path| folder.read(path),
        );
        let (again, opened) = folder.count(&["a.jsonl"]);

        assert!(empty.today().is_zero());
        assert!(opened.is_empty());
        assert_eq!(again.today(), TokenCount::new(10, 0, 0));
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
}
