//! The answer of the `/usage` probe to Claude Code's question whether to
//! trust the folder it runs in.
//!
//! The probe answers "Yes" only for its own folder (spec Decision 12), as
//! CodexBar's Claude CLI session does (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeCLISession.swift>
//! (`workspaceTrustKeys`): an arrow moves the `❯` marker toward the "Yes"
//! option of the same list, and Enter is pressed only once the marker is on
//! it, so no other option is ever picked. Unlike CodexBar, which trusts
//! whenever it started in its own folder, the folder the question names is
//! also compared with the probe's folder.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// The "Yes" options of the question, in the new and the old layout.
const YES_OPTIONS: [&str; 2] = ["yes, i trust this folder", "yes, proceed"];
/// The marker of the option Enter picks.
const MARKER: char = '❯';
/// The border of a box Claude Code draws around a question.
const BOX_BORDER: char = '│';

/// What the probe does about the trust question on the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TrustAnswer {
    /// Press Enter: the marker is on "Yes".
    Confirm,
    /// Move the marker one option up, toward "Yes".
    MoveUp,
    /// Move the marker one option down, toward "Yes".
    MoveDown,
    /// Press nothing yet: no "Yes" option with a marker is drawn.
    Wait,
    /// Never answer: the question names another folder, given as the
    /// screen shows it.
    Refuse(String),
}

/// The answer to the trust question on `screen`, for the probe started in
/// its own folder, written as each of `probe_dirs` (the folder as OTTY
/// names it, and as the system resolves its links).
///
/// A screen that names a folder must name the probe's folder, written in
/// full or from the home folder (`~/…`). A screen that names no folder is
/// about the folder the probe started in, which is always its own.
pub(super) fn answer_trust(
    screen: &str,
    probe_dirs: &[PathBuf],
) -> TrustAnswer {
    let lines: Vec<&str> = screen.lines().map(inner_text).collect();

    if let Some(folder) = other_folder(&lines, probe_dirs) {
        return TrustAnswer::Refuse(folder.to_string());
    }

    let Some(yes_row) = lines.iter().position(|line| is_yes_option(line))
    else {
        return TrustAnswer::Wait;
    };
    let (first, last) = option_list(&lines, yes_row);
    let Some(marker_row) =
        (first..=last).find(|row| lines[*row].contains(MARKER))
    else {
        return TrustAnswer::Wait;
    };

    match marker_row.cmp(&yes_row) {
        Ordering::Equal => TrustAnswer::Confirm,
        Ordering::Less => TrustAnswer::MoveDown,
        Ordering::Greater => TrustAnswer::MoveUp,
    }
}

/// The text of a screen line without the spaces and the box borders
/// around it.
fn inner_text(line: &str) -> &str {
    line.trim().trim_matches(BOX_BORDER).trim()
}

/// The folder the question names when it is not the probe's folder:
/// `None` when a line names the probe's folder or no line names a folder.
/// A line the user's shell printed before `claude` started may start with
/// a path too, so any line that names the probe's folder is enough.
fn other_folder<'a>(
    lines: &[&'a str],
    probe_dirs: &[PathBuf],
) -> Option<&'a str> {
    let mut folders = lines.iter().copied().filter(|line| is_folder(line));
    let first = folders.clone().next()?;

    let names_probe_dir = folders
        .any(|folder| probe_dirs.iter().any(|dir| names_folder(folder, dir)));
    (!names_probe_dir).then_some(first)
}

/// Whether `line` is a folder: it starts with `/` or `~/`.
fn is_folder(line: &str) -> bool {
    line.starts_with('/') || line.starts_with("~/")
}

/// Whether the folder `text` on the screen is `dir`. A folder written from
/// the home folder, `~/.cache/otty/agent-probe`, is `dir` when `dir` ends
/// with the same parts.
fn names_folder(text: &str, dir: &Path) -> bool {
    let text = text.trim_end_matches('/');

    match text.strip_prefix("~/") {
        Some(from_home) => !from_home.is_empty() && dir.ends_with(from_home),
        None => Path::new(text) == dir,
    }
}

/// Whether `line` is the "Yes" option of the question.
fn is_yes_option(line: &str) -> bool {
    let lower = line.to_lowercase();

    YES_OPTIONS.iter().any(|option| lower.contains(option))
}

