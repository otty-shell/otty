//! The reading of Claude Code's `/usage` panel from the text of its screen.
//!
//! The reading follows CodexBar's parser (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeStatusProbe.swift>:
//! the panel is the text after the last "Settings:" header and before the
//! "What's contributing to your limits usage?" hints; each "Current
//! session" or "Current week" block has a line with a percent and the word
//! "used" or "left", and a line that starts with "Resets".

use crate::billing::{Billing, LimitWindow, UsageWindows};
use crate::error::SourceError;
use crate::limits::Limits;

/// The header of the panel: the panel on screen is the text after the
/// last one.
const PANEL_HEADER: &str = "settings:";
/// The hints under the windows, which can name a window too.
const HINTS_HEADER: &str = "what's contributing to your limits usage?";
/// How many lines under a block's name its percent may be.
const PERCENT_LINES: usize = 12;
/// How many lines under a block's name its reset text may be.
const RESET_LINES: usize = 14;
/// Words of a percent that is used.
const USED_WORDS: [&str; 3] = ["used", "spent", "consumed"];
/// Words of a percent that is left.
const LEFT_WORDS: [&str; 3] = ["left", "remaining", "available"];
/// Texts of a panel that is still loading its numbers.
const LOADING_TEXTS: [&str; 3] =
    ["loading usage", "refreshing…", "refreshing..."];
/// Texts of a login that is missing or expired.
const LOGIN_TEXTS: [&str; 7] = [
    "run /login",
    "not logged in",
    "not authenticated",
    "authentication_error",
    "token_expired",
    "token has expired",
    "select login method",
];
/// Texts of a question Claude Code asks before its prompt, with what it
/// asks.
const QUESTIONS: [(&str, &str); 7] = [
    (
        "do you trust the files in this folder",
        "whether to trust its folder",
    ),
    ("yes, i trust this folder", "whether to trust its folder"),
    ("quick safety check", "whether to trust its folder"),
    ("ready to code here?", "whether to trust its folder"),
    (
        "do you want to use this api key",
        "whether to use an API key",
    ),
    ("choose the text style", "to choose its look"),
    ("press enter to continue", "to press Enter"),
];
/// Texts of usage requests refused for asking too often.
const RATE_LIMIT_TEXTS: [&str; 2] = ["rate_limit_error", "rate limited"];

/// What the screen shows, for the source that waits for the panel.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum UsageScreen {
    /// The panel with at least one window.
    Usage(Limits),
    /// Nothing to read yet: Claude Code is starting or loading the usage.
    Waiting,
    /// The panel is on the screen, but its numbers can not be read. The
    /// text says what is missing.
    Unreadable(String),
    /// Claude Code stopped at something OTTY must not answer for the user,
    /// such as a login or a question.
    Stopped(SourceError),
}

/// A block of the panel: its window and the line its name is on.
struct Block {
    name: String,
    line: usize,
}

/// What the screen `text` shows.
///
/// The windows of the newest panel are read first, and only a panel that
/// has finished loading is an answer. With no window, a login or a
/// question on the screen stops the reading; a panel with no numbers is
/// unreadable; anything else is waited for.
pub(super) fn read_usage_screen(text: &str) -> UsageScreen {
    let lower = text.to_lowercase();
    let is_loading =
        LOADING_TEXTS.iter().any(|loading| lower.contains(loading));

    let lines: Vec<&str> = newest_panel(text).lines().collect();
    let blocks = blocks(&lines);
    let windows = windows(&lines, &blocks);

    if !windows.is_empty() {
        if is_loading {
            return UsageScreen::Waiting;
        }
        let usage = UsageWindows::new(windows, None);
        return UsageScreen::Usage(Limits::new(
            Billing::UsageWindows(usage),
            None,
        ));
    }

    if let Some(error) = stop_reason(&lower) {
        return UsageScreen::Stopped(error);
    }

    if blocks.is_empty() || is_loading {
        return UsageScreen::Waiting;
    }

    UsageScreen::Unreadable(String::from(
        "the usage panel shows no percent used or left",
    ))
}

