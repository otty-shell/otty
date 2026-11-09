//! Runs the widget generator on a copy of `app/src`, so the real sources stay
//! untouched.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{fs, io, process};

use otty_xtask::architecture::allowlist::ALLOWLIST;
use otty_xtask::architecture::report::{Output, render};
use otty_xtask::architecture::{Comparison, check};
use otty_xtask::new_widget::{self, NewWidgetError};

/// A copy of the real `app/src` in a fresh temporary folder, removed when
/// the test ends.
struct AppSrcCopy {
    root: PathBuf,
}

impl AppSrcCopy {
    /// The copied `app/src`.
    fn path(&self) -> &Path {
        &self.root
    }

    /// Copy `app/src` into a folder named after `test`.
    fn new(test: &str) -> Self {
        let pid = process::id();
        let root =
            std::env::temp_dir().join(format!("otty-xtask-{test}-{pid}"));
        let app_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/src");

        // A run that crashed may have left the folder behind.
        let _ = fs::remove_dir_all(&root);
        copy_dir(&app_src, &root).expect("app/src can be copied");

        Self { root }
    }

    /// Every folder and file under the copy, with the contents of files.
    fn snapshot(&self) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        let mut entries = BTreeMap::new();
        read_entries(&self.root, &self.root, &mut entries)
            .expect("the copy can be read");

        entries
    }
}

impl Drop for AppSrcCopy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
#[ignore = "generator moves to dispatch/ and effects/ in step 3 of specs/2026-11-09-split-router"]
fn a_generated_widget_follows_the_architecture() {
    let copy = AppSrcCopy::new("follows-architecture");

    let widget = new_widget::create(copy.path(), "sample_notes")
        .expect("the widget is created");

    let created: Vec<&str> = widget.created().collect();
    assert_eq!(
        created,
        [
            "widgets/sample_notes/mod.rs",
            "widgets/sample_notes/event.rs",
            "widgets/sample_notes/state.rs",
            "widgets/sample_notes/reducer.rs",
            "widgets/sample_notes/model.rs",
            "widgets/sample_notes/view/mod.rs",
            "widgets/sample_notes/view/sample_notes_panel.rs",
            "router/sample_notes.rs",
        ]
    );
    for path in &created {
        assert!(copy.path().join(path).is_file(), "{path} is written");
    }

    let findings = check(copy.path()).expect("the copy can be read");
    let comparison = Comparison::compare(&findings, ALLOWLIST);
    let report = render(&comparison, Output::Plain);
    assert!(comparison.new_findings().is_empty(), "{report}");
    assert!(comparison.stale_entries().is_empty(), "{report}");
}

#[test]
#[ignore = "generator moves to dispatch/ and effects/ in step 3 of specs/2026-11-09-split-router"]
fn the_summary_lists_the_files_and_how_to_show_the_widget() {
    let copy = AppSrcCopy::new("summary");

    let widget = new_widget::create(copy.path(), "sample_notes")
        .expect("the widget is created");
    let summary = widget.summary();

    let changed: Vec<&str> = widget.changed().collect();
    assert_eq!(changed, ["widgets/mod.rs", "router/mod.rs", "app.rs"]);
    for path in widget.created().chain(widget.changed()) {
        assert!(summary.contains(&format!("  {path}\n")), "{summary}");
    }
    let next_steps = [
        "cargo +nightly fmt",
        "use crate::widgets::sample_notes::view::sample_notes_panel;",
        "vm: app.widgets.sample_notes.vm(),",
        "AppEvent::SampleNotes(SampleNotesEvent::Intent(intent))",
        "subscription.rs",
    ];
    for step in next_steps {
        assert!(summary.contains(step), "{step} is missing:\n{summary}");
    }
}

#[test]
#[ignore = "generator moves to dispatch/ and effects/ in step 3 of specs/2026-11-09-split-router"]
fn a_second_run_with_the_same_name_is_refused() {
    let copy = AppSrcCopy::new("second-run");
    new_widget::create(copy.path(), "sample_notes")
        .expect("the first run creates the widget");
    let before = copy.snapshot();

    let error = new_widget::create(copy.path(), "sample_notes")
        .expect_err("the second run is refused");

    assert!(matches!(error, NewWidgetError::NameTaken(_)), "{error}");
    assert_eq!(error.to_string(), "`widgets/sample_notes/` already exists");
    assert!(copy.snapshot() == before, "the second run changed files");
}

#[test]
#[ignore = "generator moves to dispatch/ and effects/ in step 3 of specs/2026-11-09-split-router"]
fn a_missing_anchor_leaves_every_file_unchanged() {
    let copy = AppSrcCopy::new("missing-anchor");
    let app_rs = copy.path().join("app.rs");
    let text = fs::read_to_string(&app_rs).expect("app.rs can be read");
    let without_anchor = text.replace(
        "let widgets = Widgets {",
        "let widgets = crate::widgets::Widgets {",
    );
    fs::write(&app_rs, without_anchor).expect("app.rs can be written");
    let before = copy.snapshot();

    let error = new_widget::create(copy.path(), "sample_notes")
        .expect_err("the run stops at the missing anchor");

    assert_eq!(
        error.to_string(),
        "app.rs: cannot find `let widgets = Widgets {`; nothing was changed"
    );
    assert!(copy.snapshot() == before, "the failed run changed files");
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;

    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }

    Ok(())
}

fn read_entries(
    root: &Path,
    dir: &Path,
    entries: &mut BTreeMap<PathBuf, Option<Vec<u8>>>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        if path.is_dir() {
            entries.insert(relative, None);
            read_entries(root, &path, entries)?;
        } else {
            entries.insert(relative, Some(fs::read(&path)?));
        }
    }

    Ok(())
}
