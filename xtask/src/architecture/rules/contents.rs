//! Rules about what a file of a given role may contain.

use syn::visit::{self, Visit};
use syn::{AttrStyle, Path, PathArguments, PathSegment, Type, TypeReference};

use super::super::finding::{Finding, Rule};
use super::super::layout::{
    Layer, STANDARD_WIDGET_FILES, child_of, is_model_file, is_view_file,
    layer_of, path_in_owner,
};
use super::super::source::{SourceFile, SourceTree};
use super::super::syntax::{
    TypeKind, declared_types, file_idents, generic_idents, line_of, path_names,
    written_paths,
};

/// The lint that the first attribute of every reducer, after its doc
/// comments, forbids: `#![forbid(clippy::disallowed_methods)]`.
const FORBID_LINT: &[&str] = &["clippy", "disallowed_methods"];

/// The marks `app.rs` puts on events, as path segments.
const MESSAGE_MARKS: &[[&str; 2]] =
    &[["Message", "Input"], ["Message", "Internal"]];

/// Name endings of the types an `event.rs` may define.
const EVENT_TYPE_ENDINGS: &[&str] = &["Intent", "Effect", "Event"];

/// Name ending of the type that joins intents and effects, which only a
/// widget has, never a child widget.
const EVENT_ENDING: &str = "Event";

/// Name ending of a view model type.
const VIEW_MODEL_ENDING: &str = "ViewModel";

/// The type that every view returns.
const ELEMENT: &str = "Element";

/// The word that names a renderer in an `Element` type.
const RENDERER: &str = "Renderer";

/// Finds the first `Element<...>` whose generic arguments name a renderer.
#[derive(Default)]
struct RendererInElement {
    line: Option<usize>,
}

impl<'ast> Visit<'ast> for RendererInElement {
    fn visit_path_segment(&mut self, segment: &'ast PathSegment) {
        if self.line.is_none()
            && segment.ident == ELEMENT
            && let PathArguments::AngleBracketed(arguments) = &segment.arguments
            && generic_idents(arguments)
                .iter()
                .any(|name| name == RENDERER)
        {
            self.line = Some(line_of(segment.ident.span()));
        }

        visit::visit_path_segment(self, segment);
    }
}

/// The `&mut XState` types of a file, by the name of the state type.
#[derive(Default)]
struct MutStates {
    found: Vec<(String, usize)>,
}

impl<'ast> Visit<'ast> for MutStates {
    fn visit_type_reference(&mut self, reference: &'ast TypeReference) {
        if reference.mutability.is_some()
            && let Type::Path(path) = reference.elem.as_ref()
            && let Some(last) = path.path.segments.last()
            && last.ident.to_string().ends_with("State")
        {
            self.found
                .push((last.ident.to_string(), line_of(last.ident.span())));
        }

        visit::visit_type_reference(self, reference);
    }
}

/// Report contents that do not fit the file's role.
pub(crate) fn check(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in tree.files() {
        findings.extend(message_marks_outside_app(file));
        findings.extend(element_names_renderer(file));

        let Some(Layer::Widget(widget)) = layer_of(file.path()) else {
            continue;
        };
        if is_pure_file(file, &widget) {
            findings.extend(task_or_element(file));
        }
        match file.name() {
            "reducer.rs" => findings.extend(reducer_without_forbid(file)),
            "event.rs" => {
                findings.extend(foreign_event_types(file));
                if child_of(file.path(), &widget).is_some()
                    && path_in_owner(file.path(), &widget) == "event.rs"
                {
                    findings.extend(child_event_types(file));
                }
            },
            _ => {},
        }
        if file.name() != "reducer.rs" {
            findings.extend(ctx_outside_reducer(file));
        }
        if is_model_file(file.path(), &widget) {
            findings.extend(model_mut_state(file));
        } else {
            findings.extend(view_models_outside_model(file));
        }
    }

    findings
}

