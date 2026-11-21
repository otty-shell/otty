//! The reading of Claude Code's `/usage` panel from the text of its screen.
//!
//! The reading follows CodexBar's parser (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeStatusProbe.swift>:
//! the panel is the text after the last "Settings:" header and before the
//! "What's contributing to your limits usage?" hints; each "Current
//! session" or "Current week" block has a line with a percent and the word
//! "used" or "left", and a line that starts with "Resets".

use otty_plugin_sdk::agents::{LimitWindow, ReadError};

/// The header of the panel: the panel on screen is the text after the
/// last one.
const PANEL_HEADER: &str = "settings:";
/// The hints under the windows, which can name a window too.
const HINTS_HEADER: &str = "what's contributing to your limits usage?";
/// The labels of a weekly block, with and without the space.
const WEEK_LABELS: [&str; 2] = ["current week", "currentweek"];
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
/// Texts of a question Claude Code asks before its prompt, other than
/// the trust of its folder, with what OTTY tells about it.
const QUESTIONS: [(&str, &str); 3] = [
    (
        "do you want to use this api key",
        "Claude Code asks whether to use an API key",
    ),
    (
        "choose the text style",
        "Claude Code asks to choose the text style",
    ),
    ("press enter to continue", "Claude Code asks to press Enter"),
];
/// Texts of the question whether to trust the folder Claude Code runs in.
const TRUST_TEXTS: [&str; 4] = [
    "do you trust the files in this folder",
    "yes, i trust this folder",
    "quick safety check",
    "ready to code here?",
];
/// Texts of usage requests refused for asking too often.
const RATE_LIMIT_TEXTS: [&str; 2] = ["rate_limit_error", "rate limited"];
/// The length of the five-hour window, in minutes.
const FIVE_HOURS: u32 = 5 * 60;
/// The length of the weekly window, in minutes.
const WEEK: u32 = 7 * 24 * 60;

/// What the screen shows, for the probe that waits for the panel.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UsageScreen {
    /// The panel with at least one window.
    Usage(Vec<LimitWindow>),
    /// Nothing to read yet: Claude Code is starting or loading the usage.
    Waiting,
    /// The panel is on the screen, but it shows no percent used or left.
    Unreadable,
    /// Claude Code asks to log in: the login is missing or expired.
    LoginAsked,
    /// Claude Code stopped at something OTTY must not answer for the user,
    /// such as a question.
    Stopped(ReadError),
    /// Claude Code asks whether to trust the folder it runs in, and
    /// nothing else. Only the probe knows whether it is its own folder.
    TrustQuestion,
}

/// A block of the panel: its window and the line its name is on.
struct Block {
    window: Window,
    line: usize,
}

/// A usage window as the panel names it: its length and, for the week of
/// one model, the model.
#[derive(Debug, Clone, PartialEq)]
struct Window {
    length_minutes: u32,
    model: Option<String>,
}

impl Window {
    /// The five-hour window.
    fn five_hours() -> Self {
        Self {
            length_minutes: FIVE_HOURS,
            model: None,
        }
    }

    /// The weekly window of all models.
    fn week() -> Self {
        Self {
            length_minutes: WEEK,
            model: None,
        }
    }

    /// The weekly window of one model, such as the week of "Fable" for
    /// `fable`: the model with its first letter in upper case.
    fn model_week(model: &str) -> Self {
        let mut letters = model.trim().chars();
        let model: String = match letters.next() {
            Some(first) => first.to_uppercase().chain(letters).collect(),
            None => String::new(),
        };

        Self {
            length_minutes: WEEK,
            model: Some(model),
        }
    }

    /// The limit window of this window with `percent_used`.
    fn limit(&self, percent_used: f64) -> LimitWindow {
        LimitWindow {
            length_minutes: Some(self.length_minutes),
            model: self.model.clone(),
            percent_used,
            resets_at_ms: None,
            reset_text: None,
        }
    }
}

/// What the screen `text` shows.
///
/// The windows of the newest panel are read first, and only a panel that
/// has finished loading is an answer. With no window, a login or a
/// question on the screen stops the reading, except the question whether
/// to trust the folder, which is named when nothing else stops; a panel
/// with no numbers is unreadable; anything else is waited for.
pub(crate) fn read_usage_screen(text: &str) -> UsageScreen {
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
        return UsageScreen::Usage(windows);
    }

    if LOGIN_TEXTS.iter().any(|login| lower.contains(login)) {
        return UsageScreen::LoginAsked;
    }
    if let Some(error) = stop_reason(&lower) {
        return UsageScreen::Stopped(error);
    }
    if TRUST_TEXTS.iter().any(|trust| lower.contains(trust)) {
        return UsageScreen::TrustQuestion;
    }

    if blocks.is_empty() || is_loading {
        return UsageScreen::Waiting;
    }

    UsageScreen::Unreadable
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
        let Some(window) = window_of(text) else {
            continue;
        };
        if blocks.iter().any(|block| block.window == window) {
            continue;
        }

        blocks.push(Block { window, line });
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
            let mut window = block.window.limit(percent_used);

            window.reset_text = below
                .iter()
                .take(RESET_LINES)
                .find(|line| is_reset(line))
                .map(|reset| reset.trim().to_string());
            Some(window)
        })
        .collect()
}

