//! The places in `app/src` that break a rule and are accepted for now or
//! for good, each with its reason.
//!
//! This list is the only place where exceptions to the architecture are
//! written down; the documentation describes the rules themselves.

use super::finding::{Finding, Rule};

/// Why an allowlist entry is there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowKind {
    /// A deliberate, permanent exception that the design allows.
    Exception,
    /// A violation that is accepted until it is fixed.
    KnownViolation,
}

/// A finding the check accepts, with the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allowed {
    rule: Rule,
    file: &'static str,
    key: &'static str,
    kind: AllowKind,
    reason: &'static str,
}

impl Allowed {
    /// The rule the accepted place breaks.
    pub fn rule(&self) -> Rule {
        self.rule
    }

    /// The file, relative to `app/src`.
    pub fn file(&self) -> &'static str {
        self.file
    }

    /// What breaks the rule in that file, as the finding names it.
    pub fn key(&self) -> &'static str {
        self.key
    }

    /// Whether this is a permanent exception or a violation to fix.
    pub fn kind(&self) -> AllowKind {
        self.kind
    }

    /// Why the place is accepted.
    pub fn reason(&self) -> &'static str {
        self.reason
    }

    /// A permanent exception that the design allows.
    pub const fn exception(
        rule: Rule,
        file: &'static str,
        key: &'static str,
        reason: &'static str,
    ) -> Self {
        Self {
            rule,
            file,
            key,
            kind: AllowKind::Exception,
            reason,
        }
    }

    /// A violation accepted until it is fixed.
    pub const fn known(
        rule: Rule,
        file: &'static str,
        key: &'static str,
        reason: &'static str,
    ) -> Self {
        Self {
            rule,
            file,
            key,
            kind: AllowKind::KnownViolation,
            reason,
        }
    }

    /// Whether this entry accepts `finding`.
    pub fn matches(&self, finding: &Finding) -> bool {
        self.rule == finding.rule()
            && self.file == finding.file()
            && self.key == finding.key()
    }
}

