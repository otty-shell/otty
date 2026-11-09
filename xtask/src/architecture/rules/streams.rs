//! The stream rule: long-running streams start only in `subscription.rs`.
//! `app.rs` also starts the keyboard stream, because only it may mark keys
//! as input.

use super::super::finding::{Finding, Rule};
use super::super::source::{SourceFile, SourceTree};
use super::super::syntax::{expanded, import_aliases, written_paths};

/// The type whose `run` functions start a stream.
const SUBSCRIPTION: &str = "Subscription";

/// The key of a `Subscription::run` stream, also for `run_with`.
const SUBSCRIPTION_RUN: &str = "Subscription::run";

/// The iced functions that start a stream, by their full path.
const STREAM_FUNCTIONS: &[&[&str]] = &[
    &["iced", "time", "every"],
    &["iced", "time", "repeat"],
    &["iced", "event", "listen"],
    &["iced", "event", "listen_with"],
    &["iced", "event", "listen_raw"],
    &["iced", "keyboard", "listen"],
    &["iced", "window", "events"],
    &["iced", "window", "frames"],
    &["iced", "window", "open_events"],
    &["iced", "window", "close_events"],
    &["iced", "window", "resize_events"],
    &["iced", "window", "close_requests"],
];

/// The file that marks keys as input, and the keyboard stream it starts.
const APP_KEYBOARD: (&str, &str) = ("app.rs", "iced::keyboard::listen");

/// Report streams that start outside `subscription.rs`.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    tree.files()
        .iter()
        .filter(|file| file.name() != "subscription.rs")
        .flat_map(streams)
        .collect()
}

/// The streams that `file` starts, except the keyboard stream of `app.rs`.
/// A path counts with its first segment resolved through the imports, so
/// `time::every` after `use iced::time;` is `iced::time::every`.
fn streams(file: &SourceFile) -> Vec<Finding> {
    let aliases = import_aliases(file.syntax());

    written_paths(file.syntax())
        .iter()
        .filter_map(|path| {
            let key = stream_key(&expanded(path.segments(), &aliases))?;
            if (file.path(), key.as_str()) == APP_KEYBOARD {
                return None;
            }

            Some(Finding::new(
                Rule::StreamOutsideSubscription,
                file.path(),
                &key,
                path.line(),
            ))
        })
        .collect()
}

/// The key of the stream that `segments` start, if they start one.
fn stream_key(segments: &[String]) -> Option<String> {
    let runs_subscription = segments
        .windows(2)
        .any(|pair| pair[0] == SUBSCRIPTION && pair[1].starts_with("run"));
    if runs_subscription {
        return Some(SUBSCRIPTION_RUN.to_string());
    }

    STREAM_FUNCTIONS
        .iter()
        .find(|function| {
            segments.len() >= function.len()
                && segments.iter().zip(function.iter()).all(|(a, b)| a == b)
        })
        .map(|function| function.join("::"))
}

#[cfg(test)]
mod tests {
    use super::super::super::finding::Rule;
    use super::super::test_support::{found, pair};
    use super::*;

    #[test]
    fn streams_start_only_in_subscription_files() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/containers/logs.rs",
                "fn logs() {\n    Subscription::run_with(a, b)\n}\n",
            ),
            (
                "widgets/containers/subscription.rs",
                "fn logs() {\n    Subscription::run_with(a, b)\n}\n",
            ),
            (
                "subscription.rs",
                "fn all() {\n    Subscription::run(stream)\n}\n",
            ),
            (
                "widgets/containers/model.rs",
                "fn text() -> &'static str {\n    \"Subscription::run\"\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::StreamOutsideSubscription),
            [pair("widgets/containers/logs.rs", "Subscription::run")]
        );
    }

    #[test]
    fn iced_streams_are_found_by_their_path_also_through_imports() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/palette/mod.rs",
                "use iced::time;\nfn tick() -> Subscription<PaletteIntent> {\n    \
                 time::every(PERIOD).map(|_| PaletteIntent::Tick)\n}\n",
            ),
            (
                "widgets/tabs/state.rs",
                "use iced::keyboard::listen as keys;\nfn k() {\n    let _ = keys();\n}\n",
            ),
            (
                "widgets/notes/model.rs",
                "fn e() {\n    let _ = iced::event::listen_with(f);\n    \
                 let _ = iced::window::events();\n}\n",
            ),
            (
                "widgets/notes/subscription.rs",
                "fn t() {\n    iced::time::every(P)\n}\n",
            ),
            (
                "update.rs",
                "use iced::{Subscription, time};\nfn f() {\n    \
                 let _ = time::Duration::from_secs(1);\n    \
                 let _ = Subscription::batch(subs);\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::StreamOutsideSubscription),
            [
                pair("widgets/notes/model.rs", "iced::event::listen_with"),
                pair("widgets/notes/model.rs", "iced::window::events"),
                pair("widgets/palette/mod.rs", "iced::time::every"),
                pair("widgets/tabs/state.rs", "iced::keyboard::listen"),
            ]
        );
    }

    #[test]
    fn app_listens_to_the_keyboard_but_starts_no_other_stream() {
        let tree = SourceTree::from_sources(&[
            (
                "app.rs",
                "fn subscription() {\n    Subscription::batch([\n        \
                 iced::keyboard::listen().map(Message::Input),\n        \
                 iced::time::every(P).map(Message::Internal),\n    ])\n}\n",
            ),
            (
                "view.rs",
                "use iced::keyboard;\nfn f() {\n    keyboard::listen();\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::StreamOutsideSubscription),
            [
                pair("app.rs", "iced::time::every"),
                pair("view.rs", "iced::keyboard::listen"),
            ]
        );
    }
}
