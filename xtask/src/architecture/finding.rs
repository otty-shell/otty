//! What the architecture check reports: the rules and the places that break
//! them.

/// One architecture rule of the `app` crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
    /// A source file belongs to no layer: wiring, widgets or shared modules.
    FileOutsideLayers,
    /// A widget imports another widget.
    WidgetImportsWidget,
    /// A widget imports the wiring layer.
    WidgetImportsWiring,
    /// A shared module imports something that is not a shared module.
    SharedImportsOutside,
    /// A widget or a part lacks a required file, or a widget has no route.
    MissingWidgetFile,
    /// `router/` holds a file that is neither a widget route nor the command
    /// route.
    UnexpectedRouterFile,
    /// A module of a widget or a part is open or closed against the
    /// visibility rule.
    WidgetModuleVisibility,
    /// A widget or part `mod.rs` re-exports from an open module or from
    /// outside its folder.
    ReexportFromOpenModule,
    /// A view imports the state of its widget or of a part.
    ViewImportsState,
    /// `Task` or `Element` in `state.rs` or in a topic file.
    TaskOrElementInPureFile,
    /// A widget's `services` or `storage` is used outside its route.
    ServicesOutsideRoute,
    /// A `reducer.rs` does not start with the `forbid` line.
    ReducerWithoutForbid,
    /// A stream is started outside `subscription.rs`.
    StreamOutsideSubscription,
    /// A route is called directly outside `router/mod.rs` and `update.rs`.
    RouteCalledDirectly,
    /// A widget's `reduce` is called outside its route.
    ReduceCalledOutsideRoute,
    /// `Message::Input` or `Message::Internal` outside `app.rs`.
    MessageMarkOutsideApp,
    /// A widget `Ctx` is defined outside `reducer.rs`.
    CtxOutsideReducer,
    /// The clippy ban on files, processes and tasks is lifted where it may
    /// not be.
    DisallowedMethodsLifted,
    /// `event.rs` defines a type that is not an intent, effect or event.
    ForeignTypeInEventFile,
    /// `model.rs` takes the widget state mutably.
    ModelTakesMutState,
    /// A mode has no screen file in `screens/`.
    MissingModeScreen,
    /// The reducer entry parameter is not named `intent`.
    ReduceParameterName,
    /// A view model accessor does not build through `from_state`.
    ViewModelNotFromState,
    /// A route function decides what to do from widget state.
    RouteBranchesOnState,
    /// A part imports its parent's own modules or `mod.rs` items, or another
    /// part.
    PartImportsOutside,
    /// A part's `reduce` is called outside its parent's `reducer.rs`.
    PartReduceOutsideParent,
}

impl Rule {
    /// Every rule, in declaration order, which is also the order of reports.
    pub const ALL: [Rule; 26] = [
        Rule::FileOutsideLayers,
        Rule::WidgetImportsWidget,
        Rule::WidgetImportsWiring,
        Rule::SharedImportsOutside,
        Rule::MissingWidgetFile,
        Rule::UnexpectedRouterFile,
        Rule::WidgetModuleVisibility,
        Rule::ReexportFromOpenModule,
        Rule::ViewImportsState,
        Rule::TaskOrElementInPureFile,
        Rule::ServicesOutsideRoute,
        Rule::ReducerWithoutForbid,
        Rule::StreamOutsideSubscription,
        Rule::RouteCalledDirectly,
        Rule::ReduceCalledOutsideRoute,
        Rule::MessageMarkOutsideApp,
        Rule::CtxOutsideReducer,
        Rule::DisallowedMethodsLifted,
        Rule::ForeignTypeInEventFile,
        Rule::ModelTakesMutState,
        Rule::MissingModeScreen,
        Rule::ReduceParameterName,
        Rule::ViewModelNotFromState,
        Rule::RouteBranchesOnState,
        Rule::PartImportsOutside,
        Rule::PartReduceOutsideParent,
    ];

