//! Rules about what a file of a given role may contain.

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    Layer, STANDARD_WIDGET_FILES, is_view_file, layer_of,
};
use super::super::source::{SourceFile, SourceTree};
use super::super::text::{find_word, is_ident_char};

/// The first line of every reducer.
const FORBID_LINE: &str = "#![forbid(clippy::disallowed_methods)]";

/// The marks `app.rs` puts on events.
const MESSAGE_MARKS: &[&str] = &["Message::Input", "Message::Internal"];

/// Name endings of the types an `event.rs` may define.
const EVENT_TYPE_ENDINGS: &[&str] = &["Intent", "Effect", "Event"];

/// Report contents that do not fit the file's role.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in tree.files() {
        findings.extend(streams_outside_subscription(file));
        findings.extend(message_marks_outside_app(file));

        let Some(Layer::Widget(widget)) = layer_of(file.path()) else {
            continue;
        };
        if is_pure_file(file, &widget) {
            findings.extend(task_or_element(file));
        }
        match file.name() {
            "reducer.rs" => findings.extend(reducer_without_forbid(file)),
            "event.rs" => findings.extend(foreign_event_types(file)),
            "model.rs" => findings.extend(model_mut_state(file)),
            _ => {},
        }
        if file.name() != "reducer.rs" {
            findings.extend(ctx_outside_reducer(file));
        }
    }

    findings
}

/// `state.rs` and topic files: plain logic, without `Task` and `Element`.
fn is_pure_file(file: &SourceFile, widget: &str) -> bool {
    if is_view_file(file.path(), widget) {
        return false;
    }

    file.name() == "state.rs" || !STANDARD_WIDGET_FILES.contains(&file.name())
}

fn task_or_element(file: &SourceFile) -> Vec<Finding> {
    ["Task", "Element"]
        .iter()
        .filter_map(|word| {
            let line = file
                .lines()
                .iter()
                .find(|line| find_word(line.text(), word).is_some())?;

            Some(Finding::new(
                Rule::TaskOrElementInPureFile,
                file.path(),
                word,
                line.number(),
            ))
        })
        .collect()
}

fn reducer_without_forbid(file: &SourceFile) -> Option<Finding> {
    let first = file.lines().first();
    if first.is_some_and(|line| line.text().trim() == FORBID_LINE) {
        return None;
    }

    let line = first.map_or(0, |line| line.number());
    Some(Finding::new(
        Rule::ReducerWithoutForbid,
        file.path(),
        "forbid",
        line,
    ))
}

fn streams_outside_subscription(file: &SourceFile) -> Option<Finding> {
    if file.name() == "subscription.rs" {
        return None;
    }

    let line = file
        .lines()
        .iter()
        .find(|line| line.text().contains("Subscription::run"))?;

    Some(Finding::new(
        Rule::StreamOutsideSubscription,
        file.path(),
        "Subscription::run",
        line.number(),
    ))
}

fn message_marks_outside_app(file: &SourceFile) -> Vec<Finding> {
    if file.path() == "app.rs" {
        return Vec::new();
    }

    MESSAGE_MARKS
        .iter()
        .filter_map(|mark| {
            let line = file
                .lines()
                .iter()
                .find(|line| find_word(line.text(), mark).is_some())?;

            Some(Finding::new(
                Rule::MessageMarkOutsideApp,
                file.path(),
                mark,
                line.number(),
            ))
        })
        .collect()
}

fn ctx_outside_reducer(file: &SourceFile) -> Vec<Finding> {
    file.lines()
        .iter()
        .filter_map(|line| {
            let name = name_after(line.text(), "struct")?;
            name.ends_with("Ctx").then(|| {
                Finding::new(
                    Rule::CtxOutsideReducer,
                    file.path(),
                    name,
                    line.number(),
                )
            })
        })
        .collect()
}

fn foreign_event_types(file: &SourceFile) -> Vec<Finding> {
    file.lines()
        .iter()
        .filter_map(|line| {
            let name = ["enum", "struct", "type"]
                .iter()
                .find_map(|keyword| name_after(line.text(), keyword))?;
            let is_event_type = EVENT_TYPE_ENDINGS
                .iter()
                .any(|ending| name.ends_with(ending));

            (!is_event_type).then(|| {
                Finding::new(
                    Rule::ForeignTypeInEventFile,
                    file.path(),
                    name,
                    line.number(),
                )
            })
        })
        .collect()
}