/// The lines under the name of a block on line `line`, up to the name of
/// the next block.
fn block_lines<'a>(lines: &'a [&'a str], line: usize) -> &'a [&'a str] {
    let below = lines.get(line + 1..).unwrap_or_default();
    let end = below
        .iter()
        .position(|text| window_of(text).is_some())
        .unwrap_or(below.len());

    &below[..end]
}

/// The window a line names, as the usage request tells it: "Current
/// session" is the five-hour window, "Current week" and "Current week (all
/// models)" the weekly one, and "Current week (Sonnet only)" the week of
/// Sonnet. The model keeps the letters the screen shows; the label may also
/// be written without spaces, "Currentweek(Fable)", as CodexBar reads it.
fn window_of(line: &str) -> Option<Window> {
    let line = line.trim();

    if line.to_lowercase().starts_with("current session") {
        return Some(Window::five_hours());
    }
    let rest = WEEK_LABELS
        .iter()
        .find_map(|label| strip_prefix_ignoring_case(line, label))?
        .trim();

    let model = rest
        .strip_prefix('(')
        .and_then(|inside| inside.split(')').next())
        .map(|model| model.trim())
        .map(|model| {
            strip_suffix_ignoring_case(model, " only").unwrap_or(model)
        })
        .map(str::trim);
    match model {
        Some(model)
            if !model.is_empty()
                && !model.eq_ignore_ascii_case("all models") =>
        {
            Some(Window::model_week(model))
        },
        _ => Some(Window::week()),
    }
}

/// `text` after `prefix`, when it starts with it in any case.
fn strip_prefix_ignoring_case<'a>(
    text: &'a str,
    prefix: &str,
) -> Option<&'a str> {
    let start = text.get(..prefix.len())?;

    start
        .eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

/// `text` before `suffix`, when it ends with it in any case.
fn strip_suffix_ignoring_case<'a>(
    text: &'a str,
    suffix: &str,
) -> Option<&'a str> {
    let start = text.len().checked_sub(suffix.len())?;
    let end = text.get(start..)?;

    end.eq_ignore_ascii_case(suffix).then(|| &text[..start])
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