/// The newest panel: the text after the last panel header, cut before the
/// hints. A screen with no header is read whole.
fn newest_panel(text: &str) -> &str {
    let lower = text.to_lowercase();

    // Lowercasing can change the length of some letters, so the places
    // found in `lower` are used only when the lengths still agree.
    if lower.len() != text.len() {
        return text;
    }

    let start = lower.rfind(PANEL_HEADER).unwrap_or(0);
    let end = lower[start..]
        .find(HINTS_HEADER)
        .map_or(text.len(), |offset| start + offset);

    &text[start..end]
}

/// The blocks of `lines`: a line that starts with "Current session" or
/// "Current week", in the order they come. A window named twice is read
/// once.
fn blocks(lines: &[&str]) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();

    for (line, text) in lines.iter().enumerate() {
        let Some(name) = window_name(text) else {
            continue;
        };
        if blocks.iter().any(|block| block.name == name) {
            continue;
        }

        blocks.push(Block { name, line });
    }

    blocks
}

/// The windows of `blocks` that show a percent.
fn windows(lines: &[&str], blocks: &[Block]) -> Vec<LimitWindow> {
    blocks
        .iter()
        .filter_map(|block| {
            let below = block_lines(lines, block.line);
            let percent_used = below
                .iter()
                .take(PERCENT_LINES)
                .find_map(|line| percent_used(line))?;
            let window = LimitWindow::new(&block.name, percent_used, None);

            let reset =
                below.iter().take(RESET_LINES).find(|line| is_reset(line));
            Some(match reset {
                Some(reset) => window.with_reset_text(reset.trim()),
                None => window,
            })
        })
        .collect()
}

/// The lines under the name of a block on line `line`, up to the name of
/// the next block.
fn block_lines<'a>(lines: &'a [&'a str], line: usize) -> &'a [&'a str] {
    let below = lines.get(line + 1..).unwrap_or_default();
    let end = below
        .iter()
        .position(|text| window_name(text).is_some())
        .unwrap_or(below.len());

    &below[..end]
}

/// The window a line names, as the usage request names it: "Current
/// session" is "5 hours", "Current week" and "Current week (all models)"
/// are "7 days", and "Current week (Sonnet only)" is "7 days · Sonnet".
fn window_name(line: &str) -> Option<String> {
    let line = line.trim().to_lowercase();

    if line.starts_with("current session") {
        return Some(String::from("5 hours"));
    }
    let rest = line.strip_prefix("current week")?.trim();

    let model = rest
        .strip_prefix('(')
        .and_then(|inside| inside.split(')').next())
        .map(|model| model.trim().trim_end_matches(" only").trim());
    match model {
        Some(model) if !model.is_empty() && model != "all models" => {
            Some(format!("7 days · {}", capitalized(model)))
        },
        _ => Some(String::from("7 days")),
    }
}

/// The percent used that `line` shows: a number before `%`, with a word
/// that says whether it is used or left. A line with neither word, or a
/// line of the status bar ("Opus 4 | 12% context"), shows none.
fn percent_used(line: &str) -> Option<f64> {
    let lower = line.to_lowercase();
    if lower.contains('|') {
        return None;
    }

    let percent = percent_before_sign(&lower)?.clamp(0.0, 100.0);

    if USED_WORDS.iter().any(|word| lower.contains(word)) {
        return Some(percent);
    }
    if LEFT_WORDS.iter().any(|word| lower.contains(word)) {
        return Some(100.0 - percent);
    }
    None
}

/// The first number of one to three digits, with an optional fraction,
/// right before a `%` sign, spaces allowed between them.
fn percent_before_sign(line: &str) -> Option<f64> {
    line.match_indices('%').find_map(|(sign, _)| {
        let before = line[..sign].trim_end();
        let start = before
            .rfind(|c: char| !(c.is_ascii_digit() || c == '.'))
            .map_or(0, |index| index + 1);
        let number = &before[start..];

        let whole = number.split('.').next().unwrap_or_default();
        if whole.is_empty() || whole.len() > 3 {
            return None;
        }
        number.parse::<f64>().ok()
    })
}

/// Whether `line` tells when the window resets: it has the word "reset" or
/// "resets".
fn is_reset(line: &str) -> bool {
    line.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| word == "reset" || word == "resets")
}

