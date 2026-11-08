//! Rules about what a file of a given role may contain.

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    Layer, STANDARD_WIDGET_FILES, is_view_file, layer_of, part_of,
};
use super::super::source::{Line, SourceFile, SourceTree};
use super::super::text::{find_word, name_after};

/// The first line of every reducer.
const FORBID_LINE: &str = "#![forbid(clippy::disallowed_methods)]";

/// The marks `app.rs` puts on events.
const MESSAGE_MARKS: &[&str] = &["Message::Input", "Message::Internal"];

/// Name endings of the types an `event.rs` may define.
const EVENT_TYPE_ENDINGS: &[&str] = &["Intent", "Effect", "Event"];

/// Name ending of a view model type.
const VIEW_MODEL_ENDING: &str = "ViewModel";

/// The type that every view returns.
const ELEMENT: &str = "Element";

/// The word that names a renderer in an `Element` type.
const RENDERER: &str = "Renderer";

/// Report contents that do not fit the file's role.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in tree.files() {
        findings.extend(streams_outside_subscription(file));
        findings.extend(message_marks_outside_app(file));
        findings.extend(element_names_renderer(file));

        let Some(Layer::Widget(widget)) = layer_of(file.path()) else {
            continue;
        };
        if is_pure_file(tree, file, &widget) {
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
        if !is_model_file(tree, file, &widget) {
            findings.extend(view_models_outside_model(file));
        }
    }

    findings
}