/// `state.rs` and topic files, of a widget or of a child widget: plain
/// logic, without `Task` and `Element`. The files of views and view models
/// are neither.
fn is_pure_file(file: &SourceFile, widget: &str) -> bool {
    if is_view_file(file.path(), widget) || is_model_file(file.path(), widget) {
        return false;
    }

    file.name() == "state.rs" || !STANDARD_WIDGET_FILES.contains(&file.name())
}

fn task_or_element(file: &SourceFile) -> Vec<Finding> {
    let idents = file_idents(file.syntax());

    ["Task", "Element"]
        .iter()
        .filter_map(|word| {
            let ident = idents.iter().find(|ident| *ident == word)?;

            Some(Finding::new(
                Rule::TaskOrElementInPureFile,
                file.path(),
                word,
                line_of(ident.span()),
            ))
        })
        .collect()
}

/// A reducer whose first attribute, after its doc comments, is not
/// `#![forbid(clippy::disallowed_methods)]`.
fn reducer_without_forbid(file: &SourceFile) -> Option<Finding> {
    let syntax = file.syntax();
    let first = syntax
        .attrs
        .iter()
        .find(|attr| !attr.path().is_ident("doc"));

    let is_forbid = first.is_some_and(|attr| {
        matches!(attr.style, AttrStyle::Inner(_))
            && attr.path().is_ident("forbid")
            && attr
                .parse_args::<Path>()
                .is_ok_and(|lint| path_names(&lint) == FORBID_LINT)
    });
    if is_forbid {
        return None;
    }

    // Without attributes, the place is the start of the file.
    let line = first.map_or(1, |attr| line_of(attr.pound_token.span));
    Some(Finding::new(
        Rule::ReducerWithoutForbid,
        file.path(),
        "forbid",
        line,
    ))
}

fn message_marks_outside_app(file: &SourceFile) -> Vec<Finding> {
    if file.path() == "app.rs" {
        return Vec::new();
    }

    let paths = written_paths(file.syntax());
    MESSAGE_MARKS
        .iter()
        .filter_map(|mark| {
            let path = paths.iter().find(|path| {
                path.segments().windows(2).any(|pair| pair == mark)
            })?;

            Some(Finding::new(
                Rule::MessageMarkOutsideApp,
                file.path(),
                &mark.join("::"),
                path.line(),
            ))
        })
        .collect()
}

/// The first `Element<...>` type in `file` whose generic arguments name a
/// renderer, such as `Element<'a, X, Theme, iced::Renderer>`.
fn element_names_renderer(file: &SourceFile) -> Option<Finding> {
    let mut finder = RendererInElement::default();
    finder.visit_file(file.syntax());

    let line = finder.line?;
    Some(Finding::new(
        Rule::ElementNamesRenderer,
        file.path(),
        RENDERER,
        line,
    ))
}

fn ctx_outside_reducer(file: &SourceFile) -> Vec<Finding> {
    declared_types(file.syntax())
        .iter()
        .filter(|declared| {
            declared.kind() == TypeKind::Struct
                && declared.name().ends_with("Ctx")
        })
        .map(|declared| {
            Finding::new(
                Rule::CtxOutsideReducer,
                file.path(),
                declared.name(),
                declared.line(),
            )
        })
        .collect()
}

/// The structs and enums named `...ViewModel` that `file` declares.
fn view_models_outside_model(file: &SourceFile) -> Vec<Finding> {
    declared_types(file.syntax())
        .iter()
        .filter(|declared| {
            declared.kind() != TypeKind::Alias
                && declared.name().ends_with(VIEW_MODEL_ENDING)
        })
        .map(|declared| {
            Finding::new(
                Rule::ViewModelOutsideModel,
                file.path(),
                declared.name(),
                declared.line(),
            )
        })
        .collect()
}

fn foreign_event_types(file: &SourceFile) -> Vec<Finding> {
    declared_types(file.syntax())
        .iter()
        .filter(|declared| {
            !EVENT_TYPE_ENDINGS
                .iter()
                .any(|ending| declared.name().ends_with(ending))
        })
        .map(|declared| {
            Finding::new(
                Rule::ForeignTypeInEventFile,
                file.path(),
                declared.name(),
                declared.line(),
            )
        })
        .collect()
}

