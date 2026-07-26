use crate::{
    ConfigError, TerminalAppearance, TerminalBehavior, TerminalBindings,
    TerminalFont, TerminalTheme,
};

/// Complete frontend-only configuration for a terminal widget.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerminalConfig {
    theme: TerminalTheme,
    font: TerminalFont,
    appearance: TerminalAppearance,
    behavior: TerminalBehavior,
    bindings: TerminalBindings,
}

impl TerminalConfig {
    /// Current terminal theme.
    pub fn theme(&self) -> &TerminalTheme {
        &self.theme
    }

    /// Current font settings.
    pub fn font(&self) -> &TerminalFont {
        &self.font
    }

    /// Current frame appearance.
    pub fn appearance(&self) -> &TerminalAppearance {
        &self.appearance
    }

    /// Current interaction policies.
    pub fn behavior(&self) -> &TerminalBehavior {
        &self.behavior
    }

    /// Current terminal-local bindings.
    pub fn bindings(&self) -> &TerminalBindings {
        &self.bindings
    }

    /// Create a configuration from independently validated parts.
    pub fn new(
        theme: TerminalTheme,
        font: TerminalFont,
        appearance: TerminalAppearance,
        behavior: TerminalBehavior,
        bindings: TerminalBindings,
    ) -> Self {
        Self {
            theme,
            font,
            appearance,
            behavior,
            bindings,
        }
    }

    /// Start a configuration builder from the validated defaults.
    pub fn builder() -> TerminalConfigBuilder {
        TerminalConfigBuilder::new()
    }
}

/// Ergonomic builder for replacing selected parts of the default config.
#[derive(Clone, Debug, Default)]
pub struct TerminalConfigBuilder {
    config: TerminalConfig,
}

impl TerminalConfigBuilder {
    /// Start with [`TerminalConfig::default`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the terminal paint theme.
    pub fn theme(mut self, theme: TerminalTheme) -> Self {
        self.config.theme = theme;
        self
    }

    /// Replace font and shaping settings.
    pub fn font(mut self, font: TerminalFont) -> Self {
        self.config.font = font;
        self
    }

    /// Replace frame and spacing settings.
    pub fn appearance(mut self, appearance: TerminalAppearance) -> Self {
        self.config.appearance = appearance;
        self
    }

    /// Replace terminal interaction policies.
    pub fn behavior(mut self, behavior: TerminalBehavior) -> Self {
        self.config.behavior = behavior;
        self
    }

    /// Replace terminal-local key bindings.
    pub fn bindings(mut self, bindings: TerminalBindings) -> Self {
        self.config.bindings = bindings;
        self
    }

    /// Finish the config; all accepted parts are already validated.
    pub fn build(self) -> Result<TerminalConfig, ConfigError> {
        Ok(self.config)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ConfigChange {
    pub(crate) repaint: bool,
    pub(crate) reflow: bool,
    pub(crate) interaction: bool,
}

impl ConfigChange {
    pub(crate) fn between(
        previous: &TerminalConfig,
        next: &TerminalConfig,
    ) -> Self {
        Self {
            repaint: previous.theme != next.theme
                || previous.appearance != next.appearance,
            reflow: previous.font != next.font,
            interaction: previous.behavior != next.behavior,
        }
    }

    pub(crate) fn needs_notify(self) -> bool {
        self.repaint || self.reflow || self.interaction
    }
}