/// The first and the last row of the list of options around the row
/// `row`: the lines next to it up to a blank line.
fn option_list(lines: &[&str], row: usize) -> (usize, usize) {
    let mut first = row;
    while first > 0 && !lines[first - 1].is_empty() {
        first -= 1;
    }

    let mut last = row;
    while last + 1 < lines.len() && !lines[last + 1].is_empty() {
        last += 1;
    }

    (first, last)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROBE_DIR: &str = "/home/user/.cache/otty/agent-probe";

    const MARKER_ON_YES: &str = "\
 Accessing workspace:

 /home/user/.cache/otty/agent-probe

 Quick safety check: Is this a project you created or one you trust?

 ❯ 1. Yes, I trust this folder
   2. No, exit

 Enter to confirm · Esc to cancel
";

    const OLD_LAYOUT_IN_A_BOX: &str = "\
╭──────────────────────────────────────────────╮
│ Do you trust the files in this folder?       │
│                                              │
│ /home/user/.cache/otty/agent-probe           │
│                                              │
│ ❯ 1. Yes, proceed                            │
│   2. No, exit                                │
╰──────────────────────────────────────────────╯
";

    const NO_FOLDER_NAMED: &str = "\
 Do you trust the files in this folder?

 ❯ 1. Yes, proceed
   2. No, exit
";

    fn probe_dirs() -> Vec<PathBuf> {
        vec![PathBuf::from(PROBE_DIR)]
    }

    #[test]
    fn the_marker_on_yes_for_the_probe_folder_is_confirmed() {
        assert_eq!(
            answer_trust(MARKER_ON_YES, &probe_dirs()),
            TrustAnswer::Confirm
        );
        assert_eq!(
            answer_trust(OLD_LAYOUT_IN_A_BOX, &probe_dirs()),
            TrustAnswer::Confirm
        );
    }

    #[test]
    fn the_marker_is_moved_toward_yes() {
        let marker_below = MARKER_ON_YES
            .replace("❯ 1. Yes", "  1. Yes")
            .replace("  2. No", "❯ 2. No");
        let marker_above = MARKER_ON_YES
            .replace("❯ 1. Yes, I trust this folder", "❯ 1. No, exit")
            .replace("  2. No, exit", "  2. Yes, I trust this folder");

        assert_eq!(
            answer_trust(&marker_below, &probe_dirs()),
            TrustAnswer::MoveUp
        );
        assert_eq!(
            answer_trust(&marker_above, &probe_dirs()),
            TrustAnswer::MoveDown
        );
    }

    /// A marker that belongs to other text, not to the options, never
    /// confirms anything.
    #[test]
    fn with_no_marker_in_the_options_nothing_is_pressed() {
        let no_marker = MARKER_ON_YES.replace("❯ 1. Yes", "  1. Yes");
        let marker_elsewhere =
            format!("❯ an earlier line\n\n{}", no_marker.as_str());
        let no_options = " Do you trust the files in this folder?\n";

        assert_eq!(answer_trust(&no_marker, &probe_dirs()), TrustAnswer::Wait);
        assert_eq!(
            answer_trust(&marker_elsewhere, &probe_dirs()),
            TrustAnswer::Wait
        );
        assert_eq!(answer_trust(no_options, &probe_dirs()), TrustAnswer::Wait);
    }

    #[test]
    fn a_question_for_another_folder_is_refused() {
        let other = MARKER_ON_YES.replace(PROBE_DIR, "/home/user/work/secret");

        assert_eq!(
            answer_trust(&other, &probe_dirs()),
            TrustAnswer::Refuse(String::from("/home/user/work/secret"))
        );
    }

    /// The probe always starts in its own folder, so a question that names
    /// no folder is about that folder.
    #[test]
    fn a_question_that_names_no_folder_is_about_the_probe_folder() {
        assert_eq!(
            answer_trust(NO_FOLDER_NAMED, &probe_dirs()),
            TrustAnswer::Confirm
        );
    }

    #[test]
    fn the_probe_folder_written_from_home_or_by_its_real_path_is_its_own() {
        let from_home = MARKER_ON_YES
            .replace(PROBE_DIR, "~/.cache/otty/agent-probe")
            .replace("probe\n", "probe/\n");
        let real_path =
            MARKER_ON_YES.replace(PROBE_DIR, "/data/cache/otty/agent-probe");
        let dirs = vec![
            PathBuf::from(PROBE_DIR),
            PathBuf::from("/data/cache/otty/agent-probe"),
        ];
        let home_only = MARKER_ON_YES.replace(PROBE_DIR, "~/");

        assert_eq!(answer_trust(&from_home, &dirs), TrustAnswer::Confirm);
        assert_eq!(answer_trust(&real_path, &dirs), TrustAnswer::Confirm);
        assert_eq!(
            answer_trust(&home_only, &dirs),
            TrustAnswer::Refuse(String::from("~/"))
        );
    }

    /// A line the user's shell printed before `claude` started can start
    /// with a path too; the folder of the question is still found.
    #[test]
    fn a_path_printed_by_the_shell_does_not_hide_the_probe_folder() {
        let screen = format!(
            "/home/user/.zshrc:12: command not found: nvm\n{MARKER_ON_YES}"
        );

        assert_eq!(answer_trust(&screen, &probe_dirs()), TrustAnswer::Confirm);
    }
}