fn model_mut_state(file: &SourceFile) -> Vec<Finding> {
    file.lines()
        .iter()
        .filter_map(|line| {
            let name = name_after(line.text(), "&mut")?;
            name.ends_with("State").then(|| {
                Finding::new(
                    Rule::ModelTakesMutState,
                    file.path(),
                    name,
                    line.number(),
                )
            })
        })
        .collect()
}

/// The identifier right after the whole word `keyword` in `text`, e.g.
/// `NotesCtx` in `pub(crate) struct NotesCtx<'a> {`.
fn name_after<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let start = if keyword.starts_with('&') {
        text.find(keyword)?
    } else {
        find_word(text, keyword)?
    };
    let rest = text[start + keyword.len()..].trim_start();
    let length = rest
        .find(|ch: char| !is_ident_char(ch))
        .unwrap_or(rest.len());

    (length > 0).then(|| &rest[..length])
}

#[cfg(test)]
mod tests {
    use super::super::super::finding::Rule;
    use super::super::test_support::{found, pair};
    use super::*;

    #[test]
    fn task_and_element_stay_out_of_state_and_topic_files() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/notes/state.rs",
                "fn a() -> Task<X> {}\nlet h: task::Handle;\n",
            ),
            ("widgets/notes/sort.rs", "fn b() -> Element<'_, X> {}\n"),
            ("widgets/notes/reducer.rs", "fn reduce() -> Task<X> {}\n"),
            (
                "widgets/notes/view/panel.rs",
                "fn view() -> Element<X> {}\n",
            ),
            ("widgets/notes/mod.rs", "use iced::Task;\n"),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::TaskOrElementInPureFile),
            [
                pair("widgets/notes/sort.rs", "Element"),
                pair("widgets/notes/state.rs", "Task"),
            ]
        );
    }

    #[test]
    fn a_reducer_must_start_with_the_forbid_line() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/notes/reducer.rs",
                "//! Doc.\n#![forbid(clippy::disallowed_methods)]\n\nuse iced::Task;\n",
            ),
            ("widgets/tabs/reducer.rs", "use iced::Task;\n"),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ReducerWithoutForbid),
            [pair("widgets/tabs/reducer.rs", "forbid")]
        );
    }

    #[test]
    fn streams_start_only_in_subscription_files() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/containers/logs.rs",
                "Subscription::run_with(a, b)\n",
            ),
            (
                "widgets/containers/subscription.rs",
                "Subscription::run_with(a, b)\n",
            ),
            ("subscription.rs", "Subscription::run(stream)\n"),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::StreamOutsideSubscription),
            [pair("widgets/containers/logs.rs", "Subscription::run")]
        );
    }

    #[test]
    fn only_app_marks_messages() {
        let tree = SourceTree::from_sources(&[
            (
                "app.rs",
                "Message::Input(event) => x,\nMessage::Internal(e) => y,\n",
            ),
            ("update.rs", "let m = Message::Input(event);\n"),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::MessageMarkOutsideApp),
            [pair("update.rs", "Message::Input")]
        );
    }

    #[test]
    fn a_widget_ctx_lives_in_the_reducer() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/notes/reducer.rs",
                "pub(crate) struct NotesCtx {\n}\n",
            ),
            (
                "widgets/tabs/state.rs",
                "pub(crate) struct TabsCtx<'a> {\n}\n",
            ),
            ("commands.rs", "pub(crate) struct CommandCtx {\n}\n"),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::CtxOutsideReducer),
            [pair("widgets/tabs/state.rs", "TabsCtx")]
        );
    }

    #[test]
    fn an_event_file_holds_only_intents_effects_and_events() {
        let tree = SourceTree::from_sources(&[(
            "widgets/notes/event.rs",
            "pub(crate) enum NotesIntent {\n}\npub(crate) enum NotesEffect {\n}\n\
             pub(crate) enum NotesEvent {\n}\npub(crate) struct NotesAction {\n}\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::ForeignTypeInEventFile),
            [pair("widgets/notes/event.rs", "NotesAction")]
        );
    }

    #[test]
    fn a_model_only_reads_the_state() {
        let tree = SourceTree::from_sources(&[
            (
                "widgets/notes/model.rs",
                "fn bad(state: &mut NotesState) {}\n",
            ),
            (
                "widgets/tabs/model.rs",
                "fn good(state: &TabsState, out: &mut Vec<u8>) {}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ModelTakesMutState),
            [pair("widgets/notes/model.rs", "NotesState")]
        );
    }
}