    /// Short, stable name of the rule, used in reports.
    ///
    /// ```
    /// use otty_xtask::architecture::finding::Rule;
    ///
    /// assert_eq!(Rule::ViewImportsState.name(), "view-imports-state");
    /// ```
    pub fn name(self) -> &'static str {
        match self {
            Rule::FileOutsideLayers => "file-outside-layers",
            Rule::WidgetImportsWidget => "widget-imports-widget",
            Rule::WidgetImportsWiring => "widget-imports-wiring",
            Rule::SharedImportsOutside => "shared-imports-outside",
            Rule::MissingWidgetFile => "missing-widget-file",
            Rule::UnexpectedRouterFile => "unexpected-router-file",
            Rule::WidgetModuleVisibility => "widget-module-visibility",
            Rule::ReexportFromOpenModule => "reexport-from-open-module",
            Rule::ViewImportsState => "view-imports-state",
            Rule::TaskOrElementInPureFile => "task-or-element-in-pure-file",
            Rule::ServicesOutsideRoute => "services-outside-route",
            Rule::ReducerWithoutForbid => "reducer-without-forbid",
            Rule::StreamOutsideSubscription => "stream-outside-subscription",
            Rule::RouteCalledDirectly => "route-called-directly",
            Rule::ReduceCalledOutsideRoute => "reduce-called-outside-route",
            Rule::MessageMarkOutsideApp => "message-mark-outside-app",
            Rule::CtxOutsideReducer => "ctx-outside-reducer",
            Rule::DisallowedMethodsLifted => "disallowed-methods-lifted",
            Rule::ForeignTypeInEventFile => "foreign-type-in-event-file",
            Rule::ModelTakesMutState => "model-takes-mut-state",
            Rule::MissingModeScreen => "missing-mode-screen",
            Rule::ReduceParameterName => "reduce-parameter-name",
            Rule::ViewModelNotFromState => "view-model-not-from-state",
            Rule::RouteBranchesOnState => "route-branches-on-state",
            Rule::PartImportsOutside => "part-imports-outside",
            Rule::PartReduceOutsideParent => "part-reduce-outside-parent",
        }
    }

    /// What the rule requires, in one plain sentence, shown above its places
    /// in the report.
    pub fn description(self) -> &'static str {
        match self {
            Rule::FileOutsideLayers => {
                "Every source file belongs to a layer: the wiring, a widget under widgets/<name>/ or a shared module."
            },
            Rule::WidgetImportsWidget => {
                "A widget never imports another widget; shared types live in domain/, and data from other widgets comes in through XCtx."
            },
            Rule::WidgetImportsWiring => {
                "A widget never imports the wiring: app, state, update, subscription, guards, view, screens or router."
            },
            Rule::SharedImportsOutside => {
                "A shared module imports only other shared modules and crates, never a widget or the wiring."
            },
            Rule::MissingWidgetFile => {
                "Every widget has mod.rs, event.rs, state.rs, reducer.rs, model.rs, a view/ folder and a route in router/<widget>.rs, and every part has the same files and folder but no route."
            },
            Rule::UnexpectedRouterFile => {
                "router/ holds only mod.rs, one route per widget and command.rs."
            },
            Rule::WidgetModuleVisibility => {
                "In a widget's mod.rs only event, types, view, storage, services and its parts are open (pub(crate) mod), event and view always are, every other module is closed, and a part's mod.rs follows the same rule."
            },
            Rule::ReexportFromOpenModule => {
                "A widget's or a part's mod.rs re-exports only from its closed modules, so every type has exactly one path."
            },
            Rule::ViewImportsState => {
                "A view never imports the state of its widget or of a part; it gets its data from the view model."
            },
            Rule::TaskOrElementInPureFile => {
                "state.rs and topic files hold plain logic, without Task or Element."
            },
            Rule::ServicesOutsideRoute => {
                "A widget's services and storage are used only by its route router/<widget>.rs, and storage also by XWidget::load."
            },
            Rule::ReducerWithoutForbid => {
                "Every reducer.rs starts with #![forbid(clippy::disallowed_methods)], so a reducer can never start background work."
            },
            Rule::StreamOutsideSubscription => {
                "Long-running streams (Subscription::run) start only in subscription.rs."
            },
            Rule::RouteCalledDirectly => {
                "Only router/mod.rs and update.rs call routes directly; other code sends a message with Task::done."
            },
            Rule::ReduceCalledOutsideRoute => {
                "A widget's reduce is called only by its route router/<widget>.rs."
            },
            Rule::MessageMarkOutsideApp => {
                "Only app.rs marks events as Message::Input or Message::Internal."
            },
            Rule::CtxOutsideReducer => {
                "A widget's XCtx is defined in its reducer.rs."
            },
            Rule::DisallowedMethodsLifted => {
                "The clippy ban on files, processes and tasks is lifted only on the router, services, storage, subscription, shell_integration, testing and tests modules."
            },
            Rule::ForeignTypeInEventFile => {
                "event.rs holds only XIntent, XEffect and XEvent; other types go to types.rs."
            },
            Rule::ModelTakesMutState => {
                "model.rs only reads the widget state to build view models and never takes it as &mut."
            },
            Rule::MissingModeScreen => {
                "Every mode has one screen file in app/src/screens/."
            },
            Rule::ReduceParameterName => {
                "The reducer entry parameter is named intent, because its type is always XIntent."
            },
            Rule::ViewModelNotFromState => {
                "A view model is built in model.rs through XViewModel::from_state."
            },
            Rule::RouteBranchesOnState => {
                "A route branches only on its intent or effect; decisions from widget state belong to the reducer, which reports them as effects."
            },
            Rule::PartImportsOutside => {
                "A part imports only its own files, its parent's types, errors and topic files, and shared modules, never its parent's state, reducer, event, model, subscription, view, services, storage or mod.rs items, or another part."
            },
            Rule::PartReduceOutsideParent => {
                "A part's reduce is called only by its parent's reducer.rs."
            },
        }
    }
}

