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
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod format",
        "view.rs formats containers values with it; give view.rs those values through the view model",
    ),
    Allowed::known(
        Rule::WidgetModuleVisibility,
        "widgets/containers/mod.rs",
        "pub(crate) mod no_engine",
        "view.rs names NoEngineCase to pick the no-engine screen; move the case into types.rs",
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
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "inspector_vm",
        "mod.rs already only calls a builder in a containers part file; turning it into from_state waits for the decision on large-widget parts",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "main_vm",
        "mod.rs already only calls a builder in a containers part file; turning it into from_state waits for the decision on large-widget parts",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "status_vm",
        "mod.rs already only calls a builder in a containers part file; turning it into from_state waits for the decision on large-widget parts",
    ),
    Allowed::known(
        Rule::ViewModelNotFromState,
        "widgets/containers/mod.rs",
        "tree_vm",
        "mod.rs already only calls a builder in a containers part file; turning it into from_state waits for the decision on large-widget parts",
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