/// Why the screen ends the reading, when it shows a question other than
/// the trust of the folder, or a refusal instead of the panel. `lower` is
/// the screen in lower case. The reason names what Claude Code asks, never
/// what the screen shows.
fn stop_reason(lower: &str) -> Option<ReadError> {
    if let Some((_, asks)) = QUESTIONS
        .iter()
        .find(|(question, _)| lower.contains(question))
    {
        return Some(ReadError::Failed((*asks).to_string()));
    }

    if RATE_LIMIT_TEXTS.iter().any(|limit| lower.contains(limit)) {
        return Some(ReadError::Failed(
            "Claude Code was refused its usage for asking too often"
                .to_string(),
        ));
    }

    None
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

    const THREE_MODEL_WEEKS: &str = "\
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

 Current week (Fable)
 ██████████████████                                 64% used
 Resets Nov 21, 9am (Europe/Berlin)

 Current week (Example Model only)
 █                                                  3% used
 Resets Nov 20, 10am (Europe/Berlin)

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

 /home/dev/.cache/otty/agent-probe

 ❯ 1. Yes, proceed
   2. No, exit
";

    const QUICK_SAFETY_CHECK: &str = "\
 Accessing workspace:

 /home/dev/.cache/otty/agent-probe

 Quick safety check: Is this a project you created or one you trust?

 ❯ 1. Yes, I trust this folder
   2. No, exit

 Enter to confirm · Esc to cancel
";

    const API_KEY_QUESTION: &str = "\
 Detected a custom API key in your environment

 ANTHROPIC_API_KEY: sk-ant-made-up

 Do you want to use this API key?

 ❯ 1. Yes
   2. No (recommended)
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

    const SESSION: Option<u32> = Some(FIVE_HOURS);
    const WEEKLY: Option<u32> = Some(WEEK);
    const RESET: &str = "Resets Nov 20, 10am (Europe/Berlin)";

    fn windows(screen: UsageScreen) -> Vec<LimitWindow> {
        match screen {
            UsageScreen::Usage(windows) => windows,
            other => panic!("the panel expected, got {other:?}"),
        }
    }

    fn window(
        length_minutes: Option<u32>,
        model: Option<&str>,
        percent_used: f64,
        reset: Option<&str>,
    ) -> LimitWindow {
        LimitWindow {
            length_minutes,
            model: model.map(str::to_string),
            percent_used,
            resets_at_ms: None,
            reset_text: reset.map(str::to_string),
        }
    }

    fn failed(text: &str) -> UsageScreen {
        UsageScreen::Stopped(ReadError::Failed(text.to_string()))
    }

    #[test]
    fn both_blocks_with_percent_used_are_read() {
        let screen = read_usage_screen(BOTH_BLOCKS_USED);

        assert_eq!(
            windows(screen),
            [
                window(SESSION, None, 29.0, Some("Resets 5pm (Europe/Berlin)")),
                window(WEEKLY, None, 17.0, Some(RESET)),
            ]
        );
    }

    #[test]
    fn percent_left_is_turned_into_percent_used() {
        let screen = read_usage_screen(BOTH_BLOCKS_LEFT);

        assert_eq!(
            windows(screen),
            [
                window(SESSION, None, 29.0, Some("Resets in 2h 15m")),
                window(WEEKLY, None, 16.5, Some("Resets Nov 20 at 10am")),
            ]
        );
    }

    #[test]
    fn a_panel_with_only_the_week_gives_one_window() {
        let screen = read_usage_screen(ONLY_WEEK);

        assert_eq!(windows(screen), [window(WEEKLY, None, 10.0, Some(RESET))]);
    }

    #[test]
    fn model_weeks_are_read_and_the_blocks_and_hints_around_are_skipped() {
        let screen = read_usage_screen(MODEL_WEEKS_AND_HINTS);

        assert_eq!(
            windows(screen),
            [
                window(SESSION, None, 2.0, Some("Resets 5pm (Europe/Berlin)")),
                window(WEEKLY, None, 8.0, Some(RESET)),
                window(WEEKLY, Some("Opus"), 0.0, Some(RESET)),
                window(WEEKLY, Some("Sonnet"), 4.0, Some(RESET)),
            ]
        );
    }

    /// Owner, issue 27 of the earlier spec: every "Current week (<model>)"
    /// block is a window, named like the usage request names it.
    #[test]
    fn every_model_week_is_read_with_the_model_name_as_shown() {
        let screen = read_usage_screen(THREE_MODEL_WEEKS);

        assert_eq!(
            windows(screen),
            [
                window(SESSION, None, 2.0, Some("Resets 5pm (Europe/Berlin)")),
                window(WEEKLY, None, 8.0, Some(RESET)),
                window(WEEKLY, Some("Opus"), 0.0, Some(RESET)),
                window(
                    WEEKLY,
                    Some("Fable"),
                    64.0,
                    Some("Resets Nov 21, 9am (Europe/Berlin)")
                ),
                window(WEEKLY, Some("Example Model"), 3.0, Some(RESET)),
            ]
        );
    }

    #[test]
    fn a_week_block_written_without_spaces_is_read() {
        let screen = read_usage_screen(
            " Settings:  Usage\n\n Currentweek(Fable)\n 64% used\n",
        );

        assert_eq!(
            windows(screen),
            [window(WEEKLY, Some("Fable"), 64.0, None)]
        );
    }

    #[test]
    fn a_model_week_is_named_with_a_capital_letter() {
        assert_eq!(Window::model_week("opus").model.as_deref(), Some("Opus"));
        assert_eq!(
            Window::model_week(" Fable ").model.as_deref(),
            Some("Fable")
        );
    }

    #[test]
    fn a_login_screen_asks_to_log_in() {
        assert_eq!(read_usage_screen(LOGIN), UsageScreen::LoginAsked);
        assert_eq!(read_usage_screen(FIRST_LOGIN), UsageScreen::LoginAsked);
    }

    /// Owner, issue 29 of the earlier spec: the probe answers the trust
    /// question of its own folder, so the reading names it instead of
    /// stopping.
    #[test]
    fn the_question_whether_to_trust_the_folder_is_named() {
        assert_eq!(read_usage_screen(TRUST), UsageScreen::TrustQuestion);
        assert_eq!(
            read_usage_screen(QUICK_SAFETY_CHECK),
            UsageScreen::TrustQuestion
        );
    }

    /// The reason names the question, never the key on the screen.
    #[test]
    fn another_question_is_never_answered_and_names_no_value() {
        let screen = read_usage_screen(API_KEY_QUESTION);

        assert_eq!(
            screen,
            failed("Claude Code asks whether to use an API key")
        );
        assert!(!format!("{screen:?}").contains("sk-ant"));
    }

    #[test]
    fn a_refusal_for_asking_too_often_stops_the_reading() {
        let screen = read_usage_screen(" Error: rate_limit_error\n");

        assert_eq!(
            screen,
            failed("Claude Code was refused its usage for asking too often")
        );
    }

    #[test]
    fn a_login_next_to_the_trust_question_asks_to_log_in() {
        let screen = format!("{FIRST_LOGIN}\n{TRUST}");

        assert_eq!(read_usage_screen(&screen), UsageScreen::LoginAsked);
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

        assert_eq!(screen, UsageScreen::Unreadable);
    }

    #[test]
    fn a_status_bar_percent_is_no_window() {
        let screen = read_usage_screen(
            " Settings:  Usage\n\n Current session\n Opus 4 | 12% context\n",
        );

        assert_eq!(screen, UsageScreen::Unreadable);
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
            [window(WEEKLY, None, 12.0, Some(RESET))]
        );
    }
}