/// One place in `app/src` that breaks a rule.
///
/// `key` names what breaks the rule (an imported module, a function, a
/// file), so a finding stays the same when lines move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    rule: Rule,
    file: String,
    key: String,
    line: usize,
}

impl Finding {
    /// The rule this place breaks.
    pub fn rule(&self) -> Rule {
        self.rule
    }

    /// The file, relative to `app/src`, with `/` between folders.
    pub fn file(&self) -> &str {
        &self.file
    }

    /// What breaks the rule in that file.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The first line where it happens, counting from 1; 0 for a missing
    /// file.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Record that `file` breaks `rule` through `key` at `line`.
    pub(crate) fn new(rule: Rule, file: &str, key: &str, line: usize) -> Self {
        Self {
            rule,
            file: file.to_string(),
            key: key.to_string(),
            line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_is_listed_once_in_declaration_order() {
        let pairs = Rule::ALL.windows(2);

        for pair in pairs {
            assert!(
                pair[0] < pair[1],
                "{:?} must come before {:?}",
                pair[0],
                pair[1]
            );
        }
        assert_eq!(Rule::ALL.first(), Some(&Rule::FileOutsideLayers));
        assert_eq!(Rule::ALL.last(), Some(&Rule::PartReduceOutsideParent));
    }

    #[test]
    fn every_rule_has_a_one_sentence_description() {
        for rule in Rule::ALL {
            let description = rule.description();

            assert!(description.ends_with('.'), "{rule:?}: {description}");
            assert!(
                description
                    .chars()
                    .next()
                    .is_some_and(|c| !c.is_lowercase())
                    || description.starts_with("router/")
                    || description.starts_with("event.rs")
                    || description.starts_with("model.rs")
                    || description.starts_with("state.rs"),
                "{rule:?}: {description}"
            );
        }
    }
}