/// `state.rs` and topic files, of a widget or of a part: plain logic,
/// without `Task` and `Element`.
fn is_pure_file(tree: &SourceTree, file: &SourceFile, widget: &str) -> bool {
    if is_view_file(tree, file.path(), widget) {
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

/// Whether `file` is the `model.rs` of `widget` or of one of its parts.
fn is_model_file(tree: &SourceTree, file: &SourceFile, widget: &str) -> bool {
    let model = match part_of(tree, file.path(), widget) {
        Some(part) => format!("widgets/{widget}/{part}/model.rs"),
        None => format!("widgets/{widget}/model.rs"),
    };

    file.path() == model
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

/// The first `Element<...>` type in `file` whose generic arguments name a
/// renderer, such as `Element<'a, X, Theme, iced::Renderer>`.
fn element_names_renderer(file: &SourceFile) -> Option<Finding> {
    let lines = file.lines();

    for (index, line) in lines.iter().enumerate() {
        for at in element_starts(line.text()) {
            let arguments = generic_arguments(lines, index, at);
            if find_word(&arguments, RENDERER).is_some() {
                return Some(Finding::new(
                    Rule::ElementNamesRenderer,
                    file.path(),
                    RENDERER,
                    line.number(),
                ));
            }
        }
    }

    None
}

/// The byte offsets just after each `Element<` in `text`, where `Element`
/// is a whole word.
fn element_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut from = 0;

    while let Some(found) = find_word(&text[from..], ELEMENT) {
        let end = from + found + ELEMENT.len();
        if text[end..].starts_with('<') {
            starts.push(end + 1);
        }
        from = end;
    }

    starts
}

/// The generic arguments that start at byte `at` of `lines[index]`, joined
/// over lines up to the `>` that closes them. The `>` of a `->` closes
/// nothing.
fn generic_arguments(lines: &[Line], index: usize, at: usize) -> String {
    let mut arguments = String::new();
    let mut depth = 1;
    let mut previous = '<';
    let mut text = &lines[index].text()[at..];
    let mut next = index + 1;

    loop {
        for ch in text.chars() {
            match ch {
                '<' => depth += 1,
                '>' if previous != '-' => {
                    depth -= 1;
                    if depth == 0 {
                        return arguments;
                    }
                },
                _ => {},
            }
            arguments.push(ch);
            previous = ch;
        }

        let Some(line) = lines.get(next) else {
            return arguments;
        };
        arguments.push(' ');
        text = line.text();
        next += 1;
    }
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

/// The structs and enums named `...ViewModel` that `file` declares.
fn view_models_outside_model(file: &SourceFile) -> Vec<Finding> {
    file.lines()
        .iter()
        .filter_map(|line| {
            let name = ["struct", "enum"]
                .iter()
                .find_map(|keyword| name_after(line.text(), keyword))?;
            name.ends_with(VIEW_MODEL_ENDING).then(|| {
                Finding::new(
                    Rule::ViewModelOutsideModel,
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
    fn a_part_view_may_build_elements_but_its_state_and_topic_files_may_not() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/run/mod.rs", ""),
            (
                "widgets/containers/run/view/run_form.rs",
                "fn view() -> Element<'_, RunIntent> {}\n",
            ),
            (
                "widgets/containers/run/view.rs",
                "fn panel() -> Element<'_, RunIntent> {}\n",
            ),
            (
                "widgets/containers/run/reducer.rs",
                "fn reduce() -> Task<RunEffect> {}\n",
            ),
            (
                "widgets/containers/run/state.rs",
                "fn a() -> Element<X> {}\n",
            ),
            ("widgets/containers/run/checks.rs", "fn b() -> Task<X> {}\n"),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::TaskOrElementInPureFile),
            [
                pair("widgets/containers/run/checks.rs", "Task"),
                pair("widgets/containers/run/state.rs", "Element"),
            ]
        );
    }

    #[test]
    fn the_rules_for_reducer_event_model_and_ctx_apply_to_part_files() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/run/mod.rs", ""),
            (
                "widgets/containers/run/reducer.rs",
                "use iced::Task;\npub(crate) struct RunCtx<'a> {\n}\n",
            ),
            (
                "widgets/containers/run/event.rs",
                "pub(crate) enum RunIntent {\n}\npub(crate) enum RunEffect {\n}\n\
                 pub(crate) struct RunEdit {\n}\n",
            ),
            (
                "widgets/containers/run/model.rs",
                "fn bad(state: &mut RunState) {}\n",
            ),
            (
                "widgets/containers/run/state.rs",
                "pub(crate) struct RunCtx {\n}\n",
            ),
        ]);

        let findings = check(&tree);

        assert_eq!(
            found(&findings, Rule::ReducerWithoutForbid),
            [pair("widgets/containers/run/reducer.rs", "forbid")]
        );
        assert_eq!(
            found(&findings, Rule::ForeignTypeInEventFile),
            [pair("widgets/containers/run/event.rs", "RunEdit")]
        );
        assert_eq!(
            found(&findings, Rule::ModelTakesMutState),
            [pair("widgets/containers/run/model.rs", "RunState")]
        );
        assert_eq!(
            found(&findings, Rule::CtxOutsideReducer),
            [pair("widgets/containers/run/state.rs", "RunCtx")]
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
    fn view_models_are_declared_only_in_model_files() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/run/mod.rs", ""),
            (
                "widgets/containers/model.rs",
                "pub(crate) struct NoEngineViewModel {\n}\n\
                 pub(crate) enum MainAreaViewModel<'a> {\n}\n",
            ),
            (
                "widgets/containers/run/model.rs",
                "pub(crate) struct RunFormViewModel<'a> {\n}\n",
            ),
            (
                "widgets/containers/no_engine.rs",
                "pub(crate) struct NoEngineFixViewModel {\n}\n",
            ),
            (
                "widgets/containers/run/view/run_form.rs",
                "struct RunFormFieldViewModel {\n}\n",
            ),
            (
                "widgets/containers/view/model.rs",
                "pub(crate) enum TableViewModel {\n}\n",
            ),
            (
                "components/pull_line.rs",
                "pub(crate) struct PullLineViewModel {\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ViewModelOutsideModel),
            [
                pair("widgets/containers/no_engine.rs", "NoEngineFixViewModel"),
                pair(
                    "widgets/containers/run/view/run_form.rs",
                    "RunFormFieldViewModel"
                ),
                pair("widgets/containers/view/model.rs", "TableViewModel"),
            ]
        );
    }

    #[test]
    fn an_element_type_never_names_the_renderer() {
        let tree = SourceTree::from_sources(&[
            (
                "view.rs",
                "pub(crate) fn view(app: &App) -> Element<'_, AppEvent, Theme, iced::Renderer> {\n}\n",
            ),
            (
                "screens/terminal.rs",
                "fn menu<'a>() -> (\n    Element<\n        'a,\n        AppEvent,\n        Theme,\n        iced::Renderer,\n    >,\n    f32,\n) {\n}\n",
            ),
            (
                "screens/mod.rs",
                "fn layer<'a>(a: Element<'a, AppEvent>, b: iced::Element<'a, X, Theme, Renderer>) {\n}\n",
            ),
            (
                "app.rs",
                "use iced::{Element, Renderer};\npub(crate) fn view(&self) -> Element<'_, Message> {\n}\n\
                 fn headless() -> iced::Renderer {\n}\nlet e = Element::from(text);\n",
            ),
            (
                "widgets/notes/view/notes_panel.rs",
                "fn view() -> iced::Element<'_, NotesIntent> {\n}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ElementNamesRenderer),
            [
                pair("screens/mod.rs", "Renderer"),
                pair("screens/terminal.rs", "Renderer"),
                pair("view.rs", "Renderer"),
            ]
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