/// The types of a child widget's `event.rs` named `...Event`.
fn child_event_types(file: &SourceFile) -> Vec<Finding> {
    declared_types(file.syntax())
        .iter()
        .filter(|declared| declared.name().ends_with(EVENT_ENDING))
        .map(|declared| {
            Finding::new(
                Rule::ChildDeclaresEvent,
                file.path(),
                declared.name(),
                declared.line(),
            )
        })
        .collect()
}

fn model_mut_state(file: &SourceFile) -> Vec<Finding> {
    let mut states = MutStates::default();
    states.visit_file(file.syntax());

    states
        .found
        .iter()
        .map(|(name, line)| {
            Finding::new(Rule::ModelTakesMutState, file.path(), name, *line)
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
                "fn a() -> Task<X> {}\nfn b(h: task::Handle) {}\n",
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
    fn a_child_view_may_build_elements_but_its_state_and_topic_files_may_not() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/child/run/mod.rs", ""),
            (
                "widgets/containers/child/run/view/run_form.rs",
                "fn view() -> Element<'_, RunIntent> {}\n",
            ),
            (
                "widgets/containers/child/run/view.rs",
                "fn panel() -> Element<'_, RunIntent> {}\n",
            ),
            (
                "widgets/containers/child/run/reducer.rs",
                "fn reduce() -> Task<RunEffect> {}\n",
            ),
            (
                "widgets/containers/child/run/state.rs",
                "fn a() -> Element<X> {}\n",
            ),
            (
                "widgets/containers/child/run/checks.rs",
                "fn b() -> Task<X> {}\n",
            ),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::TaskOrElementInPureFile),
            [
                pair("widgets/containers/child/run/checks.rs", "Task"),
                pair("widgets/containers/child/run/state.rs", "Element"),
            ]
        );
    }

    #[test]
    fn the_rules_for_reducer_event_model_and_ctx_apply_to_child_files() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/child/run/mod.rs", ""),
            (
                "widgets/containers/child/run/reducer.rs",
                "use iced::Task;\npub(crate) struct RunCtx<'a> {\n}\n",
            ),
            (
                "widgets/containers/child/run/event.rs",
                "pub(crate) enum RunIntent {\n}\npub(crate) enum RunEffect {\n}\n\
                 pub(crate) struct RunEdit {\n}\n",
            ),
            (
                "widgets/containers/child/run/model.rs",
                "fn bad(state: &mut RunState) {}\n",
            ),
            (
                "widgets/containers/child/run/state.rs",
                "pub(crate) struct RunCtx {\n}\n",
            ),
        ]);

        let findings = check(&tree);

        assert_eq!(
            found(&findings, Rule::ReducerWithoutForbid),
            [pair("widgets/containers/child/run/reducer.rs", "forbid")]
        );
        assert_eq!(
            found(&findings, Rule::ForeignTypeInEventFile),
            [pair("widgets/containers/child/run/event.rs", "RunEdit")]
        );
        assert_eq!(
            found(&findings, Rule::ModelTakesMutState),
            [pair("widgets/containers/child/run/model.rs", "RunState")]
        );
        assert_eq!(
            found(&findings, Rule::CtxOutsideReducer),
            [pair("widgets/containers/child/run/state.rs", "RunCtx")]
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
    fn only_app_marks_messages() {
        let tree = SourceTree::from_sources(&[
            (
                "app.rs",
                "fn update(message: Message) {\n    match message {\n        \
                 Message::Input(event) => x,\n        Message::Internal(e) => y,\n    }\n}\n",
            ),
            (
                "update.rs",
                "fn update() {\n    let m = Message::Input(event);\n}\n",
            ),
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
            ("widgets/containers/child/run/mod.rs", ""),
            (
                "widgets/containers/model.rs",
                "pub(crate) struct NoEngineViewModel {\n}\n\
                 pub(crate) enum MainAreaViewModel<'a> {\n}\n",
            ),
            (
                "widgets/containers/child/run/model.rs",
                "pub(crate) struct RunFormViewModel<'a> {\n}\n",
            ),
            (
                "widgets/containers/no_engine.rs",
                "pub(crate) struct NoEngineFixViewModel {\n}\n",
            ),
            (
                "widgets/containers/child/run/view/run_form.rs",
                "struct RunFormFieldViewModel {\n}\n",
            ),
            (
                "widgets/containers/view/model.rs",
                "pub(crate) enum TableViewModel {\n}\n",
            ),
            ("theme.rs", "pub(crate) struct ThemeViewModel {\n}\n"),
        ]);

        assert_eq!(
            found(&check(&tree), Rule::ViewModelOutsideModel),
            [
                pair(
                    "widgets/containers/child/run/view/run_form.rs",
                    "RunFormFieldViewModel"
                ),
                pair("widgets/containers/no_engine.rs", "NoEngineFixViewModel"),
                pair("widgets/containers/view/model.rs", "TableViewModel"),
            ]
        );
    }

    #[test]
    fn the_files_of_a_model_folder_are_model_files() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/child/run/mod.rs", ""),
            (
                "widgets/containers/model/mod.rs",
                "pub(crate) struct ShownViewModel {\n}\nfn bad(state: &mut ContainersState) {}\n",
            ),
            (
                "widgets/containers/model/main_area.rs",
                "pub(crate) enum MainAreaViewModel<'a> {\n}\n\
                 fn worse(state: &mut ContainersState) {}\n\
                 fn not_a_topic() -> Element<'_, X> {}\n",
            ),
            (
                "widgets/containers/child/run/model/run_form.rs",
                "pub(crate) struct RunFormViewModel<'a> {\n}\nfn bad(state: &mut RunState) {}\n",
            ),
        ]);

        let findings = check(&tree);

        assert!(found(&findings, Rule::ViewModelOutsideModel).is_empty());
        assert!(found(&findings, Rule::TaskOrElementInPureFile).is_empty());
        assert_eq!(
            found(&findings, Rule::ModelTakesMutState),
            [
                pair(
                    "widgets/containers/child/run/model/run_form.rs",
                    "RunState"
                ),
                pair(
                    "widgets/containers/model/main_area.rs",
                    "ContainersState"
                ),
                pair("widgets/containers/model/mod.rs", "ContainersState"),
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
                 fn headless() -> iced::Renderer {\n}\nfn wrap() {\n    let e = Element::from(text);\n}\n",
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
    fn only_the_parent_declares_an_event_type() {
        let tree = SourceTree::from_sources(&[
            ("widgets/containers/mod.rs", ""),
            ("widgets/containers/child/run/mod.rs", ""),
            (
                "widgets/containers/event.rs",
                "pub(crate) enum ContainersEvent {\n}\n",
            ),
            (
                "widgets/containers/child/run/event.rs",
                "pub(crate) enum RunIntent {\n}\n\
                 pub(crate) enum RunEvent {\n    Intent(RunIntent),\n}\n",
            ),
            (
                "widgets/containers/child/run/view/event.rs",
                "pub(crate) enum RowEvent {\n}\n",
            ),
        ]);

        let findings = check(&tree);

        assert_eq!(
            found(&findings, Rule::ChildDeclaresEvent),
            [pair("widgets/containers/child/run/event.rs", "RunEvent")]
        );
        assert!(found(&findings, Rule::ForeignTypeInEventFile).is_empty());
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

    #[test]
    fn every_mut_state_is_found_also_with_a_lifetime_or_a_path() {
        let tree = SourceTree::from_sources(&[(
            "widgets/notes/model.rs",
            "fn both(out: &mut Vec<u8>, state: &mut NotesState) {}\n\
             fn long<'a>(state: &'a mut super::state::DraftState) {}\n",
        )]);

        assert_eq!(
            found(&check(&tree), Rule::ModelTakesMutState),
            [
                pair("widgets/notes/model.rs", "DraftState"),
                pair("widgets/notes/model.rs", "NotesState"),
            ]
        );
    }
}