/// Why the screen ends the reading, when it shows a login, a question or
/// a refusal instead of the panel. `lower` is the screen in lower case.
fn stop_reason(lower: &str) -> Option<SourceError> {
    if LOGIN_TEXTS.iter().any(|login| lower.contains(login)) {
        return Some(SourceError::NotLoggedIn);
    }

    if let Some((_, asks)) = QUESTIONS
        .iter()
        .find(|(question, _)| lower.contains(question))
    {
        return Some(SourceError::NotAvailable(format!("claude asks {asks}")));
    }

    if RATE_LIMIT_TEXTS.iter().any(|limit| lower.contains(limit)) {
        return Some(SourceError::NotAvailable(String::from(
            "claude says the usage was asked too often",
        )));
    }

    None
}

/// `word` with its first letter in upper case.
fn capitalized(word: &str) -> String {
    let mut letters = word.chars();

    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOTH_BLOCKS_USED: &str = "\
╭───────────────────────────────────────────────╮
│ ✻ Welcome to Claude Code!                     │
╰───────────────────────────────────────────────╯

> /usage
───────────────────────────────────────────────────────────────────
 Settings:  Status   Config   Usage  (←/→ or tab to cycle)

 Current session
 ██████████████▌                                    29% used
 Resets 5pm (Europe/Berlin)

 Current week (all models)
 ████████▌                                          17% used
 Resets Nov 20, 10am (Europe/Berlin)

 Esc to exit
";

    const BOTH_BLOCKS_LEFT: &str = "\
 Settings:  Status   Config   Usage

 Current session
 71% left
 Resets in 2h 15m

 Current week (all models)
 83.5 % left
 Resets Nov 20 at 10am
";

    const ONLY_WEEK: &str = "\
 Settings:  Status   Config   Usage

 Current week (all models)
 █████                                              10% used
 Resets Nov 20, 10am (Europe/Berlin)
";

    const MODEL_WEEKS_AND_HINTS: &str = "\
 Settings:  Status   Config   Usage

 Current session
 █                                                  2% used
 Resets 5pm (Europe/Berlin)

 Current week (all models)
 ████                                               8% used
 Resets Nov 20, 10am (Europe/Berlin)

 Current week (Opus)
                                                    0% used
 Resets Nov 20, 10am (Europe/Berlin)

 Current week (Sonnet only)
 ██                                                 4% used
 Resets Nov 20, 10am (Europe/Berlin)

 Extra usage
 Extra usage not enabled · /extra-usage to enable

 What's contributing to your limits usage?
 Current session: 90% used by long conversations
 Resets are shown in your time zone

 Esc to exit
";

    const LOGIN: &str = "\
╭───────────────────────────────────────────────╮
│ ✻ Welcome to Claude Code!                     │
╰───────────────────────────────────────────────╯

 Invalid API key · Please run /login
";

    const FIRST_LOGIN: &str = "\
 Claude Code can be used with your Claude subscription or billed based on
 API usage through your Console account.

 Select login method:

 ❯ 1. Claude account with subscription
   2. Anthropic Console account
";

    const TRUST: &str = "\
 Do you trust the files in this folder?

 /home/user/.cache/otty/agent-probe

 ❯ 1. Yes, proceed
   2. No, exit
";

    const LOADING: &str = "\
 Settings:  Status   Config   Usage

 Loading usage data…
";

    const REFRESHING: &str = "\
 Settings:  Status   Config   Usage

 Current session
 ██████████████▌                                    29% used
 Resets 5pm (Europe/Berlin)

 Refreshing… esc to cancel
";

    const UNKNOWN_LAYOUT: &str = "\
 Settings:  Status   Config   Usage

 Current session
 ██████████████▌                                    about a third
 Resets 5pm (Europe/Berlin)
";

    const PROMPT_ONLY: &str = "\
╭───────────────────────────────────────────────╮
│ ✻ Welcome to Claude Code!                     │
╰───────────────────────────────────────────────╯

> Try \"how does <filepath> work?\"
  ? for shortcuts
";

    fn windows(screen: UsageScreen) -> Vec<LimitWindow> {
        match screen {
            UsageScreen::Usage(limits) => match limits.billing() {
                Billing::UsageWindows(usage) => usage.windows().to_vec(),
                other => panic!("usage windows expected, got {other:?}"),
            },
            other => panic!("the panel expected, got {other:?}"),
        }
    }

    fn window(name: &str, percent_used: f64, reset: &str) -> LimitWindow {
        LimitWindow::new(name, percent_used, None).with_reset_text(reset)
    }

    #[test]
    fn both_blocks_with_percent_used_are_read() {
        let screen = read_usage_screen(BOTH_BLOCKS_USED);

        assert_eq!(
            windows(screen),
            [
                window("5 hours", 29.0, "Resets 5pm (Europe/Berlin)"),
                window("7 days", 17.0, "Resets Nov 20, 10am (Europe/Berlin)"),
            ]
        );
    }

    #[test]
    fn percent_left_is_turned_into_percent_used() {
        let screen = read_usage_screen(BOTH_BLOCKS_LEFT);

        assert_eq!(
            windows(screen),
            [
                window("5 hours", 29.0, "Resets in 2h 15m"),
                window("7 days", 16.5, "Resets Nov 20 at 10am"),
            ]
        );
    }

    #[test]
    fn a_panel_with_only_the_week_gives_one_window() {
        let screen = read_usage_screen(ONLY_WEEK);

        assert_eq!(
            windows(screen),
            [window(
                "7 days",
                10.0,
                "Resets Nov 20, 10am (Europe/Berlin)"
            )]
        );
    }

    #[test]
    fn model_weeks_are_read_and_the_blocks_and_hints_around_are_skipped() {
        let screen = read_usage_screen(MODEL_WEEKS_AND_HINTS);

        let reset = "Resets Nov 20, 10am (Europe/Berlin)";
        assert_eq!(
            windows(screen),
            [
                window("5 hours", 2.0, "Resets 5pm (Europe/Berlin)"),
                window("7 days", 8.0, reset),
                window("7 days · Opus", 0.0, reset),
                window("7 days · Sonnet", 4.0, reset),
            ]
        );
    }

    #[test]
    fn the_panel_gives_no_plan() {
        let UsageScreen::Usage(limits) = read_usage_screen(BOTH_BLOCKS_USED)
        else {
            panic!("the panel expected");
        };

        assert_eq!(limits.plan(), None);
        assert_eq!(limits.recorded_at(), None);
    }

    #[test]
    fn a_login_screen_is_not_logged_in() {
        assert_eq!(
            read_usage_screen(LOGIN),
            UsageScreen::Stopped(SourceError::NotLoggedIn)
        );
        assert_eq!(
            read_usage_screen(FIRST_LOGIN),
            UsageScreen::Stopped(SourceError::NotLoggedIn)
        );
    }

    #[test]
    fn a_question_is_never_answered_and_is_not_available() {
        let screen = read_usage_screen(TRUST);

        assert_eq!(
            screen,
            UsageScreen::Stopped(SourceError::NotAvailable(String::from(
                "claude asks whether to trust its folder"
            )))
        );
    }

    #[test]
    fn a_panel_that_is_loading_is_waited_for() {
        assert_eq!(read_usage_screen(LOADING), UsageScreen::Waiting);
        assert_eq!(read_usage_screen(REFRESHING), UsageScreen::Waiting);
    }

    #[test]
    fn a_screen_with_no_panel_yet_is_waited_for() {
        assert_eq!(read_usage_screen(PROMPT_ONLY), UsageScreen::Waiting);
        assert_eq!(read_usage_screen(""), UsageScreen::Waiting);
    }

    #[test]
    fn a_panel_in_an_unknown_layout_is_unreadable() {
        let screen = read_usage_screen(UNKNOWN_LAYOUT);

        assert_eq!(
            screen,
            UsageScreen::Unreadable(String::from(
                "the usage panel shows no percent used or left"
            ))
        );
    }

    #[test]
    fn only_the_newest_panel_on_the_screen_is_read() {
        let screen = format!(
            "{}\n{}",
            BOTH_BLOCKS_USED,
            ONLY_WEEK.replace("10% used", "12% used")
        );

        assert_eq!(
            windows(read_usage_screen(&screen)),
            [window(
                "7 days",
                12.0,
                "Resets Nov 20, 10am (Europe/Berlin)"
            )]
        );
    }
}