/// Every accepted place in `app/src`.
pub const ALLOWLIST: &[Allowed] = &[
    Allowed::known(
        Rule::SharedImportsOutside,
        "domain/quick_launch.rs",
        "widgets::quick_launch",
        "the domain type still names a type of the quick_launch widget; move that type into domain/",
    ),
    Allowed::known(
        Rule::MissingWidgetFile,
        "widgets/palette/view/",
        "palette",
        "the palette view is one view.rs file; move it into view/",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/chrome/mod.rs",
        "pub(crate) mod model",
        "model is open; close it and re-export a view model only where outside code names it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod action_flow",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod action_model",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod actions",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod errors",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod form_flow",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod form_model",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod format",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod forms",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod inspect_links",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod inspect_tree",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod inspector",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod inspector_tabs",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod links",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod model",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod no_engine",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod overview",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod resource_rows",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod run_checks",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod run_form",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod run_model",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod sort",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod status",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod table_search",
        "containers keeps its parts as open flat files; close them or move them into part folders",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/explorer/mod.rs",
        "pub(crate) mod model",
        "model is open; close it and re-export a view model only where outside code names it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/explorer/mod.rs",
        "pub(crate) mod reducer",
        "reducer is open; close it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/explorer/mod.rs",
        "pub(crate) mod state",
        "state is open; close it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/modes/mod.rs",
        "pub(crate) mod model",
        "model is open; close it and re-export a view model only where outside code names it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/palette/mod.rs",
        "pub(crate) mod model",
        "model is open; close it and re-export a view model only where outside code names it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/quick_launch/mod.rs",
        "pub(crate) mod constants",
        "subscription.rs reads quick_launch::constants; move the constant to where it is used or to types.rs",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/quick_launch/mod.rs",
        "pub(crate) mod model",
        "model is open; close it and re-export a view model only where outside code names it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/quick_launch/mod.rs",
        "pub(crate) mod reducer",
        "reducer is open; close it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/quick_launch/mod.rs",
        "pub(crate) mod state",
        "state is open; close it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/settings/mod.rs",
        "pub(crate) mod model",
        "model is open; close it and re-export a view model only where outside code names it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/settings/mod.rs",
        "pub(crate) mod reducer",
        "reducer is open; close it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/settings/mod.rs",
        "pub(crate) mod state",
        "state is open; close it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/sidebar/mod.rs",
        "mod event",
        "sidebar closes event and re-exports its events; open event and drop the re-export",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/tabs/mod.rs",
        "pub(crate) mod model",
        "model is open; close it and re-export a view model only where outside code names it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/terminal_workspace/mod.rs",
        "pub(crate) mod model",
        "model is open; close it and re-export a view model only where outside code names it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/terminal_workspace/mod.rs",
        "pub(crate) mod reducer",
        "reducer is open; close it",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/terminal_workspace/mod.rs",
        "pub(crate) mod state",
        "state is open; close it",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/chrome/mod.rs",
        "event",
        "the events have two paths; import them through event:: and drop the re-export",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/containers/mod.rs",
        "event",
        "the events have two paths; import them through event:: and drop the re-export",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/explorer/mod.rs",
        "event",
        "the events have two paths; import them through event:: and drop the re-export",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/modes/mod.rs",
        "event",
        "the events have two paths; import them through event:: and drop the re-export",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/modes/mod.rs",
        "model",
        "re-exports the view model from the open model module; close model first",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/palette/mod.rs",
        "event",
        "the events have two paths; import them through event:: and drop the re-export",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/palette/mod.rs",
        "model",
        "re-exports the view model from the open model module; close model first",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/quick_launch/mod.rs",
        "event",
        "the events have two paths; import them through event:: and drop the re-export",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/quick_launch/mod.rs",
        "reducer",
        "re-exports XCtx from the open reducer module; close reducer first",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/settings/mod.rs",
        "event",
        "the events have two paths; import them through event:: and drop the re-export",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/tabs/mod.rs",
        "event",
        "the events have two paths; import them through event:: and drop the re-export",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/terminal_workspace/mod.rs",
        "event",
        "the events have two paths; import them through event:: and drop the re-export",
    ),
    Allowed::known(
        Rule::ReexportFromOpenModule,
        "widgets/terminal_workspace/mod.rs",
        "reducer",
        "re-exports XCtx from the open reducer module; close reducer first",
    ),
    Allowed::known(
        Rule::ViewImportsState,
        "widgets/quick_launch/view/context_menu.rs",
        "widgets::quick_launch::state",
        "the view reads state types; give it a view model",
    ),
    Allowed::known(
        Rule::ViewImportsState,
        "widgets/quick_launch/view/error_tab.rs",
        "widgets::quick_launch::state",
        "the view reads state types; give it a view model",
    ),
    Allowed::known(
        Rule::ViewImportsState,
        "widgets/quick_launch/view/secret_prompt.rs",
        "widgets::quick_launch::state",
        "the view reads state types; give it a view model",
    ),
    Allowed::known(
        Rule::ViewImportsState,
        "widgets/quick_launch/view/sidebar_tree.rs",
        "widgets::quick_launch::state",
        "the view reads state types; give it a view model",
    ),
    Allowed::known(
        Rule::ViewImportsState,
        "widgets/quick_launch/view/wizard_form.rs",
        "widgets::quick_launch::state",
        "the view reads state types; give it a view model",
    ),
    Allowed::known(
        Rule::TaskOrElementInPureFile,
        "widgets/containers/action_flow.rs",
        "Task",
        "a reducer part of the large containers widget; settle how large widgets split into parts first",
    ),
    Allowed::known(
        Rule::TaskOrElementInPureFile,
        "widgets/containers/form_flow.rs",
        "Task",
        "a reducer part of the large containers widget; settle how large widgets split into parts first",
    ),
    Allowed::known(
        Rule::TaskOrElementInPureFile,
        "widgets/containers/run_flow.rs",
        "Task",
        "a reducer part of the large containers widget; settle how large widgets split into parts first",
    ),
    Allowed::known(
        Rule::ServicesOutsideRoute,
        "app.rs",
        "widgets::terminal_workspace::services",
        "the shell session setup lives in terminal_workspace::services; move it where both callers may use it",
    ),
    Allowed::known(
        Rule::ServicesOutsideRoute,
        "router/settings.rs",
        "widgets::terminal_workspace::services",
        "the shell session setup lives in terminal_workspace::services; move it where both callers may use it",
    ),
    Allowed::known(
        Rule::ServicesOutsideRoute,
        "widgets/explorer/state.rs",
        "widgets::explorer::services",
        "a pure helper placed in services.rs; move it into a topic file",
    ),
    Allowed::known(
        Rule::ServicesOutsideRoute,
        "widgets/quick_launch/reducer.rs",
        "widgets::quick_launch::services",
        "a pure helper placed in services.rs; move it into a topic file",
    ),
    Allowed::known(
        Rule::ServicesOutsideRoute,
        "widgets/terminal_workspace/model.rs",
        "widgets::terminal_workspace::services",
        "a pure helper placed in services.rs; move it into a topic file",
    ),
    Allowed::known(
        Rule::StreamOutsideSubscription,
        "widgets/containers/logs.rs",
        "Subscription::run",
        "the stream is built in a topic file; move it into the widget's subscription.rs",
    ),
    Allowed::known(
        Rule::StreamOutsideSubscription,
        "widgets/explorer/watcher.rs",
        "Subscription::run",
        "the stream is built in a topic file; move it into the widget's subscription.rs",
    ),
    Allowed::known(
        Rule::MissingModeScreen,
        "view/containers.rs",
        "Containers",
        "view.rs still holds the screens of both modes",
    ),
    Allowed::known(
        Rule::MissingModeScreen,
        "view/terminal.rs",
        "Terminal",
        "view.rs still holds the screens of both modes",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/chrome/mod.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/chrome/reducer.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/explorer/mod.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/explorer/reducer.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/quick_launch/mod.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/quick_launch/reducer.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/settings/mod.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/settings/reducer.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/sidebar/mod.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/sidebar/reducer.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/tabs/mod.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/tabs/reducer.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/terminal_workspace/mod.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ReduceParameterName,
        "widgets/terminal_workspace/reducer.rs",
        "event",
        "the parameter is still named event",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "actions_vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "confirm_vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "form_vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "image_panel_vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "inspector_tabs_vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "inspector_vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "main_vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "row_menu_vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "run_form_vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "status_vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "tree_vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/explorer/mod.rs",
        "vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/palette/mod.rs",
        "vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/quick_launch/mod.rs",
        "vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/settings/mod.rs",
        "vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/sidebar/mod.rs",
        "vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/tabs/mod.rs",
        "vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/terminal_workspace/mod.rs",
        "vm",
        "the view model is built in mod.rs; build it with from_state in model.rs",
    ),
    Allowed::exception(
        Rule::RouteBranchesOnState,
        "router/command.rs",
        "route",
        "a command belongs to no widget: the pure AppCommand::is_available, which the palette list shares, decides whether it runs, and the route only asks it (reviewed by the owner)",
    ),
    Allowed::known(
        Rule::RouteBranchesOnState,
        "router/modes.rs",
        "focus_after_menu_change",
        "decides focus from the menu state before and after reduce; the reducer should report the menu opening and closing as effects (overlays and focus topic)",
    ),
    Allowed::known(
        Rule::RouteBranchesOnState,
        "router/tabs.rs",
        "handle_effect",
        "switches to Terminal mode when a tab shows in another mode; that decision belongs in a reducer",
    ),
];
